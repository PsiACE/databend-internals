+++
title = "Databend 源码阅读： Query Server 启动、Session 管理及请求处理"
description = "“Databend 源码阅读”系列文章的第二篇，帮助大家了解从 databend 启动服务到接受 SQL 请求并开始处理的流程。"
draft = false
weight = 420
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
giscus = true
+++

作者：[AriesDevil](https://github.com/AriesDevil) | Databend Labs 成员，数据库研发工程师

## Query 启动入口

Databend Query Server 的启动入口在 `databend/src/binaries/query/main.rs` 下，在初始化配置之后，它会创建一个 `GlobalServices` 和 server 关闭时负责处理 shutdown 逻辑的 `shutdown_handle` 。

```rust
GlobalServices::init(conf.clone()).await?;
let mut shutdown_handle = ShutdownHandle::create()?;
```

### GlobalServices

`GlobalServices` 负责启动 databend-query 的所有全局服务，这些服务都遵循单一责任原则。

```rust
pub struct GlobalServices {
    global_runtime: UnsafeCell<Option<Arc<Runtime>>>,
    // 负责处理 query log
    query_logger: UnsafeCell<Option<Arc<QueryLogger>>>,
    // 负责 databend query 集群发现
    cluster_discovery: UnsafeCell<Option<Arc<ClusterDiscovery>>>,
    // 负责与 storage 层交互来读写数据
    storage_operator: UnsafeCell<Option<Operator>>,
    async_insert_manager: UnsafeCell<Option<Arc<AsyncInsertManager>>>,
    cache_manager: UnsafeCell<Option<Arc<CacheManager>>>,
    catalog_manager: UnsafeCell<Option<Arc<CatalogManager>>>,
    http_query_manager: UnsafeCell<Option<Arc<HttpQueryManager>>>,
    data_exchange_manager: UnsafeCell<Option<Arc<DataExchangeManager>>>,
    session_manager: UnsafeCell<Option<Arc<SessionManager>>>,
    users_manager: UnsafeCell<Option<Arc<UserApiProvider>>>,
    users_role_manager: UnsafeCell<Option<Arc<RoleCacheManager>>>,
}
```

GlobalServices 中的全局服务都实现了单例 trait，这些全局管理器后续会有对应的源码分析文章介绍，本文介绍与 Session 处理相关的逻辑。

```rust
pub trait SingletonImpl<T>: Send + Sync {
    fn get(&self) -> T;

    fn init(&self, value: T) -> Result<()>;
}

pub type Singleton<T> = Arc<dyn SingletonImpl<T>>;
```

### ShutdownHandle

接下来会根据网络协议初始化 handlers，并把它们注册到 `shutdown_handler` 的 services 中，任何实现 `Server` trait 的类型都可以被添加到 services 中。

![query server](https://databend-internals.psiace.me/source-reading/init-session-handler/01-query-server.png)

```rust
#[async_trait::async_trait]
pub trait Server: Send {
    async fn shutdown(&mut self, graceful: bool);
    async fn start(&mut self, listening: SocketAddr) -> Result<SocketAddr>;
}
```

目前 Databend 支持三种协议提交查询请求：MySql, ClickHouse HTTP, Raw HTTP 。

```rust
// MySQL handler.
{
    let hostname = conf.query.mysql_handler_host.clone();
    let listening = format!("{}:{}", hostname, conf.query.mysql_handler_port);
    let mut handler = MySQLHandler::create(session_manager.clone());
    let listening = handler.start(listening.parse()?).await?;
    // 注册服务到 shutdown_handle 来处理 server shutdown 时候的关闭逻辑，下同
    shutdown_handle.add_service(handler);
}

// ClickHouse HTTP handler.
{
    let hostname = conf.query.clickhouse_http_handler_host.clone();
    let listening = format!("{}:{}", hostname, conf.query.clickhouse_http_handler_port);

    let mut srv = HttpHandler::create(session_manager.clone(), HttpHandlerKind::Clickhouse);
    let listening = srv.start(listening.parse()?).await?;
    shutdown_handle.add_service(srv);
}

// Databend HTTP handler.
{
    let hostname = conf.query.http_handler_host.clone();
    let listening = format!("{}:{}", hostname, conf.query.http_handler_port);

    let mut srv = HttpHandler::create(session_manager.clone(), HttpHandlerKind::Query);
    let listening = srv.start(listening.parse()?).await?;
    shutdown_handle.add_service(srv);
}
```

之后会创建一些其它服务：

- Metric Service ：指标服务。
- Admin Service ：负责处理管理信息。
- RPC Service ：query 节点的 rpc 服务，负责 query 节点之间的通信，使用 arrow flight 协议。

```rust
// Metric API service.
{
    let address = conf.query.metric_api_address.clone();
    let mut srv = MetricService::create(session_manager.clone());
    let listening = srv.start(address.parse()?).await?;
    shutdown_handle.add_service(srv);
    info!("Listening for Metric API: {}/metrics", listening);
}

// Admin HTTP API service.
{
    let address = conf.query.admin_api_address.clone();
    let mut srv = HttpService::create(session_manager.clone());
    let listening = srv.start(address.parse()?).await?;
    shutdown_handle.add_service(srv);
    info!("Listening for Admin HTTP API: {}", listening);
}

// RPC API service.
{
    let address = conf.query.flight_api_address.clone();
    let mut srv = RpcService::create(session_manager.clone());
    let listening = srv.start(address.parse()?).await?;
    shutdown_handle.add_service(srv);
    info!("Listening for RPC API (interserver): {}", listening);
}
```

最后会将这个 query 节点注册到 meta server 中。

```rust
// Cluster register.
{
    let cluster_discovery = session_manager.get_cluster_discovery();
    let register_to_metastore = cluster_discovery.register_to_metastore(&conf);
    register_to_metastore.await?;
}
```

## Session 相关

`session` 主要分为 4 个部分：

- `session_manager` ：全局唯一，负责管理 client session 。
- `session` ：每当有新的 client 连接到 server 之后会创建一个新的 session 并且注册到 `session_manager` 。
- `query_ctx` ：每一条查询语句会有一个 query_ctx，用来存储当前查询的一些上下文信息 。
- `query_ctx_shared` ：查询语句中的子查询共享的上下文信息 。

![session](https://databend-internals.psiace.me/source-reading/init-session-handler/02-session.png)

下面逐一来分析。

### SessionManager

代码位置：`query/src/sessions/session_mgr.rs` 。

```rust
pub struct SessionManager {
    pub(in crate::sessions) conf: Config,
    pub(in crate::sessions) max_sessions: usize,
    pub(in crate::sessions) active_sessions: Arc<RwLock<HashMap<String, Arc<Session>>>>,
    pub status: Arc<RwLock<SessionManagerStatus>>,

    // When session type is MySQL, insert into this map, key is id, val is MySQL connection id.
    pub(crate) mysql_conn_map: Arc<RwLock<HashMap<Option<u32>, String>>>,
    pub(in crate::sessions) mysql_basic_conn_id: AtomicU32,
}
```

`SessionManager` 主要用来创建和销毁 session，对应方法如下：

```rust
// 根据 client 协议类型来创建 session
pub async fn create_session(self: &Arc<Self>, typ: SessionType) -> Result<SessionRef>

// 根据 session id 来销毁 session
pub fn destroy_session(self: &Arc<Self>, session_id: &String)
```

### Session

代码位置：`query/src/sessions/session.rs` 。

session 主要存储 client-server 的上下文信息，代码命名已经很清晰了，这里就不再过多赘述。

```rust
pub struct Session {
    pub(in crate::sessions) id: String,
    pub(in crate::sessions) typ: RwLock<SessionType>,
    pub(in crate::sessions) session_ctx: Arc<SessionContext>,
    status: Arc<RwLock<SessionStatus>>,
    pub(in crate::sessions) mysql_connection_id: Option<u32>,
}

pub struct SessionContext {
    conf: Config,
    abort: AtomicBool,
    current_catalog: RwLock<String>,
    current_database: RwLock<String>,
    current_tenant: RwLock<String>,
    current_user: RwLock<Option<UserInfo>>,
    auth_role: RwLock<Option<String>>,
    client_host: RwLock<Option<SocketAddr>>,
    io_shutdown_tx: RwLock<Option<Sender<Sender<()>>>>,
    query_context_shared: RwLock<Option<Arc<QueryContextShared>>>,
}

pub struct SessionStatus {
    pub session_started_at: Instant,
    pub last_query_finished_at: Option<Instant>,
}
```

Session 的另一个大的功能是负责创建和获取 QueryContext，每次接收到新的 query 请求都会创建一个 QueryContext 并绑定在对应的 query 语句上。

### QueryContext

代码位置：`query/src/sessions/query_ctx.rs` 。

`QueryContext` 主要是维护查询的上下文信息，它通过 `QueryContext::create_from_shared` (query_ctx_shared) 创建。

```rust
#[derive(Clone)]
pub struct QueryContext {
    version: String,
    statistics: Arc<RwLock<Statistics>>,
    partition_queue: Arc<RwLock<VecDeque<PartInfoPtr>>>,
    shared: Arc<QueryContextShared>,
    precommit_blocks: Arc<RwLock<Vec<DataBlock>>>,
    fragment_id: Arc<AtomicUsize>,
}
```

其中 `partition_queue` 主要存储查询对应的 `PartInfo`，包括 part 的地址、版本信息、涉及数据的行数，part 使用的压缩算法、以及涉及到 column 的 meta 信息。在 pipeline build 时候会去设置 partition 。pipeline 后续会有专门的文章介绍。

`precommit_blocks` 负责暂存插入操作的时已经写入到存储， 但是尚未提交的元数据，`DataBlock` 主要包含 Column 的元信息引用和 arrow schema 的信息。

### QueryContextShared

代码位置：`query/src/sessions/query_ctx_shared.rs` 。

对于包含子查询的查询，需要共享很多上下文信息，这就是 `QueryContextShared` 存在的理由。

```rust
/// 数据需要在查询上下文中被共享，这个很重要，比如:
///     USE database_1;
///     SELECT
///         (SELECT scalar FROM table_name_1) AS scalar_1,
///         (SELECT scalar FROM table_name_2) AS scalar_2,
///         (SELECT scalar FROM table_name_3) AS scalar_3
///     FROM table_name_4;
/// 对于上面子查询, 会共享 runtime, session, progress, init_query_id
pub struct QueryContextShared {
    /// scan_progress for scan metrics of datablocks (uncompressed)
    pub(in crate::sessions) scan_progress: Arc<Progress>,
    /// write_progress for write/commit metrics of datablocks (uncompressed)
    pub(in crate::sessions) write_progress: Arc<Progress>,
    /// result_progress for metrics of result datablocks (uncompressed)
    pub(in crate::sessions) result_progress: Arc<Progress>,
    pub(in crate::sessions) error: Arc<Mutex<Option<ErrorCode>>>,
    pub(in crate::sessions) session: Arc<Session>,
    pub(in crate::sessions) runtime: Arc<RwLock<Option<Arc<Runtime>>>>,
    pub(in crate::sessions) init_query_id: Arc<RwLock<String>>,
    ...
}
```

它提供了 query 上下文所需要的一切基本信息。

## Handler

之前提到了 Databend 支持多种 handler，下面就以 mysql 为例，看一下 handler 的处理流程以及如何与 session 产生交互。

首先 `MySQLHandler` 会包含一个 `SessionManager` 的引用。

```rust
pub struct MySQLHandler {
    abort_handle: AbortHandle,
    abort_registration: Option<AbortRegistration>,
    join_handle: Option<JoinHandle<()>>,
}
```

`MySQLHandler` 在启动后，会 `spawn` 一个 tokio task 来持续监听 tcp stream，并且创建一个 session 再启动一个 task 去执行之后的查询请求。

```rust
fn accept_socket(session_mgr: Arc<SessionManager>, executor: Arc<Runtime>, socket: TcpStream) {
    executor.spawn(async move {
        // 创建 session
        match session_mgr.create_session(SessionType::MySQL).await {
            Err(error) => Self::reject_session(socket, error).await,
            Ok(session) => {
                info!("MySQL connection coming: {:?}", socket.peer_addr());
                // 执行查询
                if let Err(error) = MySQLConnection::run_on_stream(session, socket) {
                    error!("Unexpected error occurred during query: {:?}", error);
                };
            }
        }
    });
}
```

在 `MySQLConnection::run_on_stream` 中，session 会先 attach 到对应的 client host 并且注册一个 shutdown 闭包来处理关闭连接关闭时需要执行的清理，关键代码如下：

```rust
// mysql_session.rs
pub fn run_on_stream(session: SessionRef, stream: TcpStream) -> Result<()> {
    let blocking_stream = Self::convert_stream(stream)?;
    MySQLConnection::attach_session(&session, &blocking_stream)?;

    ...
}

fn attach_session(session: &SessionRef, blocking_stream: &std::net::TcpStream) -> Result<()> {
    let host = blocking_stream.peer_addr().ok();
    let blocking_stream_ref = blocking_stream.try_clone()?;
    session.attach(host, move || {
        // 注册 shutdown 逻辑
        if let Err(error) = blocking_stream_ref.shutdown(Shutdown::Both) {
            error!("Cannot shutdown MySQL session io {}", error);
        }
    });

    Ok(())
}

// session.rs
pub fn attach<F>(self: &Arc<Self>, host: Option<SocketAddr>, io_shutdown: F)
where F: FnOnce() + Send + 'static {
    let (tx, rx) = oneshot::channel();
    self.session_ctx.set_client_host(host);
    self.session_ctx.set_io_shutdown_tx(Some(tx));

    common_base::base::tokio::spawn(async move {
        // 在 session quit 时候触发清理
        if let Ok(tx) = rx.await {
            (io_shutdown)();
            tx.send(()).ok();
        }
    });
}
```

之后会启动一个 MySQL InteractiveWorker 来处理后续的查询。

```rust
let join_handle = query_executor.spawn(async move {
    let client_addr = non_blocking_stream.peer_addr().unwrap().to_string();
    let interactive_worker = InteractiveWorker::create(session, client_addr);
    let opts = IntermediaryOptions {
        process_use_statement_on_query: true,
    };
    let (r, w) = non_blocking_stream.into_split();
    let w = BufWriter::with_capacity(DEFAULT_RESULT_SET_WRITE_BUFFER_SIZE, w);
    AsyncMysqlIntermediary::run_with_options(interactive_worker, r, w, &opts).await
});
let _ = futures::executor::block_on(join_handle);
```

该 `InteractiveWorker` 会实现 `AsyncMysqlShim` trait 的方法，比如：`on_execute` 、`on_query` 等。查询到来时会回调这些方法来执行查询。

这里以 `on_query` 为例，关键代码如下：

```rust
async fn on_query<'a>(
    &'a mut self,
    query: &'a str,
    writer: QueryResultWriter<'a, W>,
) -> Result<()> {
    ...

    // response writer
    let mut writer = DFQueryResultWriter::create(writer);

    let instant = Instant::now();
    // 执行查询
    let blocks = self.base.do_query(query).await;

    // 回写结果
    let format = self.base.session.get_format_settings()?;
    let mut write_result = writer.write(blocks, &format);

    ...

    // metrics 信息
    histogram!(
        super::mysql_metrics::METRIC_MYSQL_PROCESSOR_REQUEST_DURATION,
        instant.elapsed()
    );

    write_result
}
```

在 `do_query` 中会创建 `QueryContext` 并开始解析 sql 流程来完成后续的整个 sql 查询。关键代码如下：

```rust
// 创建 QueryContext
let context = self.session.create_query_context().await?;
// 关联到查询语句
context.attach_query_str(query);

let settings = context.get_settings();

// parse sql
let stmts_hints = DfParser::parse_sql(query, context.get_current_session().get_type());
...

// 创建并生成查询计划
let mut planner = Planner::new(context.clone());
let interpreter = planner.plan_sql(query).await.and_then(|v| {
    has_result_set = has_result_set_by_plan(&v.0);
    InterpreterFactoryV2::get(context.clone(), &v.0)
})

// 执行查询，返回结果
Self::exec_query(interpreter.clone(), &context).await?;
let schema = interpreter.schema();
Ok(QueryResult::create(
    blocks,
    extra_info,
    has_result_set,
    schema,
))
```

## 尾声

以上就是从 Databend 启动服务到接受 SQL 请求并开始处理的流程。最近我们因为一些原因（ClickHouse TCP 协议偏向 ClickHouse 的底层，协议没有公开的文档说明，同时里面历史包袱比较重，排查问题浪费大量精力）去掉了 ClickHouse Native TCP Client，具体请参见: <https://github.com/datafuselabs/databend/pull/7012>

如果你阅读完代码有好的提议，欢迎来这里讨论，另外如果发现相关的问题，可以提交到 issue 来帮助我们提高 Databend 的稳定性。Databend 社区欢迎一切善意的意见和建议 :)