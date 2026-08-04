//! 项目层:每个备案项目 = 系统×框架绑定 + artifacts + 项目专属事实。
//!
//! 项目根以 `project.yaml` 标识(含 system + framework 引用),命令从 cwd 上溯查找。

pub mod evidence;
pub mod fact;
pub mod init;
pub mod matrix;

use std::path::{Path, PathBuf};

use eyre::WrapErr;
use serde::{Deserialize, Serialize};

use crate::cli::{EvidenceCommand, FactCommand, MatrixCommand, ProjectCommand};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub system: String,
    pub system_revision: String,
    pub framework: String,
    pub framework_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRevisionStatus {
    pub framework_revision: String,
    pub current_framework_revision: String,
    pub system_revision: String,
    pub current_system_revision: String,
}

impl ProjectRevisionStatus {
    pub fn is_current(&self) -> bool {
        self.framework_revision == self.current_framework_revision
            && self.system_revision == self.current_system_revision
    }
}

/// 从 cwd 上溯找含 `project.yaml` 的目录(或 `COMPLAI_PROJECT_DIR`)。
pub fn project_root() -> eyre::Result<PathBuf> {
    if let Ok(dir) = std::env::var("COMPLAI_PROJECT_DIR") {
        let p = PathBuf::from(&dir);
        if p.join("project.yaml").exists() {
            return Ok(p);
        }
        eyre::bail!("COMPLAI_PROJECT_DIR={dir} 下找不到 project.yaml");
    }
    let cwd = std::env::current_dir().wrap_err("获取当前目录失败")?;
    let mut dir: &Path = &cwd;
    loop {
        if dir.join("project.yaml").exists() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => eyre::bail!(
                "不在项目目录内(找不到 project.yaml);先 `complai project init <name> --system <slug> --framework <f>` 并 cd 进入,或设置 COMPLAI_PROJECT_DIR"
            ),
        }
    }
}

pub fn load_meta(root: &Path) -> eyre::Result<ProjectMeta> {
    let content =
        std::fs::read_to_string(root.join("project.yaml")).wrap_err("读 project.yaml 失败")?;
    serde_yml::from_str(&content).wrap_err("解析 project.yaml 失败")
}

pub(crate) fn save_meta(root: &Path, metadata: &ProjectMeta) -> eyre::Result<()> {
    let yaml = serde_yml::to_string(metadata).wrap_err("序列化 project.yaml 失败")?;
    crate::storage::atomic_write(&root.join("project.yaml"), yaml).wrap_err("写 project.yaml 失败")
}

pub fn revision_status(root: &Path) -> eyre::Result<ProjectRevisionStatus> {
    let _lock = crate::storage::WriteLock::acquire().wrap_err("锁定 Complai KB 读取失败")?;
    revision_status_unlocked(root)
}

fn revision_status_unlocked(root: &Path) -> eyre::Result<ProjectRevisionStatus> {
    let metadata = load_meta(root).wrap_err("加载项目元数据失败")?;
    let current_framework_revision =
        crate::revision::framework(&metadata.framework).wrap_err("计算当前框架 revision 失败")?;
    let current_system_revision =
        crate::revision::system(&metadata.system).wrap_err("计算当前系统 revision 失败")?;
    Ok(ProjectRevisionStatus {
        framework_revision: metadata.framework_revision,
        current_framework_revision,
        system_revision: metadata.system_revision,
        current_system_revision,
    })
}

pub(crate) fn ensure_revisions_current(root: &Path) -> eyre::Result<ProjectRevisionStatus> {
    let status = revision_status_unlocked(root).wrap_err("检查项目 KB revision 失败")?;
    if status.is_current() {
        return Ok(status);
    }

    let mut drift = Vec::new();
    if status.framework_revision != status.current_framework_revision {
        drift.push(format!(
            "framework {} -> {}",
            status.framework_revision, status.current_framework_revision
        ));
    }
    if status.system_revision != status.current_system_revision {
        drift.push(format!(
            "system {} -> {}",
            status.system_revision, status.current_system_revision
        ));
    }
    eyre::bail!(
        "项目引用的 KB 已变化（{}）；审阅变更后运行 `complai project sync`",
        drift.join(", ")
    )
}

/// 调用者持有全局锁时，把项目钉住的 revision 更新为当前 KB 内容。
pub(crate) fn refresh_revisions(root: &Path) -> eyre::Result<ProjectMeta> {
    let mut metadata = load_meta(root).wrap_err("加载项目元数据失败")?;
    let framework_revision =
        crate::revision::framework(&metadata.framework).wrap_err("计算当前框架 revision 失败")?;
    let system_revision =
        crate::revision::system(&metadata.system).wrap_err("计算当前系统 revision 失败")?;
    if metadata.framework_revision == framework_revision
        && metadata.system_revision == system_revision
    {
        return Ok(metadata);
    }
    metadata.framework_revision = framework_revision;
    metadata.system_revision = system_revision;
    save_meta(root, &metadata).wrap_err("保存项目 KB revision 失败")?;
    Ok(metadata)
}

/// 当前项目引用的 system slug(供 system 命令默认、matrix trace 用)。
pub fn current_system_slug() -> eyre::Result<String> {
    let root = project_root().wrap_err("定位当前项目失败")?;
    Ok(load_meta(&root).wrap_err("加载当前项目元数据失败")?.system)
}

pub fn run_project(cmd: ProjectCommand) -> eyre::Result<()> {
    match cmd {
        ProjectCommand::Init {
            name,
            system,
            framework,
            level,
        } => init::init(&name, &system, &framework, level),
        ProjectCommand::Show => show(),
        ProjectCommand::Sync => sync(),
    }
}

/// 给 agent 提供一个紧凑的项目路由包，避免 workflow 直接解析 `project.yaml`。
pub fn show() -> eyre::Result<()> {
    let _lock = crate::storage::WriteLock::acquire().wrap_err("锁定 Complai KB 读取失败")?;
    let root = project_root().wrap_err("定位当前项目失败")?;
    let metadata = load_meta(&root).wrap_err("加载当前项目元数据失败")?;
    let revisions = revision_status_unlocked(&root).wrap_err("检查当前项目 KB revision 失败")?;
    println!("name\t{}", metadata.name);
    println!("system\t{}", metadata.system);
    println!("system_revision\t{}", metadata.system_revision);
    println!(
        "system_current_revision\t{}",
        revisions.current_system_revision
    );
    println!(
        "system_revision_status\t{}",
        if revisions.system_revision == revisions.current_system_revision {
            "current"
        } else {
            "drifted"
        }
    );
    println!("framework\t{}", metadata.framework);
    println!("framework_revision\t{}", metadata.framework_revision);
    println!(
        "framework_current_revision\t{}",
        revisions.current_framework_revision
    );
    println!(
        "framework_revision_status\t{}",
        if revisions.framework_revision == revisions.current_framework_revision {
            "current"
        } else {
            "drifted"
        }
    );
    println!(
        "level\t{}",
        metadata
            .level
            .map_or_else(|| "-".to_string(), |level| level.to_string())
    );
    Ok(())
}

pub fn sync() -> eyre::Result<()> {
    let _lock = crate::storage::WriteLock::acquire().wrap_err("锁定 Complai 写操作失败")?;
    let root = project_root().wrap_err("定位当前项目失败")?;
    let before = load_meta(&root).wrap_err("加载当前项目元数据失败")?;
    let after = refresh_revisions(&root).wrap_err("同步项目 KB revision 失败")?;
    if before.framework_revision == after.framework_revision
        && before.system_revision == after.system_revision
    {
        println!("project KB revisions already current");
    } else {
        println!(
            "synced project KB revisions: framework {} -> {}, system {} -> {}",
            before.framework_revision,
            after.framework_revision,
            before.system_revision,
            after.system_revision
        );
    }
    Ok(())
}

pub fn run_matrix(cmd: MatrixCommand) -> eyre::Result<()> {
    match cmd {
        MatrixCommand::Show { status } => matrix::show(status.as_deref()),
        MatrixCommand::Set {
            control,
            status,
            gap,
            owner,
            remediation,
        } => matrix::set(
            &control,
            &status,
            matrix::MatrixSetOptions::from_optional(gap, owner, remediation),
        ),
        MatrixCommand::Link {
            control,
            evidence,
            fact,
            project_fact,
        } => matrix::link(&control, evidence, fact, project_fact),
        MatrixCommand::Trace { control } => matrix::trace(&control),
    }
}

pub fn run_evidence(cmd: EvidenceCommand) -> eyre::Result<()> {
    match cmd {
        EvidenceCommand::Add {
            file,
            control,
            kind,
            description,
        } => evidence::add(&file, &control, kind, description),
        EvidenceCommand::List => evidence::list(),
        EvidenceCommand::Show { id } => evidence::show(&id),
        EvidenceCommand::Find { control } => evidence::find(&control),
    }
}

pub fn run_fact(cmd: FactCommand) -> eyre::Result<()> {
    match cmd {
        FactCommand::Add {
            kind,
            title,
            control,
            body,
        } => fact::add(kind, title, control, body),
        FactCommand::Show { id } => fact::show(&id),
        FactCommand::Find { control } => fact::find(&control),
    }
}
