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

### 4. 引用式链接（Reference Links）未支持

**问题**：Markdown 支持引用式链接语法，当前未在 `mv` 流程中处理：

```markdown
[text][ref]

[ref]: ./file.md
```

**改进建议**：扩展 `find` 和 `mv` 以支持 `link reference definitions` 的查找和更新。

---

## 二、代码质量问题

### 1. Reference 模型信息不足

**问题**：当前 `Reference` 结构体缺少关键信息。

```rust
pub struct Reference {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub link_text: String,  // 只有 link，没有 link text
}
```

**缺失信息**：
- 链接文本 `[text]` 部分
- 原始整行内容
- 链接类型（图片/文档/外部 URL）

**改进建议**：

```rust
pub struct Reference {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub link_text: String,
    pub link_title: Option<String>,      // 新增：链接显示文本
    pub link_type: LinkType,              // 新增：链接类型
}

pub enum LinkType {
    Document,
    Image,
    External,
    Anchor,
}
```

### 2. 错误处理粒度不够

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

### 1. 测试覆盖不全

**问题**：
- 未覆盖 Windows 路径场景（跨平台兼容性）
- 未测试 Unicode 文件名和路径
- 边界条件：空文件、超大文件、符号链接

**改进建议**：添加测试用例：
- Windows 风格路径 (`C:\path\file.md`, `.\relative\path.md`)
- Unicode 文件名 (`中文文档.md`, `ドキュメント.md`)
- 特殊场景（空文件、符号链接、权限问题）

### 2. 文档待完善

**问题**：
- 缺少 API 文档（`cargo doc` 生成）
- 缺少使用示例和最佳实践

**改进建议**：
- 为所有公开 API 添加文档注释
- 创建 `doc/Usage.md` 详细使用指南
- 创建 `doc/API.md` 库接口说明

---

## 五、性能优化空间

### 1. 文件过滤时机

**问题**：当前在遍历后过滤，而非遍历前配置。

```rust
// 当前实现
WalkDir::new(root_dir)
    .into_iter()
    .par_bridge()
    .filter(|e| e.path().extension()... == Some("md"))
```

**改进建议**：使用 `WalkDir` 的配置方法：

```rust
WalkDir::new(root_dir)
    .into_iter()
    .filter_entry(|e| {
        e.file_type().is_dir() || 
        e.path().extension().and_then(|s| s.to_str()) == Some("md")
    })
```

### 2. 潜在内存优化

**问题**：大文件一次性读入内存。

**改进建议**：对于超大文件考虑流式处理（通常 Markdown 文件较小，优先级较低）。

---

## 六、优先级排序

| 优先级 | 改进项 | 原因 | 预估工作量 |
|--------|--------|------|------------|
| **P0** | 带空格路径支持 | 影响基本功能可用性 | 中 |
| **P0** | `is_external_url` 修复 | 可能导致链接误判 | 低 |
| **P1** | 引用式链接支持 | 常见 Markdown 语法 | 中 |
| **P1** | 配置系统 | 灵活性需求 | 中 |
| **P1** | 日志系统 | 可维护性 | 低 |
| **P2** | JSON 输出格式 | CI/CD 集成需求 | 低 |
| **P2** | 进度反馈 | 用户体验 | 低 |
| **P2** | 错误处理优化 | 问题排查效率 | 中 |
| **P3** | 目录移动支持 | 功能完整性 | 高 |
| **P3** | VSCode 扩展 | 用户体验提升 | 高 |
| **P3** | Reference 模型扩展 | 架构优化 | 中 |

---

## 七、总结

`mdref` 当前架构实用且高效，核心功能稳定。主要改进方向：

1. **功能完整性**：支持更多 Markdown 语法、路径格式
2. **可配置性**：添加配置系统提升灵活性
3. **可观测性**：日志、进度、JSON 输出
4. **跨平台兼容**：Windows 路径、Unicode 支持

建议按 P0 → P1 → P2 → P3 顺序逐步推进改进。
