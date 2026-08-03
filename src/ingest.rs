//! 统一的 Agent -> Complai 结构化写入协议。
//!
//! 原始材料可以来自任意文件格式、云文档或访谈；读取与语义抽取由 Agent 使用当前
//! 环境可用的工具完成。本模块只接收版本化 JSON，并负责严格校验、变更预览、幂等
//! upsert 和来源持久化，使输入能力与知识库写入契约彼此解耦。

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::Local;
use eyre::WrapErr;
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::cli::IngestCommand;
use crate::compliance::control::{ControlFrontmatter, ExcerptStatus};
use crate::frontmatter;
use crate::model::{
    ControlId, ControlStatus, IngestConfidence, IngestMetadata, ProjectFactKind, SourceCitation,
};
use crate::project::fact::{ProjectFactFrontmatter, ProjectFactIndexEntry};
use crate::system::fact::{
    Confidence, FactFrontmatter, FactIndexEntry, FactSource, FactSourceType, FactStatus,
};

pub const SCHEMA_VERSION: &str = "complai.ingest/v1";
const SCHEMA: &str = include_str!("../schemas/ingest-v1.schema.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngestBundle {
    pub schema_version: String,
    pub records: Vec<IngestRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IngestRecord {
    ControlContent {
        external_key: String,
        source: SourceCitation,
        confidence: IngestConfidence,
        target: ControlTarget,
        content: ControlContent,
    },
    SystemFact {
        external_key: String,
        source: SourceCitation,
        confidence: IngestConfidence,
        target: SystemTarget,
        content: SystemFactContent,
    },
    ProjectFact {
        external_key: String,
        source: SourceCitation,
        confidence: IngestConfidence,
        target: ProjectTarget,
        content: ProjectFactContent,
    },
    MatrixAssessment {
        external_key: String,
        source: SourceCitation,
        confidence: IngestConfidence,
        target: MatrixTarget,
        content: MatrixAssessmentContent,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlTarget {
    pub control: ControlId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemTarget {
    pub system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectTarget {
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixTarget {
    pub project: String,
    pub control: ControlId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlContent {
    #[serde(default)]
    pub requirement_summary: Option<String>,
    #[serde(default)]
    pub implementation_guidance: Option<String>,
    #[serde(default)]
    pub common_deficiencies: Option<String>,
    #[serde(default)]
    pub expected_evidence: Option<Vec<String>>,
    pub completeness: ExcerptStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemFactContent {
    pub domain: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub related_controls: Vec<ControlId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFactContent {
    #[serde(rename = "type")]
    pub kind: ProjectFactKind,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub control: Option<ControlId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixAssessmentContent {
    pub status: ControlStatus,
    #[serde(default)]
    pub gap: Option<String>,
    #[serde(default)]
    pub remediation: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanAction {
    Create,
    Update,
    Unchanged,
}

impl PlanAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Unchanged => "unchanged",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanItem {
    pub action: PlanAction,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyOptions {
    pub allow_low_confidence: bool,
}

pub fn run(command: IngestCommand) -> eyre::Result<()> {
    match command {
        IngestCommand::Schema => {
            print!("{SCHEMA}");
            Ok(())
        }
        IngestCommand::Validate { source } => {
            let bundle = load_bundle(Path::new(&source)).wrap_err("加载 ingest bundle 失败")?;
            println!("valid {} records ({SCHEMA_VERSION})", bundle.records.len());
            Ok(())
        }
        IngestCommand::Plan { source } => {
            let bundle = load_bundle(Path::new(&source)).wrap_err("加载 ingest bundle 失败")?;
            let plan = plan_bundle(&bundle).wrap_err("生成 ingest 变更计划失败")?;
            print_plan(&plan);
            Ok(())
        }
        IngestCommand::Apply {
            source,
            allow_low_confidence,
        } => {
            let bundle = load_bundle(Path::new(&source)).wrap_err("加载 ingest bundle 失败")?;
            apply_bundle(
                &bundle,
                ApplyOptions {
                    allow_low_confidence,
                },
            )
            .wrap_err("应用 ingest bundle 失败")
            .map(|_| ())
        }
    }
}

/// 从路径或 -（stdin）读取并校验一个 bundle。
pub fn load_bundle(path: &Path) -> eyre::Result<IngestBundle> {
    let content = if path == Path::new("-") {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .wrap_err("从 stdin 读取 ingest JSON 失败")?;
        content
    } else {
        fs::read_to_string(path)
            .wrap_err_with(|| format!("读取 ingest JSON {} 失败", path.display()))?
    };
    let bundle: IngestBundle = serde_json::from_str(&content).wrap_err("解析 ingest JSON 失败")?;
    validate_bundle(&bundle).wrap_err("校验 ingest JSON 失败")?;
    Ok(bundle)
}

/// 校验与外部状态无关的协议约束。
pub fn validate_bundle(bundle: &IngestBundle) -> eyre::Result<()> {
    if bundle.schema_version != SCHEMA_VERSION {
        eyre::bail!(
            "不支持 schema_version '{}'；当前仅支持 '{}'",
            bundle.schema_version,
            SCHEMA_VERSION
        );
    }
    if bundle.records.is_empty() {
        eyre::bail!("records 不能为空");
    }

    let mut external_keys = HashSet::new();
    let mut singleton_targets = HashSet::new();
    for (index, record) in bundle.records.iter().enumerate() {
        let prefix = format!("records[{index}]");
        validate_common(record, &prefix).wrap_err_with(|| format!("校验 {prefix} 公共字段失败"))?;
        if !external_keys.insert(record.external_key()) {
            eyre::bail!(
                "{prefix}.external_key '{}' 在 bundle 中重复",
                record.external_key()
            );
        }
        if let Some(target) = record.singleton_target()
            && !singleton_targets.insert(target.clone())
        {
            eyre::bail!("{prefix} 与另一条记录重复写入目标 '{target}'");
        }

        match record {
            IngestRecord::ControlContent { content, .. } => {
                validate_control_content(content, &prefix)
                    .wrap_err_with(|| format!("校验 {prefix} 的控制内容失败"))?;
            }
            IngestRecord::SystemFact {
                target, content, ..
            } => {
                require_nonempty(&target.system, &format!("{prefix}.target.system"))
                    .wrap_err_with(|| format!("校验 {prefix} 的系统目标失败"))?;
                validate_system_fact(content, &prefix)
                    .wrap_err_with(|| format!("校验 {prefix} 的系统事实失败"))?;
            }
            IngestRecord::ProjectFact {
                target, content, ..
            } => {
                require_nonempty(&target.project, &format!("{prefix}.target.project"))
                    .wrap_err_with(|| format!("校验 {prefix} 的项目目标失败"))?;
                validate_project_fact(content, &prefix)
                    .wrap_err_with(|| format!("校验 {prefix} 的项目事实失败"))?;
            }
            IngestRecord::MatrixAssessment {
                target, content, ..
            } => {
                require_nonempty(&target.project, &format!("{prefix}.target.project"))
                    .wrap_err_with(|| format!("校验 {prefix} 的矩阵目标失败"))?;
                validate_matrix_assessment(content, &prefix)
                    .wrap_err_with(|| format!("校验 {prefix} 的矩阵评估失败"))?;
            }
        }
    }
    Ok(())
}

/// 在不写文件的前提下校验所有目标，并计算每条记录的变更类型。
pub fn plan_bundle(bundle: &IngestBundle) -> eyre::Result<Vec<PlanItem>> {
    validate_bundle(bundle).wrap_err("校验 ingest bundle 失败")?;
    let mut plan = Vec::with_capacity(bundle.records.len());
    for record in &bundle.records {
        let metadata = record.metadata().wrap_err("计算 ingest 记录摘要失败")?;
        let action = match record {
            IngestRecord::ControlContent { target, .. } => {
                let (_, document) = load_control(&target.control)
                    .wrap_err_with(|| format!("加载控制 {} 失败", target.control))?;
                action_for(document.data.ingest.as_ref(), &metadata, false)
            }
            IngestRecord::SystemFact { target, .. } => plan_system_fact(&target.system, &metadata)
                .wrap_err_with(|| format!("检查系统 '{}' 失败", target.system))?,
            IngestRecord::ProjectFact {
                target, content, ..
            } => {
                if let Some(control) = &content.control {
                    ensure_control_exists(control)
                        .wrap_err_with(|| format!("检查项目事实控制 {control} 失败"))?;
                }
                plan_project_fact(&target.project, &metadata)
                    .wrap_err_with(|| format!("检查项目 '{}' 的事实失败", target.project))?
            }
            IngestRecord::MatrixAssessment { target, .. } => {
                plan_matrix_assessment(target, &metadata)
                    .wrap_err_with(|| format!("检查矩阵控制 {} 失败", target.control))?
            }
        };

        if let IngestRecord::SystemFact { content, .. } = record {
            for control in &content.related_controls {
                ensure_control_exists(control)
                    .wrap_err_with(|| format!("检查系统事实关联控制 {control} 失败"))?;
            }
        }
        plan.push(PlanItem {
            action,
            label: record.label(),
        });
    }
    Ok(plan)
}

/// 校验完整计划后应用 bundle；同一 external_key 和内容摘要会被幂等跳过。
pub fn apply_bundle(bundle: &IngestBundle, options: ApplyOptions) -> eyre::Result<Vec<PlanItem>> {
    let plan = plan_bundle(bundle).wrap_err("预检 ingest bundle 失败")?;
    if !options.allow_low_confidence {
        let low_confidence = bundle
            .records
            .iter()
            .filter(|record| record.confidence() == IngestConfidence::Low)
            .map(IngestRecord::label)
            .collect::<Vec<_>>();
        if !low_confidence.is_empty() {
            eyre::bail!(
                "拒绝写入 {} 条低置信度记录：{}；确认后使用 --allow-low-confidence",
                low_confidence.len(),
                low_confidence.join(", ")
            );
        }
    }

    let mut changed_frameworks = BTreeSet::new();
    for (record, item) in bundle.records.iter().zip(&plan) {
        if item.action == PlanAction::Unchanged {
            continue;
        }
        let metadata = record.metadata().wrap_err("计算 ingest 持久化元数据失败")?;
        match record {
            IngestRecord::ControlContent {
                target, content, ..
            } => {
                apply_control_content(&target.control, content, metadata)
                    .wrap_err_with(|| format!("写入控制 {} 失败", target.control))?;
                changed_frameworks.insert(target.control.framework.as_str().to_string());
            }
            IngestRecord::SystemFact {
                target, content, ..
            } => apply_system_fact(&target.system, content, metadata)
                .wrap_err_with(|| format!("写入系统 '{}' 事实失败", target.system))?,
            IngestRecord::ProjectFact {
                target, content, ..
            } => apply_project_fact(&target.project, content, metadata)
                .wrap_err_with(|| format!("写入项目 '{}' 事实失败", target.project))?,
            IngestRecord::MatrixAssessment {
                target, content, ..
            } => apply_matrix_assessment(target, content, metadata)
                .wrap_err_with(|| format!("写入矩阵控制 {} 失败", target.control))?,
        }
    }

    for framework in changed_frameworks {
        crate::compliance::build::build(&framework)
            .wrap_err_with(|| format!("重建框架 '{framework}' 索引失败"))?;
    }
    print_plan(&plan);
    Ok(plan)
}

impl IngestRecord {
    fn external_key(&self) -> &str {
        match self {
            Self::ControlContent { external_key, .. }
            | Self::SystemFact { external_key, .. }
            | Self::ProjectFact { external_key, .. }
            | Self::MatrixAssessment { external_key, .. } => external_key,
        }
    }

    fn source(&self) -> &SourceCitation {
        match self {
            Self::ControlContent { source, .. }
            | Self::SystemFact { source, .. }
            | Self::ProjectFact { source, .. }
            | Self::MatrixAssessment { source, .. } => source,
        }
    }

    fn confidence(&self) -> IngestConfidence {
        match self {
            Self::ControlContent { confidence, .. }
            | Self::SystemFact { confidence, .. }
            | Self::ProjectFact { confidence, .. }
            | Self::MatrixAssessment { confidence, .. } => *confidence,
        }
    }

    fn singleton_target(&self) -> Option<String> {
        match self {
            Self::ControlContent { target, .. } => Some(format!("control:{}", target.control)),
            Self::MatrixAssessment { target, .. } => {
                Some(format!("matrix:{}:{}", target.project, target.control))
            }
            Self::SystemFact { .. } | Self::ProjectFact { .. } => None,
        }
    }

    fn label(&self) -> String {
        match self {
            Self::ControlContent { target, .. } => format!("control {}", target.control),
            Self::SystemFact {
                target, content, ..
            } => format!("system {} / {}", target.system, content.title),
            Self::ProjectFact {
                target, content, ..
            } => format!("project {} / {}", target.project, content.title),
            Self::MatrixAssessment { target, .. } => {
                format!("matrix {} / {}", target.project, target.control)
            }
        }
    }

    fn metadata(&self) -> eyre::Result<IngestMetadata> {
        let bytes = serde_json::to_vec(self).wrap_err("序列化 ingest 记录用于计算摘要失败")?;
        let digest = sha2::Sha256::digest(bytes);
        Ok(IngestMetadata {
            external_key: self.external_key().to_string(),
            record_sha256: hex_digest(&digest),
            source: self.source().clone(),
            confidence: self.confidence(),
        })
    }
}

fn validate_common(record: &IngestRecord, prefix: &str) -> eyre::Result<()> {
    require_nonempty(record.external_key(), &format!("{prefix}.external_key"))
        .wrap_err("校验 external_key 失败")?;
    let source = record.source();
    require_nonempty(&source.kind, &format!("{prefix}.source.type"))
        .wrap_err("校验 source.type 失败")?;
    require_nonempty(&source.title, &format!("{prefix}.source.title"))
        .wrap_err("校验 source.title 失败")?;
    require_nonempty(&source.reference, &format!("{prefix}.source.reference"))
        .wrap_err("校验 source.reference 失败")?;
    require_nonempty(&source.locator, &format!("{prefix}.source.locator"))
        .wrap_err("校验 source.locator 失败")?;
    if let Some(sha256) = &source.sha256
        && (sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()))
    {
        eyre::bail!("{prefix}.source.sha256 必须是 64 位十六进制字符串");
    }
    Ok(())
}

fn validate_control_content(content: &ControlContent, prefix: &str) -> eyre::Result<()> {
    let sections = [
        (
            "requirement_summary",
            content.requirement_summary.as_deref(),
        ),
        (
            "implementation_guidance",
            content.implementation_guidance.as_deref(),
        ),
        (
            "common_deficiencies",
            content.common_deficiencies.as_deref(),
        ),
    ];
    for (name, value) in sections {
        if let Some(value) = value {
            require_nonempty(value, &format!("{prefix}.content.{name}"))
                .wrap_err_with(|| format!("校验控制内容字段 {name} 失败"))?;
        }
    }
    let present = sections.iter().filter(|(_, value)| value.is_some()).count();
    if present == 0 {
        eyre::bail!("{prefix}.content 至少需要一个正文段落");
    }
    match content.completeness {
        ExcerptStatus::Empty => eyre::bail!("{prefix}.content.completeness 不能是 empty"),
        ExcerptStatus::Complete if present != sections.len() => {
            eyre::bail!("{prefix}.content.completeness=complete 时三个正文段落必须齐全")
        }
        ExcerptStatus::Partial | ExcerptStatus::Complete => {}
    }
    if let Some(expected_evidence) = &content.expected_evidence {
        validate_string_list(
            expected_evidence,
            &format!("{prefix}.content.expected_evidence"),
        )
        .wrap_err("校验 expected_evidence 失败")?;
    }
    Ok(())
}

fn validate_system_fact(content: &SystemFactContent, prefix: &str) -> eyre::Result<()> {
    require_nonempty(&content.domain, &format!("{prefix}.content.domain"))
        .wrap_err("校验系统事实 domain 失败")?;
    require_nonempty(&content.title, &format!("{prefix}.content.title"))
        .wrap_err("校验系统事实 title 失败")?;
    require_nonempty(&content.body, &format!("{prefix}.content.body"))
        .wrap_err("校验系统事实 body 失败")?;
    validate_string_list(&content.tags, &format!("{prefix}.content.tags"))
        .wrap_err("校验系统事实 tags 失败")
}

fn validate_project_fact(content: &ProjectFactContent, prefix: &str) -> eyre::Result<()> {
    require_nonempty(&content.title, &format!("{prefix}.content.title"))
        .wrap_err("校验项目事实 title 失败")?;
    require_nonempty(&content.body, &format!("{prefix}.content.body"))
        .wrap_err("校验项目事实 body 失败")?;
    validate_string_list(&content.tags, &format!("{prefix}.content.tags"))
        .wrap_err("校验项目事实 tags 失败")
}

fn validate_matrix_assessment(content: &MatrixAssessmentContent, prefix: &str) -> eyre::Result<()> {
    for (name, value) in [
        ("gap", content.gap.as_deref()),
        ("remediation", content.remediation.as_deref()),
        ("owner", content.owner.as_deref()),
    ] {
        if let Some(value) = value {
            require_nonempty(value, &format!("{prefix}.content.{name}"))
                .wrap_err_with(|| format!("校验矩阵字段 {name} 失败"))?;
        }
    }
    if content.status == ControlStatus::Gap && content.gap.is_none() {
        eyre::bail!("{prefix}.content.status=gap 时必须提供 gap");
    }
    Ok(())
}

fn validate_string_list(values: &[String], field: &str) -> eyre::Result<()> {
    let mut unique = HashSet::new();
    for value in values {
        require_nonempty(value, field).wrap_err_with(|| format!("校验 {field} 条目失败"))?;
        if !unique.insert(value) {
            eyre::bail!("{field} 中存在重复值 '{value}'");
        }
    }
    Ok(())
}

fn require_nonempty(value: &str, field: &str) -> eyre::Result<()> {
    if value.trim().is_empty() {
        eyre::bail!("{field} 不能为空");
    }
    Ok(())
}

fn action_for(
    existing: Option<&IngestMetadata>,
    incoming: &IngestMetadata,
    missing_is_create: bool,
) -> PlanAction {
    match existing {
        Some(metadata)
            if metadata.external_key == incoming.external_key
                && metadata.record_sha256 == incoming.record_sha256 =>
        {
            PlanAction::Unchanged
        }
        Some(_) => PlanAction::Update,
        None if missing_is_create => PlanAction::Create,
        None => PlanAction::Update,
    }
}

fn load_control(
    control: &ControlId,
) -> eyre::Result<(PathBuf, frontmatter::Document<ControlFrontmatter>)> {
    let framework = control.framework.as_str();
    let index = crate::compliance::query::load_index(framework)
        .wrap_err_with(|| format!("加载框架 '{framework}' 索引失败"))?;
    let entry = index
        .controls
        .iter()
        .find(|entry| entry.id == *control)
        .ok_or_else(|| eyre::eyre!("控制 {control} 不在框架索引中"))
        .wrap_err("定位控制文件失败")?;
    let path = crate::compliance::framework_dir(framework)
        .wrap_err_with(|| format!("解析框架 '{framework}' 目录失败"))?
        .join(&entry.file);
    let content = fs::read_to_string(&path)
        .wrap_err_with(|| format!("读取控制文件 {} 失败", path.display()))?;
    let document = frontmatter::parse::<ControlFrontmatter>(&content)
        .wrap_err_with(|| format!("解析控制文件 {} 失败", path.display()))?;
    Ok((path, document))
}

fn ensure_control_exists(control: &ControlId) -> eyre::Result<()> {
    load_control(control)
        .map(|_| ())
        .wrap_err_with(|| format!("控制 {control} 不存在"))
}

fn plan_system_fact(system: &str, metadata: &IngestMetadata) -> eyre::Result<PlanAction> {
    let index = crate::system::fact::load_index(system)
        .wrap_err_with(|| format!("加载系统 '{system}' 索引失败"))?;
    let Some(entry) = index
        .facts
        .iter()
        .find(|entry| entry.external_key.as_deref() == Some(&metadata.external_key))
    else {
        return Ok(PlanAction::Create);
    };
    let path = crate::system::system_dir(system)
        .wrap_err_with(|| format!("解析系统 '{system}' 目录失败"))?
        .join(&entry.file);
    let content = fs::read_to_string(&path)
        .wrap_err_with(|| format!("读取系统事实 {} 失败", path.display()))?;
    let document = frontmatter::parse::<FactFrontmatter>(&content)
        .wrap_err_with(|| format!("解析系统事实 {} 失败", path.display()))?;
    Ok(action_for(document.data.ingest.as_ref(), metadata, true))
}

fn current_project(expected_name: &str) -> eyre::Result<(PathBuf, crate::project::ProjectMeta)> {
    let root = crate::project::project_root().wrap_err("定位当前项目失败")?;
    let metadata = crate::project::load_meta(&root).wrap_err("加载当前项目元数据失败")?;
    if metadata.name != expected_name {
        eyre::bail!(
            "bundle 目标项目为 '{expected_name}'，当前项目为 '{}'",
            metadata.name
        );
    }
    Ok((root, metadata))
}

fn plan_project_fact(project: &str, metadata: &IngestMetadata) -> eyre::Result<PlanAction> {
    let (root, _) = current_project(project).wrap_err("检查当前项目失败")?;
    let index = crate::project::fact::load_index(&root).wrap_err("加载项目事实索引失败")?;
    let Some(entry) = index
        .facts
        .iter()
        .find(|entry| entry.external_key.as_deref() == Some(&metadata.external_key))
    else {
        return Ok(PlanAction::Create);
    };
    let path = root.join("facts").join(&entry.file);
    let content = fs::read_to_string(&path)
        .wrap_err_with(|| format!("读取项目事实 {} 失败", path.display()))?;
    let document = frontmatter::parse::<ProjectFactFrontmatter>(&content)
        .wrap_err_with(|| format!("解析项目事实 {} 失败", path.display()))?;
    Ok(action_for(document.data.ingest.as_ref(), metadata, true))
}

fn plan_matrix_assessment(
    target: &MatrixTarget,
    metadata: &IngestMetadata,
) -> eyre::Result<PlanAction> {
    let (root, project) = current_project(&target.project).wrap_err("检查当前项目失败")?;
    if target.control.framework.as_str() != project.framework {
        eyre::bail!(
            "控制 {} 不属于项目框架 '{}'",
            target.control,
            project.framework
        );
    }
    let matrix = crate::project::matrix::load(&root).wrap_err("加载项目矩阵失败")?;
    let entry = matrix
        .entries
        .get(&target.control)
        .ok_or_else(|| eyre::eyre!("控制 {} 不在项目矩阵中", target.control))
        .wrap_err("定位矩阵控制失败")?;
    Ok(action_for(entry.ingest.as_ref(), metadata, false))
}

fn apply_control_content(
    control: &ControlId,
    content: &ControlContent,
    metadata: IngestMetadata,
) -> eyre::Result<()> {
    let (path, mut document) = load_control(control).wrap_err("加载待更新控制失败")?;
    document.data.excerpt_status = content.completeness;
    document.data.last_reviewed = Some(Local::now().date_naive());
    document.data.ingest = Some(metadata);
    if let Some(expected_evidence) = &content.expected_evidence {
        document.data.expected_evidence = expected_evidence.clone();
    }

    // 未提供的段落表示材料中没有可靠内容；省略它们以避免生成貌似完整的占位知识。
    let mut body = format!("# {}\n", document.data.title);
    for (heading, section) in [
        ("要求摘要", content.requirement_summary.as_deref()),
        ("实施指引", content.implementation_guidance.as_deref()),
        ("常见缺陷", content.common_deficiencies.as_deref()),
    ] {
        if let Some(section) = section {
            body.push_str(&format!("\n## {heading}\n\n{}\n", section.trim()));
        }
    }
    let serialized =
        frontmatter::serialize(&document.data, &body).wrap_err("序列化控制内容失败")?;
    fs::write(&path, serialized).wrap_err_with(|| format!("写控制文件 {} 失败", path.display()))
}

fn apply_system_fact(
    system: &str,
    content: &SystemFactContent,
    metadata: IngestMetadata,
) -> eyre::Result<()> {
    let mut index = crate::system::fact::load_index(system)
        .wrap_err_with(|| format!("加载系统 '{system}' 索引失败"))?;
    let existing_position = index
        .facts
        .iter()
        .position(|entry| entry.external_key.as_deref() == Some(&metadata.external_key));
    let id = existing_position
        .map(|position| index.facts[position].id.clone())
        .unwrap_or_else(|| next_system_fact_id(&index));
    let related_controls = content.related_controls.clone();
    let source_kind = fact_source_type(&metadata.source.kind);
    let collected_at = metadata
        .source
        .document_date
        .unwrap_or_else(|| Local::now().date_naive());
    let fact_frontmatter = FactFrontmatter {
        id: id.clone(),
        domain: content.domain.clone(),
        title: content.title.clone(),
        tags: content.tags.clone(),
        source: FactSource {
            kind: source_kind,
            reference: format!("{}#{}", metadata.source.reference, metadata.source.locator),
            collected_at,
            collector: "agent".to_string(),
        },
        confidence: match metadata.confidence {
            IngestConfidence::High => Confidence::High,
            IngestConfidence::Medium => Confidence::Medium,
            IngestConfidence::Low => Confidence::Low,
        },
        related_controls: related_controls.clone(),
        status: FactStatus::Current,
        supersedes: Vec::new(),
        ingest: Some(metadata.clone()),
    };
    let body = format!("# {}\n\n{}\n", content.title, content.body.trim());
    let serialized =
        frontmatter::serialize(&fact_frontmatter, &body).wrap_err("序列化系统事实失败")?;
    let relative_path = format!("{}/{id}.md", content.domain);
    let system_dir = crate::system::system_dir(system)
        .wrap_err_with(|| format!("解析系统 '{system}' 目录失败"))?;
    let path = system_dir.join(&relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("创建系统事实目录 {} 失败", parent.display()))?;
    }
    if let Some(position) = existing_position {
        let old_path = system_dir.join(&index.facts[position].file);
        if old_path != path && old_path.exists() {
            fs::rename(&old_path, &path).wrap_err_with(|| {
                format!(
                    "移动系统事实 {} 到 {} 失败",
                    old_path.display(),
                    path.display()
                )
            })?;
        }
        fs::write(&path, serialized)
            .wrap_err_with(|| format!("更新系统事实 {} 失败", path.display()))?;
        index.facts[position] = FactIndexEntry {
            id,
            domain: content.domain.clone(),
            title: content.title.clone(),
            tags: content.tags.clone(),
            related_controls,
            external_key: Some(metadata.external_key),
            file: relative_path,
        };
    } else {
        fs::write(&path, serialized)
            .wrap_err_with(|| format!("创建系统事实 {} 失败", path.display()))?;
        index.facts.push(FactIndexEntry {
            id,
            domain: content.domain.clone(),
            title: content.title.clone(),
            tags: content.tags.clone(),
            related_controls,
            external_key: Some(metadata.external_key),
            file: relative_path,
        });
    }
    save_system_index(system, &index).wrap_err("保存系统事实索引失败")
}

fn apply_project_fact(
    project: &str,
    content: &ProjectFactContent,
    metadata: IngestMetadata,
) -> eyre::Result<()> {
    let (root, _) = current_project(project).wrap_err("检查当前项目失败")?;
    let mut index = crate::project::fact::load_index(&root).wrap_err("加载项目事实索引失败")?;
    let existing_position = index
        .facts
        .iter()
        .position(|entry| entry.external_key.as_deref() == Some(&metadata.external_key));
    let id = existing_position
        .map(|position| index.facts[position].id.clone())
        .unwrap_or_else(|| next_project_fact_id(&index));
    let created_at = metadata
        .source
        .document_date
        .unwrap_or_else(|| Local::now().date_naive());
    let fact_frontmatter = ProjectFactFrontmatter {
        id: id.clone(),
        kind: content.kind,
        title: content.title.clone(),
        tags: content.tags.clone(),
        control: content.control.clone(),
        created_at,
        ingest: Some(metadata.clone()),
    };
    let body = format!("# {}\n\n{}\n", content.title, content.body.trim());
    let serialized =
        frontmatter::serialize(&fact_frontmatter, &body).wrap_err("序列化项目事实失败")?;
    let relative_path = format!("{}/{id}.md", content.kind.as_str());
    let path = root.join("facts").join(&relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("创建项目事实目录 {} 失败", parent.display()))?;
    }
    if let Some(position) = existing_position {
        let old_path = root.join("facts").join(&index.facts[position].file);
        if old_path != path && old_path.exists() {
            fs::rename(&old_path, &path).wrap_err_with(|| {
                format!(
                    "移动项目事实 {} 到 {} 失败",
                    old_path.display(),
                    path.display()
                )
            })?;
        }
        fs::write(&path, serialized)
            .wrap_err_with(|| format!("更新项目事实 {} 失败", path.display()))?;
        index.facts[position] = ProjectFactIndexEntry {
            id,
            kind: content.kind,
            title: content.title.clone(),
            control: content.control.clone(),
            external_key: Some(metadata.external_key),
            file: relative_path,
        };
    } else {
        fs::write(&path, serialized)
            .wrap_err_with(|| format!("创建项目事实 {} 失败", path.display()))?;
        index.facts.push(ProjectFactIndexEntry {
            id,
            kind: content.kind,
            title: content.title.clone(),
            control: content.control.clone(),
            external_key: Some(metadata.external_key),
            file: relative_path,
        });
    }
    save_project_fact_index(&root, &index).wrap_err("保存项目事实索引失败")
}

fn apply_matrix_assessment(
    target: &MatrixTarget,
    content: &MatrixAssessmentContent,
    metadata: IngestMetadata,
) -> eyre::Result<()> {
    let (root, _) = current_project(&target.project).wrap_err("检查当前项目失败")?;
    let mut matrix = crate::project::matrix::load(&root).wrap_err("加载项目矩阵失败")?;
    let entry = matrix
        .entries
        .get_mut(&target.control)
        .ok_or_else(|| eyre::eyre!("控制 {} 不在项目矩阵中", target.control))
        .wrap_err("定位矩阵控制失败")?;
    entry.status = content.status;
    if let Some(gap) = &content.gap {
        entry.gap = gap.clone();
    }
    if let Some(remediation) = &content.remediation {
        entry.remediation = remediation.clone();
    }
    if let Some(owner) = &content.owner {
        entry.owner = owner.clone();
    }
    entry.last_updated = Some(Local::now().date_naive());
    entry.ingest = Some(metadata);
    crate::project::matrix::save(&root, &matrix).wrap_err("保存项目矩阵失败")
}

fn next_system_fact_id(index: &crate::system::fact::FactIndex) -> String {
    let max = index
        .facts
        .iter()
        .filter_map(|fact| {
            fact.id
                .strip_prefix("SYS-F-")
                .and_then(|suffix| suffix.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);
    format!("SYS-F-{:04}", max + 1)
}

fn next_project_fact_id(index: &crate::project::fact::ProjectFactIndex) -> String {
    let max = index
        .facts
        .iter()
        .filter_map(|fact| {
            fact.id
                .strip_prefix("PROJ-F-")
                .and_then(|suffix| suffix.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);
    format!("PROJ-F-{:04}", max + 1)
}

fn save_system_index(system: &str, index: &crate::system::fact::FactIndex) -> eyre::Result<()> {
    let yaml = serde_yml::to_string(index).wrap_err("序列化系统事实索引失败")?;
    let path = crate::system::system_dir(system)
        .wrap_err_with(|| format!("解析系统 '{system}' 目录失败"))?
        .join("index.yaml");
    fs::write(&path, yaml).wrap_err_with(|| format!("写系统事实索引 {} 失败", path.display()))
}

fn save_project_fact_index(
    root: &Path,
    index: &crate::project::fact::ProjectFactIndex,
) -> eyre::Result<()> {
    let yaml = serde_yml::to_string(index).wrap_err("序列化项目事实索引失败")?;
    let path = root.join("facts").join("index.yaml");
    fs::write(&path, yaml).wrap_err_with(|| format!("写项目事实索引 {} 失败", path.display()))
}

fn fact_source_type(kind: &str) -> FactSourceType {
    let kind = kind.to_ascii_lowercase();
    if kind.contains("interview") || kind.contains("访谈") {
        FactSourceType::Interview
    } else if kind.contains("scan") || kind.contains("扫描") || kind.contains("ocr") {
        FactSourceType::Scan
    } else if kind == "user" || kind.contains("用户") {
        FactSourceType::User
    } else {
        FactSourceType::Doc
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn print_plan(plan: &[PlanItem]) {
    let mut creates = 0;
    let mut updates = 0;
    let mut unchanged = 0;
    for item in plan {
        println!("{}  {}", item.action.as_str(), item.label);
        match item.action {
            PlanAction::Create => creates += 1,
            PlanAction::Update => updates += 1,
            PlanAction::Unchanged => unchanged += 1,
        }
    }
    println!("\n{creates} create, {updates} update, {unchanged} unchanged");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_external_keys() {
        let json = r#"{
          "schema_version": "complai.ingest/v1",
          "records": [
            {
              "kind": "system_fact",
              "external_key": "same",
              "source": {"type":"pdf","title":"报告","reference":"a.pdf","locator":"p.1"},
              "confidence": "high",
              "target": {"system":"s1"},
              "content": {"domain":"架构","title":"拓扑","body":"三层架构"}
            },
            {
              "kind": "system_fact",
              "external_key": "same",
              "source": {"type":"pdf","title":"报告","reference":"a.pdf","locator":"p.2"},
              "confidence": "high",
              "target": {"system":"s1"},
              "content": {"domain":"部署","title":"区域","body":"两个区域"}
            }
          ]
        }"#;
        let bundle: IngestBundle = serde_json::from_str(json).expect("测试 JSON 合法");
        assert!(validate_bundle(&bundle).is_err());
    }

    #[test]
    fn complete_control_requires_all_sections() {
        let json = r#"{
          "schema_version": "complai.ingest/v1",
          "records": [{
            "kind": "control_content",
            "external_key": "control-1",
            "source": {"type":"pdf","title":"规范","reference":"standard.pdf","locator":"p.1"},
            "confidence": "high",
            "target": {"control":"dengbao-2.0:8.1.4.1"},
            "content": {"requirement_summary":"身份鉴别", "completeness":"complete"}
          }]
        }"#;
        let bundle: IngestBundle = serde_json::from_str(json).expect("测试 JSON 合法");
        assert!(validate_bundle(&bundle).is_err());
    }

    #[test]
    fn rejects_unknown_fields() {
        let json = r#"{
          "schema_version": "complai.ingest/v1",
          "records": [{
            "kind": "system_fact",
            "external_key": "fact-1",
            "unexpected": true,
            "source": {"type":"pdf","title":"报告","reference":"a.pdf","locator":"p.1"},
            "confidence": "high",
            "target": {"system":"s1"},
            "content": {"domain":"架构","title":"拓扑","body":"三层架构"}
          }]
        }"#;
        assert!(serde_json::from_str::<IngestBundle>(json).is_err());
    }
}
