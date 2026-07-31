//! `complai kb ingest <framework> <file>`:把批量摘录笔记拆成各控制文件。
//!
//! 笔记格式:每块以 `@@ <control-id>` 起始(行首 `@@` 加空白),后续行直到下一块
//! 为该控制的正文。已有控制桩的 frontmatter 保留,正文替换为摘录,
//! `excerpt_status` 置 `partial`、`last_reviewed` 置今日,随后重建索引。

use std::fs;

use chrono::Local;
use eyre::WrapErr;

use crate::frontmatter;
use crate::kb::control::{ControlFrontmatter, ExcerptStatus};
use crate::kb::{build, framework_dir, query::load_index};
use crate::model::ControlId;

pub fn ingest(framework: &str, file: &str) -> eyre::Result<()> {
    let dir = framework_dir(framework)?;
    let content = fs::read_to_string(file).wrap_err_with(|| format!("读取笔记 {file} 失败"))?;
    let blocks = parse_notes(&content);
    if blocks.is_empty() {
        eyre::bail!("笔记 {file} 中未找到任何 `@@ <control-id>` 块");
    }

    let index = load_index(framework).wrap_err("加载知识库索引失败(先 complai kb build)")?;

    let mut updated = 0usize;
    let mut skipped = 0usize;
    for (id_str, body) in &blocks {
        let cid: ControlId = match id_str.parse() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("跳过 `{id_str}`: {e}");
                skipped += 1;
                continue;
            }
        };
        let entry = match index.controls.iter().find(|e| e.id == cid) {
            Some(e) => e,
            None => {
                eprintln!("跳过 {cid}:知识库中无此控制(先 complai kb scaffold)");
                skipped += 1;
                continue;
            }
        };

        let path = dir.join(&entry.file);
        let existing = fs::read_to_string(&path)
            .wrap_err_with(|| format!("读取 {} 失败", path.display()))?;
        let mut doc = frontmatter::parse::<ControlFrontmatter>(&existing)
            .wrap_err_with(|| format!("解析 {} 失败", path.display()))?;
        doc.data.excerpt_status = ExcerptStatus::Partial;
        doc.data.last_reviewed = Some(Local::now().date_naive());
        let new_body = format!("# {}\n\n{}\n", entry.title, body.trim());
        let out = frontmatter::serialize(&doc.data, &new_body)?;
        fs::write(&path, out).wrap_err_with(|| format!("写入 {} 失败", path.display()))?;
        updated += 1;
    }

    build::build(framework)?;
    println!("ingested {updated} controls ({skipped} skipped) into {framework}");
    Ok(())
}

/// 若该行是 `@@ <control-id>` 分隔符,返回解析出的 id;否则返回 None。
///
/// 要求 `@@` 后紧跟空白或行尾,以免误吞 `@@@` 之类的 markdown 构造。
fn delim_id(line: &str) -> Option<String> {
    let rest = line.strip_prefix("@@")?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let id = rest.trim();
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

/// 解析笔记为 `(control_id_str, body)` 列表。
pub fn parse_notes(content: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_body: Vec<&str> = Vec::new();

    for line in content.lines() {
        if let Some(id) = delim_id(line) {
            if let Some(prev) = current_id.take() {
                blocks.push((prev, current_body.join("\n")));
                current_body.clear();
            }
            current_id = Some(id);
            continue;
        }
        if current_id.is_some() {
            current_body.push(line);
        }
    }
    if let Some(id) = current_id {
        blocks.push((id, current_body.join("\n")));
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_notes_splits_blocks() {
        let notes = "@@ dengbao-2.0:8.1.4.1\n## 要求摘要\n鉴别身份\n@@ dengbao-2.0:8.1.4.2\n## 要求摘要\n访问控制\n";
        let blocks = parse_notes(notes);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, "dengbao-2.0:8.1.4.1");
        assert!(blocks[0].1.contains("鉴别身份"));
        assert_eq!(blocks[1].0, "dengbao-2.0:8.1.4.2");
        assert!(blocks[1].1.contains("访问控制"));
    }

    #[test]
    fn parse_notes_ignores_at_at_at() {
        // `@@@` 不是分隔符(`@@` 后须跟空白)。
        let notes = "@@ dengbao-2.0:8.1.4.1\nbody\n@@@notadelim\nmore body\n";
        let blocks = parse_notes(notes);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].1.contains("@@@notadelim"));
    }

    #[test]
    fn delim_id_recognizes_forms() {
        assert_eq!(delim_id("@@ dengbao-2.0:8.1.4.1").as_deref(), Some("dengbao-2.0:8.1.4.1"));
        assert_eq!(delim_id("@@\tdengbao-2.0:8.1.4.1").as_deref(), Some("dengbao-2.0:8.1.4.1"));
        assert_eq!(delim_id("@@@x"), None);
        assert_eq!(delim_id("@@"), None);
        assert_eq!(delim_id("## 不是"), None);
    }
}
