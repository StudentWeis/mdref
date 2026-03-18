- 可以阅读 doc 目录中的文档快速了解项目。
- 注释应该是通用的、规范的、简洁的。
- 遵循 TDD（测试驱动开发）的思路，优先编写测试，再编写实现代码。
- 使用 context7 查询库文档。
- 最好使用 thiserror 来定义错误类型。
- 使用 script/precheck.sh 来检查代码质量和格式。

测试相关：

- 如使用 unwrap() 和 expect()，需添加 #[allow(clippy::unwrap_used)]、#[allow(clippy::expect_used)] 来避免 clippy 警告。
- 使用 tempfile 库创建临时文件。
- 使用 rstest 进行参数化测试。
