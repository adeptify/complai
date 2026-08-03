//! 项目专属事实(整改/例外/决策/发现/备注):项目过程的事实,与系统事实分开存。
//!
//! 存于项目 `facts/<kind>/PROJ-F-NNNN.md`;矩阵条目用 `project_facts` 引用。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate};
use eyre::WrapErr;
use serde::{Deserialize, Serialize};

use crate::frontmatter;
use crate::model::{ControlId, IngestMetadata, ProjectFactKind};
use crate::project::project_root;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFactFrontmatter {
    pub id: String,
    pub kind: ProjectFactKind,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub control: Option<ControlId>,
    pub created_at: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingest: Option<IngestMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFactIndexEntry {
    pub id: String,
    pub kind: ProjectFactKind,
    pub title: String,
    #[serde(default)]
    pub control: Option<ControlId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_key: Option<String>,
    pub file: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectFactIndex {
    #[serde(default)]
    pub facts: Vec<ProjectFactIndexEntry>,
}

fn index_path(root: &Path) -> PathBuf {
    root.join("facts").join("index.yaml")
}

pub fn load_index(root: &Path) -> eyre::Result<ProjectFactIndex> {
    let p = index_path(root);
    if !p.exists() {
        return Ok(ProjectFactIndex::default());
    }
    let content = fs::read_to_string(&p).wrap_err("读 facts/index.yaml 失败")?;
    serde_yml::from_str(&content).wrap_err("解析 facts/index.yaml 失败")
}

pub(crate) fn save_index(root: &Path, index: &ProjectFactIndex) -> eyre::Result<()> {
    let yaml = serde_yml::to_string(index).wrap_err("序列化项目事实索引失败")?;
    crate::storage::atomic_write(&index_path(root), yaml).wrap_err("写 facts/index.yaml 失败")
}

fn next_id(index: &ProjectFactIndex) -> String {
    let max = index
        .facts
        .iter()
        .filter_map(|f| {
            f.id.strip_prefix("PROJ-F-")
                .and_then(|s| s.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);
    format!("PROJ-F-{:04}", max + 1)
}

pub fn add(
    kind_str: String,
    title: String,
    control: Option<String>,
    body_opt: Option<String>,
) -> eyre::Result<()> {
    let _lock = crate::storage::WriteLock::acquire().wrap_err("锁定 Complai 写操作失败")?;
    let root = project_root().wrap_err("定位当前项目失败")?;
    let kind = ProjectFactKind::parse(&kind_str).wrap_err("解析项目事实类型失败")?;
    let mut index = load_index(&root).wrap_err("加载项目事实索引失败")?;
    let id = next_id(&index);

    let control_id = match &control {
        Some(c) => Some(
            c.parse()
                .wrap_err_with(|| format!("`{c}` 不是合法控制 ID"))?,
        ),
        None => None,
    };
    if let Some(control_id) = &control_id {
        let matrix = crate::project::matrix::load(&root).wrap_err("加载控制矩阵失败")?;
        if !matrix.entries.contains_key(control_id) {
            eyre::bail!("控制 {control_id} 不在当前项目矩阵中");
        }
    }

    let body = match body_opt {
        Some(b) => b,
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .wrap_err("读取 stdin(项目事实正文)失败")?;
            buf
        }
    };

    let today = Local::now().date_naive();
    let fm = ProjectFactFrontmatter {
        id: id.clone(),
        kind,
        title: title.clone(),
        tags: Vec::new(),
        control: control_id.clone(),
        created_at: today,
        ingest: None,
    };
    let body_md = format!("# {title}\n\n{body}\n");
    let content = frontmatter::serialize(&fm, &body_md).wrap_err("序列化项目事实失败")?;

    let rel = format!("{}/{id}.md", kind.as_str());
    let path = root.join("facts").join(&rel);
    crate::storage::transaction(|| {
        if let Some(parent) = path.parent() {
            crate::storage::create_dir_all(parent)
                .wrap_err_with(|| format!("创建目录 {} 失败", parent.display()))?;
        }
        crate::storage::atomic_write(&path, &content)
            .wrap_err_with(|| format!("写入 {} 失败", path.display()))?;

        index.facts.push(ProjectFactIndexEntry {
            id: id.clone(),
            kind,
            title: title.clone(),
            control: control_id.clone(),
            external_key: None,
            file: rel.clone(),
        });
        save_index(&root, &index).wrap_err("保存项目事实索引失败")
    })
    .wrap_err("新增项目事实事务失败")?;
    println!("added project fact {id} [{}]", fm.kind.as_str());
    Ok(())
}

pub fn show(id: &str) -> eyre::Result<()> {
    let root = project_root().wrap_err("定位当前项目失败")?;
    let index = load_index(&root).wrap_err("加载项目事实索引失败")?;
    let entry = index
        .facts
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| eyre::eyre!("项目事实 {id} 不存在"))
        .wrap_err("定位项目事实失败")?;
    let facts_dir = root.join("facts");
    let path = crate::paths::join_stored_path(&facts_dir, &entry.file)
        .wrap_err("解析项目事实存储路径失败")?;
    let content = fs::read_to_string(path).wrap_err_with(|| format!("读取 {} 失败", entry.file))?;
    println!("{content}");
    Ok(())
}

pub fn find(control_str: &str) -> eyre::Result<()> {
    let cid: ControlId = control_str
        .parse()
        .wrap_err_with(|| format!("`{control_str}` 不是合法控制 ID"))?;
    let root = project_root().wrap_err("定位当前项目失败")?;
    let index = load_index(&root).wrap_err("加载项目事实索引失败")?;
    let mut found = 0usize;
    for e in &index.facts {
        if e.control.as_ref() == Some(&cid) {
            println!("{}  [{}]  {}", e.id, e.kind.as_str(), e.title);
            found += 1;
        }
    }
    println!("\n{found} project facts related to {cid}");
    Ok(())
}

/// 项目事实当前只属于一个控制；未绑定事实可在首次 matrix link 时补上控制。
pub(crate) fn link_control(root: &Path, id: &str, control: &ControlId) -> eyre::Result<()> {
    let mut index = load_index(root).wrap_err("加载项目事实索引失败")?;
    let position = index
        .facts
        .iter()
        .position(|entry| entry.id == id)
        .ok_or_else(|| eyre::eyre!("项目事实 {id} 不存在"))
        .wrap_err("定位项目事实失败")?;
    if let Some(existing) = &index.facts[position].control
        && existing != control
    {
        eyre::bail!("项目事实 {id} 已绑定控制 {existing}，不能再关联到 {control}");
    }

    let facts_dir = root.join("facts");
    let path = crate::paths::join_stored_path(&facts_dir, &index.facts[position].file)
        .wrap_err("解析项目事实存储路径失败")?;
    let content = fs::read_to_string(&path)
        .wrap_err_with(|| format!("读取项目事实 {} 失败", path.display()))?;
    let mut document = frontmatter::parse::<ProjectFactFrontmatter>(&content)
        .wrap_err_with(|| format!("解析项目事实 {} 失败", path.display()))?;
    if let Some(existing) = &document.data.control
        && existing != control
    {
        eyre::bail!("项目事实正文 {id} 已绑定控制 {existing}，不能再关联到 {control}");
    }
    if index.facts[position].control.as_ref() == Some(control)
        && document.data.control.as_ref() == Some(control)
    {
        return Ok(());
    }

    index.facts[position].control = Some(control.clone());
    document.data.control = Some(control.clone());
    let serialized = frontmatter::serialize(&document.data, &document.body)
        .wrap_err("序列化项目事实关联失败")?;
    crate::storage::atomic_write(&path, serialized).wrap_err("保存项目事实正文关联失败")?;
    save_index(root, &index).wrap_err("保存项目事实索引关联失败")
}
