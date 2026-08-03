//! `complai compliance show` / `compliance list`:按需查询控制项,最小上下文。
//!
//! `list` 只读紧凑索引(ID + 标题 + 域),`show` 再按需加载单个控制正文。

use std::fs;
use std::str::FromStr;

use eyre::WrapErr;

use crate::compliance::control::ControlIndex;
use crate::compliance::framework_dir;
use crate::model::{ControlId, Domain};
use crate::paths::kb_root;

/// 加载某框架的索引(若不存在,提示先 build)。
pub fn load_index(framework: &str) -> eyre::Result<ControlIndex> {
    let path = framework_dir(framework)
        .wrap_err("解析合规框架目录失败")?
        .join("index.yaml");
    let content = fs::read_to_string(&path).wrap_err_with(|| {
        format!(
            "索引不存在({});先通过 ingest 导入框架控制，或运行 `complai compliance build {framework}`",
            path.display()
        )
    })?;
    serde_yml::from_str(&content).wrap_err("解析索引失败")
}

pub fn show(id_str: &str) -> eyre::Result<()> {
    let id: ControlId = id_str
        .parse()
        .wrap_err_with(|| format!("`{id_str}` 不是合法控制 ID"))?;
    let framework = id.framework.as_str().to_string();
    let dir = framework_dir(&framework).wrap_err("解析合规框架目录失败")?;
    let index = load_index(&framework).wrap_err("加载合规框架索引失败")?;
    let entry = index
        .controls
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| eyre::eyre!("控制 {id} 不在索引中"))
        .wrap_err("定位控制失败")?;

    let content = fs::read_to_string(dir.join(&entry.file))
        .wrap_err_with(|| format!("读取 {} 失败", entry.file))?;
    println!("{content}");
    Ok(())
}

pub fn list(framework: Option<&str>, domain: Option<&str>) -> eyre::Result<()> {
    let root = kb_root().wrap_err("解析知识库根目录失败")?;
    if !root.exists() {
        eyre::bail!(
            "知识库根目录不存在:{}(先通过 ingest 导入框架；等保 2.0 也可运行 `complai compliance scaffold dengbao-2.0`)",
            root.display()
        );
    }

    let frameworks: Vec<String> = match framework {
        Some(f) => vec![f.to_string()],
        None => fs::read_dir(&root)
            .wrap_err("读取知识库根目录失败")?
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect(),
    };

    let domain_filter = match domain {
        Some(d) => Some(Domain::from_str(d).wrap_err_with(|| format!("控制域 `{d}` 无效"))?),
        None => None,
    };

    let mut total = 0usize;
    for fw in &frameworks {
        let index = match load_index(fw) {
            Ok(i) => i,
            Err(e) => {
                // 某框架未构建索引时跳过并提示,而非整体失败。
                eprintln!("跳过 {fw}: {e}");
                continue;
            }
        };
        for entry in &index.controls {
            if let Some(d) = &domain_filter
                && &entry.domain != d
            {
                continue;
            }
            println!(
                "{}  {}  {}/{}  [{}]",
                entry.id,
                entry.title,
                entry.domain.as_str(),
                entry.category,
                entry.excerpt_status.as_str(),
            );
            total += 1;
        }
    }
    println!("\n{total} controls");
    Ok(())
}
