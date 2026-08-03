//! `complai compliance scaffold <framework>`:按内置结构表生成控制项桩文件。
//!
//! 桩文件含 frontmatter(ID + 控制点短名 + 域)与空的正文模板,
//! 正文由人工摘录填充。已存在的文件不会被覆盖,以免破坏手工编辑。

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;

use eyre::WrapErr;
use serde::Deserialize;

use crate::compliance::control::{ControlFrontmatter, ExcerptStatus};
use crate::compliance::framework_dir;
use crate::frontmatter;
use crate::model::{ControlId, Domain, Framework};

/// 内置等保 2.0 结构表(只含 ID + 控制点短名,不含标准要求正文)。
const DENGBAO_2_0_STRUCTURE: &str = include_str!("../../data/dengbao-2.0.yaml");

#[derive(Debug, Deserialize)]
struct FrameworkStructure {
    framework: String,
    level: u8,
    domains: Vec<DomainStructure>,
}

#[derive(Debug, Deserialize)]
struct DomainStructure {
    key: String,
    categories: Vec<CategoryStructure>,
}

#[derive(Debug, Deserialize)]
struct CategoryStructure {
    prefix: String,
    name: String,
    points: Vec<PointStructure>,
}

#[derive(Debug, Deserialize)]
struct PointStructure {
    id: String,
    name: String,
}

pub fn scaffold(framework: &str) -> eyre::Result<()> {
    let _lock = crate::storage::WriteLock::acquire().wrap_err("锁定 Complai 写操作失败")?;
    crate::storage::transaction(|| match framework {
        "dengbao-2.0" => scaffold_dengbao(),
        other => eyre::bail!("未知框架 `{other}`:MVP 仅支持 scaffold `dengbao-2.0`"),
    })
    .wrap_err("生成框架控制库事务失败")
}

fn scaffold_dengbao() -> eyre::Result<()> {
    let structure: FrameworkStructure =
        serde_yml::from_str(DENGBAO_2_0_STRUCTURE).wrap_err("解析内置 dengbao-2.0 结构表失败")?;
    let dir = framework_dir(&structure.framework).wrap_err("解析合规框架目录失败")?;
    let mut created = 0usize;
    let mut skipped = 0usize;
    let mut control_ids = HashSet::new();

    for domain in &structure.domains {
        let domain_enum = Domain::from_str(&domain.key)
            .wrap_err_with(|| format!("解析控制域 `{}` 失败", domain.key))?;
        for category in &domain.categories {
            for point in &category.points {
                let control_id = point.id.clone();
                if !control_id.starts_with(&format!("{}.", category.prefix)) {
                    eyre::bail!("控制 ID {control_id} 不属于类别前缀 {}", category.prefix);
                }
                if !control_ids.insert(control_id.clone()) {
                    eyre::bail!("等保结构表存在重复控制 ID {control_id}");
                }
                let rel = Path::new(&domain.key)
                    .join(&category.name)
                    .join(format!("{control_id}.md"));
                let path = dir.join(&rel);

                // 已存在的文件不覆盖:保护已摘录的内容,支持增量 scaffold。
                if path.exists() {
                    skipped += 1;
                    continue;
                }

                let fm = ControlFrontmatter {
                    id: ControlId::new(&structure.framework, &control_id),
                    framework: Framework(structure.framework.clone()),
                    domain: domain_enum.clone(),
                    category: category.name.clone(),
                    control_id,
                    title: point.name.clone(),
                    levels: vec![structure.level],
                    tags: Vec::new(),
                    mappings: BTreeMap::new(),
                    expected_evidence: Vec::new(),
                    excerpt_status: ExcerptStatus::Empty,
                    last_reviewed: None,
                    ingest: None,
                };
                let body = default_body(&point.name);
                let content =
                    frontmatter::serialize(&fm, &body).wrap_err("序列化控制桩文件失败")?;

                if let Some(parent) = path.parent() {
                    crate::storage::create_dir_all(parent)
                        .wrap_err_with(|| format!("创建目录 {} 失败", parent.display()))?;
                }
                crate::storage::atomic_write(&path, content)
                    .wrap_err_with(|| format!("写入 {} 失败", path.display()))?;
                created += 1;
            }
        }
    }

    // 生成桩文件后立即构建索引,使 `compliance list`/`compliance show` 可用。
    crate::compliance::build::build_unlocked(&structure.framework)
        .wrap_err("构建等保框架索引失败")?;

    println!(
        "scaffolded {created} controls ({skipped} already existed) for {} into {}",
        structure.framework,
        dir.display()
    );
    Ok(())
}

/// 控制桩的正文模板:三个固定小节,提示人工摘录。
fn default_body(point: &str) -> String {
    format!(
        "# {point}\n\n\
         ## 要求摘要\n\n(待摘录:用自己的话概述该控制点的要求,不复录标准原文)\n\n\
         ## 实施指引\n\n(待摘录)\n\n\
         ## 常见缺陷\n\n(待摘录)\n"
    )
}
