//! 合规知识库 (compliance KB):跨项目共享的框架控制项。

pub mod build;
pub mod control;
pub mod ingest;
pub mod query;
pub mod scaffold;

use std::path::PathBuf;

use eyre::WrapErr;

use crate::cli::KbCommand;

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

/// 分发 `kb` 子命令。
pub fn run(cmd: KbCommand) -> eyre::Result<()> {
    match cmd {
        KbCommand::Scaffold { framework } => scaffold::scaffold(&framework),
        KbCommand::Build { framework } => build::build(&framework),
        KbCommand::Show { id } => query::show(&id),
        KbCommand::List { framework, domain } => {
            query::list(framework.as_deref(), domain.as_deref())
        }
        KbCommand::Ingest { framework, file } => ingest::ingest(&framework, &file),
    }
}
