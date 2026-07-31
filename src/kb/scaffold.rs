//! `complai compliance scaffold <framework>`:按内置结构表生成控制项桩文件。
//!
//! 桩文件含 frontmatter(ID + 控制点短名 + 域)与空的正文模板,
//! 正文由人工摘录填充。已存在的文件不会被覆盖,以免破坏手工编辑。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use eyre::WrapErr;
use serde::Deserialize;

use crate::frontmatter;
use crate::kb::control::{ControlFrontmatter, ExcerptStatus};
use crate::kb::framework_dir;
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
    points: Vec<String>,
}

pub fn scaffold(framework: &str) -> eyre::Result<()> {
    match framework {
        "dengbao-2.0" => scaffold_dengbao(),
        other => eyre::bail!("未知框架 `{other}`:MVP 仅支持 scaffold `dengbao-2.0`"),
    }
}

fn scaffold_dengbao() -> eyre::Result<()> {
    let structure: FrameworkStructure = serde_yml::from_str(DENGBAO_2_0_STRUCTURE)
        .wrap_err("解析内置 dengbao-2.0 结构表失败")?;
    let dir = framework_dir(&structure.framework)?;
    let mut created = 0usize;
    let mut skipped = 0usize;

    for domain in &structure.domains {
        let domain_enum = Domain::from_str(&domain.key)
            .wrap_err_with(|| format!("解析控制域 `{}` 失败", domain.key))?;
        for category in &domain.categories {
            for (i, point) in category.points.iter().enumerate() {
                let control_id = format!("{}.{}", category.prefix, i + 1);
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
                    domain: domain_enum,
                    category: category.name.clone(),
                    control_id,
                    title: point.clone(),
                    levels: vec![structure.level],
                    tags: Vec::new(),
                    mappings: BTreeMap::new(),
                    expected_evidence: Vec::new(),
                    excerpt_status: ExcerptStatus::Empty,
                    last_reviewed: None,
                };
                let body = default_body(point);
                let content = frontmatter::serialize(&fm, &body)?;

                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .wrap_err_with(|| format!("创建目录 {} 失败", parent.display()))?;
                }
                fs::write(&path, content)
                    .wrap_err_with(|| format!("写入 {} 失败", path.display()))?;
                created += 1;
            }
        }
    }

    // 生成桩文件后立即构建索引,使 `compliance list`/`compliance show` 可用。
    crate::kb::build::build(&structure.framework)?;

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
