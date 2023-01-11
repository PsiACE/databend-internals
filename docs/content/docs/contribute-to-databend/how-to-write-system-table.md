+++
title = "如何为 Databend 添加新的系统表"
description = "系统表是 Databend 存放结构元数据的地方，比如表和字段以及内部记录的一些信息。这篇文件将会简要介绍 Databend 的系统表的实现和测试。"
draft = false
weight = 650
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
giscus = true
+++

Databend 的绝大部分系统表都位于 [query/storage](https://github.com/datafuselabs/databend/tree/main/src/query/storages/system) 这个目录下，当然，如果因为一些特殊的构建原因无法放在这个位置的话，也可以考虑临时放到 `service/databases/system` 这个目录（不推荐）。

系统表的定义主要关注两个内容：一个是表的信息，会包含表名、Schema 这些；另一个就是表中数据的生成/获取。刚好可以对应到 `SyncSystemTable` 和 `AsyncSystemTable` 这两个 Trait 中的 `get_table_info` 和 `get_full_data` 。到底是同步还是异步，取决于在获取数据时，是否涉及到异步函数的调用。

## 实现

 本文将会以 `credits` 表的实现为例，介绍 Databend 系统表的实现，代码位于 https://github.com/datafuselabs/databend/blob/main/src/query/storages/system/src/credits_table.rs 。`credits` 会返回 Databend 所用到的上游依赖的信息，包括名字、版本和许可三个字段。

首先，需要参考其他系统表的实现，去定义表对应的结构，只需要保有表信息的字段就可以了。

```Rust
pub struct CreditsTable {
    table_info: TableInfo,
}
```

接下来是为 `CreditsTable` 表实现 `create` 方法，对应的函数签名如下：

```rust
pub fn create(table_id: u64) -> Arc<dyn Table>
```

传入的 `table_id` 会在创建表时由 `sys_db_meta.next_table_id()` 生成。

`schema` 用于描述表的结构，需要使用 `TableSchemaRefExt` 和 `TableField` 来创建，字段名字和类型取决于表中的数据。

```Rust
let schema = TableSchemaRefExt::create(vec![
    TableField::new("name", TableDataType::String),
    TableField::new("version", TableDataType::String),
    TableField::new("license", TableDataType::String),
]);
```

对于字符串类数据，可以使用 `TableDataType::String` ，其他基础类型也类似。但如果你需要允许字段中存在空值，比如字段是可以为空的 64 位无符号整数，则可以使用 `TableDataType::Nullable(Box::new(TableDataType::Number(NumberDataType::UInt64)))` 的方式，`TableDataType::Nullable` 表示允许空值，`TableDataType::Number(NumberDataType::UInt64)` 表征类型是 64 位无符号整数。

接下来就是定义表的信息，基本上只需要依葫芦画瓢，把描述、表名、元数据填上就好。

```Rust
let table_info = TableInfo {
    desc: "'system'.'credits'".to_string(),
    name: "credits".to_string(),
    ident: TableIdent::new(table_id, 0),
    meta: TableMeta {
        schema,
        engine: "SystemCredits".to_string(),
        ..Default::default()
    },
   ..Default::default()
};

SyncOneBlockSystemTable::create(CreditsTable { table_info })
```

对于同步类型的表往往使用 `SyncOneBlockSystemTable` 创建，异步类型的则使用 `AsyncOneBlockSystemTable`  。

接下来，则是实现 `SyncSystemTable` ，`SyncSystemTable` 除了需要定义 `NAME` 之外，还需要实现 4 个函数 `get_table_info` 、`get_full_data`、`get_partitions` 和 `truncate `，由于后两个有默认实现，大多数时候不需要考虑实现自己的。（`AsyncSystemTable` 类似，只是没有 `truncate` ）

`NAME` 的值遵循 `system.<name>` 的格式。

```Rust
const NAME: &'static str = "system.credits";
```

`get_table_info` 只需要返回结构体中的表信息。

```Rust
fn get_table_info(&self) -> &TableInfo {
    &self.table_info
}
```

`get_full_data` 是相对重要的部分，因为每个表的逻辑都不太一样，`credits` 的三个字段基本类似，就只举 `license` 字段为例。

```
let licenses: Vec<Vec<u8>> = env!("DATABEND_CREDITS_LICENSES")
    .split_terminator(',')
    .map(|x| x.trim().as_bytes().to_vec())
    .collect();
```

`license` 字段的信息是从名为 `DATABEND_CREDITS_LICENSES` 的环境变量（参见 `common-building`）获取的，每条数据都用 `,` 进行分隔。

字符串类型的列最后是从 `Vec<Vec<u8>>` 转化过来，其中字符串需要转化为 `Vec<u8>` ，所以在迭代的时候使用 `.as_bytes().to_vec()` 做了处理。

在获取所有数据后，就可以按 `DataBlock` 的形式返回表中的数据。非空类型，使用 `from_data` ，可空类型使用 `from_opt_data` 。

```
Ok(DataBlock::new_from_columns(vec![
    StringType::from_data(names),
    StringType::from_data(versions),
    StringType::from_data(licenses),
]))
```

最后，要想将其集成到 Databend 中，还需要编辑 `src/query/service/src/databases/system/system_database.rs`，将其注册到 `SystemDatabase` 中 。

```rust
impl SystemDatabase {
    pub fn create(sys_db_meta: &mut InMemoryMetas, config: &Config) -> Self {
    ...
        CreditsTable::create(sys_db_meta.next_table_id()),
    ...
    }
}
```

## 测试

系统表的相关测试目前仍然位于 `src/query/service/tests/it/storages/system.rs` 。

对于内容不会经常动态变化的表，可以使用 Golden File 测试，其运行逻辑是将对应的表写入指定的文件中，然后对比每次测试时文件内容是否发生变化。

```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_columns_table() -> Result<()> {
    let (_guard, ctx) = crate::tests::create_query_context().await?;

    let mut mint = Mint::new("tests/it/storages/testdata");
    let file = &mut mint.new_goldenfile("columns_table.txt").unwrap();
    let table = ColumnsTable::create(1);

    run_table_tests(file, ctx, table).await?;
    Ok(())
}
```

对于内容可能会变化的表，目前缺乏充分的测试手段。可以选择测试其中模式相对固定的部分，比如行和列的数目；也可以验证输出中是否包含特定的内容。

```rust

#[tokio::test(flavor = "multi_thread")]
async fn test_metrics_table() -> Result<()> {
	...
    let result = stream.try_collect::<Vec<_>>().await?;
    let block = &result[0];
    assert_eq!(block.num_columns(), 4);
    assert!(block.num_rows() >= 1);

    let output = pretty_format_blocks(result.as_slice())?;
    assert!(output.contains("test_test_metrics_table_count"));
    #[cfg(feature = "enable_histogram")]
    assert!(output.contains("test_test_metrics_table_histogram"));

    Ok(())
}
```
