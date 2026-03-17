# mdref 项目评估

> 评估日期：2026-03-17  
> 评估范围：项目结构、核心实现、测试覆盖、CLI 行为

## 结论

项目基础质量不错：

- 模块划分清晰，`find` / `mv` / `rename` 的职责边界明确
- 测试覆盖较全面，包含库测试、CLI 测试和不少边界场景
- 本地检查通过：
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo fmt --check`

当前最值得优先处理的不是新增功能，而是几处会影响真实 Markdown 仓库可用性的正确性问题。

## 已确认的问题

### 1. 引用式链接定义的替换规则过于脆弱

相关代码：

- `src/core/mv.rs` 中 `build_replacement_patterns`
- `src/core/mv.rs` 中 `apply_replacements`

当前对引用式定义的替换依赖固定模式：

```text
]: old_url
```

这会导致合法 Markdown 写法无法更新，例如：

- `[ref]: <target.md>`
- `[ref]:    target.md`

本地复现结果：执行 `mv` 时会直接失败，并报出类似错误：

```text
Could not find link ']: target.md'
```

影响：

- `mv` 在真实文档风格下不够稳健
- 引用定义一旦使用尖括号或额外空格，功能就会中断

建议：

- 不要依赖固定字符串替换
- 在扫描阶段保存更精确的位置信息或原始 span
- 将引用定义的更新改成结构化重写，而不是文本片段匹配

### 3. URL 解码逻辑无法正确处理 Unicode 文件名

相关代码：

- `src/core/util.rs` 中 `url_decode_link`
- `src/core/find.rs` 中 `resolve_link`
- `src/core/mv.rs` 中 `resolve_reference_target`

当前 `url_decode_link` 是手写的 `%xx -> byte as char` 解码，只对 ASCII 场景基本可用。  
对于 UTF-8 百分号编码路径，如：

```text
%E4%B8%AD%E6%96%87.md
```

不会被正确还原为 `中文.md`。

本地复现结果：

- `find` 找不到对 `中文.md` 的 URL 编码引用
- `mv` 也不会更新这类链接

影响：

- 国际化文件名支持不完整
- 对包含非 ASCII 路径的文档仓库不可靠

建议：

- 使用成熟库处理 percent-decoding，而不是手写字节转字符
- 补充 URL 编码 Unicode 场景的回归测试

### 4. 扫描阶段会静默吞掉错误

相关代码：

- `src/core/find.rs` 中 `find_references`

当前实现中：

- `WalkDir` 的错误被 `e.ok()` 忽略
- `fs::read_to_string` 的错误也被 `ok()` 忽略

本地复现结果：

- 当引用文件不可读时，命令仍然成功退出
- 用户看到的是 “No references found”
- 实际上是文件没有被扫描

影响：

- 输出结果可能不可信
- 用户很难区分“确实没有引用”和“扫描失败”

建议：

- 至少以 warning 形式暴露被跳过的文件
- 或提供显式的容错模式，例如 `--ignore-errors`
- 默认行为不应静默吞错

### 5. 改写文件时会破坏原始换行风格

相关代码：

- `src/core/mv.rs` 中 `apply_replacements`

当前逻辑使用：

- `content.lines()`
- `lines.join("\n")`

这会把原本的 CRLF 文件统一写回 LF，只保留“是否有末尾换行”。

本地复现结果：

- 原文件使用 `\r\n`
- 执行 `mv` 后被改成 `\n`

影响：

- 会产生额外 diff
- 不利于跨平台仓库协作

建议：

- 保留原始换行风格
- 将文本替换逻辑改为基于原始 buffer 的位置更新，避免重建整份文件格式

## 次一级改进点

### 1. 回滚承诺与实现之间存在落差

相关代码：

- `src/core/mv.rs` 中 `mv_regular_file`
- `src/core/mv.rs` 中 `mv_directory`
- `src/core/model/move_transaction.rs`

文档注释强调了接近“原子”的回滚语义，但并不是所有文件系统操作都处于同一层保护下。  
这不一定马上会出 bug，但当前表述偏强，后续容易造成理解偏差。

建议：

- 要么继续强化事务边界
- 要么收紧文档表述，避免过度承诺

### 2. README 与当前能力不完全同步

相关文件：

- `README.md`

README 仍把“Directory path support”放在 TODO 中，但当前代码与测试已经支持目录移动和目录引用更新。

建议：

- 更新 README，区分“已支持能力”和“已知限制”
- 把真实限制写清楚，例如引用定义兼容性、错误处理策略等

## 推荐的处理顺序

### 第一优先级

1. 修复引用定义的解析与替换逻辑
2. 修复 URL 编码 Unicode 路径处理
3. 停止静默吞掉扫描错误

### 第二优先级

1. 保留原始换行风格
2. 收紧回滚语义或实现
3. 补齐对应的回归测试

### 第三优先级

1. 更新 README 和使用文档
2. 再考虑配置系统、JSON 输出、日志级别等增强项

## 建议新增的测试用例

- 引用定义使用尖括号：`[ref]: <target.md>`
- 引用定义使用多个空格：`[ref]:    target.md`
- fenced code block 中包含引用定义示例
- URL 编码的 Unicode 文件名引用
- CRLF 文件在改写后仍保持 CRLF
- 扫描遇到不可读文件时的 CLI 行为

## 总结

这个项目已经具备继续打磨的基础，尤其是测试体系和核心能力方向是对的。  
当前最关键的问题集中在“Markdown 兼容性”和“结果可信度”上。只要先把这几处正确性问题补齐，项目的实际可用性会明显提升，再去做配置、输出格式和集成能力会更稳。
