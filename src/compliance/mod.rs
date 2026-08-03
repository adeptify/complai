//! 合规知识库 (compliance KB):跨项目共享的框架控制项。

pub mod build;
pub mod control;
pub mod query;
pub mod scaffold;

use std::path::{Component, Path, PathBuf};

use eyre::WrapErr;

use crate::cli::ComplianceCommand;

/// 某个框架的目录,如 `<kb_root>/compliance/dengbao-2.0`。
pub fn framework_dir(framework: &str) -> eyre::Result<PathBuf> {
    let mut components = Path::new(framework).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        eyre::bail!("框架名 `{framework}` 必须是单个安全的路径段");
    }
    Ok(crate::paths::kb_root()
        .wrap_err("解析知识库根目录失败")?
        .join("compliance")
        .join(framework))
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
