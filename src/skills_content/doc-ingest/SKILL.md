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
   - `control_content`：权威规范中的要求摘要、实施指引和常见缺陷。
   - `system_fact`：可跨项目复用的架构、资产、数据流、部署、人员或策略事实。
   - `project_fact`：本次备案的发现、整改、例外、决策或备注。
   - `matrix_assessment`：针对一个控制项的 `met/partial/gap/na` 结论。
3. 按 `complai ingest schema` 输出 `tmp/complai-ingest.json`。为每条记录生成稳定
   `external_key`，组合来源身份、定位和记录语义；不要使用会随正文改写而变化的
   随机值。
4. 依次执行：

   ```sh
   complai ingest validate --from tmp/complai-ingest.json
   complai ingest plan --from tmp/complai-ingest.json
   complai ingest apply --from tmp/complai-ingest.json
   ```

5. 检查 plan 中的 create/update/unchanged 和目标是否符合预期，再 apply。写入后用
   `compliance show`、`system find/show`、`fact find/show` 或 `matrix show/trace`
   抽查结果。

## 判断边界

- 同一材料可以产生多种记录；按陈述性质拆分，不按文件整体归类。
- 测评报告中的系统描述属于 `system_fact`，测评发现属于 `project_fact` 或
  `matrix_assessment`。除非来源本身是权威规范，不要把测评机构解释写成
  `control_content`。
- `control_content.completeness=complete` 仅在三个正文段落均有可靠来源时使用；
  缺失内容保持 `partial`，不要补造。
- 每条记录必须带精确来源定位和置信度。低置信度记录默认不能 apply；只有用户审阅
  并明确确认后才使用 `--allow-low-confidence`。
- 控制正文使用自己的话概述，不复录用户无权再分发的标准原文。
- 只通过统一 ingest CLI 写入批量抽取结果，不直接编辑 KB、索引或矩阵文件。
