+++
title = "Databend 源码阅读： 开篇"
description = "“Databend 源码阅读”系列文章的第一篇，帮助大家更快熟悉 Databend 代码。希望通过源码阅读，来加强和社区的技术交流，引发更多思维碰撞。"
draft = false
weight = 410
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
giscus = true
+++

作者：[sundy-li](https://github.com/sundy-li) | Datafuse Labs 成员，折腾过 Clickhouse

## 前言

Databend 在 2021 年开源后，陆续受到了很多社区同学的关注。Databend 使用了 Rust 编程语言。为了吸引更多的开发者，特别是没有 Rust 开发经验的新同志，我们设计了 Rust 相关课程，同时建立了多个 Rust 兴趣小组。

Databend 在 issue 中还引入了“Good First issue”的 label 来引导社区新同学参与第一次贡献，目共有超过一百多位 contributors，算是一个不错的成果。

但 Databend 也在过去的一年中经历了数次迭代，代码日渐复杂。目前代码主干分支有 26 w 行 rust 代码，46 个 crate，对于新接触 Databend 的技术爱好者来说，贡献门槛越来越高。即使是熟悉 rust 的同学，clone 代码后，面对着茫茫码海，竟不知如何读起。在多个社区群中，也有朋友数次提到什么时候能有一个 Databend 源码阅读系列文章，帮助大家更快熟悉 Databend 代码。

因此，我们接下来会开展“Databend 源码阅读”系列文章，主要受众是社区技术开发者，希望通过源码阅读，来加强和社区的技术交流，引发更多思维碰撞。

## Databend 的故事

很多同学都问过我们一个问题：为什么你们要用 Rust 从零构建一个数据库？其实这个问题可以分为两个子问题：

### 为什么选择的是 Rust？

我们早期的成员大多是 ClickHouse、tidb 、tokudb 等知名数据库的贡献者，从技术栈来说更熟悉的是 C++ 和 Go。虎哥（[@bohutang](https://github.com/bohutang)）在疫情期间也使用 Go 实现了一个小的数据库原型 [vectorsql](https://github.com/vectorengine/vectorsql)，有同学表示 vectorsql 的架构非常优雅，值得学习借鉴。

![vectorsql](https://psiace.github.io/databend-internals/source-reading/intro/01-vectorsql.png)

语言本没有孰劣之分，要从面向的场景来聊聊。目前大多的 DMBS 使用的是 C++/Java，新型的 NewSQL 更多使用的是 Go。在以往的开发经验来看，C/C++ 已经是高性能的代名词，开发者更容易写出高运行效率的代码，但 C++ 的开发效率实在不忍直视，工具链不是很完善，开发者很难一次性写出内存安全，并发安全的代码。而 Go 可能是另外一个极端，大道至简，工具链完善，开发效率非常高，不足之处在于泛型的进度太慢了，在 DB 系统上内存不能很灵活的控制，且难于达到前者的运行性能，尤其使用 SIMD 指令还需要和汇编代码交互等。我们需要的是兼具 开发效率（内存安全，并发安全，工具链完善）& 运行效率 的语言，当时看来，Rust 可能是我们唯一的选择了，历经尝试后，我们也发现，Rust 不仅能满足我们的需求，而且很酷！

### 为什么要从零构建一个数据库系统？

总体来说，路线无非就以下两条：

- 基于知名的开源数据库做二次开发优化

    这条路线可能更多人会选择，因为有一个好的数据库底座，无需再做一些重复性的工作，在上面做二次开发的话能省不少力气，团队专注做优化改进重构，能更早推动版本，落地商业化。缺点是 fork 后的版本难于再次回馈到社区，相当于另外一套独立的系统，如 PG 下的各个子流派。

- 从零构建一套新的数据库系统

    这条路线走起来比较艰难，因为数据库系统实在太庞大了，一个子方向都足够专业人士深入研究十几年。这个方向虽然没能直接站在已有的底座上，但会让设计者更加灵活可控，无需关注太多历史的包袱。Databend 在设计之初面向的是云原生数仓的场景，和传统的数据库系统有很大的区别，如果基于传统数据库系统来做，改造代码的成本和从零做的成本可能差不多，因此我们选择的是这条路来从零打造一个全新的云数仓。

## Databend 的架构

画虎画皮难画骨，我们先从 Databend 的“骨”聊起。

![databend arch](https://psiace.github.io/databend-internals/source-reading/intro/01-databend-arch.png)

虽然我们是使用 Rust 从零开始实现的，但不是完全闭门造轮子，一些优秀的开源组件或者生态也有在其中集成。如：我们兼容了 Ansi-SQL 标准，提供了 MySQL/ClickHouse 等主流协议的支持，拥抱了万物互联的 Arrow 生态，存储格式基于大数据主流的 Parquet 格式等。我们不仅会积极地回馈了贡献给上游，如 Arrow2/Tokio 等开源库，一些通用的组件我们也抽成独立的项目开源在Github（openraft, opendal, opencache, opensrv 等）。

Databend 定义为云原生的弹性数据库，在设计之初我们不仅要做到计算存储分离，每一层的极致的弹性都是设计主要考量点。Databend 主要分为三层：MetaService Layer，Query Layer，Storage Layer，这三层都是可以弹性扩展的，意味着用户可以为自己的业务选择最适合的集群规模，并且随着业务发展来伸缩集群。

下面我们将从这三层来介绍下 Databend 的主要代码模块。

## Databend 的模块

### MetaService Layer

MetaService 主要用于存储读取持久化的元数据信息，比如 Catalogs/Users 等。

|包名|作用|
|----|----|
|metasrv|MetaService 服务，作为独立进程部署，可部署多个组成集群，底层使用 Raft 做分布式共识，Query 以 Grpc 和 MetaService 交互。|
|common/meta/types|定义了各类需要保存在 MetaService 的结构体，由于这些结构体最终需要持久化，所以涉及到数据序列化的问题，当前使用 Protobuf 格式来进行序列化和反序列化操作，这些类型相关的 Rust 结构体与 Protobuf 的相互序列化规则代码定义在 common/proto-conv 子目录中。|
|common/meta/sled-store|当前 MetaService 使用 sled 来保存持久化数据，这个子目录封装了 sled 相关的操作接口。|
|common/meta/raft-store|openraft 用户层需要实现 raft store 的存储接口用于保存数据，这个子目录就是 MetaService 实现的 openraft 的存储层，底层依赖于 sled 存储，同时这里还实现了 openraft 用户层需要自定义的状态机。|
|common/meta/api|对 query 暴露的基于 KVApi 实现的用户层 api 接口。|
|common/meta/grpc|基于 grpc 封装的 client，MetaService 的客户端使用这里封装好的 client 与 MetaService 进行通信交互。|
|raft|<https://github.com/datafuselabs/openraft>，从 async-raft 项目中衍生改进的全异步 Raft 库。|

### Query Layer

Query 节点主要用于计算，多个 query 节点可以组成 MPP 集群，理论上性能会随着 query 节点数水平扩展。SQL 在 query 中会经历以下几个转换过程：

![query](https://psiace.github.io/databend-internals/source-reading/intro/03-query.png)

从 SQL 字符串经过 Parser 解析成 AST 语法树，然后经过 Binder 绑定 catalog 等信息转成逻辑计划，再经过一系列优化器处理转成物理计划，最后遍历物理计划构建对应的执行逻辑。
query 涉及的模块有：

|包名|作用|
|----|----|
|query|Query 服务，整个函数的入口在 bin/databend-query.rs 其中包含一些子模块，这里介绍下比较重要的子模块
|     | api ：对外暴露给外部的 HTTP/RPC 接口 |
|     | catalogs：catalogs 管理，目前支持默认的 catalog（存储在 metaservice）以及 hive catalog （存储在 hive meta store) |
|     | Clusters：query 集群信息 |
|     | Config：query 的配置相关 |
|     | databases：query 支持的 database engine 相关 |
|     | evaluator：表达式计算工具类 |
|     | Interpreters：SQL 执行器，SQL 构建出 Plan 后，通过对应执行器去做物理执行 |
|     | pipelines：实现了物理算子的调度框架 |
|     | Servers：对外暴露的服务，有 clickhouse/mysql/http 等 |
|     | Sessions：session 管理相关 |
|     | Sql：包含新的 planner 设计，新的 binder 逻辑，新的 optimizers 设计 |
|     | Storages：表引擎相关，最常用为 fuse engine |
|     | table_functions：表函数相关，如 numbers |
|common/ast|基于 nom_rule 实现的新版 sql parser|
|common/datavalues|各类 Column 的定义，表示数据在内存上的布局， 后续会逐步迁移到 common/expressions|
|common/datablocks|Datablock 表示 Vec<Column> 集合，里面封装了一些常用方法,  后续会逐步迁移到 common/expressions|
|common/functions|标量函数以及聚合函数等实现注册|
|common/hashtable|实现了一个线性探测的 hashtable，主要用于 group by 聚合函数以及 join 等场景|
|common/formats|负责数据对外各类格式的 序列化反序列化，如 CSV/TSV/Json 格式等|
|opensrv|<https://github.com/datafuselabs/opensrv>|

### Storage Layer

Storage 主要涉及表的 Snapshots，Segments 以及索引信息等管理，以及和底层 IO 的交互。Storage 目前一大亮点是基于 Snapshot 隔离 实现了类似 Iceberge 方式的 Increment view,  我们可以对表在任意历史状态下进行 time travel 访问。

## 后续规划

源码阅读系列刚刚开始撰写，后续预计将按照介绍各个模块的方式进行逐步讲解，输出主要以文章为主，一些比较重要且有趣的模块设计可能会以视频直播的方式和大家一起交流。

目前只是一个初步的规划，在这个过程中会接受大家的建议做一些时间内容调整。无论如何，我们都期待通过这个系列的活动，让更多志同道合的人参与到 Databend 的开发中来，一起学习交流成长。
