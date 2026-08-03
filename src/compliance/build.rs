//! `complai compliance build <framework>`:遍历框架目录,生成紧凑索引 `index.yaml` 并校验。

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;

use eyre::WrapErr;
use walkdir::WalkDir;

use crate::compliance::control::{ControlFrontmatter, ControlIndex, ControlIndexEntry};
use crate::compliance::framework_dir;
use crate::frontmatter;
use crate::model::Framework;

pub fn build(framework: &str) -> eyre::Result<()> {
    let dir = framework_dir(framework).wrap_err("解析合规框架目录失败")?;
    if !dir.exists() {
        eyre::bail!(
            "框架目录不存在:{}(先通过 ingest 导入框架控制；等保 2.0 也可使用 scaffold)",
            dir.display()
        );
    }

    let mut entries: Vec<ControlIndexEntry> = Vec::new();
    let mut control_ids = HashSet::new();
    for entry in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // 只处理 .md,跳过 index.yaml 自身。
        if path.extension() != Some(OsStr::new("md")) {
            continue;
        }

        let rel = path
            .strip_prefix(&dir)
            .wrap_err("strip_prefix 失败(路径不在框架目录内)")?
            .to_string_lossy()
            .into_owned();

        let content =
            fs::read_to_string(path).wrap_err_with(|| format!("读取 {} 失败", path.display()))?;
        let doc = frontmatter::parse::<ControlFrontmatter>(&content)
            .wrap_err_with(|| format!("解析 {} 的 frontmatter 失败", path.display()))?;
        let fm = doc.data;

        // 通用框架的原始控制 ID 可能不适合做文件名，因此不再把物理
        // 文件名当成身份。权威不变式是 `id = framework:control_id`。
        if fm.id.framework != fm.framework || fm.id.control_id != fm.control_id {
            eyre::bail!(
                "{}:frontmatter id `{}` 与 framework/control_id `{}:{}` 不一致",
                path.display(),
                fm.id,
                fm.framework,
                fm.control_id
            );
        }
        if fm.framework.as_str() != framework {
            eyre::bail!(
                "{}:frontmatter framework `{}` 与目标框架 `{framework}` 不一致",
                path.display(),
                fm.framework
            );
        }
        if !control_ids.insert(fm.id.clone()) {
            eyre::bail!("{}:控制 ID `{}` 重复", path.display(), fm.id);
        }

        entries.push(ControlIndexEntry {
            id: fm.id,
            title: fm.title,
            domain: fm.domain,
            category: fm.category,
            levels: fm.levels,
            tags: fm.tags,
            mappings: fm.mappings,
            excerpt_status: fm.excerpt_status,
            file: rel,
        });
    }

    entries.sort_by(|a, b| crate::model::natural_control_cmp(&a.id.control_id, &b.id.control_id));

    let index = ControlIndex {
        framework: Framework(framework.to_string()),
        controls: entries,
    };
    let index_yaml = serde_yml::to_string(&index).wrap_err("序列化索引失败")?;
    let index_path = dir.join("index.yaml");
    fs::write(&index_path, index_yaml)
        .wrap_err_with(|| format!("写入 {} 失败", index_path.display()))?;

    println!(
        "built index: {} controls for {framework}",
        index.controls.len()
    );
    Ok(())
}
