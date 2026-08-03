# complai 文件模板与初始化指南(重构后)

共享 KB 分 `compliance`(框架控制项)+ `system`(业务系统事实,按 slug 共享);项目 = 系统×框架绑定 + 项目专属事实 + artifacts。本文档给出各文件模板与初始化清单。

> 设计见根目录 [PLAN.md](../PLAN.md),快速上手见 [README.md](../README.md)。

---

## 1. 目录结构

### 1.1 共享知识库(`~/.complai/kb/` 或 `COMPLAI_KB_DIR`)

```
~/.complai/kb/
├── compliance/                     # 合规框架(跨项目共享)
│   └── dengbao-2.0/
│       ├── index.yaml              # compliance build 生成(紧凑索引,无正文)
│       ├── 技术/<类别>/<id>.md      # 控制项
│       └── 管理/<类别>/<id>.md
└── system/                         # 业务系统(跨项目共享,按 slug)
    └── order-platform/             # slug = order-platform
        ├── index.yaml              # display_name + facts 索引
        └── 架构|部署|资产/.../SYS-F-NNNN.md
```

### 1.2 项目工作区(每个备案一个目录)

```
order-platform-dengbao3/            # 项目根
├── project.yaml                    # 引用:system(slug)+ framework + level
├── matrix.yaml                     # 控制矩阵(init 预填全部控制为 na)
├── facts/                          # 项目专属事实(整改/例外/决策/发现/备注)
│   ├── index.yaml
│   └── 整改|例外|.../PROJ-F-NNNN.md
├── evidence.yaml + evidence/       # 证据(项目特有)
└── drafts/                         # 报告等 artifacts
```

---

## 2. 文件模板

### 2.1 控制项 `compliance/<框架>/<域>/<类别>/<id>.md`

`compliance scaffold` 生成空桩；Agent 抽取的控制内容通过统一 `complai ingest`
协议写入。范例见 [`templates/control.example.md`](../templates/control.example.md)。

```markdown
---
id: "dengbao-2.0:8.1.4.1"
framework: dengbao-2.0
domain: 技术
category: 安全计算环境
control_id: "8.1.4.1"
title: 身份鉴别
levels: [3]
tags: [identity, authentication]
mappings:
  iso27001: ["A.5.16", "A.8.5"]
expected_evidence:
  - 多因素认证启用截图
  - 密码复杂度策略配置
excerpt_status: partial
last_reviewed: 2026-07-29
---

# 身份鉴别

## 要求摘要
(自己的话概述控制要求,不复录标准原文)

## 实施指引
(怎么落地)

## 常见缺陷
(常见不合规点)
```

字段:`id`/`framework`/`domain`(由框架定义)/`category`/`control_id`/`title`/`levels`/`tags`/`mappings`(跨框架,只存 ID)/`expected_evidence`/`excerpt_status`(empty|partial|complete)/`last_reviewed`。统一导入的记录还会带可选 `ingest` 元数据。

### 2.2 统一 ingest bundle

原始材料不要求固定格式。Agent 读取 Excel、PDF、Word、图片或云文档后，按
[`schemas/ingest-v1.schema.json`](../schemas/ingest-v1.schema.json) 生成 JSON。
一个 bundle 可同时包含 `control_content`、`system_fact`、`project_fact` 和
`matrix_assessment`。通过以下命令校验和写入：

```sh
complai ingest schema
complai ingest validate --from tmp/complai-ingest.json
complai ingest plan --from tmp/complai-ingest.json
# 检查 create/update/unchanged 和所有目标后才执行：
complai ingest apply --from tmp/complai-ingest.json
```

对尚未入库的框架，`control_content` 记录同时提供 `title`、`domain` 和
`category` 即可创建控制项；框架有级别概念时再提供 `levels`。

写入后的 `ingest` 元数据包含稳定 `external_key`、记录摘要、来源类型、标题、
引用、原文定位、可选文件哈希和文档日期，以及置信度。重复导入相同记录会标记为
`unchanged`，来源内容变化则执行 `update`。

### 2.3 系统事实 `system/<slug>/<域>/SYS-F-NNNN.md`(共享)

`system add` 或统一 `complai ingest` 生成。范例见 [`templates/fact.example.md`](../templates/fact.example.md)。

```markdown
---
id: SYS-F-0001
domain: 架构
title: 微服务拓扑
tags: [microservices, api-gateway]
source:
  type: doc
  ref: 架构设计文档 v1.2
  collected_at: 2026-07-29
  collector: agent
confidence: high
related_controls:
  - "dengbao-2.0:8.1.2"
status: current
---

系统由 user/order/payment 三个微服务组成,经 API 网关接入,服务间 mTLS。
```

`domain`:架构/组件/数据流/数据分类/资产/技术栈/部署/网络/人员/策略。`source.type`:doc/interview/scan/user。`status`:current/stale/superseded(演进用 `supersedes` 留版本链)。

### 2.4 系统 `index.yaml`(含 display_name)

```yaml
display_name: 订单平台
facts:
  - id: SYS-F-0001
    domain: 架构
    title: 微服务拓扑
    related_controls: ["dengbao-2.0:8.1.2"]
    external_key: assessment-v2:p12:architecture
    file: 架构/SYS-F-0001.md
```

### 2.5 `project.yaml`(项目元信息,纯引用)

```yaml
name: order-platform-dengbao3
system: order-platform        # 引用共享 system KB(slug)
framework: dengbao-2.0        # 引用共享 compliance KB
level: 3
```

### 2.6 `matrix.yaml`(控制矩阵)

`project init` 预填全部控制为 `na`。三类引用:`facts`=SYS-F(系统,共享)、`project_facts`=PROJ-F(项目)、`evidence`=EV(项目)。

```yaml
framework: dengbao-2.0
level: 3
scope:
  systems: [order-platform]
entries:
  "dengbao-2.0:8.1.4.1":
    status: gap
    owner: 张三
    facts: [SYS-F-0001]              # 系统事实(共享)
    project_facts: [PROJ-F-0001]     # 项目事实(整改等)
    evidence: [EV-0001]
    gap: payment-service 未启用多因素认证
    remediation: Q3 前接入 OTP
    last_updated: 2026-07-29
```

`status`:unassessed/met/partial/gap/na。新建矩阵使用 `unassessed`；`na`
仅表示经评估确认不适用。

### 2.7 项目事实 `facts/<kind>/PROJ-F-NNNN.md`(项目专属)

`fact add --kind <整改|例外|决策|发现|备注>` 生成。范例见 [`templates/project-fact.example.md`](../templates/project-fact.example.md)。

```markdown
---
id: PROJ-F-0001
kind: 整改
title: payment-service 接入多因素认证
tags: [mfa]
control: "dengbao-2.0:8.1.4.1"
created_at: 2026-07-29
---

负责人张三,计划 Q3 完成;当前设计中。方案:复用 user-service 的 OTP。
```

`kind`:整改/例外/决策/发现/备注(兼作子目录名)。

### 2.8 `evidence.yaml`(证据索引,项目)

`evidence add` 登记(算 sha256、按控制点就近存)；`evidence list/find/show`
用于查询。`type`:screenshot/config/policy-doc/log/record。

```yaml
evidence:
  EV-0001:
    id: EV-0001
    file: evidence/8.1.4.1/sample-mfa.txt
    sha256: 908245005d48ab...
    type: config
    description: user-service MFA config
    collected_at: 2026-07-29
    collector: agent
    linked_controls: ["dengbao-2.0:8.1.4.1"]
```

---

## 3. 初始化准备清单

### 3.1 共享知识库(一次性)

**compliance KB**:
1. 框架 + 可选级别(如 dengbao-2.0、3；ISO 等无级别框架不填)。
2. 用户有权使用的规范或既有材料。
3. 每条抽取记录的来源定位和置信度。

等保 2.0 先运行 `complai compliance scaffold dengbao-2.0` 生成控制桩；
其他框架由 `doc-ingest` workflow 在统一 bundle 中创建控制元数据并写入正文。

**system KB**(每个系统一次,可被多项目复用):
1. system slug(ASCII,如 order-platform)+ 中文显示名(如 订单平台)。
2. 系统事实:架构/组件/数据流/数据分类/资产/技术栈/部署/网络/人员/已有策略。

```sh
complai system init order-platform --name 订单平台
complai system add --system order-platform --domain 架构 --title "微服务拓扑" --control dengbao-2.0:8.1.2 --body "..."
```

少量人工事实可用 `system add`；从文档批量抽取的事实使用统一 ingest bundle。

### 3.2 项目(每个备案)

需准备:项目名 + system slug + 框架 + 框架可选级别。(可由 `project-init` skill 引导完成。)

```sh
complai project init order-platform-dengbao3 --system order-platform --framework dengbao-2.0 --level 3
cd order-platform-dengbao3
```

无级别框架省略 `--level`。初始矩阵中每项状态为 `unassessed`。

之后:跑 `gap-analysis` skill(或手工 `matrix set/link`)判差距;`fact add` 记整改项;`evidence add` 登记证据;`gen report` 出报告。

---

## 4. 备注

- `index.yaml`/`matrix.yaml`/`evidence.yaml` 是 CLI 管理的内部存储，不应作为导入接口手编。
- Agent 批量写入前必须先运行 `ingest validate` 和 `ingest plan`。
- 等保结构表(`data/dengbao-2.0.yaml`)的控制点编号/短名正式使用前请核对。
