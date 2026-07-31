//! `complai project init <name> --system <slug> --framework <f> --level <l>`:
//! 脚手架项目工作区。引用共享 system KB 与 compliance KB;若系统不存在则建空壳。
//! 从 compliance KB 索引预填矩阵为完整清单(每控制一条 `na`)。

use std::collections::BTreeMap;
use std::fs;

use eyre::WrapErr;

use crate::compliance::control::ControlIndex;
use crate::compliance::framework_dir;
use crate::model::Framework;
use crate::project::matrix::{Matrix, MatrixEntry, Scope};
use crate::project::ProjectMeta;
use crate::system::fact::FactIndex;
use crate::system::system_dir;

pub fn init(name: &str, system: &str, framework: &str, level: u8) -> eyre::Result<()> {
    let dir = std::path::Path::new(name);
    let project_name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(name);
    if dir.join("project.yaml").exists() {
        eyre::bail!("项目已存在:{}", dir.display());
    }

    // 系统 KB 不存在则建空壳(display_name=slug;可用 `system init` 改显示名)。
    let sys_dir = system_dir(system)?;
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
    let index_path = framework_dir(framework)?.join("index.yaml");
    if !index_path.exists() {
        eyre::bail!(
            "框架 {framework} 的知识库索引不存在({});先 `complai compliance scaffold {framework}`",
            index_path.display()
        );
    }
    let index_content = fs::read_to_string(&index_path)
        .wrap_err_with(|| format!("读取 {} 失败", index_path.display()))?;
    let index: ControlIndex =
        serde_yml::from_str(&index_content).wrap_err("解析框架索引失败")?;

    let mut entries = BTreeMap::new();
    for entry in &index.controls {
        entries.insert(entry.id.clone(), MatrixEntry::empty());
    }

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
        "initialized project `{project_name}` (system={system}, framework={framework}, level={level}, {} controls)",
        index.controls.len()
    );
    Ok(())
}
