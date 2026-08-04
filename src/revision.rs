//! 知识库内容版本计算。
//!
//! revision 只描述索引当前发布的内容，不依赖 Git、时间戳或本机绝对路径。
//! 索引文件、每个被索引正文的相对路径和原始字节共同参与摘要；因此同一快照
//! 在不同机器上得到相同结果，正文变化或路径变化都会产生新的 revision。

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use eyre::WrapErr;
use sha2::{Digest, Sha256};

use crate::compliance::control::ControlIndex;
use crate::system::fact::FactIndex;

const REVISION_DOMAIN: &[u8] = b"complai-kb-revision-v1";

pub(crate) fn framework(framework: &str) -> eyre::Result<String> {
    let root = crate::compliance::framework_dir(framework).wrap_err("解析框架知识库目录失败")?;
    let index_bytes = read_index(&root).wrap_err("读取框架知识库索引失败")?;
    let index: ControlIndex =
        serde_yml::from_slice(&index_bytes).wrap_err("解析框架知识库索引失败")?;
    if index.framework.as_str() != framework {
        eyre::bail!(
            "框架索引声明 '{}'，与目录框架 '{framework}' 不一致",
            index.framework
        );
    }
    let paths = index
        .controls
        .into_iter()
        .map(|entry| entry.file)
        .collect::<BTreeSet<_>>();
    digest_indexed_files(&root, &index_bytes, paths).wrap_err("计算框架知识库 revision 失败")
}

pub(crate) fn system(slug: &str) -> eyre::Result<String> {
    let root = crate::system::system_dir(slug).wrap_err("解析系统知识库目录失败")?;
    let index_bytes = read_index(&root).wrap_err("读取系统知识库索引失败")?;
    let index: FactIndex =
        serde_yml::from_slice(&index_bytes).wrap_err("解析系统知识库索引失败")?;
    let paths = index
        .facts
        .into_iter()
        .map(|entry| entry.file)
        .collect::<BTreeSet<_>>();
    digest_indexed_files(&root, &index_bytes, paths).wrap_err("计算系统知识库 revision 失败")
}

fn read_index(root: &Path) -> eyre::Result<Vec<u8>> {
    let path = root.join("index.yaml");
    fs::read(&path).wrap_err_with(|| format!("读取 {} 失败", path.display()))
}

fn digest_indexed_files(
    root: &Path,
    index_bytes: &[u8],
    relative_paths: BTreeSet<String>,
) -> eyre::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(REVISION_DOMAIN);
    hash_entry(&mut hasher, "index.yaml", index_bytes);

    for relative_path in relative_paths {
        let path = crate::paths::join_stored_path(root, &relative_path)
            .wrap_err("解析知识库正文路径失败")?;
        let bytes =
            fs::read(&path).wrap_err_with(|| format!("读取知识库正文 {} 失败", path.display()))?;
        hash_entry(&mut hasher, &relative_path, &bytes);
    }

    let digest = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("sha256:{digest}"))
}

/// 长度前缀避免路径与正文简单拼接时出现边界歧义。
fn hash_entry(hasher: &mut Sha256, relative_path: &str, bytes: &[u8]) {
    hash_bytes(hasher, relative_path.as_bytes());
    hash_bytes(hasher, bytes);
}

fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(
        u64::try_from(bytes.len())
            .expect("文件长度可表示为 u64")
            .to_be_bytes(),
    );
    hasher.update(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_is_order_independent_but_tracks_content_and_paths() {
        let directory = tempfile::TempDir::new().expect("临时知识库可创建");
        let root = directory.path();
        fs::write(root.join("a.md"), "alpha").expect("测试正文可写入");
        fs::write(root.join("b.md"), "beta").expect("测试正文可写入");

        let forward = BTreeSet::from(["a.md".to_string(), "b.md".to_string()]);
        let reverse = BTreeSet::from(["b.md".to_string(), "a.md".to_string()]);
        let initial = digest_indexed_files(root, b"index", forward).expect("revision 可计算");
        assert_eq!(
            initial,
            digest_indexed_files(root, b"index", reverse).expect("顺序不影响 revision")
        );

        fs::write(root.join("b.md"), "changed").expect("测试正文可更新");
        let changed = digest_indexed_files(
            root,
            b"index",
            BTreeSet::from(["a.md".to_string(), "b.md".to_string()]),
        )
        .expect("更新后的 revision 可计算");
        assert_ne!(initial, changed);

        fs::write(root.join("renamed.md"), "changed").expect("重命名测试正文可写入");
        let renamed = digest_indexed_files(
            root,
            b"index",
            BTreeSet::from(["a.md".to_string(), "renamed.md".to_string()]),
        )
        .expect("路径变化后的 revision 可计算");
        assert_ne!(changed, renamed, "相同正文的路径变化必须影响 revision");
    }

    #[test]
    fn revision_rejects_an_indexed_path_outside_the_kb() {
        let directory = tempfile::TempDir::new().expect("临时知识库可创建");
        let result = digest_indexed_files(
            directory.path(),
            b"index",
            BTreeSet::from(["../outside.md".to_string()]),
        );
        assert!(result.is_err());
    }
}
