//! 共享知识库根目录解析。
//!
//! `kb_root()` 返回所有知识库(compliance + system)的公共根目录,
//! 供 `compliance` 与 `system` 两个模块共用,避免二者互相依赖。

use std::path::PathBuf;

use eyre::WrapErr;
use sha2::Digest;

/// 知识库根目录。
///
/// 优先用 `COMPLAI_KB_DIR` 环境变量(便于测试指向 tmp/),
/// 否则回退到 `~/.complai/kb`。
pub fn kb_root() -> eyre::Result<PathBuf> {
    if let Ok(dir) = std::env::var("COMPLAI_KB_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var("HOME").wrap_err("HOME 未设置(可用 COMPLAI_KB_DIR 覆盖知识库位置)")?;
    Ok(PathBuf::from(home).join(".complai").join("kb"))
}

/// 将外部框架定义的 ID 转成稳定、单层的存储路径段。
///
/// 原始 ID 始终保留在结构化数据中；这个值仅用于物理文件布局。
/// 摘要后缀可防止不同 Unicode 或路径字符归一化后发生冲突。
pub(crate) fn safe_path_component(value: &str) -> String {
    let is_directly_safe = !matches!(value, "." | "..")
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character));
    if is_directly_safe && value.len() <= 120 {
        return value.to_string();
    }

    let normalized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || ".-_".contains(character) {
                character
            } else {
                '_'
            }
        })
        .take(80)
        .collect::<String>();
    let normalized = normalized.trim_matches(['.', '_', '-']);
    let prefix = if normalized.is_empty() {
        "item"
    } else {
        normalized
    };
    let digest = sha2::Sha256::digest(value.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{prefix}-{}", &hex[..12])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_path_component_never_preserves_path_separators() {
        let component = safe_path_component("../A/5.1");
        assert!(!component.contains('/'));
        assert!(!component.contains(".."));
        assert_eq!(component, safe_path_component("../A/5.1"));
    }
}
