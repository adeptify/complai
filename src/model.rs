//! 共享领域模型:框架、控制项标识、状态等。
//!
//! 这些类型在知识库与项目层之间共用,因此放在 crate 根。
//! 一切控制项按 `ControlId` 寻址,序列化为 `<framework>:<control_id>` 字符串,
//! 以便跨项目、跨框架用最少的引用定位。

use std::fmt;
use std::str::FromStr;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Agent 抽取记录的置信度。低置信度记录默认只允许预览，不能直接写入。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IngestConfidence {
    High,
    Medium,
    Low,
}

/// 一条结构化记录在原始材料中的可审计出处。
///
/// `kind` 故意保持开放字符串：读取能力可以持续扩展到新的文件格式和云文档，
/// 而不会迫使 ingest 协议为每种载体升级版本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCitation {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub reference: String,
    pub locator: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_date: Option<NaiveDate>,
}

/// 随规范、事实和评估结果持久化的导入元数据。
///
/// `external_key` 提供跨重复导入的稳定身份，`record_sha256` 用于区分“未变化”与
/// “同一来源内容已更新”，避免每次导入都生成重复记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngestMetadata {
    pub external_key: String,
    pub record_sha256: String,
    pub source: SourceCitation,
    pub confidence: IngestConfidence,
}

/// 合规框架,如 `dengbao-2.0`、`iso27001`。作为 `ControlId` 的命名空间。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Framework(pub String);

impl Framework {
    pub fn dengbao_2_0() -> Self {
        Self("dengbao-2.0".to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Framework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 全局唯一的控制项标识:`<framework>:<control_id>`,如 `dengbao-2.0:8.1.4.1`。
///
/// 序列化为单个字符串(而非结构体),使 frontmatter 中 `id: dengbao-2.0:8.1.4.1`
/// 既紧凑又可被 `FromStr` 反解,供 `compliance show dengbao-2.0:8.1.4.1` 直接使用。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControlId {
    pub framework: Framework,
    pub control_id: String,
}

impl ControlId {
    pub fn new(framework: impl Into<String>, control_id: impl Into<String>) -> Self {
        Self {
            framework: Framework(framework.into()),
            control_id: control_id.into(),
        }
    }
}

impl fmt::Display for ControlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.framework, self.control_id)
    }
}

impl FromStr for ControlId {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (framework, control_id) = s.split_once(':').ok_or_else(|| {
            eyre::eyre!("control id `{s}` 缺少 `:` 分隔符(应为 `<framework>:<control_id>`)")
        })?;
        if framework.is_empty() || control_id.is_empty() {
            return Err(eyre::eyre!("control id `{s}` 的框架或控制编号为空"));
        }
        Ok(Self {
            framework: Framework(framework.to_string()),
            control_id: control_id.to_string(),
        })
    }
}

impl Serialize for ControlId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ControlId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// BTreeMap<ControlId, _>(控制矩阵)需要 Ord。按框架名再按 control_id 自然序,
// 使矩阵条目以人类可读的顺序迭代(8.1.4.2 在 8.1.4.10 之前)。
impl PartialOrd for ControlId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ControlId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.framework
            .0
            .cmp(&other.framework.0)
            .then_with(|| natural_control_cmp(&self.control_id, &other.control_id))
    }
}

/// 控制项在矩阵中的符合性状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlStatus {
    /// 满足
    #[serde(rename = "met")]
    Met,
    /// 部分满足
    #[serde(rename = "partial")]
    Partial,
    /// 缺口
    #[serde(rename = "gap")]
    Gap,
    /// 不适用
    #[serde(rename = "na")]
    Na,
}

impl fmt::Display for ControlStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ControlStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Met => "met",
            Self::Partial => "partial",
            Self::Gap => "gap",
            Self::Na => "na",
        }
    }
}

impl FromStr for ControlStatus {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "met" => Ok(Self::Met),
            "partial" => Ok(Self::Partial),
            "gap" => Ok(Self::Gap),
            "na" => Ok(Self::Na),
            other => Err(eyre::eyre!(
                "未知控制状态 `{other}`(应为 met/partial/gap/na)"
            )),
        }
    }
}

/// 等保控制域:技术要求 vs 管理要求。
///
/// 序列化为中文 `技术`/`管理`,与等保基本要求的两大类对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    #[serde(rename = "技术")]
    Technical,
    #[serde(rename = "管理")]
    Management,
}

impl Domain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Technical => "技术",
            Self::Management => "管理",
        }
    }
}

impl FromStr for Domain {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "技术" => Ok(Self::Technical),
            "管理" => Ok(Self::Management),
            other => Err(eyre::eyre!("未知控制域 `{other}`(应为 技术/管理)")),
        }
    }
}

/// 按 control_id 的数字段做自然排序,使 `8.1.1.2` 排在 `8.1.1.10` 之前。
///
/// 纯字符串排序会把 `8.1.1.10` 排到 `8.1.1.2` 前面,不利于人工阅读。
pub fn natural_control_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.split('.');
    let mut bi = b.split('.');
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => match (x.parse::<u64>(), y.parse::<u64>()) {
                (Ok(xn), Ok(yn)) => match xn.cmp(&yn) {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                },
                _ => match x.cmp(y) {
                    std::cmp::Ordering::Equal => continue,
                    other => return other,
                },
            },
        }
    }
}

/// 项目专属事实的类型(整改/例外/决策/发现/备注)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectFactKind {
    #[serde(rename = "整改")]
    Remediation,
    #[serde(rename = "例外")]
    Exception,
    #[serde(rename = "决策")]
    Decision,
    #[serde(rename = "发现")]
    Finding,
    #[serde(rename = "备注")]
    Note,
}

impl ProjectFactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Remediation => "整改",
            Self::Exception => "例外",
            Self::Decision => "决策",
            Self::Finding => "发现",
            Self::Note => "备注",
        }
    }

    pub fn parse(s: &str) -> eyre::Result<Self> {
        match s {
            "整改" => Ok(Self::Remediation),
            "例外" => Ok(Self::Exception),
            "决策" => Ok(Self::Decision),
            "发现" => Ok(Self::Finding),
            "备注" => Ok(Self::Note),
            other => Err(eyre::eyre!(
                "未知项目事实类型 `{other}`(应为 整改/例外/决策/发现/备注)"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_id_display_parse_roundtrip() {
        let id = ControlId::new("dengbao-2.0", "8.1.4.1");
        assert_eq!(id.to_string(), "dengbao-2.0:8.1.4.1");
        let parsed: ControlId = id.to_string().parse().unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn control_id_parse_rejects_bad() {
        assert!("no-colon".parse::<ControlId>().is_err());
        assert!(":8.1.4.1".parse::<ControlId>().is_err());
        assert!("dengbao-2.0:".parse::<ControlId>().is_err());
    }

    #[test]
    fn natural_control_cmp_orders_numerically() {
        use std::cmp::Ordering;
        assert_eq!(natural_control_cmp("8.1.1.2", "8.1.1.10"), Ordering::Less);
        assert_eq!(
            natural_control_cmp("8.1.1.10", "8.1.1.9"),
            Ordering::Greater
        );
        assert_eq!(natural_control_cmp("8.1.4.1", "8.1.4.1"), Ordering::Equal);
    }

    #[test]
    fn control_id_ord_keeps_btreemap_natural() {
        use std::collections::BTreeMap;
        let mut m = BTreeMap::new();
        m.insert(ControlId::new("dengbao-2.0", "8.1.1.10"), 10);
        m.insert(ControlId::new("dengbao-2.0", "8.1.1.2"), 2);
        let keys: Vec<_> = m.keys().collect();
        assert_eq!(keys[0].control_id, "8.1.1.2");
        assert_eq!(keys[1].control_id, "8.1.1.10");
    }

    #[test]
    fn control_status_roundtrip() {
        for s in ["met", "partial", "gap", "na"] {
            let st: ControlStatus = s.parse().unwrap();
            assert_eq!(st.to_string(), s);
        }
        assert!("bogus".parse::<ControlStatus>().is_err());
    }

    #[test]
    fn domain_parse() {
        assert_eq!(Domain::from_str("技术").unwrap(), Domain::Technical);
        assert_eq!(Domain::from_str("管理").unwrap(), Domain::Management);
        assert!(Domain::from_str("x").is_err());
    }
}
