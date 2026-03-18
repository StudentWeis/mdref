# Benchmarking

本文档描述 `mdref` 当前 benchmark 系统的设计目标、场景覆盖和推荐用法。

## 设计目标

- 覆盖真实热点路径，而不是只测一个孤立函数
- 使用固定 profile 生成可重复的 fixture，保证不同提交之间可以比较
- 将 fixture 创建与被测代码分离，避免把数据生成成本混入结果
- 让结果名称稳定，便于 `criterion` baseline 对比

## 覆盖范围

当前 benchmark 关注两类场景：

- `find`
  - `find_links`
  - `find_references_file`
  - `find_references_directory`
- `move`
  - `mv_file`
  - `mv_directory`

`rename` 没有单独建 benchmark，因为它只是 `mv` 的轻量包装，额外收益很低。

## Fixture 设计

fixture 由 `benches/support/mod.rs` 统一生成，包含这些固定元素：

- `targets/hot.md`：文件级引用热点，也是 `mv_file` 的源文件
- `bundle/`：目录级引用热点，也是 `mv_directory` 的源目录
- `content/**/doc_*.md`：大量引用 `hot.md` 和 `bundle/` 的文档
- `assets/diagram.png`：让 `find_links` 能覆盖图片链接

每个 `content` 文档固定包含 6 个可识别链接：

- 3 个 inline link
- 1 个 image link
- 2 个 reference definition

这样 `find_links` 的链接密度稳定，`find_references` 的命中数也可直接推导。

## Profile

| Profile | Content dirs | Content docs | Markdown files |
|--------|--------------:|-------------:|---------------:|
| small  | 3 | 12 | 16 |
| medium | 7 | 42 | 46 |
| large  | 40 | 160 | 164 |

补充说明：

- `find` 组覆盖 `small` / `medium` / `large`
- `move` 组只覆盖 `small` / `medium`
- `move` 使用 `iter_batched`，每次迭代都会重新生成 fixture，避免文件移动带来的状态污染

## 运行方式

快速跑一轮：

```sh
script/bench.sh quick
```

保存 baseline：

```sh
script/bench.sh save-baseline main
```

和已有 baseline 做对比：

```sh
script/bench.sh compare main
```

`compare` 使用 Criterion 的严格 baseline 模式；如果 baseline 名称不存在，命令会直接失败，避免静默跳过比较。

如果只想跑某个场景，可以直接用 `criterion` filter：

```sh
cargo bench --bench benchmark -- find_references_file/medium --noplot
```

## 结果解读

- `find_links` 的 throughput 使用代表文档字节数
- `find_references_*` 的 throughput 使用扫描的 Markdown 文件数
- `mv_*` 的 throughput 使用本次需要更新的引用数量

因此同一 benchmark 名称在不同 profile 之间可以直接比较扩展性，在同一 profile 的不同提交之间可以直接比较回归。
