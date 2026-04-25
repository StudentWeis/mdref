# mdref 项目审查与改进建议（2026-04-25）

## 审查范围

本次审查覆盖了以下内容：

- 项目入口与发布配置：[Cargo.toml](../Cargo.toml)、[README.md](../README.md)、[CONTRIBUTING.md](../CONTRIBUTING.md)
- 质量门禁与发布脚本：[scripts/precheck.sh](../scripts/precheck.sh)、[scripts/release_prepare.sh](../scripts/release_prepare.sh)、[scripts/bench.sh](../scripts/bench.sh)
- 核心实现：[src/lib.rs](../src/lib.rs)、[src/main.rs](../src/main.rs)、[src/core/find.rs](../src/core/find.rs)、[src/core/mv.rs](../src/core/mv.rs)、[src/core/rename.rs](../src/core/rename.rs)
- CLI 层：[src/commands/mod.rs](../src/commands/mod.rs)、[src/commands/find.rs](../src/commands/find.rs)、[src/commands/mv.rs](../src/commands/mv.rs)、[src/commands/rename.rs](../src/commands/rename.rs)
- 测试与设计文档：[tests/cli_tests.rs](../tests/cli_tests.rs)、[tests/lib_mv_tests.rs](../tests/lib_mv_tests.rs)、[doc/DESIGN.md](./DESIGN.md)、[doc/TESTING.md](./TESTING.md)、[doc/DirectoryMove.md](./DirectoryMove.md)

另外执行了仓库质量门禁：

```bash
./scripts/precheck.sh --check
```

结果：`cargo check`、`cargo clippy`、`cargo test`、`cargo test --benches --no-run` 全部通过；当前测试结果为 `268 passed, 1 ignored`。

## 总体判断

这个项目当前的基础质量是好的，问题不在“能不能工作”，而在“后续能否持续低成本演进”。

做得比较扎实的部分：

- CLI 与库层分离明确，公共 API 从 [src/lib.rs](../src/lib.rs) 暴露，命令行入口保持较薄。
- 错误类型集中在 [src/error.rs](../src/error.rs)，并使用 `thiserror`，整体方向正确。
- `mv` / `rename` 的实现已经考虑了预览、回滚、目录移动、链接重写、大小写仅变更等细节。
- 测试面较完整，既有库级行为测试，也有 CLI 级进程契约测试。
- 发布流程已经做了脚本化，包含格式化、lint、测试、构建体积记录、benchmark smoke check 与 changelog 生成。

因此，后续改进更适合围绕“可维护性、文档一致性、API 收敛、行为边界说明”来做，而不是大面积重写。

## 优先级最高的改进点

### 2. 拆分 [src/core/mv.rs](../src/core/mv.rs)，降低核心变更风险

优先级：高

观察依据：

- [src/core/mv.rs](../src/core/mv.rs) 当前约 `1773` 行，是项目里最重的核心文件。
- 该文件同时承担了路径校验、大小写场景处理、目录移动映射、外部引用规划、内部引用规划、文件改写、事务回滚、预览构造等多个职责。
- 这类文件即使功能正确，也会显著拉高后续 bug 修复和评审成本。

建议方向：

- 先按职责拆分，而不是按“函数多少”拆分。
- 比较自然的边界是：`validate`、`plan`、`apply`、`case_only`、`preview`、`transaction`。
- 第一阶段只做模块搬迁和命名收敛，不改行为；第二阶段再考虑局部抽象优化。


### 4. 收敛公共 API 表面积，重新评估 `*_with_progress` 这一层包装

优先级：中高

观察依据：

- [src/lib.rs](../src/lib.rs) 目前暴露了 `find_references` / `find_references_with_progress`、`mv` / `mv_with_progress`、`rename` / `rename_with_progress` 三组 API。
- [src/core/rename.rs](../src/core/rename.rs) 本质上又是对 `mv` 的语义包装。
- 这套 API 没有错，但会带来接口数量扩张、示例文档重复、后续参数演进时同步改动较多的问题。

建议方向：

- 评估是否统一为单一 API，并把 `progress: Option<&ProgressBar>` 当作显式可选参数。
- 如果希望保持易用性，可以引入参数结构体或 builder，而不是持续增加平行函数。
- 若短期不改 API，也建议至少在文档中明确“哪些函数是核心入口，哪些是便捷包装”。

### 5. 把符号链接与 canonicalize 语义写进文档，而不是只留在实现和测试里

优先级：中

观察依据：

- [src/core/find.rs](../src/core/find.rs) 与 [src/core/mv.rs](../src/core/mv.rs) 大量依赖 `canonicalize()` 来判断链接目标与移动路径。
- [tests/lib_mv_tests.rs](../tests/lib_mv_tests.rs) 已覆盖 Unix 下的符号链接场景，说明这块行为是系统语义的一部分，而不只是实现细节。
- 当前用户文档没有明确回答：是否跟随符号链接、跨平台差异是什么、符号链接引用在移动后如何处理。

建议方向：

- 在 [README.md](../README.md) 或专题文档中新增“路径解析语义”章节。
- 明确写出：跟随真实路径、断链如何处理、Unix/非 Unix 差异、目录扫描对符号链接的边界。
- 这会明显降低用户对“为什么引用被这样改写”的困惑。

### 6. 把 benchmark 工作流补齐到测试文档中

优先级：中

观察依据：

- [scripts/bench.sh](../scripts/bench.sh) 已经支持 `quick`、`full`、`save-baseline`、`compare`、`list`。
- [CONTRIBUTING.md](../CONTRIBUTING.md) 只给了很短的 benchmark 命令示例。
- [doc/TESTING.md](./TESTING.md) 目前完全没有解释性能测试与回归比对方式。

建议方向：

- 在 [doc/TESTING.md](./TESTING.md) 中加入 benchmark 一节。
- 说明什么时候跑 `quick`，什么时候跑 `full`，什么时候应该保存 baseline 和做 compare。
- 如果未来关注性能回归，可以把“如何解读 Criterion 输出”也补进去。

### 7. 把事务回滚的保证范围与限制说明得更清楚

优先级：中

观察依据：

- [src/core/model/move_transaction.rs](../src/core/model/move_transaction.rs) 已经把回滚行为建模得比较明确。
- [doc/DirectoryMove.md](./DirectoryMove.md) 也解释了目录移动的规划和回滚思路。
- 但目前仍然缺少更直接的说明：哪些场景是“尽量原子”，哪些场景受底层文件系统语义限制。

建议方向：

- 在 API rustdoc 或专题文档中写清楚“best-effort rollback”的边界。
- 区分 `rename` 型移动与 `copy + delete` 型移动的恢复方式。
- 对跨文件系统、权限错误、部分文件写入失败等情况给出用户可理解的行为说明。

### 8. 补充少量更贴近真实使用的 CLI 契约测试

优先级：中低

观察依据：

- [tests/cli_tests.rs](../tests/cli_tests.rs) 已经覆盖了版本、帮助、JSON 成功输出、JSON 错误输出与若干代表性命令流。
- 但全局 `--progress`、human 模式下 `dry-run` 的终端表现、部分边界组合参数仍然主要由命令模块内部测试承担。
- 当前测试策略是合理的，但再补几条“从二进制入口打到底”的薄集成测试，能更好守住 CLI 契约。

建议方向：

- 增加针对 `--progress` 的 smoke test，确认不会污染 JSON 输出。
- 增加 `mv --dry-run` / `rename --dry-run` 在 CLI 层的代表性测试。
- 保持这部分测试薄而少，重点覆盖入口契约，而不是重复库层逻辑。

### 9. 明确 nightly `rustfmt` 依赖，降低贡献者环境摩擦

优先级：中低

观察依据：

- [rust-toolchain.toml](../rust-toolchain.toml) 当前固定在稳定版 `1.95.0`。
- 但 [scripts/precheck.sh](../scripts/precheck.sh) 在本地模式下直接执行 `cargo +nightly fmt`。
- [CONTRIBUTING.md](../CONTRIBUTING.md) 目前要求运行 `./scripts/precheck.sh`，但没有明确说明需要额外安装 nightly 工具链。

建议方向：

- 在贡献文档里明确 nightly `rustfmt` 是前置依赖，或者给出一条安装命令。
- 如果并未使用 nightly 专属格式化能力，可以评估是否回到稳定版 `cargo fmt`。
- 至少要让失败信息和文档对齐，避免新贡献者第一次运行 precheck 就卡在环境问题上。

## 可以排在后面的改进点

### 10. 重新组织 [src/core/find.rs](../src/core/find.rs) 中的“解析逻辑 + 测试”排布

优先级：低到中

观察依据：

- [src/core/find.rs](../src/core/find.rs) 当前约 `959` 行。
- 文件同时包含 Markdown AST 遍历、reference definition 解析、路径解析和一大批测试。
- 这里的复杂度低于 `mv`，但阅读负担仍然偏高。

建议方向：

- 优先抽出 reference definition 解析与路径解析辅助函数。
- 如果后续继续演进 `find` 语义，再考虑把测试移动到独立模块或更细的子模块中。

### 11. 为核心模块补充更系统的 rustdoc 示例

优先级：低

观察依据：

- [src/core/rename.rs](../src/core/rename.rs) 的说明相对完整。
- 但核心 `find` / `mv` 的边界、返回值语义、典型调用方式并没有形成一致的 rustdoc 质量。

建议方向：

- 先给库公开入口补齐示例与参数语义。
- 再挑最容易被误用的函数补充失败场景说明。

## 建议的实施顺序

如果只安排一到两个迭代，我建议按下面顺序推进：

1. 先修文档一致性：更新 [doc/DESIGN.md](./DESIGN.md)，补充 benchmark / 路径语义说明。
2. 再做低风险重构：抽 `commands` 公共辅助逻辑。
3. 最后处理高收益重构：拆 [src/core/mv.rs](../src/core/mv.rs)。

这个顺序的好处是：先把认知成本降下来，再进入真正的结构调整，避免边改边猜系统边界。

## 结论

`mdref` 当前不是“需要救火”的项目，而是一个已经具备较好工程基础、值得继续打磨的项目。

最有价值的下一步，不是新增很多功能，而是把已经做出来的能力讲清楚、把最重的核心文件拆开、把命令层的重复收掉。这样能显著降低未来继续扩展 `find` / `mv` / `rename` 时的心智负担和回归风险。
