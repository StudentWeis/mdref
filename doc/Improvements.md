# mdref 缺陷与改进点分析

本文档记录 `mdref` 项目当前的缺陷、改进点及优先级建议。

## 一、功能缺陷

### 3. 目录级别操作支持有限

**问题**：
- `find` 对目录目标有基础支持，但 `mv` 移动整个目录未实现
- 缺少批量移动目录并更新所有引用的能力

**改进建议**：实现 `mv` 对目录的支持，包括：
- 递归移动目录下所有文件
- 批量更新所有引用路径

## 二、代码质量问题

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

---

## 三、架构改进点

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

## 四、测试与文档

### 2. 文档待完善

**问题**：
- 缺少 API 文档（`cargo doc` 生成）
- 缺少使用示例和最佳实践

**改进建议**：
- 为所有公开 API 添加文档注释
- 创建 `doc/Usage.md` 详细使用指南
- 创建 `doc/API.md` 库接口说明

## 六、优先级排序

| 优先级 | 改进项 | 原因 | 预估工作量 |
|--------|--------|------|------------|
| **P1** | 配置系统 | 灵活性需求 | 中 |
| **P1** | 日志系统 | 可维护性 | 低 |
| **P2** | JSON 输出格式 | CI/CD 集成需求 | 低 |
| **P2** | 进度反馈 | 用户体验 | 低 |
| **P2** | 错误处理优化 | 问题排查效率 | 中 |
| **P3** | 目录移动支持 | 功能完整性 | 高 |
| **P3** | VSCode 扩展 | 用户体验提升 | 高 |
