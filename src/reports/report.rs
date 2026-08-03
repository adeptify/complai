//! `complai gen report`:把控制矩阵渲染为 Markdown 合规差距报告。
//!
//! 报告按 域 -> 类别 分组,列出每个控制项的状态/缺口/责任人/证据/事实引用。
//! 控制标题与分组结构从通用知识库索引取(矩阵只存 ID),仍是按需引用。

use std::collections::{BTreeMap, HashMap};
use std::fs;

use eyre::WrapErr;

use crate::compliance::control::ControlIndexEntry;
use crate::compliance::query::load_index;
use crate::model::{ControlId, ControlStatus};
use crate::project::matrix::MatrixEntry;
use crate::project::project_root;

/// 报告分组:域 -> 类别 -> (control_id, 标题, 矩阵条目)。
type GroupedEntries<'a> =
    BTreeMap<String, BTreeMap<String, Vec<(String, String, &'a MatrixEntry)>>>;

pub fn generate() -> eyre::Result<()> {
    let _lock = crate::storage::WriteLock::acquire().wrap_err("锁定 Complai 写操作失败")?;
    let root = project_root().wrap_err("定位当前项目失败")?;
    let matrix = crate::project::matrix::load(&root).wrap_err("加载控制矩阵失败")?;
    let framework = matrix.framework.as_str().to_string();
    let kb_index =
        load_index(&framework).wrap_err("加载知识库索引失败(先 complai compliance build)")?;

    // 控制标题/域/类别从 KB 索引取;矩阵只有 ID。
    let kb_by_id: HashMap<ControlId, &ControlIndexEntry> = kb_index
        .controls
        .iter()
        .map(|e| (e.id.clone(), e))
        .collect();

    // 按 domain -> category 分组。
    let mut groups: GroupedEntries = BTreeMap::new();
    let mut missing = 0usize;
    for (id, entry) in &matrix.entries {
        match kb_by_id.get(id) {
            Some(e) => {
                groups
                    .entry(e.domain.as_str().to_string())
                    .or_default()
                    .entry(e.category.clone())
                    .or_default()
                    .push((id.control_id.clone(), e.title.clone(), entry));
            }
            None => missing += 1,
        }
    }

    let total = matrix.entries.len();
    let gaps = matrix
        .entries
        .values()
        .filter(|e| e.status == ControlStatus::Gap)
        .count();
    let met = matrix
        .entries
        .values()
        .filter(|e| e.status == ControlStatus::Met)
        .count();
    let partial = matrix
        .entries
        .values()
        .filter(|e| e.status == ControlStatus::Partial)
        .count();
    let na = matrix
        .entries
        .values()
        .filter(|e| e.status == ControlStatus::Na)
        .count();
    let unassessed = matrix
        .entries
        .values()
        .filter(|e| e.status == ControlStatus::Unassessed)
        .count();

    let mut out = String::new();
    out.push_str("# 合规差距报告\n\n");
    out.push_str(&format!("- 框架: {}\n", matrix.framework));
    if let Some(level) = matrix.level {
        out.push_str(&format!("- 等级: {level}\n"));
    }
    out.push_str(&format!("- 范围: {}\n", matrix.scope.systems.join(", ")));
    out.push_str(&format!(
        "- 统计: 共 {total} 项 | 未评估 {unassessed} | 满足 {met} | 部分满足 {partial} | 缺口 {gaps} | 不适用 {na}\n"
    ));
    if missing > 0 {
        out.push_str(&format!("- ⚠ {missing} 个矩阵控制项在知识库索引中缺失\n"));
    }
    out.push('\n');

    for (domain, cats) in &groups {
        out.push_str(&format!("## {domain}\n\n"));
        for (category, entries) in cats {
            out.push_str(&format!("### {category}\n\n"));
            for (cid, title, entry) in entries {
                out.push_str(&format!("- **{cid} {title}** [{}]", entry.status));
                if !entry.owner.is_empty() {
                    out.push_str(&format!(" · owner={}", entry.owner));
                }
                out.push('\n');
                if !entry.gap.is_empty() {
                    out.push_str(&format!("  - 缺口: {}\n", entry.gap));
                }
                if !entry.remediation.is_empty() {
                    out.push_str(&format!("  - 整改: {}\n", entry.remediation));
                }
                if !entry.evidence.is_empty() {
                    out.push_str(&format!("  - 证据: {}\n", entry.evidence.join(", ")));
                }
                if !entry.facts.is_empty() {
                    out.push_str(&format!("  - 系统事实: {}\n", entry.facts.join(", ")));
                }
                if !entry.project_facts.is_empty() {
                    out.push_str(&format!(
                        "  - 项目事实: {}\n",
                        entry.project_facts.join(", ")
                    ));
                }
            }
            out.push('\n');
        }
    }

    let report_path = root.join("drafts").join("compliance-report.md");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).wrap_err("创建 drafts 目录失败")?;
    }
    crate::storage::atomic_write(&report_path, &out).wrap_err("写报告失败")?;
    println!(
        "wrote {} ({} controls, {} gaps)",
        report_path.display(),
        total,
        gaps
    );
    Ok(())
}
