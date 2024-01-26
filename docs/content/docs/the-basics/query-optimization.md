+++
title = "查询优化"
description = "查询优化是数据库系统的一个重要话题。本文介绍了查询优化的相关概念及发展历史，Cascades 优化器以及云数仓所面临的查询优化挑战。"
draft = false
weight = 250
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "查询优化是数据库系统的一个重要话题。本文介绍了查询优化的相关概念及发展历史，Cascades 优化器以及云数仓所面临的查询优化挑战。"
toc = true
top = false
giscus = true
+++

> 本文根据 [@leiysky](https://github.com/leiysky) 的分享整理而来，略去了查询优化的细节，详细的 PPT 可以参考 [SQL Processing & Query Optimization](https://github.com/datafuselabs/datafuse-presentations/tree/master/meetup-20210827-query-optimization) 。

## 查询优化

### 什么是查询优化

查询优化指的是为给定的查询选择最佳执行计划。

那么什么样的执行计划称得上是最佳计划呢？

- 更快的速度（更低的延迟），这是一个非常直观的评价标准。
- 在 OLTP 场景下，则更强调性价比。
- 而对于 OLAP 场景，则追求更高的吞吐量。
- 从商业口号的角度看，可能偏向于一次“正确”的技术选型。

### 为什么可以优化 SQL 查询

目前有两种主要的查询优化方案，一种是基于关系代数和算法的等价优化方案，一种是基于评估成本的优化方案。

根据命名，不难看出优化的灵感来源和这两种方案在优化上的取舍。

### 如何进行查询优化

查询优化通常包含以下四个步骤：

- 构建框架来列举可能的计划
- 编写转换规则
- 引入成本模型来评估不同的计划
- 选择最理想的计划

## 查询优化的历史

### IBM System R's Optimizer

世界上第一个查询优化器是 IBM System R 的优化器。

其建立背景是：

- 磁盘比内存慢得多，查找数据的开销非常大。
- 内存很小。
- 单 CPU 核心（不存在并行）。

### PostgreSQL's Optimizer

PostgreSQL 是世界上最成功的开源 RDBMS 之一，有着悠久的历史（1996 年首次发布）。

- 引入成本模型。
- 基于动态规划的 Join 重排。
- Genetic Query Optimizer：基于遗传算法的查询优化器。

### SQL Server's Optimizer

SQL Server，由微软和 Sybase 在 20 世纪 90 年代开发的商业 RDBMS 。

Goetz Graefe（Volcano/Cascades的作者）为 SQL Server 设计了 Cascades 查询优化框架。

该优化器框架已被广泛用于微软开发的不同查询系统（如 SQL Server、SQL Server PDW、Cosmos SCOPE、Synapse）。

世界上最好的查询优化器（也许）。

## Volcano/Cascades 优化器框架

枚举计划并评估成本的探索框架。

- [The Volcano Optimizer Generator: Extensibility and Efficient Search](https://15721.courses.cs.cmu.edu/spring2017/papers/14-optimizer1/graefe-icde1993.pdf)
- [The Cascades Framework for Query Optimization](https://www.cse.iitb.ac.in/infolab/Data/Courses/CS632/Papers/Cascades-graefe.pdf)

### Cascades 之禅

- 自顶向下探索
- 模式匹配
- 基于规则“Rule-based”
- 记忆化

### 开源的 Cascades 实现

- Apache Calcite: a Volcano/Cascades style optimizer framework, widely used in Apache world(e.g. Drill, Flink)
-  GreenPlum Orca: optimizer component of GreenPlum, also used by HAWQ, Hologres, Alicloud ADB
- CockroachDB's Cascades optimizer

## 云数仓所面临的查询优化挑战

- 对非常大的数据集进行成本估算
- 复杂的成本模型因子
- 不同的优化方法，取决于存储系统的设计
- 可测试性、可追踪性、可调试性
