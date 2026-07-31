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

`compliance scaffold` 生成空桩;`compliance ingest` 或手工填正文。范例见 [`templates/control.example.md`](../templates/control.example.md)。

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

字段:`id`/`framework`/`domain`(技术|管理)/`category`/`control_id`/`title`/`levels`/`tags`/`mappings`(跨框架,只存 ID)/`expected_evidence`/`excerpt_status`(empty|partial|complete)/`last_reviewed`。

### 2.2 摘录笔记 `notes.md`(`compliance ingest` 输入)

每块以 `@@ <control-id>` 起首。模板见 [`templates/notes.md`](../templates/notes.md)。

### 2.3 系统事实 `system/<slug>/<域>/SYS-F-NNNN.md`(共享)

`system add` / `system ingest` 生成。范例见 [`templates/fact.example.md`](../templates/fact.example.md)。

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

`status`:met/partial/gap/na。

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

`evidence add` 登记(算 sha256、按控制点就近存)。`type`:screenshot/config/policy-doc/log/record。

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
1. 框架 + 等级(如 dengbao-2.0、3)。
2. 每控制项摘录:要求摘要/实施指引/常见缺陷(自己话,不复录原文)。
3. (可选)期望证据、跨框架映射。

```sh
complai compliance scaffold dengbao-2.0              # 生成控制桩
# 写 notes.md(@@ <id> 分块)
complai compliance ingest dengbao-2.0 notes.md       # 灌正文 + 重建索引
```

**system KB**(每个系统一次,可被多项目复用):
1. system slug(ASCII,如 order-platform)+ 中文显示名(如 订单平台)。
2. 系统事实:架构/组件/数据流/数据分类/资产/技术栈/部署/网络/人员/已有策略。

```sh
complai system init order-platform --name 订单平台
complai system add --system order-platform --domain 架构 --title "微服务拓扑" --control dengbao-2.0:8.1.2 --body "..."
# 或批量:complai system ingest --from facts.yaml --system order-platform
```

### 3.2 项目(每个备案)

需准备:项目名 + system slug + 框架 + 等级(定级)。(可由 `project-init` skill 引导完成。)

```sh
complai project init order-platform-dengbao3 --system order-platform --framework dengbao-2.0 --level 3
cd order-platform-dengbao3
```

之后:跑 `gap-analysis` skill(或手工 `matrix set/link`)判差距;`fact add` 记整改项;`evidence add` 登记证据;`gen report` 出报告。

---

## 4. 备注

- 手写 YAML 列表可用流式(`[3]`)或块式;`id`/`control` 含冒号建议加引号。
- 各 `index.yaml`/`matrix.yaml`/`evidence.yaml` 可手编,但推荐用 CLI 维护(ID/哈希/索引一致)。
- 等保结构表(`data/dengbao-2.0.yaml`)的控制点编号/短名正式使用前请核对。
