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
            other => eyre::bail!(
                "未知证据类型 `{other}`(应为 screenshot/config/policy-doc/log/record)"
            ),
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

fn save_index(root: &Path, index: &EvidenceIndex) -> eyre::Result<()> {
    let yaml = serde_yml::to_string(index).wrap_err("序列化证据索引失败")?;
    fs::write(index_path(root), yaml).wrap_err("写 evidence.yaml 失败")
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
    let cid: ControlId = control_str
        .parse()
        .wrap_err_with(|| format!("`{control_str}` 不是合法控制 ID"))?;
    let root = project_root()?;

    let bytes = fs::read(file_path)
        .wrap_err_with(|| format!("读取证据文件 {file_path} 失败"))?;
    let hash = sha2::Sha256::digest(&bytes);
    let sha: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
    let kind = EvidenceType::parse(&kind_str)?;

    let fname = Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| eyre::eyre!("证据文件路径无有效文件名:{file_path}"))?;
    // 按控制点就近存放;control_id(如 8.1.4.1)无冒号,可直接作子目录。
    let rel = format!("evidence/{}/{}", cid.control_id, fname);
    let dest = root.join(&rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("创建目录 {} 失败", parent.display()))?;
    }
    fs::write(&dest, &bytes).wrap_err_with(|| format!("写入证据副本 {} 失败", dest.display()))?;

    let mut index = load_index(&root)?;
    let id = next_evidence_id(&index);
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
    save_index(&root, &index)?;

    println!("added evidence {id} for {control_str}");
    Ok(())
}
