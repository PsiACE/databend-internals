+++
title = "如何为 Databend 添加新的测试"
description = "测试是提高软件健壮性、加速迭代进程的不二法宝。本文将会介绍如何为 Databend 添加不同种类的测试。"
draft = false
weight = 620
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "测试是提高软件健壮性、加速迭代进程的不二法宝。本文将会介绍如何为 Databend 添加不同种类的测试。"
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

## 如何编写和运行功能测试

在全新的 SQL 逻辑测试加入之后，功能测试暂时出现两种方案并行的情况，在接下来的一段时间应该会逐步过渡到 SQL 逻辑测试。

从本质上讲，这两类功能测试都遵循 Golden Files 风格，总体上的流程都是先启动 databend 实例，然后使用对应的客户端/驱动去执行查询，再比较查询结果和预期结果之间的差异，并判断测试是否通过。

sqllogictest 从设计上会提供更全面的能力：

- 拓展比较结果文件的方式到其他协议（涵盖 http handler）
- 提示每个语句的结果
- 提供错误处理的能力
- 支持排序、重试等测试逻辑

### 编写

**stateless/statefull 测试**

stateless/statefull 测试放在 `tests/suites` 目录下。

输入是一系列 sql ，对应目录中的 `*.sql` 文件。

```sql
SELECT '==Array(Int32)==';

CREATE TABLE IF NOT EXISTS t2(id Int null, arr Array(Int32) null) Engine = Fuse;

INSERT INTO t2 VALUES(1, [1,2,3]);
INSERT INTO t2 VALUES(2, [1,2,4]);
INSERT INTO t2 VALUES(3, [3,4,5]);
SELECT max(arr), min(arr) FROM t2;
SELECT arg_max(id, arr), arg_min(id, arr) FROM (SELECT id, arr FROM t2);
```

输出是一系列纯文本，如果没有输出则需要置空，对应目录中的 `*.result` 文件。

```text
==Array(Int32)==
[3, 4, 5]	[1, 2, 3]
3	1
```

对于 SQL 中存在错误的情况，有两种方式：

- 既可以沿用上面的方式，此时同样需要在 result 中标注。
- 也可以采用 `ErrorCode` 注释的方式，这里在 result 中置空就好。

```sql
SELECT INET_ATON('hello');-- {ErrorCode 1060}
```

**sqllogictest 测试**

sqllogictest 测试放在 `tests/logictest` 目录下。

语句规范在 sqlite sqllogictest 的基础上进行拓展，可以分成以下几类：

- `statement ok` ：SQL 语句正确，且成功执行。
- `statement error <error regex>` ：SQL 语句输出期望的错误。
- `statement query <desired_query_schema_type> <options> <labels>` ：SQL语句成功执行并输出预期结果。

```sql
statement query B label(mysql,http)
select count(1) > 1 from information_schema.columns;

----  mysql
1

----  http
true
```

上面的例子展示了如何对 mysql 和 http 分别设计对应的输出结果。

sqllogictest 同样支持生成测试用例 `python3 gen_suites.py` 。

### 运行

这几类测试都有对应的 `make` 命令：

- `stateless` 测试：`make stateless-test` 。
- `sqllogictest` 测试：`make sqllogic-test` 。
