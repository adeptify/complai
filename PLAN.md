# Complai 当前架构与 Roadmap

## 产品目标

Complai 是供 AI Agent 调用的本地合规工程 CLI。它不限定合规框架，也不负责读取
特定文档格式；Agent 负责理解用户有权使用的材料，Complai 负责把结构化结果安全、
可追溯地写入框架知识库、系统知识库和评估项目。

核心模型：

```text
一个项目 = 一个系统 × 一个合规框架 × 一次评估范围
```

## 当前存储模型

```text
$COMPLAI_KB_DIR/                         # 默认 ~/.complai/kb
├── .complai.lock                        # 所有写操作的跨进程锁
├── compliance/<framework>/
│   ├── index.yaml
│   └── controls-or-framework-layout/
└── system/<slug>/
    ├── index.yaml
    └── <safe-domain>/SYS-F-NNNN.md

<project>/
├── project.yaml                         # system + framework + optional level
├── matrix.yaml                          # 状态与事实/证据引用
├── facts/index.yaml
├── facts/<kind>/PROJ-F-NNNN.md
├── evidence.yaml
├── evidence/<control>/<evidence-id>-<filename>
└── drafts/compliance-report.md
```

框架控制域、类别和级别均由框架定义。等保 2.0 三级可通过内置结构表 scaffold；
ISO、NIST、SOC 2、PCI DSS 等其他框架通过统一 ingest 协议创建控制项。

## 当前写入不变式

- 所有外部标识在用于物理路径前转换为安全、稳定的路径段。
- 索引中保存的相对路径在读取时重新验证，不能逃出所属根目录。
- 所有状态文件使用同目录临时文件完成原子替换。
- 写命令通过 KB 根目录文件锁串行化，避免 ID 分配和索引更新竞争。
- ingest 在完整 plan 通过后执行；普通错误会回滚本 bundle 已写文件。
- 等保结构表显式保存控制 ID，重排 YAML 不改变控制身份。
- 矩阵、证据和事实的控制关联在同一事务中同步。
- 项目矩阵初始状态为 `unassessed`；`partial`、`gap` 和 `na` 必须提供理由。

## 主要命令面

- `compliance scaffold/build/list/show`：管理共享框架控制库。
- `system init/add/find/show`：管理跨项目复用的系统事实。
- `project init/show`：绑定已存在的系统与非空框架。
- `fact add/find/show`：管理项目专属发现、整改、例外、决策和备注。
- `evidence add/list/find/show`：登记不可变证据副本。
- `matrix show/set/link/trace`：维护评估状态与可追溯关系。
- `ingest schema/validate/plan/apply`：执行版本化、幂等的 Agent 批量写入。
- `gen report`：生成当前项目的 Markdown 合规差距报告。

## 后续 Roadmap

### KB 版本与团队协作

为 framework KB 和 system KB 定义不可变版本标识，并在 `project.yaml` 中钉住所用
版本，使历史矩阵与报告可以复现。团队共享方案需要同时设计发布、拉取、冲突处理和
控制 ID 退役规则。

### 更强的崩溃恢复

当前单文件不会截断，普通运行时错误会回滚 ingest；进程或机器在多文件提交中途崩溃
时仍可能留下已完成的原子更新。后续可增加落盘事务日志，在下次启动时自动完成提交
或回滚。

### 事实演化与复用

补齐系统事实 `supersedes` 的版本链和过期策略，并评估项目事实是否需要从单控制扩展
为多控制关系。证据继续保持项目级，跨项目复用需要先定义访问边界与失效规则。

### 更多分发渠道

当前发布 crates.io 包。若要支持 `uv tool install complai`，需要引入 Maturin 二进制
wheel、PyPI 包名和 Linux/macOS/Windows 发布矩阵。

发布操作遵循 [docs/releasing.md](docs/releasing.md)。
