//! `complai system init <slug> --name <display>`:创建一个共享系统知识库。

use std::fs;

use eyre::WrapErr;

use crate::system::fact::FactIndex;
use crate::system::system_dir;

pub fn init(slug: &str, name: String) -> eyre::Result<()> {
    let dir = system_dir(slug)?;
    let index_path = dir.join("index.yaml");
    if index_path.exists() {
        eyre::bail!("系统 `{slug}` 已存在:{}", index_path.display());
    }
    fs::create_dir_all(&dir).wrap_err("创建 system 目录失败")?;
    let index = FactIndex {
        display_name: Some(name),
        facts: Vec::new(),
    };
    let yaml = serde_yml::to_string(&index).wrap_err("序列化 system index 失败")?;
    fs::write(&index_path, yaml).wrap_err("写 system index 失败")?;
    println!("initialized system `{slug}`");
    Ok(())
}
