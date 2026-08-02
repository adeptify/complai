---
name: doc-ingest
description: >-
  解析并导入 Excel（.xlsx）等合规审计材料，把表格列映射为系统事实、控制项正文
  或矩阵状态。用于用户要求导入资产清单、数据流、控制要求清单或差距表，或者需要
  通过 `complai parse`、`system ingest`、`compliance ingest` 或 `matrix set`
  完成文档灌库时。
---

# 文档灌库 (doc-ingest)

把外部文档(主要是 .xlsx:资产清单、控制清单、差距表)转成知识库内容。

核心分工:`complai parse` 做"字节 -> 表格"(确定性);**列含义识别与字段映射由 agent 判断**(需理解)。

## 流程

1. **抽取表格**:`complai parse <file>.xlsx` -> 每张表输出为 Markdown 表格(首行作表头)。
2. **判断表类型并按列映射**(agent 判断;不确定时向用户确认列含义,不要臆测):

   | 表类型 | 识别线索 | 灌入目标 | 命令 |
   |---|---|---|---|
   | 资产清单 / 数据流 / 部署等 | 列含 资产名/类型/位置/数据项 等 | 系统知识(facts) | 产出 `facts.yaml` -> `complai system ingest --from facts.yaml` |
   | 控制要求清单 | 列含 控制ID + 要求摘要/实施指引/常见缺陷 | 控制项正文 | 产出 `notes.md`(`@@ <控制ID>` 分块)-> `complai compliance ingest <framework> notes.md` |
   | 差距表 | 列含 控制ID + 状态 + 缺口 | 矩阵状态 | `complai matrix set <id> <status> --gap "..."` |

3. **核对**:`complai system find --control <id>` / `complai compliance show <id>` / `complai matrix show --status gap`。

## facts.yaml 格式(`system ingest --from`)

```yaml
facts:
  - domain: 资产            # 架构/组件/数据流/数据分类/资产/技术栈/部署/网络/人员/策略
    title: 用户数据库
    control: "dengbao-2.0:8.1.4.8"   # 可选,关联控制
    type: doc              # doc/interview/scan/user,默认 user
    ref: 资产清单.xlsx      # 可选,来源
    body: "MySQL 8.0 主从,杭州 AZ,每日全备+增量"
  - domain: 数据流
    title: 订单 PII 处理
    control: "dengbao-2.0:8.1.4.8"
    body: "订单含手机号/地址,落库前 AES-256 加密"
```

## notes.md 格式(`compliance ingest`,控制项正文)

```
@@ dengbao-2.0:8.1.4.1
## 要求摘要
<控制要求清单该行的"要求摘要"列>

## 实施指引
<该行"实施指引"列>

## 常见缺陷
<该行"常见缺陷"列>
```

## 约束
- 列映射由 agent 判断;不确定列含义时向用户确认,不要臆测。
- 只调 CLI 写入;不直接编辑 `system/`、控制文件或 `matrix.yaml`。
- 一张表可能同时含多类信息,按行拆分到合适的灌入路径。
