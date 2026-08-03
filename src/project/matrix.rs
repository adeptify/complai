//! 控制矩阵:控制项 -> 状态 -> 系统事实/项目事实/证据引用 -> 责任人。
//!
//! 矩阵只存 ID 引用:控制定义在 compliance KB、系统事实在共享 system KB、
//! 项目事实与证据在项目内。`trace` 跨这三层按 ID 拉取聚焦包(最小上下文)。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::{Local, NaiveDate};
use eyre::WrapErr;
use serde::{Deserialize, Serialize};

use crate::model::{ControlId, ControlStatus, Framework, IngestMetadata};
use crate::project::{current_system_slug, project_root};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixEntry {
    pub status: ControlStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub owner: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    /// 系统事实(SYS-F),引用共享 system KB。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<String>,
    /// 项目事实(PROJ-F),引用本项目 `facts/`。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_facts: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gap: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub remediation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingest: Option<IngestMetadata>,
}

impl MatrixEntry {
    pub fn empty() -> Self {
        Self {
            status: ControlStatus::Na,
            owner: String::new(),
            evidence: Vec::new(),
            facts: Vec::new(),
            project_facts: Vec::new(),
            gap: String::new(),
            remediation: String::new(),
            last_updated: None,
            ingest: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    #[serde(default)]
    pub systems: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matrix {
    pub framework: Framework,
    pub level: u8,
    pub scope: Scope,
    #[serde(default)]
    pub entries: BTreeMap<ControlId, MatrixEntry>,
}

fn matrix_path(root: &Path) -> PathBuf {
    root.join("matrix.yaml")
}

pub fn load(root: &Path) -> eyre::Result<Matrix> {
    let content = fs::read_to_string(matrix_path(root)).wrap_err("读 matrix.yaml 失败")?;
    serde_yml::from_str(&content).wrap_err("解析 matrix.yaml 失败")
}

pub(crate) fn save(root: &Path, matrix: &Matrix) -> eyre::Result<()> {
    let yaml = serde_yml::to_string(matrix).wrap_err("序列化 matrix 失败")?;
    fs::write(matrix_path(root), yaml).wrap_err("写 matrix.yaml 失败")
}

pub fn show(status_filter: Option<&str>) -> eyre::Result<()> {
    let root = project_root()?;
    let matrix = load(&root)?;
    let filter = match status_filter {
        Some(s) => Some(
            ControlStatus::from_str(s)
                .wrap_err_with(|| format!("未知状态 `{s}`(应为 met/partial/gap/na)"))?,
        ),
        None => None,
    };

    let mut shown = 0usize;
    for (id, entry) in &matrix.entries {
        if let Some(f) = filter
            && entry.status != f
        {
            continue;
        }
        println!(
            "{}  [{}]  owner={}  ev={} sf={} pf={}  gap: {}",
            id,
            entry.status,
            if entry.owner.is_empty() {
                "-"
            } else {
                entry.owner.as_str()
            },
            entry.evidence.len(),
            entry.facts.len(),
            entry.project_facts.len(),
            if entry.gap.is_empty() {
                "(无)"
            } else {
                entry.gap.as_str()
            },
        );
        shown += 1;
    }
    println!("\n{shown} entries");
    Ok(())
}

pub fn set(
    control_str: &str,
    status_str: &str,
    gap: Option<String>,
    owner: Option<String>,
) -> eyre::Result<()> {
    let cid: ControlId = control_str
        .parse()
        .wrap_err_with(|| format!("`{control_str}` 不是合法控制 ID"))?;
    let status = ControlStatus::from_str(status_str)
        .wrap_err_with(|| format!("未知状态 `{status_str}`(应为 met/partial/gap/na)"))?;
    let root = project_root()?;
    let mut matrix = load(&root)?;
    {
        let entry = matrix
            .entries
            .entry(cid.clone())
            .or_insert_with(MatrixEntry::empty);
        entry.status = status;
        if let Some(g) = gap {
            entry.gap = g;
        }
        if let Some(o) = owner {
            entry.owner = o;
        }
        entry.last_updated = Some(Local::now().date_naive());
    }
    save(&root, &matrix)?;
    println!("set {cid} -> {status}");
    Ok(())
}

pub fn link(
    control_str: &str,
    evidence: Option<String>,
    fact: Option<String>,
    project_fact: Option<String>,
) -> eyre::Result<()> {
    if evidence.is_none() && fact.is_none() && project_fact.is_none() {
        eyre::bail!("至少需要 --evidence / --fact / --project-fact 之一");
    }
    let cid: ControlId = control_str
        .parse()
        .wrap_err_with(|| format!("`{control_str}` 不是合法控制 ID"))?;
    let root = project_root()?;
    let mut matrix = load(&root)?;
    {
        let entry = matrix
            .entries
            .entry(cid.clone())
            .or_insert_with(MatrixEntry::empty);
        if let Some(ev) = &evidence
            && !entry.evidence.contains(ev)
        {
            entry.evidence.push(ev.clone());
        }
        if let Some(f) = &fact
            && !entry.facts.contains(f)
        {
            entry.facts.push(f.clone());
        }
        if let Some(pf) = &project_fact
            && !entry.project_facts.contains(pf)
        {
            entry.project_facts.push(pf.clone());
        }
        entry.last_updated = Some(Local::now().date_naive());
    }
    save(&root, &matrix)?;
    println!("linked to {cid}: evidence={evidence:?} fact={fact:?} project_fact={project_fact:?}");
    Ok(())
}

/// 聚合控制正文(compliance KB)+ 系统事实(共享 system KB)+ 项目事实 + 证据 + 矩阵状态。
pub fn trace(control_str: &str) -> eyre::Result<()> {
    let cid: ControlId = control_str
        .parse()
        .wrap_err_with(|| format!("`{control_str}` 不是合法控制 ID"))?;
    let root = project_root()?;
    let matrix = load(&root)?;
    let entry = matrix
        .entries
        .get(&cid)
        .ok_or_else(|| eyre::eyre!("控制 {cid} 不在矩阵中"))?;

    // 1) 控制正文(compliance KB)。
    let framework = matrix.framework.as_str().to_string();
    let kb_dir = crate::compliance::framework_dir(&framework)?;
    let kb_index = crate::compliance::query::load_index(&framework)?;
    let ctl = kb_index
        .controls
        .iter()
        .find(|e| e.id == cid)
        .ok_or_else(|| eyre::eyre!("控制 {cid} 不在知识库索引中"))?;
    let control_body = fs::read_to_string(kb_dir.join(&ctl.file))
        .wrap_err_with(|| format!("读取控制正文 {} 失败", ctl.file))?;
    println!("=== 控制正文 ===\n{control_body}");

    // 2) 系统事实(共享 system KB,按项目引用的 system slug)。
    let slug = current_system_slug()?;
    let sys_index = crate::system::fact::load_index(&slug)?;
    println!("=== 系统事实 ({}) ===", entry.facts.len());
    for fid in &entry.facts {
        match sys_index.facts.iter().find(|e| &e.id == fid) {
            Some(f) => println!("{}  {}  {}", f.id, f.domain, f.title),
            None => println!("{fid}  (系统 `{slug}` 索引中缺失)"),
        }
    }

    // 3) 项目事实(项目 facts/)。
    let pf_index = crate::project::fact::load_index(&root)?;
    println!("=== 项目事实 ({}) ===", entry.project_facts.len());
    for pid in &entry.project_facts {
        match pf_index.facts.iter().find(|e| &e.id == pid) {
            Some(f) => println!("{}  [{}]  {}", f.id, f.kind.as_str(), f.title),
            None => println!("{pid}  (项目事实索引中缺失)"),
        }
    }

    // 4) 证据(项目)。
    let ev_index = crate::project::evidence::load_index(&root)?;
    println!("=== 证据 ({}) ===", entry.evidence.len());
    for eid in &entry.evidence {
        match ev_index.evidence.get(eid) {
            Some(e) => println!(
                "{}  [{}]  {}  sha256={:.12}",
                e.id,
                e.kind.as_str(),
                e.file,
                e.sha256,
            ),
            None => println!("{eid}  (索引中缺失)"),
        }
    }

    // 5) 矩阵状态。
    println!("=== 矩阵状态 ===");
    println!(
        "status={} owner={} gap={}",
        entry.status,
        if entry.owner.is_empty() {
            "-"
        } else {
            entry.owner.as_str()
        },
        if entry.gap.is_empty() {
            "(无)"
        } else {
            entry.gap.as_str()
        },
    );
    Ok(())
}
