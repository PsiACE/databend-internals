+++
title = "轻松了解 Databend 构建"
description = "Databend 除了支持本机构建外，还可以使用 build tool 来进行跨平台构建。"
draft = false
weight = 620
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Databend 除了支持本机构建外，还可以使用 build tool 来进行跨平台构建。"
toc = true
top = false
giscus = true
+++

## 如何构建 Databend

### Make

Databend 在 `Makefile` 中封装了大量常见命令。采用 make 构建只会开启默认特性，并且会一次性构建 `databend-meta`、`databend-query` 以及 `databend-metactl` 。

按 [前文](https://databend-internals.psiace.me/docs/contribute-to-databend/development-environment/) 设置好开发环境后。

- 执行 `make build` 即可轻松构建 debug 版本。
- `make build-release` 则会构建 release 版本，并会采用 objcopy 减少二进制体积。

### Cargo

使用 cargo 构建的好处在于可以按需开启特性，并灵活控制要构建的目标二进制文件。

常用的命令格式如：

```bash
RUSTFLAGS="--cfg tokio_unstable" cargo build --bin=databend-query --features=tokio-console
```

即可构建启用 `tokio-console` 支持的 databend-query ，使用 `RUSTFLAGS="--cfg tokio_unstable"` 是因为 `tokio` 的 `tracing` 特性还没有稳定下来。

**Databend features 速览**

- `simd = ["common-arrow/simd"]`：默认开启的特性，启用 arrow2 的 SIMD 支持（meta & query）。
- `tokio-console = ["common-tracing/console", "common-base/tracing"]`：用于 tokio 监控和调试，（meta & query）。
- `memory-profiling = ["common-base/memory-profiling", "tempfile"]`：用于内存分析，（meta & query）。
- `storage-hdfs = ["opendal/services-hdfs", "common-io/storage-hdfs"]`：用于提供 hdfs 支持，（query）。
- `hive = ["common-hive-meta-store", "thrift", "storage-hdfs"]`：用于提供 hive 支持，（query）。

### 跨平台构建

Databend 提供了 build-tool image，可以简化跨平台构建所需工作。

示例选用 `x86_64-unknown-linux-musl` 目标平台，其他支持平台也类似：

```bash
IMAGE='datafuselabs/build-tool:x86_64-unknown-linux-musl' RUSTFLAGS='-C link-arg=-Wl,--compress-debug-sections=zlib-gabi' ./scripts/setup/run_build_tool.sh cargo build --target x86_6
4-unknown-linux-musl
```
