//! 错误类型别名。整个 crate 用 `eyre::Report` 传播错误,
//! 并在每个 `?` 前用 `.wrap_err(...)` 补充"在做什么"的上下文(见 CLAUDE.md)。

pub type Result<T> = eyre::Result<T>;
