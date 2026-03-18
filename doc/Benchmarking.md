# Benchmarking

本文档是 `mdref` 当前 benchmark 体系的正式评审报告与使用说明，覆盖以下内容：

- benchmark 设计目标与当前覆盖范围
- fixture 与指标口径说明
- 现状评估、主要风险与优先级判断
- 建议的改进方向与落地顺序
- 当前阶段的推荐使用方式

本文结论基于以下实现与配套内容：

- `benches/benchmark.rs`
- `benches/support/mod.rs`
- `script/bench.sh`
- `tests/bench_fixture_tests.rs`
- `tests/bench_script_tests.rs`

## 执行摘要

`mdref` 已经具备一套结构清晰、可以投入日常使用的 benchmark 体系。当前方案具备以下优势：

- 使用 `criterion`，支持稳定命名和 baseline 对比
- 使用统一 fixture 生成逻辑，避免各 benchmark 自行拼装数据
- 重点覆盖 `find` 与 `move` 这两条真实热点路径，而不是仅测试底层微小函数
- 提供统一脚本入口，便于本地 smoke、baseline 保存和回归比较

整体判断如下：

- **作为本地性能观察工具：可用且已有较好基础**
- **作为稳定的性能回归依据：仍需补强计时边界、统计口径、测试覆盖和自动化验证**

当前最重要的两个问题是：

1. `mv_*` benchmark 的计时区间可能混入 fixture 析构与临时目录清理成本。
2. `mv_directory` 的 throughput 指标目前更接近近似值，而非一次目录移动的实际总 rewrite 数。

## 设计目标

当前 benchmark 体系的设计目标可以归纳为以下几点：

1. **覆盖真实热点路径**  
   优先衡量用户实际感知到的高频操作，而不是只测单个纯函数。

2. **保证结果可比较**  
   使用固定 profile 和稳定命名，便于不同提交之间进行 `criterion` baseline 对比。

3. **隔离数据准备成本**  
   fixture 生成由统一模块负责，尽量避免把测试数据构建混入被测逻辑时间。

4. **降低使用门槛**  
   通过 `script/bench.sh` 提供统一入口，减少基准执行方式分散的问题。

从方向上看，这些目标合理，且当前实现已经覆盖了大部分目标。

## 当前覆盖范围

当前 benchmark 覆盖两组场景：

- `find`
  - `find_links`
  - `find_references_file`
  - `find_references_directory`
- `move`
  - `mv_file`
  - `mv_directory`

`rename` 当前未单独建立 benchmark，这一决定在现阶段是合理的，因为它主要是 `mv` 的轻量包装，单独维护一套基准的收益较低。

### Profile 覆盖

| Profile | Content dirs | Content docs | Markdown files |
|--------|--------------:|-------------:|---------------:|
| small  | 3 | 12 | 16 |
| medium | 7 | 42 | 46 |
| large  | 40 | 160 | 164 |

补充说明：

- `find` 组覆盖 `small` / `medium` / `large`
- `move` 组覆盖 `small` / `medium`
- `move` 使用 `iter_batched`，以避免单次移动造成 fixture 状态污染

## Fixture 设计评估

fixture 由 `benches/support/mod.rs` 统一生成，核心结构如下：

- `targets/hot.md`：文件级引用热点，也是 `mv_file` 的源文件
- `bundle/`：目录级引用热点，也是 `mv_directory` 的源目录
- `content/**/doc_*.md`：大批量引用 `hot.md` 和 `bundle/` 的 Markdown 文档
- `assets/diagram.png`：用于覆盖图片链接场景

每个 `content` 文档固定包含 6 个可识别链接：

- 3 个 inline link
- 1 个 image link
- 2 个 reference definition

### 优点

- fixture 结构统一，统计口径集中在 `FixtureSummary` 中维护
- 文档链接密度固定，便于 `find_links` 和 `find_references` 的横向比较
- 数据规模随 profile 变化，能够观察不同规模下的扩展性
- `move` 场景使用临时目录，可避免多次迭代之间互相污染

### 当前限制

- `FixtureSummary` 对 `mv_directory` 的吞吐定义尚未覆盖“总 rewrite 数”
- correctness 测试主要覆盖 `Small` profile，不能完全证明全部 profile 的统计一致性

总体来看，fixture 设计本身是 benchmark 体系的强项，但仍需要通过测试和更精确的 summary 统计提升可信度。

## 指标口径说明

当前 benchmark 的 throughput 口径如下：

- `find_links`：代表文档字节数
- `find_references_*`：扫描的 Markdown 文件数
- `mv_file`：指向热点文件的引用数量
- `mv_directory`：指向热点目录的主要引用数量

### 解释建议

上述口径中，前三项语义相对稳定；`mv_directory` 需要特别说明：

- 它当前更接近“主要引用规模”的近似值
- 它不完全等于目录移动过程中发生的全部 rewrite 数
- 因此其 `elements/sec` 更适合做**同一实现上的相对比较**，不宜被过度解读为绝对吞吐能力

如果未来要把 `mv_directory` 结果作为严格性能门禁依据，建议将 summary 扩展为“实际总 rewrite 数”。

## 现状评估

### 总体评价

当前 benchmark 体系已经满足“可运行、可对比、可用于本地观察”的基本要求，但距离“高可信回归分析工具”还有差距。

### 主要优点

1. **覆盖真实业务路径**  
   benchmark 直接调用 `find_links`、`find_references`、`mv` 等实际路径，结果更接近真实使用场景。

2. **profile 与命名稳定**  
   这为 `criterion` baseline 对比提供了良好基础。

3. **fixture 生成集中管理**  
   有利于统一数据规模、统计口径和测试校验。

4. **执行入口统一**  
   `script/bench.sh` 已经把常用运行模式标准化，便于团队日常使用。

### 主要问题

#### 1. `mv_*` benchmark 的计时边界不够干净

当前 `move` 组虽然通过 `iter_batched` 避免了 fixture 构建成本进入计时区间，但 timed closure 中仍持有整个 `BenchmarkFixture`。由于该结构内部持有 `TempDir`，closure 结束时触发的临时目录清理可能落入计时范围。

这会带来以下影响：

- benchmark 结果混入文件系统删除成本
- 样本之间的波动增大
- 结果更容易受到目录规模和运行环境差异影响

这是当前 benchmark 体系中**优先级最高的准确性问题**。

#### 2. `mv_directory` throughput 的语义不够严格

当前 `bundle_directory_references` 主要反映外部文档对目标目录的引用规模，但目录移动时实际需要重写的链接可能更多，包括目录内部文件对外部目标的相对链接调整。

这意味着：

- 当前 throughput 是有价值的近似指标
- 但它不是严格意义上的“总修改量”
- 指标解释需要在文档中显式约束

#### 3. `move` 的 profile 覆盖弱于 `find`

当前 `find` 覆盖 `small` / `medium` / `large`，而 `move` 只覆盖 `small` / `medium`。这会导致：

- `move` 在更大规模下的扩展性缺少直接观察数据
- `find` 与 `move` 的性能结论粒度不一致

如果这是基于执行时间或环境噪声做出的权衡，建议在文档中明确说明原因。

#### 4. fixture 测试覆盖范围不足

`tests/bench_fixture_tests.rs` 当前主要验证 `Small` profile。这样无法充分保证：

- `Medium` / `Large` 的目录结构正确
- summary 计数与实际引用数一致
- 不同规模下关键 benchmark 路径始终存在

这类问题不会一定导致 benchmark 失败，但会导致结果口径悄悄漂移。

#### 5. bench 脚本测试覆盖不完整

`tests/bench_script_tests.rs` 目前重点覆盖了 `compare` 模式必须使用严格 baseline，这一点是正确且重要的；但其余模式仍缺少自动化保护，包括：

- `quick`
- `full`
- `save-baseline`
- `list`
- 非法参数
- 缺少 baseline 参数时的 usage 分支

这会提高脚本回归风险。

#### 6. 缺少 CI benchmark smoke 或回归门禁

当前 benchmark 更像是本地能力，而不是持续验证的一部分。这意味着：

- benchmark 代码能编译，不代表 benchmark 流程持续可执行
- 性能回归更多依赖人工观察
- benchmark 入口脚本的稳定性缺少持续验证

## 问题优先级

建议按以下优先级处理：

| 优先级 | 问题 | 原因 |
|---|---|---|
| P1 | `mv_*` 计时边界 | 直接影响 benchmark 结果准确性 |
| P1 | `mv_directory` throughput 语义 | 直接影响结果解释正确性 |
| P2 | fixture 测试扩展到全部 profile | 防止 benchmark 数据口径漂移 |
| P2 | bench 脚本测试补齐 | 提高入口稳定性 |
| P3 | CI benchmark smoke | 让 benchmark 持续可执行 |
| P3 | `move` 大规模 profile 与抗噪优化 | 提升长期观测价值 |

## 改进建议

### 1. 修正 `mv_*` 的计时边界

目标是确保被计时的区间尽量只包含 `mv(...)` 本身，而不包含 fixture 清理。

建议做法：

- 让 fixture 析构发生在计时区间之外
- 明确区分 setup、measured work、teardown 三个阶段
- 如有必要，重新组织 benchmark closure，避免通过 `black_box(fixture)` 间接延长 fixture 生命周期

### 2. 明确 `mv_directory` 的 throughput 统计语义

有两种可接受方案：

- **方案 A：扩展 summary**  
  新增“实际总 rewrite 数”统计，使 `mv_directory` 的吞吐口径更严格。

- **方案 B：保留当前统计并修正文档**  
  明确说明当前值仅表示主要引用规模，用于相对比较，不代表全部修改量。

如果短期优先控制改动规模，建议先采用方案 B；如果后续要把 benchmark 纳入更严格的性能回归流程，建议推进方案 A。

### 3. 扩展 fixture correctness 测试

建议至少覆盖：

- `Small` / `Medium` / `Large` 的 summary 校验
- `find_references(...)` 返回结果与 summary 的一致性校验
- 关键路径文件与目录存在性校验

这样可以显著降低 benchmark 结果口径漂移的风险。

### 4. 补齐 `script/bench.sh` 的行为测试

建议新增以下脚本测试：

- `quick` 是否正确转发参数
- `full` 是否正确转发参数
- `save-baseline <name>` 是否传递正确 baseline 参数
- `list` 是否输出预期调用模式
- 非法参数是否返回错误并输出 usage
- 缺少 baseline 名称时是否返回错误并输出 usage

### 5. 增加 CI benchmark smoke

建议最低成本地加入一轮：

- `script/bench.sh quick`

目标不是立即建立性能门禁，而是先保证 benchmark 流程持续可运行。

### 6. 提升 benchmark 可重复性

对于包含文件系统操作的 benchmark，建议进一步降低环境噪声，例如：

- 在对比基线时固定线程数
- 评估是否需要提高 `move` 组的 sample size 或 measurement time
- 在文档中明确推荐的 benchmark 运行环境和解读方式

## 当前阶段的推荐使用方式

在问题尚未全部修复前，建议按以下方式使用 benchmark：

- 本地开发或提交前：运行 `script/bench.sh quick` 做 smoke check
- 准备进行性能比较时：先执行 `script/bench.sh save-baseline <name>` 保存 baseline
- 对比实现差异时：执行 `script/bench.sh compare <name>`
- 解读 `mv_directory` 结果时：重点关注不同提交之间的相对变化，不要过度解读其绝对吞吐值

如果只想运行单个场景，可以直接使用 `criterion` filter，例如：

```sh
cargo bench --bench benchmark -- find_references_file/medium --noplot
```

## 结论

`mdref` 当前 benchmark 体系的基础设计是正确的，尤其在以下方面表现较好：

- 场景选择贴近真实热点路径
- fixture 生成集中统一
- profile 与 benchmark 命名稳定
- baseline 对比流程清晰

但要把这套体系提升为更可信的性能回归工具，仍建议优先完成以下事项：

1. 修正 `mv_*` 的计时边界。
2. 明确或收紧 `mv_directory` 的 throughput 语义。
3. 将 fixture 与脚本测试补齐到更完整的覆盖范围。
4. 在 CI 中加入至少一轮 benchmark smoke。

在这些问题修复之前，可以继续将当前 benchmark 体系用于日常性能观察和提交间相对比较，但不建议对个别绝对吞吐值做过强结论。
