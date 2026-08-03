//! `complai project init <name> --system <slug> --framework <f> [--level <l>]`:
//! 脚手架项目工作区。引用共享 system KB 与 compliance KB;若系统不存在则建空壳。
//! 从 compliance KB 索引预填矩阵为完整清单(每控制一条 `unassessed`)。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use eyre::WrapErr;

use crate::compliance::control::ControlIndex;
use crate::compliance::framework_dir;
use crate::model::Framework;
use crate::project::ProjectMeta;
use crate::project::matrix::{Matrix, MatrixEntry, Scope};
use crate::system::fact::FactIndex;
use crate::system::system_dir;

pub fn init(name: &str, system: &str, framework: &str, level: Option<u8>) -> eyre::Result<()> {
    let dir = std::path::Path::new(name);
    let project_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or(name);
    if dir.join("project.yaml").exists() {
        eyre::bail!("项目已存在:{}", dir.display());
    }

    // 系统 KB 不存在则建空壳(display_name=slug;可用 `system init` 改显示名)。
    let sys_dir = system_dir(system).wrap_err("解析系统知识库目录失败")?;
    if !sys_dir.join("index.yaml").exists() {
        fs::create_dir_all(&sys_dir).wrap_err("创建 system 目录失败")?;
        let sys_index = FactIndex {
            display_name: Some(system.to_string()),
            facts: Vec::new(),
        };
        let yaml = serde_yml::to_string(&sys_index).wrap_err("序列化 system index 失败")?;
        fs::write(sys_dir.join("index.yaml"), yaml).wrap_err("写 system index 失败")?;
    }

    // 从 compliance KB 索引预填矩阵。
    let index_path = framework_dir(framework)
        .wrap_err("解析合规框架目录失败")?
        .join("index.yaml");
    if !index_path.exists() {
        eyre::bail!(
            "框架 {framework} 的知识库索引不存在({});先通过 ingest 导入控制，等保 2.0 也可使用 scaffold",
            index_path.display()
        );
    }
    let index_content = fs::read_to_string(&index_path)
        .wrap_err_with(|| format!("读取 {} 失败", index_path.display()))?;
    let index: ControlIndex = serde_yml::from_str(&index_content).wrap_err("解析框架索引失败")?;

    // 级别是框架能力，不是所有项目的必填字段。只有一个候选级别时
    // 可安全推断（例如当前等保三级结构表）；多级别框架必须由用户选择。
    let declared_levels = index
        .controls
        .iter()
        .flat_map(|control| control.levels.iter().copied())
        .collect::<BTreeSet<_>>();
    let level = match level {
        Some(0) => eyre::bail!("框架级别必须大于 0"),
        Some(value) if !declared_levels.is_empty() && !declared_levels.contains(&value) => {
            eyre::bail!(
                "级别 {value} 不在框架 {framework} 声明的级别中（{}）",
                declared_levels
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Some(value) => Some(value),
        None if declared_levels.len() == 1 => declared_levels.first().copied(),
        None if declared_levels.len() > 1 => {
            eyre::bail!(
                "框架 {framework} 有多个级别（{}）；请使用 --level 明确选择",
                declared_levels
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        None => None,
    };

    let mut entries = BTreeMap::new();
    for entry in &index.controls {
        if control_applies_to_level(level, &entry.levels) {
            entries.insert(entry.id.clone(), MatrixEntry::empty());
        }
    }
    let control_count = entries.len();

    let scope = Scope {
        systems: vec![system.to_string()],
        boundary_ref: None,
    };
    let matrix = Matrix {
        framework: Framework(framework.to_string()),
        level,
        scope,
        entries,
    };

    fs::create_dir_all(dir.join("facts")).wrap_err("创建 facts 目录失败")?;
    fs::create_dir_all(dir.join("evidence")).wrap_err("创建 evidence 目录失败")?;
    fs::create_dir_all(dir.join("drafts")).wrap_err("创建 drafts 目录失败")?;

    let meta = ProjectMeta {
        name: project_name.to_string(),
        system: system.to_string(),
        framework: framework.to_string(),
        level,
    };
    let meta_yaml = serde_yml::to_string(&meta).wrap_err("序列化 project.yaml 失败")?;
    fs::write(dir.join("project.yaml"), meta_yaml).wrap_err("写 project.yaml 失败")?;
    fs::write(dir.join("facts/index.yaml"), "facts: []\n").wrap_err("写 facts/index.yaml 失败")?;
    fs::write(dir.join("evidence.yaml"), "evidence: {}\n").wrap_err("写 evidence.yaml 失败")?;
    let matrix_yaml = serde_yml::to_string(&matrix).wrap_err("序列化 matrix.yaml 失败")?;
    fs::write(dir.join("matrix.yaml"), matrix_yaml).wrap_err("写 matrix.yaml 失败")?;

    println!(
        "initialized project `{project_name}` (system={system}, framework={framework}, level={}, {} controls)",
        level.map_or_else(|| "-".to_string(), |value| value.to_string()),
        control_count
    );
    Ok(())
}

/// 没有声明级别的控制适用于所有项目；有级别的控制只在选中该级别时入矩阵。
fn control_applies_to_level(selected_level: Option<u8>, control_levels: &[u8]) -> bool {
    control_levels.is_empty() || selected_level.is_none_or(|level| control_levels.contains(&level))
}

#[cfg(test)]
mod tests {
    use super::control_applies_to_level;

    #[test]
    fn level_specific_controls_are_filtered() {
        assert!(control_applies_to_level(Some(1), &[]));
        assert!(control_applies_to_level(Some(1), &[1, 2]));
        assert!(!control_applies_to_level(Some(1), &[2, 3]));
        assert!(control_applies_to_level(None, &[2, 3]));
    }
}
