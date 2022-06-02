+++
title = "编写和运行测试"
description = "测试是提高软件健壮性、加速迭代进程的不二法宝。本文将会介绍如何为 Databend 添加不同种类的测试。"
draft = false
weight = 620
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "测试是提高软件健壮性、加速迭代进程的不二法宝。本文将会介绍如何为 Databend 添加并运行不同种类的测试。"
toc = true
top = false
giscus = true
+++

> 在「[产品力：Databend 的质量保障](https://psiace.github.io/databend-internals/docs/productivity-topics/quality-assurance-in-databend/)」一文中，已经介绍到组成 Databend 测试的两个重要部分 —— 单元测试和功能测试。如有遗忘，不妨回顾一下。

## 如何编写和运行单元测试

Databend 的单元测试组织形式有别于一般的 Rust 项目，是直接一股脑放在 `tests/it` 目录下的。同时，在各个 crate 的 `Cargo.toml` 中，也针对性地禁用了 `doctest` 和 `bin/lib test` 。

**优点**

- 减少需要构建的测试目标，提高测试编译/链接速度。*
- 当需要添加新单元测试时（不修改 `src`），只需要编译对应的 `it(test)` ，节省时间。

**缺点**

由于 `tests/it` 会把需要测试的 crate 当作一个外部对象，所有待测试的内容都需要被设定为 `pub` 。不利于软件设计上的分层，整个项目结构会迅速的被破坏，需要引入编码规范并更加依赖开发者的主动维护。

### 编写

可以简单地将单元测试分为两类，一类是不需要外部文件介入的纯 Rust 测试，一类是 Golden Files 测试。

**纯 Rust 测试**

与平时编写 Rust 单元测试相同，只是引用待测试 crate 时需要使用该 crate 的名字，且待测试的内容需要设为 `pub` 。另外，Databend 内部有一些用于模拟全局状态的函数，可能会有助于编写测试。

```Rust
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_credits_table() -> Result<()> {
    let ctx = crate::tests::create_query_context().await?;

    let table = CreditsTable::create(1);
    let source_plan = table.read_plan(ctx.clone(), None).await?;

    let stream = table.read(ctx, &source_plan).await?;
    let result = stream.try_collect::<Vec<_>>().await?;
    let block = &result[0];
    assert_eq!(block.num_columns(), 3);
    Ok(())
}
```

上面示例用于粗浅测试 `credits_table`，构建 `read_plan` 来读取新建的 `CreditsTable` 表，再对列数进行断言。

**Golden Files 测试**

Golden Files 测试是一种常用的测试手段，如果执行情况和预期结果存在差异则认为测试失败。进来，

Databend 使用 `goldenfile` 这个 crate 来编写 Rust 中的 Golden Files 测试。目前 Databend 有计划用此替代 `assert_blocks` 系列断言。

```Rust
#[test]
fn test_expr_error() {
    let mut mint = Mint::new("tests/it/testdata");
    let mut file = mint.new_goldenfile("expr-error.txt").unwrap();

    let cases = &[
        r#"5 * (a and ) 1"#,
        r#"a + +"#,
        r#"CAST(col1 AS foo)"#,
        // TODO(andylokandy): This is a bug being tracking in https://github.com/segeljakt/pratt/issues/7
        r#"1 a"#,
        r#"CAST(col1)"#,
        r#"G.E.B IS NOT NULL AND
            col1 NOT BETWEEN col2 AND
                AND 1 + col3 DIV sum(col4)"#,
    ];

    for case in cases {
        run_parser!(file, expr, case);
    }
}
```

编写 Golden Files 测试需要指定挂载的目录和对应预期结果的文件。在执行测试的主体部分（在 `run_parser` 这个宏中），除了封装运行测试的必要逻辑外，还需要定义输出时的格式。

同时，测试文件必须按指定格式编写。或者，使用 `REGENERATE_GOLDENFILES=1` 会重新生成。

下面 Golden File 的例子节选自 `common/ast` 模块测试的 `testdata/expr-error.txt`，`Output` 对应解析 `5 * (a and ) 1` 的预期结果。

```text
---------- Input ----------
5 * (a and ) 1
---------- Output ---------
error: 
  --> SQL:1:12
  |
1 | 5 * (a and ) 1
  | -          ^ expected more tokens for expression
  | |           
  | while parsing expression
```

### 运行

单元测试的运行可以运行 `make unit-test` 或者是 `cargo test --workspace` 。

二者的区别在于 `make unit-test` 封装了 ulimit 命令控制最大文件数和栈的大小以确保测试能够顺利运行，如果使用 MacOS 则更建议使用 `make unit-test` 。

通过过滤机制，可以轻松指定运行名字中具有特定内容的测试，例如 `cargo test test_expr_error` 。
