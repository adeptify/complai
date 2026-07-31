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
use crate::model::{ControlId, ProjectFactKind};
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFactIndexEntry {
    pub id: String,
    pub kind: ProjectFactKind,
    pub title: String,
    #[serde(default)]
    pub control: Option<ControlId>,
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

fn save_index(root: &Path, index: &ProjectFactIndex) -> eyre::Result<()> {
    let yaml = serde_yml::to_string(index).wrap_err("序列化项目事实索引失败")?;
    fs::write(index_path(root), yaml).wrap_err("写 facts/index.yaml 失败")
}

fn next_id(index: &ProjectFactIndex) -> String {
    let max = index
        .facts
        .iter()
        .filter_map(|f| f.id.strip_prefix("PROJ-F-").and_then(|s| s.parse::<u32>().ok()))
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
    let root = project_root()?;
    let kind = ProjectFactKind::parse(&kind_str)?;
    let mut index = load_index(&root)?;
    let id = next_id(&index);

    let control_id = match &control {
        Some(c) => Some(c.parse().wrap_err_with(|| format!("`{c}` 不是合法控制 ID"))?),
        None => None,
    };

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
    };
    let body_md = format!("# {title}\n\n{body}\n");
    let content = frontmatter::serialize(&fm, &body_md)?;

    let rel = format!("{}/{id}.md", kind.as_str());
    let path = root.join("facts").join(&rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).wrap_err_with(|| format!("创建目录 {} 失败", parent.display()))?;
    }
    fs::write(&path, content).wrap_err_with(|| format!("写入 {} 失败", path.display()))?;

    index.facts.push(ProjectFactIndexEntry {
        id: id.clone(),
        kind,
        title,
        control: control_id,
        file: rel,
    });
    save_index(&root, &index)?;
    println!("added project fact {id} [{}]", fm.kind.as_str());
    Ok(())
}

pub fn show(id: &str) -> eyre::Result<()> {
    let root = project_root()?;
    let index = load_index(&root)?;
    let entry = index
        .facts
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| eyre::eyre!("项目事实 {id} 不存在"))?;
    let content = fs::read_to_string(root.join("facts").join(&entry.file))
        .wrap_err_with(|| format!("读取 {} 失败", entry.file))?;
    println!("{content}");
    Ok(())
}

pub fn find(control_str: &str) -> eyre::Result<()> {
    let cid: ControlId = control_str
        .parse()
        .wrap_err_with(|| format!("`{control_str}` 不是合法控制 ID"))?;
    let root = project_root()?;
    let index = load_index(&root)?;
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
