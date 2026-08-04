//! `complai` 命令行界面定义。

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "complai",
    version,
    about = "合规审计 agent 的命令行工具:合规库、系统、项目、矩阵、事实、证据、报告"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 按需获取内置 agent skill 的上下文与工作流
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    /// 从 Agent 生成的版本化 JSON bundle 校验、预览并写入知识库
    Ingest {
        #[command(subcommand)]
        command: IngestCommand,
    },
    /// 合规知识库(框架控制项,跨项目共享)
    Compliance {
        #[command(subcommand)]
        command: ComplianceCommand,
    },
    /// 业务系统知识库(系统事实,跨项目共享,按 system slug)
    System {
        #[command(subcommand)]
        command: SystemCommand,
    },
    /// 项目管理
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// 项目专属事实(整改/例外/决策/发现/备注)
    Fact {
        #[command(subcommand)]
        command: FactCommand,
    },
    /// 控制矩阵
    Matrix {
        #[command(subcommand)]
        command: MatrixCommand,
    },
    /// 证据库
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// 生成报告/交付物
    Gen {
        #[command(subcommand)]
        command: GenCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum SkillCommand {
    /// 列出可按需加载的内置 skill
    List,
    /// 输出指定 skill 的完整上下文与 prompt
    Get { skill_name: String },
}

#[derive(Subcommand, Debug)]
pub enum IngestCommand {
    /// 输出当前 ingest JSON Schema
    Schema,
    /// 校验 JSON 协议和字段约束，不访问写入目标
    Validate {
        #[arg(long = "from")]
        source: String,
    },
    /// 校验目标并预览 create/update/unchanged，不写文件
    Plan {
        #[arg(long = "from")]
        source: String,
    },
    /// 预检通过后幂等写入；低置信度记录默认拒绝
    Apply {
        #[arg(long = "from")]
        source: String,
        #[arg(long)]
        allow_low_confidence: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ComplianceCommand {
    /// 按内置结构表生成控制项桩文件
    Scaffold { framework: String },
    /// 遍历框架目录,生成 index.yaml 并校验
    Build { framework: String },
    /// 显示单个控制项全文(按需加载正文)
    Show { id: String },
    /// 列出控制项(只读紧凑索引)
    List {
        #[arg(long)]
        framework: Option<String>,
        #[arg(long)]
        domain: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum SystemCommand {
    /// 创建一个共享系统知识库
    Init {
        slug: String,
        #[arg(long)]
        name: String,
    },
    /// 新增一条系统事实(默认写当前项目引用的系统,或 --system)
    Add {
        #[arg(long)]
        system: Option<String>,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        control: Option<String>,
        #[arg(long = "type", default_value = "user")]
        kind: String,
        #[arg(long = "ref")]
        reference: Option<String>,
        #[arg(long)]
        body: Option<String>,
    },
    /// 显示一条系统事实全文
    Show {
        #[arg(long)]
        system: Option<String>,
        id: String,
    },
    /// 查找关联到某控制的系统事实
    Find {
        #[arg(long)]
        system: Option<String>,
        #[arg(long)]
        control: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProjectCommand {
    /// 初始化项目工作区(指定系统 + 框架)
    Init {
        name: String,
        #[arg(long)]
        system: String,
        #[arg(long)]
        framework: String,
        /// 框架定义了级别时指定，例如等保三级
        #[arg(long)]
        level: Option<u8>,
    },
    /// 显示当前项目绑定的系统、框架、级别和 KB revision 状态
    Show,
    /// 审阅 KB 变更后，把当前 framework/system revision 写入项目
    Sync,
}

#[derive(Subcommand, Debug)]
pub enum FactCommand {
    /// 新增一条项目专属事实
    Add {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        control: Option<String>,
        #[arg(long)]
        body: Option<String>,
    },
    /// 显示一条项目事实全文
    Show { id: String },
    /// 查找关联到某控制的项目事实
    Find {
        #[arg(long)]
        control: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum MatrixCommand {
    /// 列出矩阵条目
    Show {
        #[arg(long)]
        status: Option<String>,
    },
    /// 设置控制项状态
    Set {
        control: String,
        /// unassessed/met/partial/gap/na
        status: String,
        /// partial/gap 的缺口，或 na 的不适用理由
        #[arg(long)]
        gap: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        /// 整改计划或完成情况
        #[arg(long)]
        remediation: Option<String>,
    },
    /// 给控制项关联证据/系统事实/项目事实
    Link {
        control: String,
        #[arg(long)]
        evidence: Option<String>,
        #[arg(long)]
        fact: Option<String>,
        #[arg(long = "project-fact")]
        project_fact: Option<String>,
    },
    /// 聚合控制正文 + 系统事实 + 项目事实 + 证据(最小上下文包)
    Trace { control: String },
}

#[derive(Subcommand, Debug)]
pub enum EvidenceCommand {
    /// 登记一条证据(算 sha256,按控制点就近存放)
    Add {
        file: String,
        #[arg(long)]
        control: String,
        #[arg(long = "type", default_value = "screenshot")]
        kind: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// 列出项目中的全部证据
    List,
    /// 显示一条证据的详细信息
    Show { id: String },
    /// 查找关联到某个控制项的证据
    Find {
        #[arg(long)]
        control: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum GenCommand {
    /// 生成合规差距报告(Markdown,按域分组)
    Report,
}
