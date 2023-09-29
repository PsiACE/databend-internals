+++
title = "Databend 全链路追踪"
description = "全链路追踪意味着能够追踪到每一个调用请求的完整调用链路、收集性能数据并反馈异常。Databend 使用 tracing 赋能可观测性，实现全链路追踪。"
draft = false
weight = 630
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "全链路追踪意味着能够追踪到每一个调用请求的完整调用链路、收集性能数据并反馈异常。Databend 使用 tracing 赋能可观测性，实现全链路追踪。"
toc = true
top = false
giscus = true
+++

## Databend 与 Tracing

初步了解 Databend 怎么实现全链路追踪。

### 初识 Tracing

![Tracing Logo](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/01-tracing.svg)

Tracing 是由 Tokio 团队维护的 Rust 应用追踪框架，用来收集结构化的、基于事件的诊断信息。

项目地址：https://github.com/tokio-rs/tracing

**示例：**

```rust
use tracing::{info, Level};
use tracing_subscriber;

fn main() {
    let collector = tracing_subscriber::fmt()
        // filter spans/events with level TRACE or higher.
        .with_max_level(Level::TRACE)
        // build but do not install the subscriber.
        .finish();

    tracing::collect::with_default(collector, || {
        info!("This will be logged to stdout");
    });
    info!("This will _not_ be logged to stdout");
}
```

### Databend 中的 Tracing

Databend 的 `tracing-subscriber` 被统一整合在 `common/tracing`，由 query 和 meta 共用。

```rust
// Use env RUST_LOG to initialize log if present.
// Otherwise use the specified level.
let directives = env::var(EnvFilter::DEFAULT_ENV).unwrap_or_else(|_x| level.to_string());
let env_filter = EnvFilter::new(directives);
let subscriber = Registry::default()
    .with(env_filter)                # 根据环境变量过滤
    .with(JsonStorageLayer)          # 利用 tracing-bunyan-formatter 格式化为 json
    .with(stdout_logging_layer)      # 标准输出
    .with(file_logging_layer)        # 输出到文件，默认位于 `_logs` 目录下
    .with(jaeger_layer);             # opentelemetry-jaeger

#[cfg(feature = "console")]
let subscriber = subscriber.with(console_subscriber::spawn()); # tokio console

tracing::subscriber::set_global_default(subscriber)
    .expect("error setting global tracing subscriber");
```

具体到内部的 `tracing` 记录，大致有两类：

1. 普通：与其他 `log` 方式一样，利用 `info!`、`debug!` 来收集信息。

   ```rust
   use common_tracing::tracing;

   tracing::info!("{:?}", conf);
   tracing::info!("DatabendQuery {}", *databend_query::DATABEND_COMMIT_VERSION);
   ```

2. Instruments：在调用函数时创建并进入 tracing span（跨度），span 表示程序在特定上下文中执行的时间段。

   ```rust
   use common_tracing::tracing::debug_span;
   #[tracing::instrument(level = "debug", skip_all)]
   async fn read(&mut self) -> Result<Option<DataBlock>> {
       ...
           fetched_metadata = read_metadata_async(&mut self.reader)
               .instrument(debug_span!("parquet_source_read_meta"))
               .await
               .map_err(|e| ErrorCode::ParquetError(e.to_string()))?;
       ...
   }
   ```

**示例：**

```json
{
  "v": 0,
  "name": "databend-query-test_cluster@0.0.0.0:3307",
  "msg": "Shutdown server.",
  "level": 30,
  "hostname": "dataslime",
  "pid": 53341,
  "time": "2022-05-11T00:51:56.374807359Z",
  "target": "databend_query",
  "line": 153,
  "file": "query/src/bin/databend-query.rs"
}
```

### 观测 Databend 追踪的方式

Databend 原生提供了多种观测方式，以方便诊断和调试：

1. http tracing ：访问 localhost:8080/v1/logs（根据配置）。
2. stdout/filelog ：检查终端输出或 `_logs` 目录（根据配置）。
3. system.tracing 表 ：执行 `select * from system.tracing limit 20;` 。
4. jaeger ：运行 jaeger ，访问 http://127.0.0.1:16686/ 。
5. console ：按特定方式构建后，运行 tokio-console 。

## Jaeger 分布式追踪

使用 Jaeger 对 Databend 进行全链路追踪。

### Opentelemetry & Jaeger

**OpenTelemetry** 是工具、API 和 SDK 的集合。使用它来检测、生成、收集和导出遥测数据（度量、日志和追踪），以帮助您分析软件的性能和行为。

**Jaeger** 是一个开源的端到端分布式追踪系统。由 Uber 捐赠给 CNCF 。它可以用于监视基于微服务的分布式系统，提供以下能力：

- 分布式上下文传播
- 分布式事务监视
- 根本原因分析
- 服务依赖性分析
- 性能/延迟优化

![Opentelemetry & Jaeger](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/02-jaeger-and-opentelemetry.png)

### Step by Step

遵循下述步骤，即可使用 Jaeger 探索 Databend ：

1. 构建二进制程式：`cargo build`（可以使用 `--bin` 指定）。
2. 指定 Jaeger endpoint ，并将日志级别设定为 `DEBUG` ，接着运行需要调试的应用程式。例如，`DATABEND_JAEGER_AGENT_ENDPOINT=localhost:6831 LOG_LEVEL=DEBUG ./databend-query` 。
3. 运行 jaeger ：`docker run -d -p6831:6831/udp -p6832:6832/udp -p16686:16686 jaegertracing/all-in- one:latest` 。
4. 打开 `http://127.0.0.1:16686/` 以查看 jaeger 收集的信息。

**注意** 只有正确配置 `DATABEND_JAEGER_AGENT_ENDPOINT` 才能启用 Jaeger 支持。

### 结果探索

![dot graph](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/03-jaeger-dot-graph.png)

_x 轴是执行时刻，y 轴是持续的时间，圆点反映 span 的聚集程度。_

执行下述语句即可得到上图所示追踪结果：

```SQL
CREATE TABLE t1(a INT);
INSERT INTO t1 VALUES(1);
INSERT INTO t1 SELECT * FROM t1;
```

**Timeline**

下图是点击最大的圆点得到的追踪情况：

![span tracing](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/04-jaeger-span-tracing.png)

使用 timeline 模式来展现 tracing 的各个跨度之间的关系。以时间为主线进行分析,方便使用者观看在某个时间点观看程序信息。

点开第一个跨度，可以看到这是执行 `INSERT INTO t1 SELECT *FROM t1` 查询时的情况。

![span info](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/05-jaeger-span-info.png)

**Graph**

切换到 graph 模式，可以看到各个 span 之间的调用链，每个 span 具体用时 ,以及百分比。

![span graph](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/06-jaeger-span-graph.png)

通过这个视图使用者很容易知道系统瓶颈,快速定位问题。

**Compare**

连起来的各个部分形成整个 trace 的调用链。因为比较时一般会比较两个相同类型的调用，所以看到的会是重合后的视图。

![span compare](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/07-jaeger-span-compare.png)

对于颜色的一个说明：

- 深绿色，表示这个 span 只存在于 trace-B 中，A 没有这个 span
- 深红色，表示这个 span 只存在于 trace-A 中，B 没有这个 span
- 浅绿色，表示这个 span 在 trace-B（右边这个）的数量多
- 浅红色，表示这个 span 在 trace-A（左边这个）的数量多

## tokio-console 诊断

tokio-rs 团队出品的诊断和调试工具，可以帮助我们诊断与 tokio 运行时相关的问题。

### console 是什么

![tokio console](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/08-tokio-console.png)

tokio-console 是专为异步程序设计的调试与诊断工具，能够列出 tokio 的任务，提供对程序的任务和资源的实时、易于导航的视图，总结了它们的当前状态和历史行为。主要包含以下组件：

- 传输诊断数据的协议
- 用于收集诊断数据的仪器
- 当然，用于展示和探索诊断数据的实用工具

项目地址：https://github.com/tokio-rs/console

### Step by Step

1. 使用特定的 `rustflags` 和 `features` 来构建：
   `RUSTFLAGS="--cfg tokio_unstable" cargo build --features tokio-console` ，也可以只构建单个二进制程式，使用 `--bin` 进行指定。
2. 将日志级别设定为 `TRACE` ，运行需要调试的应用程式 `LOG_LEVEL=TRACE databend-query` 或者 `databend-meta --single --log-level=TRACE`。可以使用 `TOKIO_CONSOLE_BIND` 指定端口，以避免潜在的端口抢占问题。
3. 运行 `tokio-console`，默认连接到 http://127.0.0.1:6669 。

### 结果探索

**任务**

先看什么是 tokio 任务：

1. 任务是一个轻量级的、非阻塞的执行单元。类似操作系统的线程，但是是由 tokio 运行时管理，一般叫做“绿色线程”，与 Go 的 goroutine，Kotlin 的 coroutine 类似。
2. 任务是协同调度的。大多数操作系统实现抢占式多任务。操作系统允许每个线程运行一段时间，然后抢占它，暂停该线程并切换到另一个线程。另一方面，任务实现协同多任务。一个任务被允许运行直到它让出执行权，运行时会切换到执行下一个任务。

![tokio console basic](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/09-tokio-console-basic.png)

**基础视图**

通过左右切换，可以得到总忙时间或轮询次数等指标对任务进行排序。控制台通过高亮来提示较大差异，比如从毫秒到秒的切换。

![tokio console sort](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/10-tokio-console-sort.png)

控制台还实现了一个“警告”系统。通过监视应用程序中任务的运行时操作，控制台可以检测可能提示 bug 或性能问题的行为模式，并突出显示这些行为模式供用户分析。比如已经运行了很长时间而没有让步的任务，唤醒的次数比被其他任务唤醒的次数还要多的任务。

**任务视图**

上下切换选中任务，enter 查看关于每个任务的翔实数据，比如轮询持续时间的直方图。

![tokio console task](https://databend-internals.psiace.me/contribute-to-databend/tracing-in-databend/11-tokio-console-task.png)

不仅列出任务。console 还会插入异步互斥和信号量等资源。Tokio Console 的资源详细信息视图显示了哪些任务已经进入临界区，哪些任务正在等待获得访问权限。

## 还能做什么

与分布式追踪和日志系统相关的一些其他内容。

### 可观测性 + +

目前还有一系列关于可观测性和 Tracing 的 Issue 有待解决：

- Integrate tokio metrics for query task based tokio runtime monitoring #4205
- Configure on jaeger tracing address similar to metrics api server #3633
- Summary of todos about distributed tracing #1227
- Query traces and analysis, based on user behavior. #1177
- Http stack traces #1085
- Shuffle read/write metrics #1004

另外，更进一步的考量是，如何基于可观测性来自动/半自动地发现问题并对系统进行调优。

### 了解更多 . . .

**Tracing**

- https://github.com/tokio-rs/tracing
- https://docs.rs/tracing/latest/tracing/
- https://tokio.rs/blog/2019-08-tracing

**Jaeger**

- https://github.com/open-telemetry/opentelemetry-rust/tree/main/opentelemetry-jaeger
- https://21-lessons.com/ship-rust-tracing-spans-to-jaeger/

**tokio-console**

- https://github.com/tokio-rs/console
- https://hackmd.io/@aws-rust-platform/ByChcdB-t
- https://tokio.rs/blog/2021-12-announcing-tokio-console
