---
name: doc-ingest
description: >-
  从 Excel、PDF、Word、图片、飞书、腾讯文档等任意可访问材料中抽取合规知识，
  分类为控制内容、系统事实、项目事实或矩阵评估，并通过版本化 JSON bundle
  校验、预览和幂等写入 Complai。用于用户要求导入备案材料、测评报告、资产清单、
  数据流、控制要求、差距记录或云文档时。
---

# 文档灌库

原始输入不要求模板。使用当前环境适合该来源的读取能力完成“来源 -> 内容”：
本地文档使用对应的 PDF、文档、表格或 OCR 工具；云文档优先使用 Connector/API，
其次使用已登录浏览器，仍不可访问时请用户授权或导出。不要要求用户先整理成
Complai 专用格式。

Complai 只接收 Agent 生成的版本化 JSON。先获取当前二进制的权威 schema：

```sh
complai ingest schema
```

## 工作流

1. 读取材料并保留来源标识、文件哈希（可得时）、文档日期以及页码、工作表、
   单元格、章节或云文档块等定位信息。
2. 按语义把内容拆成原子记录：
   - `control_content`：权威规范中的控制元数据、要求摘要、实施指引和常见缺陷。
   - `system_fact`：可跨项目复用的架构、资产、数据流、部署、人员或策略事实。
   - `project_fact`：本次备案的发现、整改、例外、决策或备注。
   - `matrix_assessment`：针对一个控制项的 `unassessed/met/partial/gap/na` 状态。
   对 `system_fact` 使用统一粒度：一条事实只表达“一个明确对象在一个明确范围内，
   由一个明确来源支撑，并按同一生命周期变化的可独立验证主张”。对象、环境、
   系统边界、机制、来源、确认时间、置信度或更新周期任一不同，就拆成不同事实；
   不要按框架或控制项复制同一事实。
3. 按 `complai ingest schema` 输出 `tmp/complai-ingest.json`。为每条记录生成稳定
   `external_key`，组合来源身份、定位和记录语义；不要使用会随正文改写而变化的
   随机值。
   创建尚不存在的框架控制项时，在 `control_content` 中提供 `title`、`domain`
   和 `category`；框架有级别概念时再提供 `levels`。目标 `control` 使用
   `<framework>:<control-id>`，Complai 会创建控制文件并重建框架索引。
4. **校验并预览**：

   ```sh
   complai ingest validate --from tmp/complai-ingest.json
   complai ingest plan --from tmp/complai-ingest.json
   ```

5. **检查计划**：确认 plan 中的 create/update/unchanged、记录数量和目标均符合预期。
   发现误分类、意外覆盖或错误目标时，修正 bundle 并重新 validate/plan，不要 apply。
6. **写入**：计划确认无误后运行：

   ```sh
   complai ingest apply --from tmp/complai-ingest.json
   ```

7. **抽查**：写入后用
   `compliance show`、`system find/show`、`fact find/show` 或 `matrix show/trace`
   抽查结果。已进入项目时先运行 `complai project show`：同时写入 KB 与当前项目的
   bundle 会在事务内推进 revision；纯 KB bundle 会保留 drift，审阅变更后运行
   `complai project sync`。

## `system_fact` 输出检查清单

生成 bundle 前逐条检查；任一项不满足时先重新拆分，不要进入 apply：

- [ ] 只包含一个可独立判真的主张；任一子句能独立变化时已经拆开。
- [ ] 对象、系统边界、适用环境和覆盖群体明确。
- [ ] 全部内容共享同一来源定位、确认时间和置信度。
- [ ] 标题是客观主张，不是“安全情况”等主题，也不是“符合某控制”等结论。
- [ ] 正文写明必要机制和影响真实性的例外；独立例外另拆事实。
- [ ] `external_key` 对“来源定位 + 原子主张”稳定且唯一。
- [ ] 同一事实可以关联多个框架控制，但没有因框架不同复制正文。
- [ ] 不确定内容已限定范围、降低置信度或不输出。

以下内容必须拆分：MFA、堡垒机审计、日志告警、备份和恢复测试，因为它们可以
独立变化并由不同证据证明。认证流程中同一配置下共同变化的“密码 + 硬件令牌”
可以保留为一条事实。

## 判断边界

- 同一材料可以产生多种记录；按陈述性质拆分，不按文件整体归类。
- 测评报告中的系统描述属于 `system_fact`，测评发现属于 `project_fact` 或
  `matrix_assessment`。除非来源本身是权威规范，不要把测评机构解释写成
  `control_content`。
- `control_content.completeness=complete` 仅在三个正文段落均有可靠来源时使用；
  缺失内容保持 `partial`，不要补造。
- `unassessed` 只表示尚未评估；`partial`、`gap` 和 `na` 必须在 `gap` 字段说明
  缺口或不适用理由，`met` 不应携带 `gap`。
- 每条记录必须带精确来源定位和置信度。低置信度记录默认不能 apply；只有用户审阅
  并明确确认后才使用 `--allow-low-confidence`。
- 控制正文使用自己的话概述，不复录用户无权再分发的标准原文。
- 只通过统一 ingest CLI 写入批量抽取结果，不直接编辑 KB、索引或矩阵文件。
- 不要为了消除 drift 自动运行 `project sync`；纯 KB 变更应先由用户审阅。
