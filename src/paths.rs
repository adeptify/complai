//! 共享知识库根目录解析。
//!
//! `kb_root()` 返回所有知识库(compliance + system)的公共根目录,
//! 供 `compliance` 与 `system` 两个模块共用,避免二者互相依赖。

use std::path::PathBuf;

use eyre::WrapErr;

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
