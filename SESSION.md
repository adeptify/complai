# SESSION.md

过程中发现的、暂不阻塞但值得回头处理的实现/结构/工作流问题。

## 等保结构表的控制点 ID 由位置派生,重排即漂移

`data/dengbao-2.0.yaml` 里每个 category 的 `points` 是纯短名列表,
控制点 ID 在 `src/kb/scaffold.rs` 里按 `enumerate()` 派生(`{prefix}.{i+1}`,
见 `scaffold_dengbao` 内 `format!("{}.{}", category.prefix, i + 1)`)。

后果:在 yaml 里重排某个 category 的 `points`、或中间插入一个控制点,
会导致该 category 下所有后续控制点的 ID 漂移。而控制点 ID 是控制项
frontmatter 的主键,又被 `matrix.yaml` / `evidence.yaml` / 系统事实的
`related_controls` 反向引用,漂移会静默切断这些引用。

可能的修法(择一,未实施):
- 让 `points` 成为对象数组,显式带 `id` 与短名,ID 不再靠位置派生;
  相应 `FrameworkStructure` 的 `points: Vec<String>` 改为 `Vec<PointStructure>`。
- 或保持短名列表,但 scaffold 时按已有桩文件的 ID 增量匹配,禁止重排。
