//! 合规知识库的控制项 schema。
//!
//! 每个控制项是一个 Markdown+frontmatter 文件。frontmatter 可被索引与查询,
//! 正文是我们自己写的摘要/指引(不复录标准原文,见版权约束)。

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::model::{ControlId, Domain, Framework, IngestMetadata};

/// 知识库摘录状态:控制项正文是否已手工填充。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExcerptStatus {
    #[serde(rename = "empty")]
    #[default]
    Empty,
    #[serde(rename = "partial")]
    Partial,
    #[serde(rename = "complete")]
    Complete,
}

impl ExcerptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Partial => "partial",
            Self::Complete => "complete",
        }
    }
}

/// 控制项 frontmatter(可查询的结构化字段)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFrontmatter {
    /// 全局唯一 ID,如 `dengbao-2.0:8.1.4.1`。
    pub id: ControlId,
    pub framework: Framework,
    pub domain: Domain,
    /// 控制类别,如 `安全计算环境`。
    pub category: String,
    /// 框架内原始编号,如 `8.1.4.1`。
    pub control_id: String,
    /// 控制点短名,如 `身份鉴别`。
    pub title: String,
    /// 适用等级,如 `[3]`。
    #[serde(default)]
    pub levels: Vec<u8>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 跨框架映射:框架名 -> 对应控制 ID 列表。只存 ID,正文各自存放。
    #[serde(default)]
    pub mappings: BTreeMap<String, Vec<ControlId>>,
    /// 期望证据类型,指导证据采集。
    #[serde(default)]
    pub expected_evidence: Vec<String>,
    #[serde(default)]
    pub excerpt_status: ExcerptStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reviewed: Option<NaiveDate>,
    /// 最近一次结构化导入的来源。手工维护或旧版控制项可以没有该字段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingest: Option<IngestMetadata>,
}

/// 一个完整的控制项:frontmatter + Markdown 正文。
#[derive(Debug, Clone)]
pub struct Control {
    pub fm: ControlFrontmatter,
    pub body: String,
}

/// 索引中的单条紧凑记录(无正文),用于导航与最小上下文检索。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlIndexEntry {
    pub id: ControlId,
    pub title: String,
    pub domain: Domain,
    pub category: String,
    #[serde(default)]
    pub levels: Vec<u8>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub mappings: BTreeMap<String, Vec<ControlId>>,
    pub excerpt_status: ExcerptStatus,
    /// 相对框架目录的文件路径。
    pub file: String,
}

/// 框架的紧凑索引(生成产物,`compliance build` 写入 `index.yaml`)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlIndex {
    pub framework: Framework,
    pub controls: Vec<ControlIndexEntry>,
}
