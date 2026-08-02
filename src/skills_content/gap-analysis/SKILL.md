---
name: gap-analysis
description: >-
  基于合规控制、系统事实、项目事实和证据逐项评估符合性，填写控制矩阵并生成差距
  报告。用于用户要求开展等保或其他框架的差距分析、判定 met/partial/gap/na、
  记录缺口与负责人、关联事实或证据，或者生成合规差距报告时。
---

# 差距分析 (gap-analysis)

把"业务系统知识库"对照"等保合规知识库",逐控制项判定符合性,把结果写入控制矩阵。

核心模型:`控制矩阵 = 差距分析(合规知识库, 业务系统知识库) + 证据`。

## 前置条件
- 通用知识库已就绪:`complai compliance list --framework dengbao-2.0` 能列出控制项。
  若控制正文仍"待摘录",先停下提示用户补摘录,不要臆测要求。
- 项目已初始化并有若干系统事实:`complai system find --control dengbao-2.0:<id>` 能查到事实。

## 工作流(最小上下文,逐控制项)

1. **取控制清单**(只读紧凑索引,不加载正文):
   ```
   complai compliance list --framework dengbao-2.0 --domain 技术
   complai compliance list --framework dengbao-2.0 --domain 管理
   ```

2. **逐项拉取聚焦包**(每个控制只加载相关材料,不整库):
   - `complai compliance show dengbao-2.0:<id>` —— 控制要求摘要/实施指引/常见缺陷
   - `complai system find --control dengbao-2.0:<id>` —— 关联的系统事实
   - `complai matrix trace dengbao-2.0:<id>` —— 现有矩阵状态与已挂证据/事实

3. **判定状态并落盘**:
   ```
   complai matrix set dengbao-2.0:<id> <status> --gap "<缺口描述>" --owner "<责任人>"
   complai matrix link dengbao-2.0:<id> --fact SYS-F-xxxx     # 有支撑事实时
   complai matrix link dengbao-2.0:<id> --evidence EV-xxxx     # 有证据时
   ```

4. **状态判定准则**:
   - `met`:系统事实 + 证据完整覆盖该控制要求。
   - `partial`:部分满足,有明确缺口(用 `--gap` 描述缺什么)。
   - `gap`:未满足或无证据,`--gap` 写明缺口。
   - `na`:不适用(如纯云系统对"安全物理环境"不适用),`--gap` 写明不适用理由。

5. **产出报告**:`complai gen report` 生成差距报告(`drafts/compliance-report.md`)。

## 约束
- 只通过 complai CLI 读写状态;不要直接编辑 `matrix.yaml`/`evidence.yaml`/`system/`。
- 按域分批处理,避免一次性加载过多控制正文(保持最小上下文)。
- 判定须基于已加载的事实与证据;证据不足时标 `gap` 而非 `met`,不要臆测。
