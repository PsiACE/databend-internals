+++
title = "Databend 性能剖析方法与工具"
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

pprof 是 Google 开源的代码性能分析工具，可以直接生成代码分析报告，不仅支持通过命令式交互查看，也便于可视化展示。Databend 使用 [pprof-rs](https://crates.io/crates/pprof) 完成对 pprof 工具的支持。

### 必备工作

- CPU 分析属于 Databend 的内置能力，部署一个 Databend 实例即可开始使用。

### 命令行交互

```bash
go tool pprof http://localhost:<your-databend-port>/debug/pprof/profile?seconds=<your-profile-second>
```

若 http 端口为 8080 ，采样时间为 20 秒，结果示例如下：

```bash
$ go tool pprof http://localhost:8080/debug/pprof/profile?seconds=20
Fetching profile over HTTP from http://localhost:8080/debug/pprof/profile?seconds=20
Saved profile in ~/pprof/pprof.samples.cpu.001.pb.gz
Type: cpu
Time: Jul 15, 2022 at 9:45am (CST)
Duration: 20s, Total samples = 141.41ms ( 0.71%)
Entering interactive mode (type "help" for commands, "o" for options)
(pprof) top
Showing nodes accounting for 141.41ms, 100% of 141.41ms total
Showing top 10 nodes out of 218
      flat  flat%   sum%        cum   cum%
  141.41ms   100%   100%   141.41ms   100%  backtrace::backtrace::libunwind::trace
         0     0%   100%    10.10ms  7.14%  <&mut regex_syntax::utf8::Utf8Sequences as core::iter::traits::iterator::Iterator>::next
         0     0%   100%    10.10ms  7.14%  <<std::thread::Builder>::spawn_unchecked_<sled::threadpool::queue::spawn_to<sled::pagecache::iterator::scan_segment_headers_and_tail::{closure#0}::{closure#0}, core::option::Option<(u64, sled::pagecache::logger::SegmentHeader)>>::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}
         0     0%   100%    10.10ms  7.14%  <<std::thread::Builder>::spawn_unchecked_<sled::threadpool::queue::spawn_to<sled::pagecache::iterator::scan_segment_headers_and_tail::{closure#0}::{closure#0}, core::option::Option<(u64, sled::pagecache::logger::SegmentHeader)>>::{closure#0}::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}
         0     0%   100%    10.10ms  7.14%  <<std::thread::Builder>::spawn_unchecked_<sled::threadpool::queue::spawn_to<sled::pagecache::iterator::scan_segment_headers_and_tail::{closure#0}::{closure#0}, core::option::Option<(u64, sled::pagecache::logger::SegmentHeader)>>::{closure#0}::{closure#2}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}
         0     0%   100%    10.10ms  7.14%  <<std::thread::Builder>::spawn_unchecked_<sled::threadpool::queue::spawn_to<sled::pagecache::iterator::scan_segment_headers_and_tail::{closure#0}::{closure#0}, core::option::Option<(u64, sled::pagecache::logger::SegmentHeader)>>::{closure#0}::{closure#3}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}
         0     0%   100%    10.10ms  7.14%  <[&str]>::iter
         0     0%   100%    10.10ms  7.14%  <[(char, &[char])]>::binary_search_by::<<[(char, &[char])]>::binary_search_by_key<char, regex_syntax::unicode::simple_fold::imp::{closure#0}>::{closure#0}>
         0     0%   100%    10.10ms  7.14%  <[(char, &[char])]>::binary_search_by_key::<char, regex_syntax::unicode::simple_fold::imp::{closure#0}>
         0     0%   100%    10.10ms  7.14%  <[(char, &[char])]>::binary_search_by_key::<char, regex_syntax::unicode::simple_fold::imp::{closure#0}>::{closure#0}
```

### 可视化

执行下述命令可以进行可视化：

```bash
go tool pprof -http=0.0.0.0:<your-profile-port> <your profile data>
```

例如，执行下述语句可以在 8088 端口开启 WEB UI 。

```bash
go tool pprof -http=0.0.0.0:8088 ~/pprof/pprof.samples.cpu.001.pb.gz 
```

访问 `http://0.0.0.0:8088/ui/flamegraph` 即可得到火焰图。

![pprof flamegraph](https://psiace.github.io/databend-internals/contribute-to-databend/how-to-profile/01-pprof-flamegraph.png)

### 注意事项

Databend 暂时不支持在 musl 平台上运行 pprof 。

## Memory Profiling

内存分析，在应用程序进行堆分配时记录堆栈追踪，用于监视当前和历史内存使用情况，以及检查内存泄漏。

通过与 `jemalloc` 的集成，Databend 得以整合多种内存分析能力。这里使用 `jeprof` 进行内存分析。

### 必备工作

- [安装 Jemalloc](https://github.com/jemalloc/jemalloc/blob/dev/INSTALL.md)，并启用其剖析能力 `./configure --enable-prof`
- 在构建二进制文件时启用 `memory-profiling` 特性：`cargo build --features memory-profiling`
- 在创建 Databend 实例时，设置环境变量 `MALLOC_CONF=prof:true` 以启用内存分析。示例：

  ```bash
  MALLOC_CONF=prof:true ./target/debug/databend-query
  ```

### 堆快照转储

```bash
jeprof <your-profile-target> http://localhost:<your-databend-port>/debug/mem
```

下面的例子选用 debug 模式下编译的 databend-query 作为 target，端口为 8080，结果如下所示：

```bash
$ jeprof ./target/debug/databend-query http://localhost:8080/debug/mem
Using local file ./target/debug/databend-query.
Gathering CPU profile from http://localhost:8080/debug/mem/pprof/profile?seconds=30 for 30 seconds to
  ~/jeprof/databend-query.1658367127.localhost
Be patient...
Wrote profile to ~/jeprof/databend-query.1658367127.localhost
Welcome to jeprof!  For help, type 'help'.
(jeprof) top
Total: 11.1 MB
     6.0  54.6%  54.6%      6.0  54.6% ::alloc_zeroed
     5.0  45.4% 100.0%      5.0  45.4% ::alloc
     0.0   0.0% 100.0%      0.5   4.5% ::add_node::{closure#0}
     0.0   0.0% 100.0%      5.0  45.4% ::alloc_impl
     0.0   0.0% 100.0%      5.0  45.4% ::allocate
     0.0   0.0% 100.0%      4.5  40.8% ::allocate_in
     0.0   0.0% 100.0%      0.5   4.5% ::apply_batch_inner
     0.0   0.0% 100.0%     11.1 100.0% ::block_on::
     0.0   0.0% 100.0%     11.1 100.0% ::block_on::::{closure#0}
     0.0   0.0% 100.0%      0.5   4.5% ::clone
(jeprof) 
```

### 生成内存分配调用图

常见的用例之一是查找内存泄漏，通过比较间隔前后的内存画像即可完成这一工作。

在下面的命令行中，以 10s 为间隔，获取前后两个时间节点的内存画像。

```bash
curl 'http://localhost:<your-databend-port>/debug/mem/pprof/profile?seconds=0' > a.prof
sleep 10
curl 'http://localhost:<your-databend-port>/debug/mem/pprof/profile?seconds=0' > b.prof
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
    <your-profile-target> \
    --base=a.prof \
    b.prof \
    > mem.pdf
```

同样选用 debug 模式下编译的 databend-query 作为 target，端口为 8080，结果如图所示：

![jeprof call graph](https://psiace.github.io/databend-internals/contribute-to-databend/how-to-profile/02-jeprof-mem.png)

### 注意事项

目前无法在 Mac 上进行内存分析，不管是 x86_64 还是 aarch64 平台。
