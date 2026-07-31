# SESSION.md

过程中发现的、暂不阻塞但值得回头处理的实现/结构/工作流问题。

## 等保结构表的控制点 ID 由位置派生,重排即漂移

`data/dengbao-2.0.yaml` 里每个 category 的 `points` 是纯短名列表,
控制点 ID 在 `src/compliance/scaffold.rs` 里按 `enumerate()` 派生(`{prefix}.{i+1}`,
见 `scaffold_dengbao` 内 `format!("{}.{}", category.prefix, i + 1)`)。

后果:在 yaml 里重排某个 category 的 `points`、或中间插入一个控制点,
会导致该 category 下所有后续控制点的 ID 漂移。而控制点 ID 是控制项
frontmatter 的主键,又被 `matrix.yaml` / `evidence.yaml` / 系统事实的
`related_controls` 反向引用,漂移会静默切断这些引用。

可能的修法(择一,未实施):
- 让 `points` 成为对象数组,显式带 `id` 与短名,ID 不再靠位置派生;
  相应 `FrameworkStructure` 的 `points: Vec<String>` 改为 `Vec<PointStructure>`。
- 或保持短名列表,但 scaffold 时按已有桩文件的 ID 增量匹配,禁止重排。

## `matrix set` 无法写 `remediation` 字段

`MatrixEntry`(`src/project/matrix.rs`)有 `remediation` 字段,但
`matrix set` 只接受 `--gap` / `--owner`,`MatrixCommand::Set`(见 `src/cli.rs`)
没有暴露 `--remediation`。目前该字段只能手编 `matrix.yaml` 写入。

修法:`matrix set` 增加一个 `--remediation <String>` 可选参数,
仿照 `--gap` 的处理路径(`matrix.rs` 的 `set` 函数)即可。

## KB 版本管理与团队共享尚未设计

现状:`COMPLAI_KB_DIR`(默认 `~/.complai/kb`)是纯本地目录,
compliance KB / system KB / 项目工作区混在一起,无版本、无共享机制。
团队共享诉求出现后,需要拆分(详见对话中的方案讨论):

- `project.yaml` 只有 `system` + `framework` 引用,**没有版本钉扎字段**。
  KB 漂移后,同一份 `matrix.yaml` / 报告无法复现。应记录评估时所用的
  framework tag 与 system tag。
- 框架结构表 `data/dengbao-2.0.yaml` 当前编译期 `include_str!` 进二进制
  (`src/compliance/scaffold.rs`)。若 compliance KB 独立成共享 repo,该表应随之
  搬入,`scaffold` 改从 `COMPLAI_KB_DIR` 读取,而非内置。
- 控制点 ID 必须保持"只增不改、废弃不重编号",否则 matrix/evidence/
  related_controls 引用会断裂(与本文档第一条隐患同源)。


