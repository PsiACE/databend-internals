+++
title = "Databend 源码阅读： Storage 概要和 Read Partitions"
description = "“Databend 源码阅读”系列文章的第六篇，介绍了 Databend 存储的基本情况，以及读取分区的相关源码分析。"
draft = false
weight = 460
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
giscus = true

+++

作者：[zhyass](https://github.com/zhyass) | Databend Labs 成员，数据库研发工程师

## 引言

Databend 将存储引擎抽象成一个名为 `Table` 的接口，源码位于 `query/catalog/src/table.rs`。

`Table` 接口定义了 `read`、`append`、`alter`、`optimize`、`truncate` 以及 `recluster` 等方法，负责数据的读写和变更。解释器（interpreter）通过调用 `Table trait` 的方法生成物理执行的 `pipeline`。

通过实现 `Table` 接口的方法，可以定义 Databend 的存储引擎，不同的实现对应不同的引擎。

Storage 主要关注 `Table` 接口的具体实现，涉及表的元信息，索引信息的管理，以及与底层 IO 的交互。

## 目录

| 包名                       | 作用                                                         |
| -------------------------- | ------------------------------------------------------------ |
| common/cache               | 定义与管理缓存，包括磁盘缓存和内存缓存。类型包含表 meta 缓存、查询结果缓存、表数据缓存等。 |
| common/index               | 定义与使用索引，目前支持 bloom filter index、page index、range index。 |
| common/locks               | 管理与使用锁，支持表级别的锁。                              |
| common/pruner              | 分区剪裁算法，包括 internal column pruner、limiter pruner、page pruner、topn pruner、range pruner。 |
| common/table_meta          | 表 meta 的数据结构定义。                                     |
| hive                       | hive 表的交互                                                |
| iceberg                    | iceberg 交互                                                 |
| information_schema、system | 系统表定义                                                   |
| memory、null、random       | 用于开发和测试的引擎                                         |
| view                       | 视图相关                                                     |
| stage                      | stage 数据源的读取                                           |
| parquet                    | 把 parquet 文件作为数据源                                    |
| fuse                       | fuse 引擎模块                                                |
| fuse/src/io                | table meta、index、block 的读写 IO 交互                      |
| fuse/src/pruning           | fuse 分区裁剪                                                |
| fuse/src/statistics        | column statistics 和 cluster statistics 等统计信息           |
| fuse/src/table_functions   | table function 实现                                          |
| fuse/src/operation         | fuse 引擎对 table trait 方法的具体实现。并包含了如 ReadSource、CommitSink 等 processor 算子的定义 |

## Read Partitions

以下以 fuse 引擎中 read partitions 的实现流程为例，简要分析 Storage 相关源码。

Partitions 的定义位于 `query/catalog/src/plan/partition.rs`。

```Rust
pub struct Partitions {
    // partitions 的分发类型。
    pub kind: PartitionsShuffleKind,
    // 一组实现了 PartInfo 接口的 partition，
    pub partitions: Vec<PartInfoPtr>,
    // partitions 是否为 lazy。
    pub is_lazy: bool,
}
```

Table 接口中的 `read_partitions` 通过分析查询中的过滤条件，剪裁掉不需要的分区，返回可能满足条件的 Partitions。

```Rust
#[async_trait::async_trait]
impl Table for FuseTable {
    #[minitrace::trace]
    #[async_backtrace::framed]
    async fn read_partitions(
        &self,
        ctx: Arc<dyn TableContext>,
        push_downs: Option<PushDownInfo>,
        dry_run: bool,
    ) -> Result<(PartStatistics, Partitions)> {
        self.do_read_partitions(ctx, push_downs, dry_run).await
    }
}
```

Fuse 引擎会以 segment 为单位构建 lazy 类型的 `FuseLazyPartInfo`。通过这种方式，`prune_snapshot_blocks` 可以下推到 pipeline 初始化阶段执行，特别是在分布式集群模式下，可以有效提高剪裁执行效率。

```Rust
pub struct FuseLazyPartInfo {
    // segment 在 snapshot 中的索引位置。
    pub segment_index: usize,
    pub segment_location: Location,
}
```

分区剪裁流程的实现位于 `query/storages/fuse/src/pruning/fuse_pruner.rs` 文件中，具体流程如下：

1. 基于 `push_downs` 条件构造各类剪裁器（pruner），并实例化 `FusePruner`。
2. 调用 `FusePruner` 中的 `pruning` 方法，创建 `max_concurrency` 个分批剪裁任务。每个批次包括多个 segment 位置，首先根据 `internal_column_pruner` 筛选出无需的 segments，再读取 `SegmentInfo`，并根据 segment 级别的 `MinMax` 索引进行范围剪裁。
3. 读取过滤后的 `SegmentInfo` 中的 `BlockMetas`，并按照 `internal_column_pruner`、`limit_pruner`、`range_pruner`、`bloom_pruner`、`page_pruner` 等算法的顺序，剔除无需的 blocks。
4. 执行 `TopNPrunner` 进行过滤，从而得到最终剪裁后的 `block_metas`。

```Rust
pub struct FusePruner {
    max_concurrency: usize,
    pub table_schema: TableSchemaRef,
    pub pruning_ctx: Arc<PruningContext>,
    pub push_down: Option<PushDownInfo>,
    pub inverse_range_index: Option<RangeIndex>,
    pub deleted_segments: Vec<DeletedSegmentInfo>,
}

pub struct PruningContext {
    pub limit_pruner: Arc<dyn Limiter + Send + Sync>,
    pub range_pruner: Arc<dyn RangePruner + Send + Sync>,
    pub bloom_pruner: Option<Arc<dyn BloomPruner + Send + Sync>>,
    pub page_pruner: Arc<dyn PagePruner + Send + Sync>,
    pub internal_column_pruner: Option<Arc<InternalColumnPruner>>,
    // Other Fields ...
}

impl FusePruner {
    pub async fn pruning(
        &mut self,
        mut segment_locs: Vec<SegmentLocation>,
        delete_pruning: bool,
    ) -> Result<Vec<(BlockMetaIndex, Arc<BlockMeta>)>> {
        ...
    }
}
```

剪裁结束后，以 Block 为单位构造 `FusePartInfo`，生成 `partitions`，接着调用 `set_partitions` 方法将 `partitions` 注入 `QueryContext` 的分区队列中。在执行任务时，可以通过 `get_partition` 方法从队列中取出。

```Rust
pub struct FusePartInfo {
    pub location: String, 
    pub create_on: Option<DateTime<Utc>>,
    pub nums_rows: usize,
    pub columns_meta: HashMap<ColumnId, ColumnMeta>,
    pub compression: Compression,
    pub sort_min_max: Option<(Scalar, Scalar)>,
    pub block_meta_index: Option<BlockMetaIndex>,
}
```

## Conclusion

Databend 的存储引擎设计采用了抽象接口的方式，具有高度的可扩展性，可以很方便地支持多种不同的存储引擎。Storage 模块的主要职责是实现 Table 接口的方法，其中 Fuse 引擎部分尤为关键。

通过对数据的并行处理，以及数据剪裁等手段，可以有效地提高数据的处理效率。鉴于篇幅限制，本文仅对读取分区的流程进行了简单阐述，更深入的解析将在后续的文章中逐步展开。
