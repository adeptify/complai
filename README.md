# complai

合规审计 agent:Rust CLI (`complai`) + 跨客户端 Agent Skill。面向公司内合规工程师
(准备方),做等保 2.0 等资质的差距分析、证据收集与报告生成。

架构(共享 KB 分 `compliance`+`system`;项目=系统×框架绑定+项目事实+artifacts)详见
[PLAN.md](PLAN.md)。

## 构建

```sh
cargo build
cargo test                 # 单元 + 集成 + insta 快照
cargo clippy --all-targets # 无警告
```

从 crates.io 安装已发布版本:

```sh
cargo install complai --locked
```

## 环境变量

- `COMPLAI_KB_DIR`:共享知识库(默认 `~/.complai/kb`),含 `compliance/` + `system/`。
- `COMPLAI_PROJECT_DIR`:项目根(或从 cwd 上溯找 `project.yaml`)。

## 闭环示例

```sh
# 通用知识库(共享,一次性)
complai compliance scaffold dengbao-2.0              # -> compliance/dengbao-2.0/(70 控制桩)
complai compliance list --framework dengbao-2.0

complai system init order-platform --name 订单平台   # -> system/order-platform/(共享)

# 项目(绑定 system × framework)
complai project init order-platform-dengbao3 --system order-platform --framework dengbao-2.0 --level 3
cd order-platform-dengbao3

# Agent 从 Excel/PDF/Word/图片/云文档抽取后生成统一 JSON bundle
complai ingest schema
complai ingest validate --from tmp/complai-ingest.json
complai ingest plan --from tmp/complai-ingest.json
complai ingest apply --from tmp/complai-ingest.json

complai matrix link dengbao-2.0:8.1.4.1 --fact SYS-F-0001 --project-fact PROJ-F-0001
complai evidence add mfa.png --control dengbao-2.0:8.1.4.1 --type config
complai matrix trace dengbao-2.0:8.1.4.1   # 跨三层:控制正文+系统事实+项目事实+证据
complai gen report                         # -> drafts/compliance-report.md
```

原始材料不要求 Complai 专用格式。Agent 使用当前环境可用的 PDF、文档、表格、
OCR、Connector 或浏览器能力读取内容，再按 `complai ingest schema` 生成受控 JSON。
CLI 负责协议与目标校验、变更预览、幂等写入以及来源追踪。

`0.3.0` 起不再提供 `complai parse`、`compliance ingest`、`system ingest`，
也不再接受 `facts.yaml` 或 `notes.md`；所有 Agent 批量写入统一使用版本化 JSON
bundle。少量人工维护仍可使用 `system add`、`fact add` 和 `matrix set/link`。

## Agent skills

参照 discovery stub + CLI 内置工作流模式,agent 客户端只安装
`skills/complai/SKILL.md`。真正的 workflow prompt 放在 `src/skills_content/`,
随 crate 编译进二进制,因此内容始终与已安装的 `complai` 版本一致,不会出现
skill 缓存与 CLI 版本漂移。完整分层、description 和维护约定见
[`skills/SKILLS.md`](skills/SKILLS.md)。

CLI 与 Agent Skill 分别安装：Cargo 管理包含实际 workflow 的二进制，
[`npx skills`](https://github.com/vercel-labs/skills) 管理各 Agent 客户端中的轻量
discovery skill。只使用 CLI 不需要 Node.js；安装 Agent Skill 时需要 Node.js/npm
提供的 `npx`。

```sh
cargo install complai --locked
npx skills add adeptify/complai --skill complai

complai skill list
complai skill get project-init
```

`npx skills` 默认安装到当前项目。全局安装到 Codex、从本地 checkout 开发安装，
以及后续管理可分别使用：

```sh
npx skills add adeptify/complai --skill complai --agent codex --global --yes
npx skills add ./skills/complai --agent codex
npx skills list
npx skills update complai        # 全局安装时增加 --global
npx skills remove complai        # 全局安装时增加 --global
```

`npx skills update` 只更新 discovery skill；升级 CLI 仍运行
`cargo install complai --locked`，以便内置 workflow 与二进制版本保持一致。
项目级安装会在使用方项目生成 `skills-lock.json`，应与该项目一同提交，以便后续
`list`、`update` 和 `remove` 使用同一来源。

当前内置 workflow:

- `project-init`:立项并绑定系统与框架。
- `doc-ingest`:从任意可访问材料抽取并统一灌入知识库或矩阵。
- `gap-analysis`:逐控制项分析差距并生成报告。

新增或修改 workflow 时,编辑 `src/skills_content/<name>/SKILL.md` 并注册到
`src/skill.rs`;顶层 `skills/` 只保留 `complai` discovery skill。
发布前运行 `scripts/check-agent-skills.sh`，确保 `npx skills` 默认只发现该入口。

## 备注

- 内置 `data/dengbao-2.0.yaml` 控制点编号/短名按 GB/T 22239-2019 三级填(只含 ID+短名,
  不含标准要求正文；正文须由 Agent 从用户有权使用的材料中抽取并用自己的话概述)。
  正式使用前请核对。
- `gen` 是 Rust 2024 保留关键字,报告模块命名为 `reports`。
- 文件模板与初始化清单见 [docs/file-templates.md](docs/file-templates.md)。
