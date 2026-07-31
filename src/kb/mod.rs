//! 合规知识库 (compliance KB):跨项目共享的框架控制项。

pub mod build;
pub mod control;
pub mod ingest;
pub mod query;
pub mod scaffold;

use std::path::PathBuf;

use eyre::WrapErr;

use crate::cli::ComplianceCommand;

/// 知识库根目录。
///
/// 优先用 `COMPLAI_KB_DIR` 环境变量(便于测试指向 tmp/),
/// 否则回退到 `~/.complai/kb`。
pub fn kb_root() -> eyre::Result<PathBuf> {
    if let Ok(dir) = std::env::var("COMPLAI_KB_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var("HOME")
        .wrap_err("HOME 未设置(可用 COMPLAI_KB_DIR 覆盖知识库位置)")?;
    Ok(PathBuf::from(home).join(".complai").join("kb"))
}

/// 某个框架的目录,如 `<kb_root>/compliance/dengbao-2.0`。
pub fn framework_dir(framework: &str) -> eyre::Result<PathBuf> {
    Ok(kb_root()?.join("compliance").join(framework))
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
        ComplianceCommand::Ingest { framework, file } => ingest::ingest(&framework, &file),
    }
}
