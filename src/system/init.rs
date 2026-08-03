//! `complai system init <slug> --name <display>`:创建一个共享系统知识库。

use eyre::WrapErr;

use crate::system::fact::FactIndex;
use crate::system::system_dir;

pub fn init(slug: &str, name: String) -> eyre::Result<()> {
    let _lock = crate::storage::WriteLock::acquire().wrap_err("锁定 Complai 写操作失败")?;
    let dir = system_dir(slug).wrap_err("解析系统知识库目录失败")?;
    let index_path = dir.join("index.yaml");
    if index_path.exists() {
        eyre::bail!("系统 `{slug}` 已存在:{}", index_path.display());
    }
    let index = FactIndex {
        display_name: Some(name),
        facts: Vec::new(),
    };
    let yaml = serde_yml::to_string(&index).wrap_err("序列化 system index 失败")?;
    crate::storage::transaction(|| {
        crate::storage::atomic_write(&index_path, yaml).wrap_err("写 system index 失败")
    })
    .wrap_err("初始化系统存储事务失败")?;
    println!("initialized system `{slug}`");
    Ok(())
}
