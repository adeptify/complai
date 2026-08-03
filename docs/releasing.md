# 发布流程

Complai 的 GitHub 源码、Git tag 和 crates.io 包必须指向同一份已验证内容。

## 准备版本

1. 根据 SemVer 更新 `Cargo.toml` 版本，并让 Cargo 同步 `Cargo.lock`。
2. 确认 README 描述的是当前行为，不维护旧版本操作说明。
3. 运行完整发布检查：

   ```sh
   cargo fmt --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features --locked
   shellcheck scripts/check-agent-skills.sh
   scripts/check-agent-skills.sh
   cargo package --locked
   cargo publish --dry-run
   ```

## 提交、标记并发布

版本提交使用 `chore: release complai <version>`。检查通过后依次执行：

```sh
git push origin main
git tag -a v<version> -m "complai <version>"
git push origin v<version>
cargo publish
```

发布后确认 crates.io 已显示新版本，并核对远端 tag 指向版本提交。不要在 tag 与
crates.io 包之间继续插入源码修改；如需修正已发布内容，提升版本后重新走完整流程。
