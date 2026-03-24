# mdref 缺陷与改进点分析

本文档记录 `mdref` 项目当前的缺陷、改进点及优先级建议。

### P1: 缺少 `.gitignore` 感知

**文件**：`src/core/find.rs` — `find_references()`、`src/core/mv.rs` — `collect_markdown_files()`

**问题**：`walkdir` 会遍历 `.git`、`node_modules`、`target` 等目录中的 `.md` 文件。在大型项目中既浪费性能，也可能产生误匹配。

**修复方向**：使用 `ignore` crate（ripgrep 使用的同一个库）替代 `walkdir`，自动尊重 `.gitignore` 规则。

### P2: 同文件系统移动未优先使用 `fs::rename`

**文件**：`src/core/mv.rs` — `mv_regular_file()`

**问题**：当前使用 `fs::copy` + `fs::remove_file` 实现文件移动。对于同一文件系统内的移动，`fs::rename` 是原子操作且更高效。copy-then-delete 方式在大文件场景下性能差，且在 copy 成功但 delete 失败时会留下重复文件。

**修复方向**：优先尝试 `fs::rename`，失败时（跨文件系统）再 fallback 到 copy + delete。

### P3: `process_md_file` 注释步骤编号跳跃

**文件**：`src/core/find.rs` — `process_md_file()`

**问题**：注释中步骤编号从 Step 1 → Step 2 → Step 4，缺少 Step 3，疑似重构后遗留。

---

## 代码质量问题

### 错误处理粒度不够

**问题**：缺少具体的错误上下文，用户难以定位问题根源。

**改进建议**：扩展错误类型，增加更多上下文信息：

```rust
#[derive(Error, Debug)]
pub enum MdrefError {
    #[error("IO error reading '{path}': {source}")]
    IoRead { path: PathBuf, source: std::io::Error },

    #[error("IO error writing '{path}': {source}")]
    IoWrite { path: PathBuf, source: std::io::Error },

    #[error("Invalid link syntax at {file}:{line}:{column}: {details}")]
    InvalidLinkSyntax {
        file: PathBuf,
        line: usize,
        column: usize,
        details: String,
    },

    #[error("Unsupported link type: {0}")]
    UnsupportedLinkType(String),
}
```

## 架构改进点

### 1. 缺少配置系统

**问题**：
- 硬编码 `.md` 扩展名，不支持自定义
- 无法配置忽略目录（如 `.git`、`node_modules`）

**改进建议**：添加 `.mdref.toml` 配置文件：

```toml
[general]
# 支持的文件扩展名
extensions = ["md", "markdown"]

# 忽略的目录
ignore_dirs = [".git", "node_modules", "target", "build"]

# 是否跟随符号链接
follow_symlinks = false

[output]
# 默认输出格式: human, json
format = "human"
```

### 2. 缺少进度反馈机制

**问题**：大规模目录扫描时无进度指示，移动操作无进度条显示。

**改进建议**：
- 添加 `--progress` 标志显示进度条
- 使用 `indicatif` crate 实现进度显示

### 3. 命令层输出格式单一

**问题**：仅支持人类可读格式，无 JSON/机器可读输出，不便于集成到 CI/CD。

**改进建议**：添加 `--format json` 选项：

```json
{
  "operation": "find",
  "target": "./examples/main.md",
  "references": [
    {
      "path": "./examples/other.md",
      "line": 7,
      "column": 1,
      "link_text": "main.md"
    }
  ]
}
```

### 4. 缺少日志系统

**问题**：无 `--verbose` 或 `--debug` 模式，排查问题困难。

**改进建议**：集成 `tracing` 或 `log` crate，支持多级别日志：

```rust
use tracing::{info, debug, warn};

debug!("Processing file: {}", path.display());
info!("Found {} references", refs.len());
warn!("Skipping unsupported link: {}", link);
```

---

## 测试与文档

### 文档待完善

**问题**：
- 缺少 API 文档（`cargo doc` 生成）
- 缺少使用示例和最佳实践

**改进建议**：
- 为所有公开 API 添加文档注释
- 创建 `doc/Usage.md` 详细使用指南
- 创建 `doc/API.md` 库接口说明
