# complai 重构:系统知识共享化 + 项目专属事实

## 目标
把系统知识从「项目私有」提升为「共享 KB」(跨框架/跨年度复用:同一系统今年过等保、明年过 ISO,架构/数据流/资产是同一份)。项目变为「系统 × 框架绑定 + artifacts + 项目专属事实」。共享 KB 分 `compliance` + `system` 两部分。

## 新存储布局

```
$COMPLAI_KB_DIR (默认 ~/.complai/kb)/
├── compliance/<framework>/                  # 原 kb/<framework>/ 迁移到这
│   ├── index.yaml
│   └── 技术|管理/<category>/<id>.md
└── system/<slug>/                           # 新:共享业务系统知识
    ├── index.yaml                           # display_name + facts 索引
    └── <domain>/SYS-F-NNNN.md               # 架构/部署/资产/...

projects/<name>/
├── project.yaml        # name + system(引用) + framework(引用) + level
├── matrix.yaml         # entries 各加 project_facts 字段
├── facts/              # 新:项目专属事实(整改/例外/决策/发现/备注)
│   ├── index.yaml
│   └── <kind>/PROJ-F-NNNN.md
├── evidence.yaml + evidence/                # 不变(项目特有)
└── drafts/                                  # 不变(报告等 artifacts)
```

`project.yaml`:
```yaml
name: order-platform-dengbao3-2026
system: order-platform        # 引用共享 system KB(slug)
framework: dengbao-2.0        # 引用共享 compliance KB
level: 3
```

## 数据模型变更

- `FactIndex`(system)+ `display_name: Option<String>`;slug 隐式为目录名。
- `ProjectMeta` + `system: String`(slug 引用)。
- `MatrixEntry` + `project_facts: Vec<String>`(PROJ-F 引用);`facts` 仍为 SYS-F(系统事实)。
- 新 `ProjectFact` schema:frontmatter `{ id, kind, title, tags, control: Option<ControlId>, status, created_at }` + body;`kind` 枚举 `整改/例外/决策/发现/备注`(serde 中文 rename,兼作子目录名)。`ProjectFactIndex` + entries。
- system slug:ASCII(字母/数字/连字符),作目录名与 project.yaml 引用值;中文显示名存 system index.yaml。

## 命令变更

### `compliance`(compliance KB)- 路径加一层 `compliance/`
- `compliance scaffold/build/show/list` -> 操作 `compliance/<framework>/`。
- 新增 `compliance_root() = kb_root/compliance/`;原 `framework_dir` 改名 `compliance_dir`。

### `system`(共享 system KB,从项目迁出)
- 新 `system init <slug> --name "<display>"` -> 建 `system/<slug>/index.yaml`(display_name + 空 facts)。
- `system add --domain --title [--control] [--type] [--ref] [--body]` -> 写 `system/<slug>/`。**slug 默认取当前 project.yaml 的 system 字段**,或 `--system <slug>` 显式指定。
- `system show <id>` / `system find --control <id>` -> 同上,slug 默认项目引用。
- 新 `system_root() = kb_root/system/`、`system_dir(slug)`;`system_current_slug()` 从项目读 system 引用。

### `project`
- `project init <name> --system <slug> --framework <f> --level <l>` -> 写 project.yaml(含 system 引用);若 `system/<slug>` 不存在则建(display_name=slug,提示可用 `system init` 改名);从 compliance KB 索引预填矩阵。

### `fact`(项目专属事实,新命令组)
- `fact add --kind <整改|例外|决策|发现|备注> --title <t> [--control <id>] [--body <text>]` -> `facts/<kind>/PROJ-F-NNNN.md`。
- `fact show <id>` / `fact find --control <id>`。

### `matrix`
- `matrix link <control> [--evidence <id>] [--fact <id>] [--project-fact <id>]` -> `--fact`=SYS-F(系统),`--project-fact`=PROJ-F。
- `matrix trace <control>` -> 控制正文(compliance KB)+ 系统事实(共享 system KB,按项目 system 引用)+ 项目事实(项目 `facts/`)+ 证据(项目)+ 矩阵状态。
- `matrix show/set` -> 基本不变(show 多显示 project_facts 计数)。

### `ingest`(统一 Agent 写入协议)
- `ingest schema` 输出版本化 JSON Schema。
- `ingest validate/plan/apply --from <bundle.json>` 统一写入控制内容、系统事实、
  项目事实和矩阵评估；保存来源定位与置信度，并按 `external_key` 幂等 upsert。
- 原始 Excel、PDF、Word、图片和云文档由 Agent 使用当前环境的读取能力处理，
  CLI 不再提供格式专用 `parse` 或目标专用批量 ingest 命令。

## 模块/代码变更

- `src/kb/mod.rs`:加 `compliance_root()`/`compliance_dir()`;`framework_dir` -> `compliance_dir`。
- `src/compliance/{scaffold,build,query,control}.rs`:操作 `compliance/`。
- 新 `src/system/` 模块(`mod.rs` + `fact.rs` + `init.rs`):共享 Fact schema 与
  add/show/find；批量写入统一收敛到 `src/ingest.rs`。
- `src/project/system.rs` 删除(fact 逻辑迁到 `src/system/`)。
- `src/project/{mod,init}.rs`:init 加 `--system`;project 不再建 `system/`;改建 `facts/` 空索引。
- 新 `src/project/fact.rs`:ProjectFact schema + add/show/find(项目专属)。
- `src/project/matrix.rs`:`MatrixEntry` + `project_facts`;`link` 加 `--project-fact`;`trace` 跨三层(compliance + system + project facts + evidence)拉取。
- `src/cli.rs` + `src/main.rs` + `src/lib.rs`:加 `system init`、`fact` 命令组、`project init --system`、`matrix link --project-fact`、`system *` 的 `--system` 选项。
- `src/model.rs`:加 `ProjectFactKind` 枚举。
- `src/ingest.rs` + `schemas/ingest-v1.schema.json`:统一协议、严格校验、计划、
  幂等写入和来源追踪；删除 `src/parse.rs` 与旧的专用 ingest 实现。

## demo 数据 & 测试

- 重建 demo:`kb scaffold` -> `tmp/kb/compliance/dengbao-2.0`;`system init order-platform --name 订单平台`;系统 facts 灌到 `tmp/kb/system/order-platform`;`project init ... --system order-platform`。
- 更新 `tests/flow.rs`:覆盖四种统一 ingest 记录、重复导入幂等、共享 system 和完整报告流程。
- 新增项目 fact 单元测试。

## 实施阶段(增量)

- **A 存储分层 + system 共享化**:`compliance_root`/`system_root`;建 `src/system/` 模块,迁 Fact 逻辑到共享 `system/<slug>/`,加 slug/display_name/`system init`;`kb/*` 路径改 `compliance/`。
- **B 项目重构**:`project.yaml` +system;`project init --system`;删 `project/system.rs`,加 `src/project/fact.rs` + `fact` 命令 + ProjectFact schema。
- **C matrix**:`MatrixEntry` +project_facts;`matrix link --project-fact`;`matrix trace` 跨三层。
- **D 接线 + demo 重建 + 测试 + clippy**。

## 不在本次范围
- 现有 demo 数据(tmp/)直接重建,不做迁移工具(尚无真实数据)。
- `system` 的 fact 版本链(`supersedes`)已有 schema,本次不额外做演化逻辑。
- evidence 仍项目级(后续若需跨框架复用证据再议)。
