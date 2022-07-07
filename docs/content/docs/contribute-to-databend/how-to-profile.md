+++
title = "怎样完成 Databend 性能剖析"
description = "Databend 整合了一些性能剖析工具，可以方便进行深入分析。本文将会介绍如何进行 CPU / Memory Profiling 。"
draft = false
weight = 650
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Databend 整合了一些性能剖析工具，可以方便进行深入分析。本文将会介绍如何进行 CPU / Memory Profiling 。"
toc = true
top = false
giscus = true
+++

## CPU Profiling

CPU 分析，按照一定的频率采集所监听的应用程序 CPU（含寄存器）的使用情况，可确定应用程序在主动消耗 CPU 周期时花费时间的位置。

pprof 是 Google 开源的代码性能分析工具，可以直接生成代码分析报告，不仅支持通过命令式交互查看，也很方便进行可视化展示。Databend 使用 [pprof-rs](https://crates.io/crates/pprof) 完成对 pprof 工具的支持。

### 必备工作

CPU 分析属于 Databend 的内置能力，部署一个 Databend 实例即可开始使用。

### 命令行交互

```bash
go tool pprof http://localhost:<your-databend-port>/debug/pprof/profile?seconds=20
```

结果如下所示：

```bash
Fetching profile over HTTP from http://localhost:8080/debug/pprof/profile?seconds=20
Saved profile in pprof/pprof.cpu.007.pb.gz
Type: cpu
Entering interactive mode (type "help" for commands, "o" for options)
(pprof) top
Showing nodes accounting for 5011, 100% of 5011 total
Dropped 248 nodes (cum <= 25)
Showing top 10 nodes out of 204
      flat  flat%   sum%        cum   cum%
      5011   100%   100%       5011   100%  backtrace::backtrace::libunwind::trace
         0     0%   100%        162  3.23%  <&alloc::vec::Vec<T,A> as core::iter::traits::collect::IntoIterator>::into_iter
         0     0%   100%         45   0.9%  <&mut I as core::iter::traits::iterator::Iterator>::next
         0     0%   100%         77  1.54%  <[A] as core::slice::cmp::SlicePartialEq<B>>::equal
         0     0%   100%         35   0.7%  <[u8; 8] as ahash::convert::Convert<u64>>::convert
         0     0%   100%        199  3.97%  <[u8] as ahash::convert::ReadFromSlice>::read_last_u64
         0     0%   100%         73  1.46%  <[u8] as ahash::convert::ReadFromSlice>::read_last_u64::as_array
         0     0%   100%        220  4.39%  <[u8] as ahash::convert::ReadFromSlice>::read_u64
         0     0%   100%        701 13.99%  <ahash::fallback_hash::AHasher as core::hash::Hasher>::write
         0     0%   100%         26  0.52%  <ahash::random_state::RandomState as core::hash::BuildHasher>::build_hash
```

### 可视化

执行下述命令可以进行可视化：

```bash
go tool pprof -http=0.0.0.0:<your-profile-port> $HOME/pprof/pprof.cpu.007.pb.gz
```

### 注意事项

Databend 暂时不支持在 musl 平台上运行 pprof 。

## Memory Profiling

内存分析，在应用程序进行堆分配时记录堆栈追踪，用于监视当前和历史内存使用情况，以及检查内存泄漏。

通过与 `jemalloc` 的集成（可选），Databend 得以整合多种内存分析能力。这里使用 `jeprof` 进行内存分析。

### 必备工作

- 在构建二进制文件时启用 `memory-profiling` 特性：`cargo build --features memory-profiling`
- 在创建 Databend 实例时，设置环境变量 `MALLOC_CONF=prof:true` 以启用内存分析。 

### 堆快照转储

```bash
jeprof ./target/debug/databend-query http://localhost:<your-databend-port>/debug/mem
```

结果如下所示：

```bash
Using local file ./target/debug/databend-query.
Gathering CPU profile from http://localhost:8080/debug/mem/pprof/profile?seconds=30 for 30 seconds to ~/jeprof/databend-query.1650949265.localhost
Be patient...
Wrote profile to /home/zhaobr/jeprof/databend-query.1650949265.localhost
Welcome to jeprof!  For help, type 'help'.
 (jeprof) top
Total: 16.2 MB
    10.2  62.7%  62.7%     10.2  62.7% ::alloc
    6.0  37.3% 100.0%      6.0  37.3% ::alloc_zeroed
    0.0   0.0% 100.0%     10.2  62.7% ::allocate
    0.0   0.0% 100.0%      0.5   3.3% ::call
    0.0   0.0% 100.0%      4.0  24.7% ::default
    0.0   0.0% 100.0%      1.2   7.2% ::deref
    0.0   0.0% 100.0%      1.2   7.2% ::deref::__stability (inline)
    0.0   0.0% 100.0%      1.2   7.2% ::deref::__static_ref_initialize (inline)
    0.0   0.0% 100.0%      0.5   3.1% ::from
    0.0   0.0% 100.0%      9.2  56.6% ::from_iter
(jeprof)
```

### 生成内存分配调用图

常见的用例之一是查找内存泄漏，通过比较间隔前后的内存画像即可完成这一工作。

在下面的命令行中，以 10s 为间隔，获取前后两个时间节点的内存画像。

```bash
curl 'http://localhost:8080/debug/mem/pprof/profile?seconds=0' > a.prof
sleep 10
curl 'http://localhost:8080/debug/mem/pprof/profile?seconds=0' > b.prof
```

接着，可以利用这两份内存画像来生成 `pdf` 格式的内存分配调用图。

```bash
jeprof \
    --show_bytes \
    --nodecount=1024 \
    --nodefraction=0.001 \
    --edgefraction=0.001 \
    --maxdegree=64 \
    --pdf \
    ./target/debug/databend-meta \
    --base=a.prof \
    b.prof \
    > mem.pdf
```

结果如图所示：

![jeprof call graph](https://user-images.githubusercontent.com/44069/174307263-a2c9bbe6-e417-48b7-bf4d-cbbbaad03a6e.png)

### 注意事项

目前无法在 Mac 上进行内存分析，不管是 x86_64 还是 aarch64 平台。
