//! 共享业务系统知识库:被审计系统的事实,按 system slug 存放,跨项目复用。
//!
//! 一个系统(如 order-platform)可被多个项目(等保、ISO)引用,系统知识只存一份。

pub mod fact;
pub mod init;

use std::path::PathBuf;

use eyre::WrapErr;

use crate::cli::SystemCommand;
use crate::paths::kb_root;

/// 系统知识库根:`<kb_root>/system`。
pub fn system_root() -> eyre::Result<PathBuf> {
    Ok(kb_root().wrap_err("解析知识库根目录失败")?.join("system"))
}

/// 某系统的目录:`<kb_root>/system/<slug>`。
pub fn system_dir(slug: &str) -> eyre::Result<PathBuf> {
    validate_slug(slug).wrap_err("校验 system slug 失败")?;
    Ok(system_root()
        .wrap_err("解析系统知识库根目录失败")?
        .join(slug))
}

/// slug 同时是稳定引用和物理目录名，因此只接受可移植的 ASCII kebab-case。
fn validate_slug(slug: &str) -> eyre::Result<()> {
    let valid = !slug.is_empty()
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        && slug
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && slug
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric);
    if !valid {
        eyre::bail!("system slug `{slug}` 只能包含 ASCII 字母、数字和中间连字符");
    }
    Ok(())
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
            let slug = resolve_slug(system.as_deref()).wrap_err("解析目标系统失败")?;
            fact::add(&slug, domain, title, control, kind, reference, body)
        }
        SystemCommand::Show { system, id } => {
            let slug = resolve_slug(system.as_deref()).wrap_err("解析目标系统失败")?;
            fact::show(&slug, &id)
        }
        SystemCommand::Find { system, control } => {
            let slug = resolve_slug(system.as_deref()).wrap_err("解析目标系统失败")?;
            fact::find(&slug, &control)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_slug;

    #[test]
    fn system_slug_is_a_single_portable_path_component() {
        for slug in ["order-platform", "system1", "A-2"] {
            validate_slug(slug).expect("合法 slug 应通过");
        }
        for slug in ["", "-system", "system-", "../system", "a/b", "系统"] {
            assert!(validate_slug(slug).is_err(), "{slug} 应被拒绝");
        }
    }
}
