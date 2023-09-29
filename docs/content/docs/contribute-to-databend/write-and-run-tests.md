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

> 在「[产品力：Databend 的质量保障](https://databend-internals.psiace.me/docs/productivity-topics/quality-assurance-in-databend/)」一文中，已经介绍到组成 Databend 测试的两个重要部分 —— 单元测试和功能测试。如有遗忘，不妨回顾一下。

## 单元测试

Databend 的单元测试组织形式有别于一般的 Rust 项目，是直接一股脑放在 `tests/it` 目录下的。同时，在各个 crate 的 `Cargo.toml` 中，也针对性地禁用了 `doctest` 和 `bin/lib test` 。

**优点**

- 减少需要构建的测试目标，提高测试编译/链接速度。
- 当需要添加新单元测试时（不修改 `src`），只需要编译对应的 `it(test)` ，节省时间。

**缺点**

- `tests/it` 会把需要测试的 crate 当作一个外部对象，所有待测试的内容都需要被设定为 `pub` 。不利于软件设计上的分层，整个项目结构会迅速的被破坏，需要引入编码规范并更加依赖开发者的主动维护。

### 编写

可以简单地将单元测试分为两类，一类是不需要外部文件介入的纯 Rust 测试，一类是 Golden Files 测试。

**Rust 测试**

与平时编写 Rust 单元测试相同，只是待测试的内容需要设为 `pub` ，且引用待测试 crate 需要使用该 crate 的名字。

Databend 提供一些用于模拟全局状态的函数，如 `create_query_context` 等，可能会有助于编写测试。

```rust
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

上面示例来自 `credits_table` 的测试，先构建 `read_plan` 读取新建的 `CreditsTable` 表，再对列数进行断言。

**Golden Files 测试**

> Golden File Testing are like unit tests, except the expected output is stored in a separate file. -- Max Grigorev at [ZuriHac](https://wiki.haskell.org/ZuriHac2010)

Golden Files 测试是一种常用的测试手段，相当于是一类快照测试，如果执行情况和预期结果存在差异则认为测试失败。

Databend 使用 `goldenfile` 这个 crate 来编写 Golden Files 测试。目前 Databend 有计划用此替代 `assert_blocks` 系列断言。

```rust
#[test]
fn test_expr_error() {
    let mut mint = Mint::new("tests/it/testdata");
    let mut file = mint.new_goldenfile("expr-error.txt").unwrap();

    let cases = &[
        r#"5 * (a and ) 1"#,
        r#"a + +"#,
        r#"CAST(col1 AS foo)"#,
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

编写 Golden Files 测试时需要指定挂载的目录和对应预期结果的文件。

在执行测试的主体部分（如上面示例中的 `run_parser!` 宏），除了封装运行测试的必要逻辑外，还需要定义输出时的格式。

测试文件必须按指定格式编写。或者，使用 `REGENERATE_GOLDENFILES=1` 生成。

下面 Golden File 的例子节选自 `common/ast` 模块测试的 `testdata/expr-error.txt`，`Output` 对应解析 `5 * (a and ) 1` 的预期结果。

```plain text
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

### 排查

**Rust 测试**

同其他项目中的 Rust 测试一样，可以根据友好的错误提示轻松定位出现故障的测试。如果需要详细的 Backtrace ，可以在运行测试命令时添加环境变量 `RUST_BACKTRACE=1` 。

```bash
failures:

---- buffer::buffer_read_number_ext::test_read_number_ext stdout ----
Error: Code: 1046, displayText = Cannot parse value:[] to number type, cause: lexical parse error: 'the string to parse was empty' at index 0.

<Backtrace disabled by default. Please use RUST_BACKTRACE=1 to enable> 
thread 'buffer::buffer_read_number_ext::test_read_number_ext' panicked at 'assertion failed: `(left == right)`
  left: `1`,
 right: `0`: the test returned a termination value with a non-zero status code (1) which indicates a failure', /rustc/cd282d7f75da9080fda0f1740a729516e7fbec68/library/test/src/lib.rs:185:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

**Golden Files 测试**

Golden Files 测试的执行命令与 Rust 测试相同，但在错误提示方面有所差异。得益于 goldenfiles 引入了 `similar-assert` ，可以轻松识别 diff ：

```plain text
Differences (-left|+right):
 ---------- Output ---------
 'I'm who I'm.'
 ---------- AST ------------
 Literal {
     span: [
-        QuotedString(0..18),
+        QuotedString(0..16),
     ],
     lit: String(
         "I'm who I'm.",
     ),
 }
.cargo/git/checkouts/rust-goldenfile-6352648ef139d984/16c5783/src/differs.rs:15:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

上面示例中，`+` 对应测试实际结果，`-` 对应测试预期结果，其他为相关的上下文。

goldenfiles 的报错可能会涉及多个测试文件，受限于长文本支持和空格显示，排查仍可能存在不便。

这里提供一个相对友好的排查思路：

1. 确保之前的更改都已经提交，然后运行 `REGENERATE_GOLDENFILES=1 cargo test -p <package> --test it` 重新生成对应的测试。
2. 执行 `git diff` 来显示前后 goldenfiles 文件的差异。
3. 仔细辨别问题出现原因，确定是否存在预期外的问题。

## 功能测试

功能测试主要由 SQL 逻辑测试（sqllogictest）和 stateful 测试两个部分组成。

从本质上讲，这两类功能测试流程相同：

- 启动 databend 实例。
- 使用对应的客户端/驱动执行查询。
- 对比查询情况和预期行为之间的差异，判断测试是否通过。

在设计上，SQL 逻辑测试可以提供更全面的能力：

- 拓展比较结果文件的方式到其他协议（涵盖 http handler）。
- 提示每个语句的结果。
- 提供错误处理的能力。
- 支持排序、重试等测试逻辑。

### 编写

**SQL 逻辑测试**

SQL 逻辑测试放在 `tests/logictest` 目录下。

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

上面的例子展示了如何对 mysql 和 http 分别设计对应的输出结果。其中 `B` 表示结果为布尔类型，`label` 用来标记协议。

SQL 逻辑测试同样支持测试集生成 `python3 gen_suites.py` 。

**stateful 测试**

stateful 测试放在 `tests/suites` 目录下：

> 这里展示的是一类 stateless 写法，stateful 与之类似，区别在于 stateful 会加载数据集并执行查询。

- 输入是一系列 sql 语句，对应目录中的 `*.sql` 文件。

    ```sql
    SELECT '==Array(Int32)==';

    CREATE TABLE IF NOT EXISTS t2(id Int null, arr Array(Int32) null) Engine = Fuse;

    INSERT INTO t2 VALUES(1, [1,2,3]);
    INSERT INTO t2 VALUES(2, [1,2,4]);
    INSERT INTO t2 VALUES(3, [3,4,5]);
    SELECT max(arr), min(arr) FROM t2;
    SELECT arg_max(id, arr), arg_min(id, arr) FROM (SELECT id, arr FROM t2);
    ```

- 输出对应查询结果（含报错），如果没有输出则需要置空，对应目录中的 `*.result` 文件。

    ```plain text
    ==Array(Int32)==
    [3, 4, 5]	[1, 2, 3]
    3	1
    ```

测试可以覆盖 SQL 执行过程中遇到预期错误的情况，有两种方式：

- 沿用上面的方法，在 `result` 文件中标注具体报错信息。
- 也可以采用 `ErrorCode` 注释的方式，此时无需在 `result` 文件中添加对应内容。

    ```sql
    SELECT INET_ATON('hello');-- {ErrorCode 1060}
    ```

### 运行

> 由于 stateful 测试和 sqllogictest 测试均由 Python 编写，在运行前请确保你已经安装全部的依赖。

这几类测试都有对应的 `make` 命令，并支持集群模式测试：

- `sqllogictest` 测试：`make sqllogic-test` & `make sqllogic-cluster-test` 。
- `stateful` 测试：`make stateful-test` & `make stateful-cluster-test` 。（一般在 CI 中运行，本地需要正确配置 MINIO 环境）。

### 排查

**sqllogictest 测试**

sqllogictest 测试能提供精准到语句的报错，并提供更多有效的上下文帮助排查问题。

```
AssertionError: Expected:
INFORMATION_SCHEMA
default
 Actual:
  INFORMATION_SCHEMA
          db_12_0003
             default
 Statement:
Parsed Statement
    at_line: 77,
    s_type: Statement: query, type: T, query_type: T, retry: False,
    suite_name: gen/02_function/02_0005_function_compare,
    text:
        select * from system.databases where name not like '%sys%' order by name;
    results: [(<re.Match object; span=(0, 4), match='----'>, 83, 'INFORMATION_SCHEMA\ndefault')],
    runs_on: {'mysql', 'clickhouse', 'http'},
 Start Line: 83, Result Label: 
make: *** [Makefile:82: sqllogic-test] Error 1
```

**stateful 测试**

目前 stateful 测试能够提供文件级的报错和 diff ，但无法确定报错是由哪一条语句产生。

> 这里展示的是过去 stateless 引发的报错，stateful 与之类似。

```
02_0057_function_nullif:                                                [ FAIL ] - result differs with:
--- /projects/datafuselabs/databend/tests/suites/0_stateless/02_function/02_0057_function_nullif.result
+++ /projects/datafuselabs/databend/tests/suites/0_stateless/02_function/02_0057_function_nullif.stdout
@@ -3,7 +3,7 @@
 1
 1
 NULL
-a
+b
 b
 a
 NULL

Having 1 errors! 207 tests passed.                     0 tests skipped.
The failure tests:
    /projects/datafuselabs/databend/tests/suites/0_stateless/02_function/02_0057_function_nullif.sql
```

**提示**

- 移除 `databend-query-standalone-embedded-meta.sh` 等脚本中的 `nohup` 有助于在测试时同时输出日志到终端，可能同样有助于排查。
- stateful 超时类错误（Timeout!）的默认时间限制为 10 分钟。为方便排查，可以将 `databend-test` 文件中的 `timeout` 改短。

## SQLacner for Databend 测试

### 一、运行

1、启动databend-query并创建用户

```bash
./databend/bin/databend-query &
mysql -uroot -h127.0.0.1 -P3307 -e "CREATE USER 'sqlancer' IDENTIFIED BY 'sqlancer'; GRANT ALL ON *.* TO sqlancer;"
```

2、打包sqlancer

```bash
git clone https://github.com/sqlancer/sqlancer.git
cd sqlancer
mvn package -DskipTests
```

3、运行sqlancer

```bash
#方式一，运行test单元
DATABEND_AVAILABLE=true mvn -Dtest=TestDatabend test
#方式二，运行jar包，可指定参数
cd target
java -jar sqlancer-*.jar --num-threads 4 --random-string-generation ALPHANUMERIC databend --oracle WHERE
#运行成功每5s输出一条信息
[2022/10/01 19:33:12] Executed 1037 queries (207 queries/s; 0.80/s dbs, successful statements: 100%). Threads shut down: 0.
[2022/10/01 19:33:17] Executed 2133 queries (219 queries/s; 0.00/s dbs, successful statements: 100%). Threads shut down: 0.
[2022/10/01 19:33:22] Executed 3248 queries (223 queries/s; 0.00/s dbs, successful statements: 100%). Threads shut down: 0.
[2022/10/01 19:33:27] Executed 4351 queries (220 queries/s; 0.00/s dbs, successful statements: 100%). Threads shut down: 0.
```

4、列举以下常用options（主可选项定义在MainOptions类，另外一个定义在DatabendOptions类）

-h：查看帮助

--num-threads：线程数

--timeout-seconds：运行时间，单位秒，默认-1无限执行

--num-tries：指定多少次异常后停止测试，默认100

--database-prefix：数据库的前缀名

--random-string-generation：随机字符串模式

--host：主机，默认localhost

--port：端口，默认3307

--username：用户名，默认sqlancer

--password：密码，默认sqlancer

databend：指定的DBMS

--oracle：测试的方式

Non-optimizing Reference Engine Construction (NoREC)

- NoREC

Ternary Logic Partitioning (TLP)

- QUERY_PARTITIONING
- WHERE
- GROUP_BY
- HAVING
- DISTINCT
- AGGREGATE

### 二、排查

当sqlancer停止测试后，会先清除原来的日志文件（注意备份），然后将所有错误写入到新的日志文件中 `./sqlancer/logs/databend/*.log` 其中 `*-cur.log` 为某个db执行的sql记录。

#### 1、TLP检测出logic bug

若TLP检测出logic bug，日志文件会包含以下信息（错误的 query sql与重现db的sql）：其中cardinality不一致说明发生了logic bug。

```
--java.lang.AssertionError: the size of the result sets mismatch (3 and 6)!
-- Time: 2022/08/29 16:57:50
-- Database: databend1
-- Database version: 8.0.26-v0.8.12-nightly-74d0287-simd(rust-1.64.0-nightly-2022-08-27T03:09:50.519081067Z)
-- seed value: 1
DROP DATABASE IF EXISTS databend1;
CREATE DATABASE databend1;
USE databend1;
CREATE TABLE t0(c0BOOLEAN BOOLEAN NULL DEFAULT(true));
CREATE TABLE t1(c0FLOAT DOUBLE NULL DEFAULT(0.8522535562515259), c1VARCHAR VARCHAR NULL);
INSERT INTO t1(c0float) VALUES (0.48751503229141235);
INSERT INTO t1(c0float, c1varchar) VALUES (0.8522535562515259, NULL);
INSERT INTO t1(c1varchar) VALUES ('2'), ('78');
INSERT INTO t0(c0boolean) VALUES (false), (false), (true);
INSERT INTO t1(c0float) VALUES (0.8522535562515259), (0.48751503229141235);
INSERT INTO t1(c0float, c1varchar) VALUES (0.48751503229141235, '7555834'), (0.7298239469528198, '8');
-- SELECT t0.c0boolean FROM t0;
-- cardinality: 3
-- SELECT t0.c0boolean FROM t0 WHERE (NULL BETWEEN NULL AND NULL) UNION ALL SELECT t0.c0boolean FROM t0 WHERE (NOT (NULL BETWEEN NULL AND NULL)) UNION ALL SELECT t0.c0boolean FROM t0 WHERE (((NULL BETWEEN NULL AND NULL)) IS NULL);
-- cardinality: 6
```

TLP发现的bug例如：[sqlancer: expression expansion error · Issue #7360 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7360)

#### 2、NoREC检测出logic bug

若NoREC检测出logic bug，日志文件会包含以下信息（错误的query sql与重现db的sql）：

第一条 query sql 返回的是结果的row数，第二条 query sql 返回的是count的和，若两数不一致则出现logic bug。

```
java.lang.AssertionError: SELECT t1.c0float, t1.c1varchar, t0.c0boolean FROM t1 RIGHT  JOIN t0 ON true WHERE (NOT -1257754687); -- 3
SELECT SUM(count) FROM (SELECT (((NOT -1257754687) IS NOT NULL AND (NOT -1257754687)) ::BIGINT)as count FROM t1 RIGHT JOIN t0 ON true) as res -- 0
-- Time: 2022/09/24 09:27:50
-- Database: databend1
-- Database version: 8.0.26-v0.8.46-nightly-f524701-simd(rust-1.66.0-nightly-2022-09-23T16:20:13.611527635Z)
-- seed value: 1
DROP DATABASE IF EXISTS databend1;
CREATE DATABASE databend1;
USE databend1;
CREATE TABLE t0(c0BOOLEAN BOOLEAN NULL DEFAULT(true));
CREATE TABLE t1(c0FLOAT DOUBLE NULL DEFAULT(0.8522535562515259), c1VARCHAR VARCHAR NULL);
INSERT INTO t1(c0float) VALUES (0.48751503229141235);
```

NoREC发现的bug例如：[bug: select error · Issue #7863 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7863)

#### 3、NoREC与TLP检测出其他Bug

NoREC与TLP测试期间还会检测出使得databend错乱的bug：

如果报错信息有 `Cause by` 且报错信息很复杂则极有可能是bug，也可能是sqlancer生成的sql语法错误。

情况比较多，例如：

- [Can't construct type from Float32(Float32) and Int64(Int64) · Discussion #7162 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/discussions/7162)
- 这种bug较难发现，因为它极有可能是语法错误：[bug: return error after adding form and join · Issue #8000 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/8000)
- 根据执行的速度，判断可能存在的问题：[long SQL makes parser work really slow. · Issue #7225 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7225)
- [bug: const types and nullable types are orthogonal · Issue #7241 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7241)
- [bug: ERROR 2013 (HY000): Lost connection to server during query · Issue #7949 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7949)
- [Bug in numerical_coercion of in operator · Issue #7203 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7203)

#### 4、据不完全统计目前测出的Bug

[Can't construct type from Float32(Float32) and Int64(Int64) · Discussion #7162 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/discussions/7162)

[long SQL makes parser work really slow. · Issue #7225 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7225)

[bug: const types and nullable types are orthogonal · Issue #7241 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7241)

[Bug in numerical_coercion of in operator · Issue #7203 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7203)

[sqlancer: expression expansion error · Issue #7360 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7360)

[bug: Code: 4000, displayText = block pruning failure, task 1050895 panicked. · Issue #7366 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7366)

[bug: where clause error · Issue #7457 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7457)

[bug: expression evaluation error · Issue #7460 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7460)

[bug: Code: 4000, displayText = unexpected end of file (failed to fill whole buffer) (while in processor thread 15). · Issue #7461 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7461)

[bug: expression evaluation error · Issue #7460 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7460)

[bug: the content of the result sets mismatch · Issue #7463 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7463)

[bug: Code: 4000, displayText = block pruning failure, task 4274595 panicked. · Issue #7464 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7464)

[bug: Code: 1058, displayText = downcast column error · Issue #7483 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7483)

[bug: Hash table capacity overflow · Issue #7495 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7495)

[bug: cannot convert NULL to a non-nullable type · Issue #7498 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7498)

[bug: expression explain error · Issue #7543 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7543)

[bug: select view error · Issue #7573 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7573)

[bug: select error · Issue #7863 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7863)

[bug: where clause explain error · Issue #7864 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7864)

[bug: ERROR 2013 (HY000): Lost connection to server during query · Issue #7949 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/7949)

[bug: return error after adding form and join · Issue #8000 · datafuselabs/databend (github.com)](https://github.com/datafuselabs/databend/issues/8000)