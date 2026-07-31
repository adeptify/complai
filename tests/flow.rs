//! 端到端集成测试(重构后):共享 KB(compliance + system)+ 项目(矩阵 + 项目事实 + 证据)。
//!
//! 涉及 COMPLAI_KB_DIR / COMPLAI_PROJECT_DIR(进程级全局),用 `#[serial]` 串行化。

use serial_test::serial;
use tempfile::TempDir;

use complai::{compliance, project, reports, system};

#[test]
#[serial]
fn kb_scaffold_produces_full_index() {
    let kb_dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("COMPLAI_KB_DIR", kb_dir.path()); }
    compliance::scaffold::scaffold("dengbao-2.0").unwrap();
    let index = compliance::query::load_index("dengbao-2.0").unwrap();
    assert_eq!(index.controls.len(), 70);
    assert_eq!(index.controls[0].id.control_id, "8.1.1.1");
    assert_eq!(index.controls[1].id.control_id, "8.1.1.2");
    let last = index.controls.last().unwrap();
    assert!(last.id.control_id.starts_with("8.1.10."));
}

#[test]
#[serial]
fn scaffolded_control_file_matches_snapshot() {
    let kb_dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("COMPLAI_KB_DIR", kb_dir.path()); }
    compliance::scaffold::scaffold("dengbao-2.0").unwrap();
    let dir = compliance::framework_dir("dengbao-2.0").unwrap();
    let content =
        std::fs::read_to_string(dir.join("技术/安全计算环境/8.1.4.1.md")).unwrap();
    insta::assert_snapshot!(content);
}

#[test]
#[serial]
fn system_init_and_ingest() {
    let kb_dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("COMPLAI_KB_DIR", kb_dir.path()); }
    compliance::scaffold::scaffold("dengbao-2.0").unwrap();
    system::init::init("s1", "系统一".to_string()).unwrap();

    let yaml = r#"facts:
  - domain: 资产
    title: 用户数据库
    control: "dengbao-2.0:8.1.4.8"
    type: doc
    ref: 资产清单.xlsx
    body: MySQL 8.0 主从
  - domain: 部署
    title: 多可用区
    body: 跨 2 AZ
"#;
    let tmp = TempDir::new().unwrap();
    let yaml_path = tmp.path().join("facts.yaml");
    std::fs::write(&yaml_path, yaml).unwrap();
    system::fact::ingest("s1", yaml_path.to_str().unwrap()).unwrap();

    let idx = system::fact::load_index("s1").unwrap();
    assert_eq!(idx.display_name.as_deref(), Some("系统一"));
    assert_eq!(idx.facts.len(), 2);
    assert_eq!(idx.facts[0].id, "SYS-F-0001");
    assert!(idx.facts[0]
        .related_controls
        .iter()
        .any(|c| c.control_id == "8.1.4.8"));
}

#[test]
#[serial]
fn project_end_to_end_produces_gap_report() {
    let kb_dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("COMPLAI_KB_DIR", kb_dir.path()); }
    compliance::scaffold::scaffold("dengbao-2.0").unwrap();
    system::init::init("order-platform", "订单平台".to_string()).unwrap();

    let proj_root = TempDir::new().unwrap();
    let proj_path = proj_root.path().join("p");
    project::init::init(proj_path.to_str().unwrap(), "order-platform", "dengbao-2.0", 3).unwrap();
    unsafe { std::env::set_var("COMPLAI_PROJECT_DIR", &proj_path); }

    // 系统事实(共享 system KB)
    system::fact::add(
        "order-platform",
        "架构".into(),
        "微服务拓扑".into(),
        Some("dengbao-2.0:8.1.2".into()),
        "user".into(),
        None,
        Some("三个微服务,经 API 网关接入,mTLS。".into()),
    )
    .unwrap();
    // 项目事实(整改项)
    project::fact::add(
        "整改".into(),
        "payment-service 接入 MFA".into(),
        Some("dengbao-2.0:8.1.4.1".into()),
        Some("负责人张三,计划 Q3 完成。".into()),
    )
    .unwrap();
    // 矩阵:标缺口 + 关联系统事实 + 项目事实
    project::matrix::set(
        "dengbao-2.0:8.1.4.1",
        "gap",
        Some("payment-service 未启用多因素".into()),
        Some("张三".into()),
    )
    .unwrap();
    project::matrix::link(
        "dengbao-2.0:8.1.4.1",
        None,
        Some("SYS-F-0001".into()),
        Some("PROJ-F-0001".into()),
    )
    .unwrap();

    reports::report::generate().unwrap();
    let report =
        std::fs::read_to_string(proj_path.join("drafts/compliance-report.md")).unwrap();
    assert!(report.contains("共 70 项"));
    assert!(report.contains("缺口 1"));
    assert!(report.contains("身份鉴别"));
    assert!(report.contains("payment-service 未启用多因素"));
    assert!(report.contains("SYS-F-0001"));
}

#[test]
#[serial]
fn system_shared_across_projects() {
    // 同一 system 被两个项目引用;系统事实只存一份,project init 不重建已有系统。
    let kb_dir = TempDir::new().unwrap();
    unsafe { std::env::set_var("COMPLAI_KB_DIR", kb_dir.path()); }
    compliance::scaffold::scaffold("dengbao-2.0").unwrap();
    system::init::init("shared-sys", "共享系统".to_string()).unwrap();
    system::fact::add(
        "shared-sys",
        "架构".into(),
        "拓扑".into(),
        Some("dengbao-2.0:8.1.2".into()),
        "user".into(),
        None,
        Some("body".into()),
    )
    .unwrap();

    let pr1 = TempDir::new().unwrap();
    let p1 = pr1.path().join("a");
    project::init::init(p1.to_str().unwrap(), "shared-sys", "dengbao-2.0", 3).unwrap();
    let pr2 = TempDir::new().unwrap();
    let p2 = pr2.path().join("b");
    project::init::init(p2.to_str().unwrap(), "shared-sys", "dengbao-2.0", 3).unwrap();

    let idx = system::fact::load_index("shared-sys").unwrap();
    assert_eq!(idx.facts.len(), 1);
    assert_eq!(idx.display_name.as_deref(), Some("共享系统"));
}
