+++
title = "大规模并行处理"
description = "大规模并行处理是大数据计算引擎的一个重要特性，可以提供高吞吐、低时延的计算能力。那么，当我们在讨论大规模并行处理时，究竟在讨论什么？"
draft = false
weight = 270
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "大规模并行处理是大数据计算引擎的一个重要特性，可以提供高吞吐、低时延的计算能力。那么，当我们在讨论大规模并行处理时，究竟在讨论什么？"
toc = true
top = false
giscus = true
+++

##  背景

企业或个人都可能会收集和存储大量的数据，特别是近年来大数据技术的兴起，人们拥有更多接触数据和利用数据的机会和意愿，那么随着数据量的增长，对存储容量和计算能力的要求也进一步提高了。

## 概要

大规模并行处理（MPP，Massively Parallel Processing）意味着可以由多个计算节点（处理器）协同处理程序的不同部分，而每个计算节点都可能具备独立的系统资源（磁盘、内存、操作系统）。

计算节点将工作拆分成易于管理、调度和执行的任务执行，通过添加额外的计算节点可以完成水平拓展。随着计算节点数目的增加，对数据的查询处理速度就越快，从而减少大数据集上处理复杂查询所需的时间。

## 特性

采用大规模并行处理架构设计的系统往往具备以下特性：

- 任务并行执行
- 数据分布式存储
- 分布式计算
- 私有资源
- 水平拓展
- Shared Nothing
