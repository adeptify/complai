---
name: complai
description: >-
  安装、升级、配置并操作 Complai 合规审计 CLI，通过内置工作流管理框架控制库、
  系统事实、项目矩阵、证据和报告。用于用户要求通过 Cargo 安装 CLI 或通过
  `npx skills` 安装 Complai Agent Skill、初始化等保等合规项目、导入审计材料、
  开展差距分析、维护控制矩阵、收集证据或生成合规报告时。触发词包括 install
  complai、set up complai、初始化合规项目、等保项目、文档灌库、差距分析、
  控制矩阵、证据收集和合规报告。
---

# Complai CLI

把本 skill 作为轻量入口。实际 workflow guide 与 prompt 编译在 `complai` CLI 中，与安装的 crate 版本保持一致；不要从源码目录直接读取子 skill，也不要一次加载全部 workflow。

## 分发边界

- `npx skills` 只安装或更新本 discovery skill，不安装 `complai` 二进制，也不单独
  分发内置 workflow。
- Cargo 安装或升级 CLI；`complai skill list/get` 从当前二进制发现和加载 workflow。
- 用户要求安装或更新 Agent Skill 时运行
  `npx skills add adeptify/complai --skill complai` 或 `npx skills update complai`；
  全局安装增加 `--global`。

## 安装与验证

先运行 `complai --version`。仅在命令不存在或用户明确要求升级时，通过 crates.io 安装：

```sh
cargo install complai --locked
complai --version
```

若 `cargo` 不存在，提示用户先安装 stable Rust 与 Cargo，再继续。不要改用源码安装或未经确认的下载脚本。

## 初始化环境

- 共享知识库默认位于 `~/.complai/kb`。仅在用户指定其他位置时设置 `COMPLAI_KB_DIR`。
- 进入包含 `project.yaml` 的项目目录工作；只有无法进入该目录时才为当前任务设置 `COMPLAI_PROJECT_DIR`。
- 先运行 `complai compliance list --framework dengbao-2.0` 检查框架库。仅在尚未初始化时运行 `complai compliance scaffold dengbao-2.0`。
- 控制正文必须来自用户有权使用的材料；不能臆造标准原文。

## 按需加载工作流

先发现，再加载与当前任务匹配的一个 workflow：

```sh
complai skill list
skill_name=project-init
complai skill get "${skill_name}"
```

把 `skill get` 的完整输出作为当前工作流指令，并继续遵守用户指令及更高优先级规则。
只加载 `complai skill list` 实际列出的名称；该输出是当前 CLI 版本的权威索引。

| 用户任务 | 加载命令 |
|---|---|
| 创建系统、绑定框架、初始化项目 | `complai skill get project-init` |
| 从任意可访问文档导入事实、控制正文或差距 | `complai skill get doc-ingest` |
| 逐控制项判断状态、关联材料并生成差距报告 | `complai skill get gap-analysis` |

上表是当前常见任务的路由提示。如果 `skill list` 与上表不同，以 CLI 输出为准，
根据摘要选择名称后再用 `skill get` 获取正文；不要假设旧名称仍然可用。

## 命令入口

```sh
complai compliance --help  # 框架控制库
complai ingest --help      # 统一结构化导入
complai system --help      # 跨项目共享的系统事实
complai project --help     # 项目初始化
complai fact --help        # 项目专属事实
complai matrix --help      # 控制矩阵与最小上下文追踪
complai evidence --help    # 证据登记
complai gen --help         # 报告生成
```

在执行多步骤任务前加载对应 workflow；单一命令的精确参数以当前安装版本的 `--help` 为准。

## 操作约束

- 通过 CLI 读写 Complai 数据，不直接修改共享 KB、矩阵、事实或证据索引。
- 先用 `list`、`find`、`show` 或 `trace` 读取最小必要上下文，再执行写命令。
- 批量抽取只使用统一 ingest bundle；先获取 `ingest schema`，再依次运行
  `ingest validate` 和 `ingest plan`，确认计划后才 `ingest apply`。
- 缺少框架正文、系统事实、证据或用户决策时明确指出，不要补造。
- 项目名、系统 slug、框架、等级、状态和抽取映射按已加载 workflow 的确认规则处理。
