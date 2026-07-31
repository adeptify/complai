# complai

合规审计 agent:Rust CLI (`complai`) + Claude Code skill。面向公司内合规工程师(准备方),
做等保 2.0 等资质的差距分析、证据收集与报告生成。

架构(共享 KB 分 `compliance`+`system`;项目=系统×框架绑定+项目事实+artifacts)详见
[PLAN.md](PLAN.md)。

## 构建

```sh
cargo build
cargo test                 # 19 个测试:单元 + 集成 + insta 快照
cargo clippy --all-targets # 无警告
```

## 环境变量

- `COMPLAI_KB_DIR`:共享知识库(默认 `~/.complai/kb`),含 `compliance/` + `system/`。
- `COMPLAI_PROJECT_DIR`:项目根(或从 cwd 上溯找 `project.yaml`)。

## 闭环示例

```sh
# 通用知识库(共享,一次性)
complai compliance scaffold dengbao-2.0              # -> compliance/dengbao-2.0/(70 控制桩)
complai compliance ingest dengbao-2.0 notes.md       # 批量摘录(@@ <id> 分块)填控制正文
complai compliance list --framework dengbao-2.0

complai system init order-platform --name 订单平台   # -> system/order-platform/(共享)
complai system add --system order-platform --domain 架构 \
  --title "微服务拓扑" --control dengbao-2.0:8.1.2 --body "..."
complai system ingest --from facts.yaml --system order-platform   # 批量灌系统事实

# 项目(绑定 system × framework)
complai project init order-platform-dengbao3 --system order-platform --framework dengbao-2.0 --level 3
cd order-platform-dengbao3
complai fact add --kind 整改 --title "payment-service 接入 MFA" \
  --control dengbao-2.0:8.1.4.1 --body "负责人张三,计划 Q3。"
complai matrix set dengbao-2.0:8.1.4.1 gap --gap "未启用多因素" --owner 张三
complai matrix link dengbao-2.0:8.1.4.1 --fact SYS-F-0001 --project-fact PROJ-F-0001
complai evidence add mfa.png --control dengbao-2.0:8.1.4.1 --type config
complai matrix trace dengbao-2.0:8.1.4.1   # 跨三层:控制正文+系统事实+项目事实+证据
complai gen report                         # -> drafts/compliance-report.md
```

`complai parse <file>.xlsx` 抽 Excel 为 Markdown 表格,供 agent 灌库。

Skills(agent 编排 CLI,均只调 CLI、不直接改文件):`project-init`(立项:绑定系统×框架)、`gap-analysis`(差距分析)、`doc-ingest`(xlsx 灌库)。

## 备注

- 内置 `data/dengbao-2.0.yaml` 控制点编号/短名按 GB/T 22239-2019 三级填(只含 ID+短名,
  不含标准要求正文;正文需手工摘录以规避版权)。正式使用前请核对。
- `gen` 是 Rust 2024 保留关键字,报告模块命名为 `reports`。
- 文件模板与初始化清单见 [docs/file-templates.md](docs/file-templates.md)。
