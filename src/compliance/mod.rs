//! 合规知识库 (compliance KB):跨项目共享的框架控制项。

pub mod build;
pub mod control;
pub mod query;
pub mod scaffold;

use std::path::PathBuf;

use eyre::WrapErr;

use crate::cli::ComplianceCommand;

/// 某个框架的目录,如 `<kb_root>/compliance/dengbao-2.0`。
pub fn framework_dir(framework: &str) -> eyre::Result<PathBuf> {
    validate_framework_name(framework).wrap_err("校验框架名失败")?;
    Ok(crate::paths::kb_root()
        .wrap_err("解析知识库根目录失败")?
        .join("compliance")
        .join(framework))
}

/// 框架名同时作为稳定 ID 和目录名，只接受跨平台一致的可移植字符。
fn validate_framework_name(framework: &str) -> eyre::Result<()> {
    let valid = !framework.is_empty()
        && framework
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && framework
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && framework
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric);
    if !valid {
        eyre::bail!("框架名 `{framework}` 只能包含 ASCII 字母、数字及中间的 `-`、`_`、`.`");
    }
    Ok(())
}

/// 分发 `compliance` 子命令。
pub fn run(cmd: ComplianceCommand) -> eyre::Result<()> {
    match cmd {
        ComplianceCommand::Scaffold { framework } => scaffold::scaffold(&framework),
        ComplianceCommand::Build { framework } => build::build(&framework),
        ComplianceCommand::Show { id } => query::show(&id),
        ComplianceCommand::List { framework, domain } => {
            query::list(framework.as_deref(), domain.as_deref())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_framework_name;

    #[test]
    fn framework_name_is_a_portable_path_component() {
        for framework in ["dengbao-2.0", "iso27001_2022", "NIST.CSF-2"] {
            validate_framework_name(framework).expect("合法框架名应通过");
        }
        for framework in [
            "",
            ".hidden",
            "framework-",
            "../other",
            "a/b",
            "a\\b",
            "等保",
        ] {
            assert!(
                validate_framework_name(framework).is_err(),
                "{framework} 应被拒绝"
            );
        }
    }
}
