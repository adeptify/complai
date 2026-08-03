//! 共享业务系统知识库:被审计系统的事实,按 system slug 存放,跨项目复用。
//!
//! 一个系统(如 order-platform)可被多个项目(等保、ISO)引用,系统知识只存一份。

pub mod fact;
pub mod init;

use std::path::PathBuf;

use crate::cli::SystemCommand;
use crate::paths::kb_root;

/// 系统知识库根:`<kb_root>/system`。
pub fn system_root() -> eyre::Result<PathBuf> {
    Ok(kb_root()?.join("system"))
}

/// 某系统的目录:`<kb_root>/system/<slug>`。
pub fn system_dir(slug: &str) -> eyre::Result<PathBuf> {
    Ok(system_root()?.join(slug))
}

/// 解析操作目标系统:优先 `--system`,否则读当前项目 project.yaml 的 `system` 字段。
pub fn resolve_slug(explicit: Option<&str>) -> eyre::Result<String> {
    if let Some(s) = explicit {
        return Ok(s.to_string());
    }
    crate::project::current_system_slug()
}

pub fn run_system(cmd: SystemCommand) -> eyre::Result<()> {
    match cmd {
        SystemCommand::Init { slug, name } => init::init(&slug, name),
        SystemCommand::Add {
            system,
            domain,
            title,
            control,
            kind,
            reference,
            body,
        } => {
            let slug = resolve_slug(system.as_deref())?;
            fact::add(&slug, domain, title, control, kind, reference, body)
        }
        SystemCommand::Show { system, id } => {
            let slug = resolve_slug(system.as_deref())?;
            fact::show(&slug, &id)
        }
        SystemCommand::Find { system, control } => {
            let slug = resolve_slug(system.as_deref())?;
            fact::find(&slug, &control)
        }
    }
}
