//! `complai compliance show` / `compliance list`:按需查询控制项,最小上下文。
//!
//! `list` 只读紧凑索引(ID + 标题 + 域),`show` 再按需加载单个控制正文。

use std::fs;
use std::path::Path;
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

    let path =
        crate::paths::join_stored_path(&dir, &entry.file).wrap_err("解析控制正文存储路径失败")?;
    let content = fs::read_to_string(path).wrap_err_with(|| format!("读取 {} 失败", entry.file))?;
    println!("{content}");
    Ok(())
}

/// 验证控制 ID 已存在于对应框架索引，供手工事实写入避免产生悬空引用。
pub(crate) fn ensure_control_exists(id: &ControlId) -> eyre::Result<()> {
    let framework = id.framework.as_str();
    let index =
        load_index(framework).wrap_err_with(|| format!("加载控制 {id} 所属框架索引失败"))?;
    if !index.controls.iter().any(|entry| entry.id == *id) {
        eyre::bail!("控制 {id} 不存在于框架索引");
    }
    Ok(())
}

pub fn list(framework: Option<&str>, domain: Option<&str>) -> eyre::Result<()> {
    let root = kb_root().wrap_err("解析知识库根目录失败")?;
    let compliance_root = root.join("compliance");
    if !compliance_root.exists() {
        eyre::bail!(
            "合规知识库目录不存在:{}(先通过 ingest 导入框架；等保 2.0 也可运行 `complai compliance scaffold dengbao-2.0`)",
            compliance_root.display()
        );
    }

    let frameworks: Vec<String> = match framework {
        Some(f) => vec![f.to_string()],
        None => discover_frameworks(&compliance_root).wrap_err("枚举合规框架失败")?,
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

fn discover_frameworks(compliance_root: &Path) -> eyre::Result<Vec<String>> {
    let entries = fs::read_dir(compliance_root)
        .wrap_err_with(|| format!("读取合规知识库目录 {} 失败", compliance_root.display()))?;
    let mut frameworks = Vec::new();
    for entry in entries {
        let entry = entry.wrap_err("读取合规知识库目录项失败")?;
        if entry
            .file_type()
            .wrap_err("读取合规知识库目录项类型失败")?
            .is_dir()
        {
            frameworks.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    frameworks.sort();
    Ok(frameworks)
}

#[cfg(test)]
mod tests {
    use super::discover_frameworks;

    #[test]
    fn framework_discovery_stays_inside_the_compliance_directory() {
        let root = tempfile::TempDir::new().expect("临时知识库可创建");
        let compliance = root.path().join("compliance");
        std::fs::create_dir_all(compliance.join("iso27001-2022")).expect("框架目录可创建");
        std::fs::create_dir_all(root.path().join("system/order-platform")).expect("系统目录可创建");

        assert_eq!(
            discover_frameworks(&compliance).expect("框架可枚举"),
            vec!["iso27001-2022"]
        );
    }
}
