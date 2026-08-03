//! 证据库:证明控制项符合性的证据文件(截图/配置/策略文档/日志/记录)。
//!
//! 每条证据记录文件路径、sha256(可追溯)、关联的控制与事实。
//! 证据文件按控制点就近存放于 `evidence/<control_id>/`。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate};
use eyre::WrapErr;
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::model::ControlId;
use crate::project::project_root;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EvidenceType {
    #[serde(rename = "screenshot")]
    Screenshot,
    #[serde(rename = "config")]
    Config,
    #[serde(rename = "policy-doc")]
    PolicyDoc,
    #[serde(rename = "log")]
    Log,
    #[serde(rename = "record")]
    Record,
}

impl EvidenceType {
    pub fn parse(s: &str) -> eyre::Result<Self> {
        match s {
            "screenshot" => Ok(Self::Screenshot),
            "config" => Ok(Self::Config),
            "policy-doc" => Ok(Self::PolicyDoc),
            "log" => Ok(Self::Log),
            "record" => Ok(Self::Record),
            other => {
                eyre::bail!("未知证据类型 `{other}`(应为 screenshot/config/policy-doc/log/record)")
            }
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Screenshot => "screenshot",
            Self::Config => "config",
            Self::PolicyDoc => "policy-doc",
            Self::Log => "log",
            Self::Record => "record",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub file: String,
    pub sha256: String,
    #[serde(rename = "type")]
    pub kind: EvidenceType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub collected_at: NaiveDate,
    pub collector: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_controls: Vec<ControlId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_facts: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceIndex {
    #[serde(default)]
    pub evidence: BTreeMap<String, Evidence>,
}

fn index_path(root: &Path) -> PathBuf {
    root.join("evidence.yaml")
}

pub fn load_index(root: &Path) -> eyre::Result<EvidenceIndex> {
    let p = index_path(root);
    if !p.exists() {
        return Ok(EvidenceIndex::default());
    }
    let content = fs::read_to_string(&p).wrap_err("读 evidence.yaml 失败")?;
    serde_yml::from_str(&content).wrap_err("解析 evidence.yaml 失败")
}

pub(crate) fn save_index(root: &Path, index: &EvidenceIndex) -> eyre::Result<()> {
    let yaml = serde_yml::to_string(index).wrap_err("序列化证据索引失败")?;
    crate::storage::atomic_write(&index_path(root), yaml).wrap_err("写 evidence.yaml 失败")
}

fn next_evidence_id(index: &EvidenceIndex) -> String {
    let max = index
        .evidence
        .keys()
        .filter_map(|k| k.strip_prefix("EV-").and_then(|s| s.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    format!("EV-{:04}", max + 1)
}

pub fn add(
    file_path: &str,
    control_str: &str,
    kind_str: String,
    description: Option<String>,
) -> eyre::Result<()> {
    let _lock = crate::storage::WriteLock::acquire().wrap_err("锁定 Complai 写操作失败")?;
    let cid: ControlId = control_str
        .parse()
        .wrap_err_with(|| format!("`{control_str}` 不是合法控制 ID"))?;
    let root = project_root().wrap_err("定位当前项目失败")?;
    let matrix = crate::project::matrix::load(&root).wrap_err("加载控制矩阵失败")?;
    if !matrix.entries.contains_key(&cid) {
        eyre::bail!("控制 {cid} 不在当前项目矩阵中");
    }
    let mut index = load_index(&root).wrap_err("加载证据索引失败")?;
    let id = next_evidence_id(&index);

    let bytes = fs::read(file_path).wrap_err_with(|| format!("读取证据文件 {file_path} 失败"))?;
    let hash = sha2::Sha256::digest(&bytes);
    let sha: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
    let kind = EvidenceType::parse(&kind_str).wrap_err("解析证据类型失败")?;

    let fname = Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| eyre::eyre!("证据文件路径无有效文件名:{file_path}"))
        .wrap_err("定位证据文件名失败")?;
    // 证据 ID 进入物理文件名，使相同原始文件名的多次采集各自保留不可变副本。
    let control_directory = crate::paths::safe_path_component(&cid.control_id);
    let safe_filename = crate::paths::safe_path_component(fname);
    let rel = format!("evidence/{control_directory}/{id}-{safe_filename}");
    let dest = root.join(&rel);
    let ev = Evidence {
        id: id.clone(),
        file: rel,
        sha256: sha,
        kind,
        description: description.unwrap_or_default(),
        collected_at: Local::now().date_naive(),
        collector: "agent".to_string(),
        linked_controls: vec![cid],
        linked_facts: Vec::new(),
    };
    index.evidence.insert(id.clone(), ev);
    crate::storage::transaction(|| {
        crate::storage::atomic_write(&dest, &bytes)
            .wrap_err_with(|| format!("写入证据副本 {} 失败", dest.display()))?;
        save_index(&root, &index).wrap_err("保存证据索引失败")
    })
    .wrap_err("登记证据事务失败")?;

    println!("added evidence {id} for {control_str}");
    Ok(())
}

/// 让证据反向索引与矩阵关联保持一致；同一证据可支撑多个控制项。
pub(crate) fn link_control(root: &Path, id: &str, control: &ControlId) -> eyre::Result<()> {
    let mut index = load_index(root).wrap_err("加载证据索引失败")?;
    let evidence = index
        .evidence
        .get_mut(id)
        .ok_or_else(|| eyre::eyre!("证据 {id} 不存在"))
        .wrap_err("定位证据失败")?;
    if evidence.linked_controls.contains(control) {
        return Ok(());
    }
    evidence.linked_controls.push(control.clone());
    save_index(root, &index).wrap_err("保存证据反向关联失败")
}

pub fn list() -> eyre::Result<()> {
    let root = project_root().wrap_err("定位当前项目失败")?;
    let index = load_index(&root).wrap_err("加载证据索引失败")?;
    for evidence in index.evidence.values() {
        print_summary(evidence);
    }
    println!("\n{} evidence records", index.evidence.len());
    Ok(())
}

pub fn show(id: &str) -> eyre::Result<()> {
    let root = project_root().wrap_err("定位当前项目失败")?;
    let index = load_index(&root).wrap_err("加载证据索引失败")?;
    let evidence = index
        .evidence
        .get(id)
        .ok_or_else(|| eyre::eyre!("证据 {id} 不存在"))
        .wrap_err("定位证据失败")?;

    println!("id\t{}", evidence.id);
    println!("type\t{}", evidence.kind.as_str());
    println!("file\t{}", evidence.file);
    println!("sha256\t{}", evidence.sha256);
    println!("collected_at\t{}", evidence.collected_at);
    println!("collector\t{}", evidence.collector);
    println!("description\t{}", display_or_dash(&evidence.description));
    println!(
        "controls\t{}",
        if evidence.linked_controls.is_empty() {
            "-".to_string()
        } else {
            evidence
                .linked_controls
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    let linked_facts = if evidence.linked_facts.is_empty() {
        "-".to_string()
    } else {
        evidence.linked_facts.join(", ")
    };
    println!("facts\t{linked_facts}");
    Ok(())
}

pub fn find(control_str: &str) -> eyre::Result<()> {
    let control: ControlId = control_str
        .parse()
        .wrap_err_with(|| format!("`{control_str}` 不是合法控制 ID"))?;
    let root = project_root().wrap_err("定位当前项目失败")?;
    let index = load_index(&root).wrap_err("加载证据索引失败")?;
    let matches = index
        .evidence
        .values()
        .filter(|evidence| evidence.linked_controls.contains(&control))
        .collect::<Vec<_>>();
    for evidence in &matches {
        print_summary(evidence);
    }
    println!("\n{} evidence records related to {control}", matches.len());
    Ok(())
}

fn print_summary(evidence: &Evidence) {
    println!(
        "{}  [{}]  {}  {}",
        evidence.id,
        evidence.kind.as_str(),
        evidence.file,
        display_or_dash(&evidence.description)
    );
}

fn display_or_dash(value: &str) -> &str {
    if value.is_empty() { "-" } else { value }
}
