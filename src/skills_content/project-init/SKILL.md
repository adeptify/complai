---
name: project-init
description: >-
  创建并初始化任意合规框架的评估项目，确认系统、框架及可选级别后通过
  `complai project init` 绑定系统与框架并预填控制矩阵。用于用户要求新建
  等保、ISO、NIST、SOC 2、PCI DSS 或其他合规项目，复用或创建系统知识库、
  确认框架等级、初始化矩阵或规划后续文档导入与差距分析时。
---

# 项目初始化

为一次合规评估初始化项目工作区：绑定“一个系统 × 一个合规框架”，
并从 compliance KB 预填完整控制矩阵。

同一系统评估多个框架时，为每个框架分别建项，复用同一 system slug。

## 需先确认的信息

1. 确认项目名，例如 `order-platform-iso27001-2026`。
2. 确认框架 slug，例如 `dengbao-2.0` 或 `iso27001-2022`。
3. 仅在框架定义级别时确认等级。等保定级不明时，根据受侵害客体、影响程度和
   系统数据分类请用户确认；不要为 ISO 等无级别框架填虚假等级。
4. 确认被评估系统的 slug 和显示名。已有系统复用 slug；新系统则使用 ASCII slug。

## 工作流

1. **核对框架控制库**：
   ```sh
   complai compliance list --framework <框架>
   ```
   - `dengbao-2.0` 尚未初始化时，运行 `complai compliance scaffold dengbao-2.0`。
   - 其他框架尚未入库时，暂停建项，加载 `doc-ingest` workflow，从用户有权使用的
     规范材料创建 `control_content` 记录。导入完成后回到本 workflow。

2. **复用或新建系统**：
   - 新系统：`complai system init <slug> --name "<显示名>"`。
   - 已有系统：直接复用 slug。

3. **初始化项目**：
   无级别框架运行：
   ```sh
   complai project init <项目名> --system <slug> --framework <框架>
   ```
   有级别框架运行：
   ```sh
   complai project init <项目名> --system <slug> --framework <框架> --level <等级>
   ```
   矩阵会预填完整控制清单，每项初始状态为 `unassessed`。

4. **进入项目并核对**：
   ```sh
   cd <项目名>
   complai project show
   complai matrix show --status unassessed
   ```

## 后续

- **若系统是新建的**：加载 `doc-ingest` workflow，从已有备案、设计或测评材料抽取
  架构/数据流/资产等事实；少量人工事实也可用 `complai system add`。
- **差距分析**：加载 `gap-analysis` workflow。
- **整改/证据**：使用 `fact add --kind 整改 ...` 和 `evidence add ...`。
- **报告**：使用 `gen report`。

## 约束

- 框架、可选级别与系统归属由用户确认，不要臆测。
- 只通过 CLI 写入状态；`project init` 前确保 compliance KB 已有该框架索引。
- 每个项目只绑定一个系统和一个框架。
