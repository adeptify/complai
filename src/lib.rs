//! complai:合规审计 agent 的命令行工具与库。
//!
//! 详见仓库根 `PLAN.md`。本期(MVP)聚焦等保 2.0 的通用知识库与项目闭环。

pub mod cli;
pub mod compliance;
pub mod error;
pub mod frontmatter;
pub mod model;
pub mod parse;
pub mod paths;
pub mod project;
pub mod reports;
pub mod system;
