//! complai:合规审计 agent 的命令行工具与库。
//!
//! 详见仓库根 `PLAN.md`。内置结构表覆盖等保 2.0；其他合规框架可通过
//! 统一 ingest 协议创建控制库，并复用同一项目、矩阵、证据和报告闭环。

pub mod cli;
pub mod compliance;
pub mod error;
pub mod frontmatter;
pub mod ingest;
pub mod model;
pub mod paths;
pub mod project;
pub mod reports;
mod revision;
pub mod skill;
mod storage;
pub mod system;
