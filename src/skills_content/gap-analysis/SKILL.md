---
name: gap-analysis
description: >-
  基于合规控制、系统事实、项目事实和证据逐项评估符合性，填写控制矩阵并生成差距
  报告。用于用户要求开展等保、ISO、NIST、SOC 2、PCI DSS 或其他框架的差距分析、
  管理 unassessed/met/partial/gap/na 状态、
  记录缺口与负责人、关联事实或证据，或者生成合规差距报告时。
---

# 差距分析

把系统与项目事实对照当前项目的合规框架，逐控制项判定符合性，
关联可追溯材料后把结果写入控制矩阵。

核心模型：`控制矩阵 = 差距分析(合规知识库, 系统事实, 项目事实, 证据)`。

## 前置条件

- 进入已初始化的项目，运行 `complai project show` 获取当前框架，不要硬编码框架名。
- 运行 `complai compliance list --framework <框架>` 确认控制清单已就绪。若控制正文为
  `empty` 或仍有待摘录占位内容，停止评估该项并提示用户导入权威材料，不要臆测要求。

## 工作流

1. **取控制清单**：先只读紧凑索引，再按索引中的实际域分批处理。

   ```sh
   complai compliance list --framework <框架>
   complai compliance list --framework <框架> --domain "<控制域>"
   ```

2. **逐项拉取聚焦包**：将 `<control>` 替换为 list 输出的完整
   `<framework>:<control-id>`，每次只加载一个控制的相关材料。

   ```sh
   complai compliance show <control>
   complai system find --control <control>
   complai fact find --control <control>
   complai evidence find --control <control>
   complai matrix trace <control>
   ```

3. **先关联支撑材料**：只关联上一步确认真实存在且与该控制相关的 ID。

   ```sh
   complai matrix link <control> --fact SYS-F-xxxx
   complai matrix link <control> --project-fact PROJ-F-xxxx
   complai matrix link <control> --evidence EV-xxxx
   ```

4. **判定状态**：
   - `unassessed`：尚未完成评估；它是暂存状态，不是符合性结论。
   - `met`：相关系统/项目事实和证据完整覆盖控制要求，不写 `--gap`。
   - `partial`：部分满足；用 `--gap` 明确说明剩余缺口。
   - `gap`：未满足或证据不足；用 `--gap` 写明缺口。
   - `na`：经评估确认不适用；用 `--gap` 写明不适用理由。

5. **最后写入结论**：支撑材料全部关联成功后再设置状态。

   ```sh
   complai matrix set <control> met --owner "<责任人>"
   complai matrix set <control> <partial|gap|na> --gap "<缺口或不适用理由>" \
     --owner "<责任人>" --remediation "<整改计划或完成情况>"
   ```

6. **产出报告**：运行 `complai gen report`，生成
   `drafts/compliance-report.md`。显式报告剩余 `unassessed` 数量，不要将它们当成 `na`。

## 约束

- 只通过 Complai CLI 读写状态；不要直接编辑 `matrix.yaml`、`evidence.yaml` 或知识库文件。
- 按框架的实际控制域分批处理，避免一次加载过多控制正文。
- 判定必须基于已加载的事实与证据；证据不足时标记 `gap` 而非 `met`，不要臆测。
