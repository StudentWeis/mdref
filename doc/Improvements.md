# mdref 缺陷与改进点分析

本文档记录 `mdref` 项目当前的缺陷、改进点及优先级建议。

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
```
