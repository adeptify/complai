//! 端到端集成测试(重构后):共享 KB(compliance + system)+ 项目(矩阵 + 项目事实 + 证据)。
//!
//! 涉及 COMPLAI_KB_DIR / COMPLAI_PROJECT_DIR(进程级全局),用 `#[serial]` 串行化。

use serial_test::serial;
use tempfile::TempDir;

use complai::{compliance, ingest, model, project, reports, system};

#[test]
#[serial]
fn kb_scaffold_produces_full_index() {
    let kb_dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("COMPLAI_KB_DIR", kb_dir.path());
    }
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
    unsafe {
        std::env::set_var("COMPLAI_KB_DIR", kb_dir.path());
    }
    compliance::scaffold::scaffold("dengbao-2.0").unwrap();
    let dir = compliance::framework_dir("dengbao-2.0").unwrap();
    let content = std::fs::read_to_string(dir.join("技术/安全计算环境/8.1.4.1.md")).unwrap();
    insta::assert_snapshot!(content);
}

#[test]
#[serial]
fn unified_ingest_applies_all_record_types_idempotently() {
    let kb_dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("COMPLAI_KB_DIR", kb_dir.path());
    }
    compliance::scaffold::scaffold("dengbao-2.0").unwrap();
    system::init::init("order-platform", "订单平台".to_string()).unwrap();
    let project_parent = TempDir::new().unwrap();
    let project_path = project_parent.path().join("assessment");
    project::init::init(
        project_path.to_str().unwrap(),
        "order-platform",
        "dengbao-2.0",
        Some(3),
    )
    .unwrap();
    unsafe {
        std::env::set_var("COMPLAI_PROJECT_DIR", &project_path);
    }

    let json = r#"{
      "schema_version": "complai.ingest/v1",
      "records": [
        {
          "kind": "control_content",
          "external_key": "standard:8.1.4.1",
          "source": {
            "type": "pdf",
            "title": "安全要求摘录",
            "reference": "standard.pdf",
            "locator": "第 18 页"
          },
          "confidence": "high",
          "target": {"control": "dengbao-2.0:8.1.4.1"},
          "content": {
            "requirement_summary": "对登录用户进行身份鉴别。",
            "completeness": "partial"
          }
        },
        {
          "kind": "system_fact",
          "external_key": "assessment:p12:database",
          "source": {
            "type": "pdf",
            "title": "订单平台测评报告",
            "reference": "assessment.pdf",
            "locator": "第 12 页",
            "document_date": "2026-06-20"
          },
          "confidence": "high",
          "target": {"system": "order-platform"},
          "content": {
            "domain": "资产",
            "title": "用户数据库",
            "body": "MySQL 8.0 主从部署。",
            "related_controls": ["dengbao-2.0:8.1.4.8"]
          }
        },
        {
          "kind": "project_fact",
          "external_key": "assessment:p42:finding",
          "source": {
            "type": "pdf",
            "title": "订单平台测评报告",
            "reference": "assessment.pdf",
            "locator": "第 42 页"
          },
          "confidence": "high",
          "target": {"project": "assessment"},
          "content": {
            "type": "发现",
            "title": "未启用多因素认证",
            "body": "运维人员仅使用口令登录。",
            "control": "dengbao-2.0:8.1.4.1"
          }
        },
        {
          "kind": "matrix_assessment",
          "external_key": "assessment:p42:8.1.4.1",
          "source": {
            "type": "pdf",
            "title": "订单平台测评报告",
            "reference": "assessment.pdf",
            "locator": "第 42 页"
          },
          "confidence": "high",
          "target": {
            "project": "assessment",
            "control": "dengbao-2.0:8.1.4.1"
          },
          "content": {
            "status": "gap",
            "gap": "运维人员登录未启用多因素认证",
            "owner": "安全负责人"
          }
        }
      ]
    }"#;
    let bundle: ingest::IngestBundle = serde_json::from_str(json).unwrap();
    let first_plan = ingest::apply_bundle(
        &bundle,
        ingest::ApplyOptions {
            allow_low_confidence: false,
        },
    )
    .unwrap();
    assert_eq!(
        first_plan
            .iter()
            .filter(|item| item.action == ingest::PlanAction::Unchanged)
            .count(),
        0
    );

    let second_plan = ingest::apply_bundle(
        &bundle,
        ingest::ApplyOptions {
            allow_low_confidence: false,
        },
    )
    .unwrap();
    assert!(
        second_plan
            .iter()
            .all(|item| item.action == ingest::PlanAction::Unchanged)
    );

    let idx = system::fact::load_index("order-platform").unwrap();
    assert_eq!(idx.display_name.as_deref(), Some("订单平台"));
    assert_eq!(idx.facts.len(), 1);
    assert_eq!(idx.facts[0].id, "SYS-F-0001");
    assert_eq!(
        idx.facts[0].external_key.as_deref(),
        Some("assessment:p12:database")
    );
    assert!(
        idx.facts[0]
            .related_controls
            .iter()
            .any(|c| c.control_id == "8.1.4.8")
    );

    let matrix = project::matrix::load(&project_path).unwrap();
    let control: complai::model::ControlId = "dengbao-2.0:8.1.4.1".parse().unwrap();
    let matrix_entry = matrix.entries.get(&control).unwrap();
    assert_eq!(matrix_entry.status, complai::model::ControlStatus::Gap);
    assert_eq!(
        matrix_entry
            .ingest
            .as_ref()
            .map(|metadata| metadata.external_key.as_str()),
        Some("assessment:p42:8.1.4.1")
    );

    let framework_dir = compliance::framework_dir("dengbao-2.0").unwrap();
    let control_body =
        std::fs::read_to_string(framework_dir.join("技术/安全计算环境/8.1.4.1.md")).unwrap();
    assert!(control_body.contains("对登录用户进行身份鉴别"));
    assert!(control_body.contains("standard:8.1.4.1"));
}

#[test]
#[serial]
fn unified_ingest_requires_confirmation_for_low_confidence() {
    let kb_dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("COMPLAI_KB_DIR", kb_dir.path());
        std::env::remove_var("COMPLAI_PROJECT_DIR");
    }
    system::init::init("s1", "系统一".to_string()).unwrap();
    let json = r#"{
      "schema_version": "complai.ingest/v1",
      "records": [{
        "kind": "system_fact",
        "external_key": "interview:network",
        "source": {
          "type": "interview",
          "title": "网络架构访谈",
          "reference": "interview-2026-08-03",
          "locator": "问题 4"
        },
        "confidence": "low",
        "target": {"system": "s1"},
        "content": {
          "domain": "网络",
          "title": "生产区隔离",
          "body": "受访者表示生产区与办公区已隔离。"
        }
      }]
    }"#;
    let bundle: ingest::IngestBundle = serde_json::from_str(json).unwrap();
    let result = ingest::apply_bundle(
        &bundle,
        ingest::ApplyOptions {
            allow_low_confidence: false,
        },
    );
    assert!(result.is_err());
    assert!(system::fact::load_index("s1").unwrap().facts.is_empty());

    ingest::apply_bundle(
        &bundle,
        ingest::ApplyOptions {
            allow_low_confidence: true,
        },
    )
    .unwrap();
    assert_eq!(system::fact::load_index("s1").unwrap().facts.len(), 1);
}

#[test]
#[serial]
fn project_end_to_end_produces_gap_report() {
    let kb_dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("COMPLAI_KB_DIR", kb_dir.path());
    }
    compliance::scaffold::scaffold("dengbao-2.0").unwrap();
    system::init::init("order-platform", "订单平台".to_string()).unwrap();

    let proj_root = TempDir::new().unwrap();
    let proj_path = proj_root.path().join("p");
    project::init::init(
        proj_path.to_str().unwrap(),
        "order-platform",
        "dengbao-2.0",
        Some(3),
    )
    .unwrap();
    unsafe {
        std::env::set_var("COMPLAI_PROJECT_DIR", &proj_path);
    }

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
    let report = std::fs::read_to_string(proj_path.join("drafts/compliance-report.md")).unwrap();
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
    unsafe {
        std::env::set_var("COMPLAI_KB_DIR", kb_dir.path());
    }
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
    project::init::init(p1.to_str().unwrap(), "shared-sys", "dengbao-2.0", Some(3)).unwrap();
    let pr2 = TempDir::new().unwrap();
    let p2 = pr2.path().join("b");
    project::init::init(p2.to_str().unwrap(), "shared-sys", "dengbao-2.0", None).unwrap();

    let idx = system::fact::load_index("shared-sys").unwrap();
    assert_eq!(idx.facts.len(), 1);
    assert_eq!(idx.display_name.as_deref(), Some("共享系统"));
    assert_eq!(
        project::load_meta(&p2).expect("等保项目元数据可加载").level,
        Some(3)
    );
}

#[test]
#[serial]
fn generic_framework_controls_can_be_ingested_and_assessed() {
    let kb_dir = TempDir::new().expect("临时 KB 目录可创建");
    unsafe {
        std::env::set_var("COMPLAI_KB_DIR", kb_dir.path());
        std::env::remove_var("COMPLAI_PROJECT_DIR");
    }

    let json = r#"{
      "schema_version": "complai.ingest/v1",
      "records": [{
        "kind": "control_content",
        "external_key": "iso27001:2022:A.5.1",
        "source": {
          "type": "licensed-standard",
          "title": "ISO/IEC 27001:2022 control notes",
          "reference": "iso27001-notes.pdf",
          "locator": "A.5.1"
        },
        "confidence": "high",
        "target": {"control": "iso27001-2022:A.5.1"},
        "content": {
          "title": "Policies for information security",
          "domain": "Organizational controls",
          "category": "Information security policies",
          "requirement_summary": "Define, approve, communicate, and review information security policies.",
          "expected_evidence": ["Approved information security policy"],
          "completeness": "partial"
        }
      }]
    }"#;
    let bundle: ingest::IngestBundle = serde_json::from_str(json).expect("通用框架 bundle 应合法");
    let plan = ingest::plan_bundle(&bundle).expect("新框架应可预览");
    assert_eq!(plan[0].action, ingest::PlanAction::Create);
    ingest::apply_bundle(
        &bundle,
        ingest::ApplyOptions {
            allow_low_confidence: false,
        },
    )
    .expect("新框架应可写入");
    let repeated_plan = ingest::plan_bundle(&bundle).expect("重复导入应可预览");
    assert_eq!(repeated_plan[0].action, ingest::PlanAction::Unchanged);

    let index = compliance::query::load_index("iso27001-2022").expect("新框架应自动建立索引");
    assert_eq!(index.controls.len(), 1);
    assert_eq!(index.controls[0].id.control_id, "A.5.1");
    assert_eq!(
        index.controls[0].domain,
        model::Domain::new("Organizational controls").expect("控制域合法")
    );

    system::init::init("global-service", "Global Service".to_string()).expect("系统应可初始化");
    let project_parent = TempDir::new().expect("临时项目目录可创建");
    let project_path = project_parent.path().join("iso-assessment");
    project::init::init(
        project_path.to_str().expect("项目路径是 UTF-8"),
        "global-service",
        "iso27001-2022",
        None,
    )
    .expect("无等级框架应可初始化项目");
    unsafe {
        std::env::set_var("COMPLAI_PROJECT_DIR", &project_path);
    }

    // 旧版把无内容的 `na` 当初始值；加载时应将这种可明确识别的旧数据
    // 视为未评估，但不改动有理由的真实 `na`。
    let matrix_path = project_path.join("matrix.yaml");
    let legacy_matrix = std::fs::read_to_string(&matrix_path)
        .expect("初始矩阵应可读取")
        .replacen("status: unassessed", "status: na", 1);
    std::fs::write(&matrix_path, legacy_matrix).expect("旧版矩阵测试数据应可写入");

    let control: model::ControlId = "iso27001-2022:A.5.1".parse().expect("通用控制 ID 合法");
    let matrix = project::matrix::load(&project_path).expect("矩阵应可加载");
    assert_eq!(matrix.level, None);
    assert_eq!(
        matrix.entries.get(&control).expect("控制已预填").status,
        model::ControlStatus::Unassessed
    );
    project::show().expect("项目路由信息应可查询");
    reports::report::generate().expect("初始报告应可生成");
    let initial_report = std::fs::read_to_string(project_path.join("drafts/compliance-report.md"))
        .expect("初始报告应可读取");
    assert!(initial_report.contains("未评估 1"));
    assert!(project::matrix::set("iso27001-2022:A.5.1", "na", None, None).is_err());
    assert!(
        project::matrix::link(
            "iso27001-2022:A.5.1",
            Some("EV-9999".to_string()),
            None,
            None,
        )
        .is_err()
    );

    let evidence_source = project_path.join("policy.txt");
    std::fs::write(&evidence_source, "approved policy").expect("测试证据文件应可写入");
    project::evidence::add(
        evidence_source.to_str().expect("证据路径是 UTF-8"),
        "iso27001-2022:A.5.1",
        "policy-doc".to_string(),
        Some("Approved information security policy".to_string()),
    )
    .expect("证据应可登记");
    project::evidence::list().expect("证据应可列出");
    project::evidence::show("EV-0001").expect("证据应可查看");
    project::evidence::find("iso27001-2022:A.5.1").expect("证据应可按控制查找");
    project::matrix::link(
        "iso27001-2022:A.5.1",
        Some("EV-0001".to_string()),
        None,
        None,
    )
    .expect("存在的证据应可关联");
    project::matrix::set("iso27001-2022:A.5.1", "met", None, None)
        .expect("有支撑的控制应可标记满足");

    reports::report::generate().expect("通用框架报告应可生成");
    let report = std::fs::read_to_string(project_path.join("drafts/compliance-report.md"))
        .expect("报告应可读取");
    assert!(report.contains("框架: iso27001-2022"));
    assert!(!report.contains("等级:"));
    assert!(report.contains("未评估 0"));
    assert!(report.contains("满足 1"));
}
