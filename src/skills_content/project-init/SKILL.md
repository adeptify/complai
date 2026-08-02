---
name: project-init
description: >-
  创建并初始化合规备案项目，确认系统、框架和等级后通过 `complai project init`
  绑定系统与框架并预填控制矩阵。用于用户要求新建合规项目、复用或创建系统知识库、
  确认等保定级、初始化矩阵或规划后续文档灌库与差距分析时。
---

# 项目初始化 (project-init)

为一次合规备案(如过等保三级)初始化项目工作区:绑定「一个系统 × 一个合规框架」,并从 compliance KB 预填控制矩阵为完整清单。

一个项目 = 一个系统 + 一个框架。同一系统过多个框架(等保 + ISO)时,各建一个项目,复用同一 system slug。

## 需先确认的信息(向用户问清,不要臆测)

1. **项目名**:本次备案标识,如 `order-platform-dengbao3-2026`。
2. **框架**:如 `dengbao-2.0`。
3. **等级(定级)**:如 3。等保定级依据「受侵害客体 + 程度」;若不确定,先看系统事实里的数据分类(个人信息/重要数据)再定,或直接问用户。
4. **系统**:被审计系统。
   - 已在共享 system KB?向用户拿 slug(如 `order-platform`)。
   - 新系统?确定 slug(ASCII,如 `order-platform`)+ 中文显示名(如 `订单平台`)。

## 工作流

1. **核对 compliance KB 就绪**:
   ```
   complai compliance list --framework <框架>
   ```
   若无控制项或报索引不存在,先 `complai compliance scaffold <框架>`(必要时 `compliance ingest <框架> notes.md` 补正文)。

2. **系统(复用或新建)**:
   - 新系统:`complai system init <slug> --name "<显示名>"`
   - 已有系统:直接用其 slug(可 `complai system find --system <slug> --control <某id>` 看是否已有事实)。

3. **建项目**:
   ```
   complai project init <项目名> --system <slug> --framework <框架> --level <等级>
   ```
   自动从 compliance KB 预填矩阵为完整控制清单(每项 `na`);系统不存在则建空壳。

4. **进入项目并核对**:
   ```
   cd <项目名>            # 或 export COMPLAI_PROJECT_DIR=$PWD/<项目名>
   complai matrix show    # 看预填的完整控制清单
   ```

## 后续(本 skill 不执行,提示用户)

- **若系统是新建的**:先录系统事实(架构/数据流/资产/...):
  `complai system add --system <slug> ...` 或 `system ingest --from facts.yaml`(可配合 `parse` + `doc-ingest` skill 从现有文档灌)。
- **差距分析**:跑 `gap-analysis` skill(逐控制判状态、落缺口)。
- **整改/证据**:`fact add --kind 整改 ...`、`evidence add ...`。
- **报告**:`gen report`。

## 约束
- 定级与系统归属由用户确认,不要臆测。
- 只调 CLI;`project init` 前确保 compliance KB 已 scaffold。
- 项目只绑定一个系统 + 一个框架;多框架复用同一 system slug,各建项目。
