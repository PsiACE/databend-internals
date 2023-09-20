+++
title = "Databend 源码阅读：配置管理"
description = "“Databend 源码阅读”系列文章的第四篇，从配置入手，解读环境变量、配置文件和命令行选项之间是如何映射的。"
draft = false
weight = 440
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
giscus = true
+++

对于 Databend 这样复杂的数据库服务端程序，往往需要支持大量的可配置选项，以帮助运维人员根据实际使用需要管理和调优系统。

Databend 目前支持三种配置方式：命令行、环境变量和配置文件，优先级依次递减。

- 一般情况下，推荐使用配置文件来记录和管理各种配置。
- 对于 K8S 集群，为了灵活变更部分配置（比如，特性开关），使用环境变量可能是更优雅的形式。
- 命令行则用于调整本地环境下的少数冲突配置。

## Databend Query 中的映射

对于 `databend-query` ，不管是什么形式的配置，其配置选项几乎可以看作是代码的扁平化树形映射，即基本符合代码中「配置域」+「配置项」的逻辑。

- 环境变量和配置文件中，利用 `serfig` 将代码嵌套展开，使用 `_` 做为分隔符。
- 命令行中稍有不同：一方面，分隔符使用 `-`；另一方面，部分命令行选项的名称中没有绑定配置域。

为了更好理解这里的映射关系，我们可以深入到具体一项配置，下面将围绕 `admin_api_address` 这个配置项展开。

- 在环境变量上，需要使用 `QUERY_ADMIN_API_ADDRESS` ，`QUERY` 表征这个配置所处的域，而 `ADMIN_API_ADDRESS` 是具体的配置项。
- 在配置文件中，通常是使用 toml 来进行配置。 `[query]` 表征配置所处的域，`admin_api_address` 为具体的配置项。

    ```toml
    [query]
    ...
    # Databend Query http address.
    # For admin RESET API.
    admin_api_address = "0.0.0.0:8081"
    ...
    ```
- 命令行中需要使用 `--admin-api-address` 进行配置，这一项没有绑定「配置域」。如果是配置 `--storage-s3-access-key-id` ，那么「storage」+ 「s3」构成配置域，「access-key-id」是具体的配置项。

在了解如何对 `admin_api_address` 进行配置后，让我们进入到配置相关的代码，进一步查看映射关系的代码形式（位于 `src/query/config/src/config.rs`）。

```rust
pub struct Config {
    ...

    // Query engine config.
    #[clap(flatten)]
    pub query: QueryConfig,
    
    ...
}

/// Query config group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Args)]
#[serde(default, deny_unknown_fields)]
pub struct QueryConfig {
    ...
    
    #[clap(long, default_value = "127.0.0.1:8080")]
    pub admin_api_address: String,
    
    ...
}
```

因为代码中使用了嵌套的层级结构，最上层是 `Config`，而 `admin_api_address` 是 `pub query: QueryConfig` 中的一个配置项，经过 `serfig` 处理后，需要使用 `QUERY` 或者 `[query]` 表征其所处的域，配置项就还是 `admin_api_address` 。

而命令行中具体的配置项名称和默认值会受到 `#[clap(long = "<long-name>", default_value = "<value>")]` 控制），`clap` 会接管配置：
- `admin_api_address` 就变成了 `--admin-api-address`。
- `--storage-s3-access-key-id` 而言，其实际的代码层级是 `Config` -> `StorageConfig` ->  `S3StorageConfig` -> `access_key_id`，字段之上有标注 `#[clap(long = "storage-s3-access-key-id", default_value_t)]` ，所以需要使用 `--storage-s3-access-key-id` 进行配置。

## Databend Meta 中的映射

`databend-meta` 的配置文件和命令行逻辑与 `databend-query` 是基本一致的。但是环境变量是通过 `serfig` 内置的 `serde-env` 自行定义的映射关系（但同样可以尝试按「配置域」+「配置项」进行理解）。

同样具体到单独的某项配置来看一下，这里以 `log_dir` 为例。

- 在环境变量上，需要使用 `METASRV_LOG_DIR` ，`METASRV` 表征这个配置所处的域，而 `LOG_DIR` 是具体的配置项。
- 而在配置文件中，这一配置项作用于全局，只需要：

    ```toml
    log_dir                 = "./.databend/logs1"
    ```
- 在命令行中当然也直接 `--log-dir` 进行配置。

让我们通过代码来解构其映射，代码位于 `src/meta/service/src/configs/outer_v0.rs`。

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Parser)]
#[clap(about, version = &**METASRV_COMMIT_VERSION, author)]
#[serde(default)]
pub struct Config {
    ...
    /// Log file dir
    #[clap(long = "log-dir", default_value = "./.databend/logs")]
    pub log_dir: String,
    ...
}
```

配置文件和命令行参数相关的配置项是由 `Config` 结构体管理的，逻辑与 `databend-query` 一致，就不再赘述。

而环境变量的配置项是由 `ConfigViaEnv` 结构体进行处理的，如下：

```rust
/// #[serde(flatten)] doesn't work correctly for env.
/// We should work around it by flatten them manually.
/// We are seeking for better solutions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ConfigViaEnv {
    ...
    pub metasrv_log_dir: String,
    ...
}
```

与 `Config` 之间的映射关系位于 `impl From<Config> for ConfigViaEnv` 和 `impl Into<Config> for ConfigViaEnv` 这两个部分。对于 `metasrv_log_dir` 而言，就是映射到前面的 `log_dir` 字段。
