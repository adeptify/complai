//! Markdown + YAML frontmatter 的解析与序列化。
//!
//! 文件形如:
//! ```text
//! ---
//! <yaml>
//! ---
//! <markdown body>
//! ```
//! 手写解析围栏以完全掌控边界情况,避免引入额外依赖的 API 摩擦;
//! YAML 部分仍交给 `serde_yml` 反序列化。

use serde::de::DeserializeOwned;
use serde::Serialize;

use eyre::WrapErr;

/// 解析后的 frontmatter 文档:结构化数据 + Markdown 正文。
pub struct Document<T> {
    pub data: T,
    pub body: String,
}

/// 从 Markdown 文本解析 frontmatter。
///
/// 要求文件首行即 `---`(允许 BOM),并在后续某行以 `---` 或 `...` 闭合。
pub fn parse<T: DeserializeOwned + 'static>(content: &str) -> eyre::Result<Document<T>> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);

    let first = content.lines().next().unwrap_or("");
    if first.trim() != "---" {
        eyre::bail!("frontmatter 缺失:文件须以 `---` 开头");
    }

    // 收集首行 `---` 之后、闭合围栏之前的所有行作为 YAML。
    let mut yaml_lines: Vec<&str> = Vec::new();
    let mut body_start: Option<usize> = None;
    for (idx, line) in content.lines().enumerate().skip(1) {
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "..." {
            body_start = Some(idx + 1);
            break;
        }
        yaml_lines.push(line);
    }
    let body_start = body_start.ok_or_else(|| eyre::eyre!("frontmatter 未以 `---` 闭合"))?;

    let yaml_str = yaml_lines.join("\n");
    let data: T = serde_yml::from_str(&yaml_str).wrap_err("解析 frontmatter YAML 失败")?;

    let body: String = content
        .lines()
        .skip(body_start)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_start_matches('\n')
        .to_string();

    Ok(Document { data, body })
}

/// 将结构化数据序列化为 frontmatter,后接 Markdown 正文。
///
/// 去掉 `serde_yml` 可能输出的 `---`/`...` 文档标记,再由我们自己包上围栏,
/// 以免出现 `---\n---\n` 双重围栏。
pub fn serialize<T: Serialize>(data: &T, body: &str) -> eyre::Result<String> {
    let yaml = serde_yml::to_string(data).wrap_err("序列化 frontmatter 失败")?;
    let yaml = yaml.strip_prefix("---\n").unwrap_or(&yaml);
    let yaml = yaml.strip_suffix("...\n").unwrap_or(yaml);
    let yaml = yaml.trim_end();

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(yaml);
    out.push_str("\n---\n\n");
    out.push_str(body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Data {
        name: String,
        level: u8,
        tags: Vec<String>,
    }

    #[test]
    fn serialize_parse_roundtrip() {
        let data = Data {
            name: "身份鉴别".into(),
            level: 3,
            tags: vec!["auth".into()],
        };
        let body = "# 身份鉴别\n\n一些正文。\n";
        let serialized = serialize(&data, body).unwrap();
        let doc: Document<Data> = parse(&serialized).unwrap();
        assert_eq!(doc.data, data);
        // parse 不保留尾部换行(生产中 body 从未被回读,仅 frontmatter 重要),按内容比较。
        assert_eq!(doc.body.trim_end(), body.trim_end());
    }

    #[test]
    fn parse_rejects_missing_frontmatter() {
        let res: eyre::Result<Document<Data>> = parse("# 仅正文\n无 frontmatter");
        assert!(res.is_err());
    }

    #[test]
    fn parse_rejects_unclosed_frontmatter() {
        let res: eyre::Result<Document<Data>> =
            parse("---\nname: x\nlevel: 1\ntags: []\n正文未闭合");
        assert!(res.is_err());
    }
}
