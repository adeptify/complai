# Complai Agent Skills

Complai 采用「一个 discovery skill + CLI 内置 workflow」的分发模式。Agent 客户端
只安装 `complai/SKILL.md`；实际 workflow 随 crate 编译进 `complai` 二进制，并通过
`complai skill list/get` 按需加载。这样 workflow prompt 始终与已安装的 CLI 版本一致。

## 分层

| 层级 | 内容 | 源码位置 | 加载时机 |
|---|---|---|---|
| Discovery | CLI 安装、环境初始化、workflow 路由 | `skills/complai/SKILL.md` | Agent 会话触发 `$complai` 时 |
| Discovery UI | 名称、简述和默认 prompt | `skills/complai/agents/openai.yaml` | Agent 客户端展示 skill 时 |
| Workflow | 项目初始化、文档灌库、差距分析 | `src/skills_content/<name>/SKILL.md` | `complai skill get <name>` 时 |
| Registry | workflow 名称、摘要与二进制嵌入 | `src/skill.rs` | CLI 编译及运行时 |

## Skill 目录

| 名称 | 类型 | Description | 获取方式 |
|---|---|---|---|
| `complai` | Discovery | 安装、配置并操作 CLI，根据用户任务发现和加载内置 workflow | 安装 `skills/complai/` |
| `project-init` | Workflow | 创建合规项目并绑定系统、框架与等级 | `complai skill get project-init` |
| `doc-ingest` | Workflow | 解析审计文档并导入事实、控制正文或矩阵 | `complai skill get doc-ingest` |
| `gap-analysis` | Workflow | 基于控制、事实与证据评估差距并生成报告 | `complai skill get gap-analysis` |

## 安装与发现

```sh
cargo install complai --locked
complai skill list
complai skill get project-init
```

## 维护约定

1. 只在 `skills/complai/` 维护可安装的 discovery skill。
2. 在 `src/skills_content/<name>/SKILL.md` 新增或修改 workflow，不要在顶层
   `skills/` 为每个 workflow 创建重复目录。
3. 在 `src/skill.rs` 注册名称、紧凑摘要和 `include_str!` 路径。
4. 每个 frontmatter description 都要同时说明能力和触发场景；面向人的紧凑说明
   维护在上表与 CLI registry 中。
5. 发布前验证 `complai skill list/get`、测试、Clippy 和 `cargo package`。
