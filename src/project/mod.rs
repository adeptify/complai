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
    pub framework: String,
    pub level: u8,
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

/// 当前项目引用的 system slug(供 system 命令默认、matrix trace 用)。
pub fn current_system_slug() -> eyre::Result<String> {
    let root = project_root()?;
    Ok(load_meta(&root)?.system)
}

pub fn run_project(cmd: ProjectCommand) -> eyre::Result<()> {
    match cmd {
        ProjectCommand::Init { name, system, framework, level } => {
            init::init(&name, &system, &framework, level)
        }
    }
}

pub fn run_matrix(cmd: MatrixCommand) -> eyre::Result<()> {
    match cmd {
        MatrixCommand::Show { status } => matrix::show(status.as_deref()),
        MatrixCommand::Set { control, status, gap, owner } => {
            matrix::set(&control, &status, gap, owner)
        }
        MatrixCommand::Link { control, evidence, fact, project_fact } => {
            matrix::link(&control, evidence, fact, project_fact)
        }
        MatrixCommand::Trace { control } => matrix::trace(&control),
    }
}

pub fn run_evidence(cmd: EvidenceCommand) -> eyre::Result<()> {
    match cmd {
        EvidenceCommand::Add { file, control, kind, description } => {
            evidence::add(&file, &control, kind, description)
        }
    }
}

pub fn run_fact(cmd: FactCommand) -> eyre::Result<()> {
    match cmd {
        FactCommand::Add { kind, title, control, body } => fact::add(kind, title, control, body),
        FactCommand::Show { id } => fact::show(&id),
        FactCommand::Find { control } => fact::find(&control),
    }
}
