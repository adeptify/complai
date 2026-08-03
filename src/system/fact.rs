//! 系统事实(fact):被审计系统的事实性描述,存于共享 `system/<slug>/`。
//!
//! 每个 fact 一个 Markdown+frontmatter 文件;`index.yaml` 含 display_name + 紧凑索引。
//! fact 可由 agent 解析文档生成、或用户直接补充(见 `source.type`)。

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use chrono::{Local, NaiveDate};
use eyre::WrapErr;
use serde::{Deserialize, Serialize};

use crate::frontmatter;
use crate::model::{ControlId, IngestMetadata};
use crate::system::system_dir;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FactSourceType {
    #[serde(rename = "doc")]
    Doc,
    #[serde(rename = "interview")]
    Interview,
    #[serde(rename = "scan")]
    Scan,
    #[serde(rename = "user")]
    User,
}

impl FactSourceType {
    pub fn parse(s: &str) -> eyre::Result<Self> {
        match s {
            "doc" => Ok(Self::Doc),
            "interview" => Ok(Self::Interview),
            "scan" => Ok(Self::Scan),
            "user" => Ok(Self::User),
            other => eyre::bail!("未知 fact 来源类型 `{other}`(应为 doc/interview/scan/user)"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum Confidence {
    #[serde(rename = "high")]
    High,
    #[default]
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum FactStatus {
    #[default]
    #[serde(rename = "current")]
    Current,
    #[serde(rename = "stale")]
    Stale,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactSource {
    #[serde(rename = "type")]
    pub kind: FactSourceType,
    #[serde(rename = "ref")]
    pub reference: String,
    pub collected_at: NaiveDate,
    pub collector: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactFrontmatter {
    pub id: String,
    pub domain: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub source: FactSource,
    #[serde(default)]
    pub confidence: Confidence,
    #[serde(default)]
    pub related_controls: Vec<ControlId>,
    #[serde(default)]
    pub status: FactStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingest: Option<IngestMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactIndexEntry {
    pub id: String,
    pub domain: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub related_controls: Vec<ControlId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_key: Option<String>,
    pub file: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactIndex {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub facts: Vec<FactIndexEntry>,
}

fn index_path(slug: &str) -> eyre::Result<PathBuf> {
    Ok(system_dir(slug)?.join("index.yaml"))
}

pub fn load_index(slug: &str) -> eyre::Result<FactIndex> {
    let p = index_path(slug)?;
    if !p.exists() {
        eyre::bail!(
            "系统 `{slug}` 的知识库不存在({});先 `complai system init {slug} --name <显示名>`",
            p.display()
        );
    }
    let content = fs::read_to_string(&p).wrap_err("读 system index 失败")?;
    serde_yml::from_str(&content).wrap_err("解析 system index 失败")
}

fn save_index(slug: &str, index: &FactIndex) -> eyre::Result<()> {
    let yaml = serde_yml::to_string(index).wrap_err("序列化 fact 索引失败")?;
    fs::write(index_path(slug)?, yaml).wrap_err("写 system index 失败")
}

fn next_fact_id(index: &FactIndex) -> String {
    let max = index
        .facts
        .iter()
        .filter_map(|f| {
            f.id.strip_prefix("SYS-F-")
                .and_then(|s| s.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);
    format!("SYS-F-{:04}", max + 1)
}

struct FactDraft {
    domain: String,
    title: String,
    body: String,
    control: Option<String>,
    kind: String,
    reference: Option<String>,
}

fn create_fact(slug: &str, index: &mut FactIndex, draft: &FactDraft) -> eyre::Result<String> {
    let id = next_fact_id(index);
    let kind = FactSourceType::parse(&draft.kind)?;
    let related_controls = match &draft.control {
        Some(c) => vec![
            c.parse()
                .wrap_err_with(|| format!("`{c}` 不是合法控制 ID"))?,
        ],
        None => vec![],
    };
    let today = Local::now().date_naive();
    let fm = FactFrontmatter {
        id: id.clone(),
        domain: draft.domain.clone(),
        title: draft.title.clone(),
        tags: Vec::new(),
        source: FactSource {
            kind,
            reference: draft
                .reference
                .clone()
                .unwrap_or_else(|| "manual".to_string()),
            collected_at: today,
            collector: "agent".to_string(),
        },
        confidence: Confidence::default(),
        related_controls: related_controls.clone(),
        status: FactStatus::default(),
        supersedes: Vec::new(),
        ingest: None,
    };
    let body_md = format!("# {}\n\n{}\n", draft.title, draft.body);
    let content = frontmatter::serialize(&fm, &body_md)?;

    let rel = format!("{}/{id}.md", draft.domain);
    let path = system_dir(slug)?.join(&rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("创建目录 {} 失败", parent.display()))?;
    }
    fs::write(&path, content).wrap_err_with(|| format!("写入 {} 失败", path.display()))?;

    index.facts.push(FactIndexEntry {
        id: id.clone(),
        domain: draft.domain.clone(),
        title: draft.title.clone(),
        tags: Vec::new(),
        related_controls,
        external_key: None,
        file: rel,
    });
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
pub fn add(
    slug: &str,
    domain: String,
    title: String,
    control: Option<String>,
    kind_str: String,
    reference: Option<String>,
    body_opt: Option<String>,
) -> eyre::Result<()> {
    let mut index = load_index(slug)?;
    let body = match body_opt {
        Some(b) => b,
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .wrap_err("读取 stdin(事实正文)失败")?;
            buf
        }
    };
    let draft = FactDraft {
        domain,
        title,
        body,
        control,
        kind: kind_str,
        reference,
    };
    let id = create_fact(slug, &mut index, &draft)?;
    save_index(slug, &index)?;
    println!("added fact {id} to system `{slug}`");
    Ok(())
}

pub fn show(slug: &str, id: &str) -> eyre::Result<()> {
    let index = load_index(slug)?;
    let entry = index
        .facts
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| eyre::eyre!("fact {id} 不存在于系统 `{slug}`"))?;
    let content = fs::read_to_string(system_dir(slug)?.join(&entry.file))
        .wrap_err_with(|| format!("读取 {} 失败", entry.file))?;
    println!("{content}");
    Ok(())
}

pub fn find(slug: &str, control_str: &str) -> eyre::Result<()> {
    let cid: ControlId = control_str
        .parse()
        .wrap_err_with(|| format!("`{control_str}` 不是合法控制 ID"))?;
    let index = load_index(slug)?;
    let mut found = 0usize;
    for e in &index.facts {
        if e.related_controls.iter().any(|c| c == &cid) {
            println!("{}  {}  {}", e.id, e.domain, e.title);
            found += 1;
        }
    }
    println!("\n{found} facts in system `{slug}` related to {cid}");
    Ok(())
}
