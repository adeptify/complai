//! 文件型知识库的写入原语。
//!
//! Complai 的一次业务操作经常同时更新正文和索引。这里集中提供两个基础保证：
//! 单文件永远通过同目录临时文件原子替换；所有写操作通过 KB 根目录上的跨进程锁
//! 串行化。全局锁刻意牺牲少量并行度，换取当前文件存储模型下简单、可证明的顺序。

use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use eyre::WrapErr;
use fs2::FileExt;

thread_local! {
    static TRANSACTION: RefCell<Option<TransactionState>> = const { RefCell::new(None) };
}

struct TransactionState {
    seen: HashSet<PathBuf>,
    backups: Vec<FileBackup>,
    created_directories: Vec<PathBuf>,
}

struct FileBackup {
    path: PathBuf,
    previous: Option<Vec<u8>>,
}

/// 持有期间阻止另一个 Complai 进程修改任何 KB 或项目状态。
pub(crate) struct WriteLock {
    file: File,
}

impl WriteLock {
    pub(crate) fn acquire() -> eyre::Result<Self> {
        let root = crate::paths::kb_root().wrap_err("解析写锁目录失败")?;
        fs::create_dir_all(&root)
            .wrap_err_with(|| format!("创建写锁目录 {} 失败", root.display()))?;
        let path = root.join(".complai.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .wrap_err_with(|| format!("打开写锁文件 {} 失败", path.display()))?;
        file.lock_exclusive()
            .wrap_err_with(|| format!("获取写锁 {} 失败", path.display()))?;
        Ok(Self { file })
    }
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        // 关闭文件本身也会释放锁；显式 unlock 让正常路径更早释放。Drop 无法返回错误，
        // 因此这里只能忽略罕见的解锁失败，由操作系统在句柄关闭时完成清理。
        let _ = FileExt::unlock(&self.file);
    }
}

/// 对一组文件变更提供错误回滚。
///
/// 进程崩溃时仍由每次单文件原子替换保证文件不会截断；普通 I/O 或校验错误则会把
/// 本事务触及的所有已有文件恢复，并删除本事务新建的文件。
pub(crate) fn transaction<T>(operation: impl FnOnce() -> eyre::Result<T>) -> eyre::Result<T> {
    TRANSACTION
        .with(|slot| {
            if slot.borrow().is_some() {
                return Err(eyre::eyre!("不支持嵌套存储事务"));
            }
            *slot.borrow_mut() = Some(TransactionState {
                seen: HashSet::new(),
                backups: Vec::new(),
                created_directories: Vec::new(),
            });
            Ok(())
        })
        .wrap_err("启动存储事务失败")?;

    match operation() {
        Ok(value) => {
            TRANSACTION.with(|slot| {
                slot.borrow_mut().take();
            });
            Ok(value)
        }
        Err(operation_error) => {
            let rollback_result = rollback();
            match rollback_result {
                Ok(()) => Err(operation_error),
                Err(rollback_error) => {
                    Err(operation_error.wrap_err(format!("回滚存储事务失败: {rollback_error:#}")))
                }
            }
        }
    }
}

/// 在目标文件所在目录创建临时文件，完整落盘后原子替换目标。
pub(crate) fn atomic_write(path: &Path, bytes: impl AsRef<[u8]>) -> eyre::Result<()> {
    record_backup(path).wrap_err("记录原子写入前状态失败")?;
    let parent = path
        .parent()
        .ok_or_else(|| eyre::eyre!("目标路径没有父目录: {}", path.display()))
        .wrap_err("定位原子写入目录失败")?;
    create_dir_all(parent).wrap_err("创建原子写入目录失败")?;
    atomic_write_untracked(path, bytes.as_ref())
}

/// 递归创建目录，并让当前事务在失败时清理本次新建的空目录。
pub(crate) fn create_dir_all(path: &Path) -> eyre::Result<()> {
    let mut missing = Vec::new();
    let mut candidate = path;
    while !candidate.exists() {
        missing.push(candidate.to_path_buf());
        candidate = candidate
            .parent()
            .ok_or_else(|| eyre::eyre!("目录没有可创建的父路径: {}", path.display()))
            .wrap_err("定位待创建目录的父路径失败")?;
    }

    for directory in missing.into_iter().rev() {
        match fs::create_dir(&directory) {
            Ok(()) => record_created_directory(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error)
                    .wrap_err_with(|| format!("创建目录 {} 失败", directory.display()));
            }
        }
    }
    Ok(())
}

/// 删除文件并让当前事务在失败时恢复它。
pub(crate) fn remove_file_if_exists(path: &Path) -> eyre::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    record_backup(path).wrap_err("记录删除前状态失败")?;
    fs::remove_file(path).wrap_err_with(|| format!("删除 {} 失败", path.display()))
}

fn record_backup(path: &Path) -> eyre::Result<()> {
    TRANSACTION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(transaction) = slot.as_mut() else {
            return Ok(());
        };
        let path = path.to_path_buf();
        if !transaction.seen.insert(path.clone()) {
            return Ok(());
        }
        let previous = if path.exists() {
            Some(
                fs::read(&path)
                    .wrap_err_with(|| format!("备份事务文件 {} 失败", path.display()))?,
            )
        } else {
            None
        };
        transaction.backups.push(FileBackup { path, previous });
        Ok(())
    })
}

fn rollback() -> eyre::Result<()> {
    let (backups, created_directories) = TRANSACTION.with(|slot| {
        slot.borrow_mut().take().map_or_else(
            || (Vec::new(), Vec::new()),
            |transaction| (transaction.backups, transaction.created_directories),
        )
    });
    for backup in backups.into_iter().rev() {
        match backup.previous {
            Some(previous) => atomic_write_untracked(&backup.path, &previous)
                .wrap_err_with(|| format!("恢复 {} 失败", backup.path.display()))?,
            None if backup.path.exists() => fs::remove_file(&backup.path)
                .wrap_err_with(|| format!("删除事务新文件 {} 失败", backup.path.display()))?,
            None => {}
        }
    }
    for directory in created_directories.into_iter().rev() {
        match fs::remove_dir(&directory) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => {
                return Err(error)
                    .wrap_err_with(|| format!("清理事务目录 {} 失败", directory.display()));
            }
        }
    }
    Ok(())
}

fn record_created_directory(path: PathBuf) {
    TRANSACTION.with(|slot| {
        if let Some(transaction) = slot.borrow_mut().as_mut() {
            transaction.created_directories.push(path);
        }
    });
}

fn atomic_write_untracked(path: &Path, bytes: &[u8]) -> eyre::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| eyre::eyre!("目标路径没有父目录: {}", path.display()))
        .wrap_err("定位原子写入目录失败")?;
    fs::create_dir_all(parent)
        .wrap_err_with(|| format!("创建原子写入目录 {} 失败", parent.display()))?;

    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .wrap_err_with(|| format!("在 {} 创建临时文件失败", parent.display()))?;
    temporary
        .write_all(bytes)
        .wrap_err_with(|| format!("写临时文件 {} 失败", temporary.path().display()))?;
    temporary
        .flush()
        .wrap_err_with(|| format!("刷新临时文件 {} 失败", temporary.path().display()))?;
    temporary
        .as_file()
        .sync_all()
        .wrap_err_with(|| format!("同步临时文件 {} 失败", temporary.path().display()))?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .wrap_err_with(|| format!("原子替换 {} 失败", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_and_replaces_complete_files() {
        let directory = tempfile::TempDir::new().expect("临时目录可创建");
        let path = directory.path().join("state.yaml");

        atomic_write(&path, b"version: 1\n").expect("首次原子写入成功");
        atomic_write(&path, b"version: 2\n").expect("原子替换成功");

        let content = fs::read_to_string(path).expect("结果文件可读取");
        assert_eq!(content, "version: 2\n");
    }

    #[test]
    fn transaction_restores_replaced_and_new_files_after_error() {
        let directory = tempfile::TempDir::new().expect("临时目录可创建");
        let existing = directory.path().join("existing.yaml");
        let created_directory = directory.path().join("nested");
        let created = created_directory.join("created.yaml");
        atomic_write(&existing, b"original\n").expect("初始文件可写入");

        let result: eyre::Result<()> = transaction(|| {
            atomic_write(&existing, b"changed\n").wrap_err("修改已有文件失败")?;
            atomic_write(&created, b"new\n").wrap_err("创建新文件失败")?;
            eyre::bail!("模拟后续写入失败");
        });

        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(existing).expect("原文件可读取"),
            "original\n"
        );
        assert!(!created.exists());
        assert!(!created_directory.exists());
    }
}
