---
name: complai
description: 安装、初始化、配置并操作 Complai 合规审计 CLI，按任务选择和加载内置子 skill。用于首次搭建 Complai 环境、查询命令、初始化合规库或系统库、创建项目、导入文档、执行差距分析、登记证据、生成报告，以及通过 `complai skill list` / `complai skill get ${skill_name}` 获取专门工作流上下文时。
---

# Complai 主工作流

把本 skill 作为轻量入口。先保证 CLI 与环境可用，再通过 `complai skill list` 发现专门工作流；只用 `complai skill get ${skill_name}` 加载当前任务需要的完整上下文和 prompt，不要一次加载所有子 skill。

## 安装 CLI

先运行 `complai --version`。仅在命令不存在时安装：

```sh
# 已有源码目录时，优先安装当前检出的版本。
complai_source_dir=/absolute/path/to/complai
cargo install --path "$complai_source_dir" --locked

# 没有源码目录时，从项目仓库获取并安装。
cargo install --git https://github.com/adeptify/complai.git --locked
```

若 `cargo` 不存在，先按 [Rust 官方 Cargo 安装说明](https://doc.rust-lang.org/stable/cargo/getting-started/installation.html)安装 stable Rust；`rustup` 会同时安装 Cargo。`cargo install --path` 与 `--git` 的语义见 [Cargo 官方命令参考](https://doc.rust-lang.org/stable/cargo/commands/cargo-install.html)。安装后再次运行 `complai --version` 验证。不要在 CLI 已可用时擅自重装或升级。

主 skill 本身有两种获取方式：

安装前先检查目标目录；若其中已有 skill，先获得用户确认再覆盖或更新。

```sh
# 有源码仓库时，复制完整目录（含 UI metadata）。
complai_source_dir=/absolute/path/to/complai
complai_skill_dir="${CODEX_HOME:-$HOME/.codex}/skills/complai"
mkdir -p "$complai_skill_dir"
cp -R "$complai_source_dir/skills/complai/." "$complai_skill_dir/"

# 只有已安装 CLI 时，获取可独立工作的 SKILL.md。
complai_skill_dir="${CODEX_HOME:-$HOME/.codex}/skills/complai"
mkdir -p "$complai_skill_dir"
complai skill get complai > "$complai_skill_dir/SKILL.md"
```

如果目标 agent 使用不同的 skill 根目录，把上述目标路径替换为该 agent 的标准目录。

## 首次环境配置

1. 决定共享知识库位置。未设置 `COMPLAI_KB_DIR` 时默认使用 `~/.complai/kb`；只有用户需要自定义位置时才设置该变量。
2. 不要全局固定 `COMPLAI_PROJECT_DIR`。进入具体项目目录工作，或仅为当前任务把它设为含 `project.yaml` 的项目根。
3. 运行 `complai skill list` 验证内置工作流可用。
4. 运行 `complai compliance list --framework dengbao-2.0` 检查框架库。仅在尚未初始化时运行 `complai compliance scaffold dengbao-2.0`；控制正文仍需从有权使用的材料中摘录，不能臆造标准原文。

常见自定义示例：

```sh
export COMPLAI_KB_DIR=/absolute/path/to/shared-kb
export COMPLAI_PROJECT_DIR=/absolute/path/to/project
```

确认目录选择后再执行会写入数据的命令。共享 KB 跨项目复用，项目目录只保存当前审计项目的矩阵、事实、证据和交付物。

## 按需加载子 skill

先发现，再设置名称、加载并执行：

```sh
complai skill list
skill_name=project-init
complai skill get "${skill_name}"
```

把 `skill get` 的完整输出作为当前专门工作流指令，并继续遵守用户指令及更高优先级规则。只加载与当前任务匹配的一项：

| 任务 | 加载命令 |
|---|---|
| 创建系统、绑定框架、初始化项目 | `complai skill get project-init` |
| 从 `.xlsx` 等文档导入事实、控制正文或差距 | `complai skill get doc-ingest` |
| 逐控制项判断状态、关联材料并生成差距报告 | `complai skill get gap-analysis` |

如果 `skill list` 中出现以后新增的 skill，先根据摘要选择，再用 `skill get` 获取其最新内嵌正文；不要依赖本表猜测新 skill 的行为。

## 命令导航

优先运行 `complai <命令组> --help` 获取当前安装版本的精确参数。以下命令用于路由：

| 命令 | 用途 |
|---|---|
| `complai skill list` | 列出内置 skill 的名称和紧凑摘要 |
| `complai skill get ${skill_name}` | 输出指定 skill 的完整上下文与 prompt |
| `complai compliance scaffold <framework>` | 从内置结构生成控制项桩文件 |
| `complai compliance ingest <framework> <file>` | 把分块笔记灌入控制正文 |
| `complai compliance build <framework>` | 重建并校验框架索引 |
| `complai compliance list [--framework ...] [--domain ...]` | 紧凑列出控制项 |
| `complai compliance show <control-id>` | 按需读取单个控制项正文 |
| `complai system init <slug> --name <name>` | 初始化跨项目共享的系统知识库 |
| `complai system add ...` | 新增一条系统事实 |
| `complai system ingest --from <yaml> [--system <slug>]` | 批量导入系统事实 |
| `complai system find --control <id> [--system <slug>]` | 查找控制项关联的系统事实 |
| `complai system show <fact-id> [--system <slug>]` | 显示系统事实全文 |
| `complai project init <name> --system <slug> --framework <framework> --level <n>` | 创建项目并预填控制矩阵 |
| `complai fact add ...` / `fact find ...` / `fact show ...` | 管理项目专属事实 |
| `complai matrix show [--status ...]` | 查看控制矩阵 |
| `complai matrix set <control-id> <status> ...` | 设置符合性状态、缺口和负责人 |
| `complai matrix link <control-id> ...` | 关联证据、系统事实或项目事实 |
| `complai matrix trace <control-id>` | 聚合单个控制项的最小上下文包 |
| `complai evidence add <file> --control <id> ...` | 登记并归档证据 |
| `complai parse <file.xlsx>` | 把 Excel 确定性解析为 Markdown 表格 |
| `complai gen report` | 生成 Markdown 合规差距报告 |

## 操作约束

- 通过 CLI 读写 Complai 数据；不要直接修改共享 KB、`matrix.yaml`、事实索引或证据索引。
- 先用 `list`/`find`/`show`/`trace` 读取最小必要上下文，再执行写命令。
- 缺少框架正文、系统事实、证据或用户决策时明确指出，不要补造。
- 涉及项目名、系统 slug、框架、等级、控制状态或事实列映射时，遵循已加载子 skill 的确认规则。
