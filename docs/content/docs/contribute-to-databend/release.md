+++
title = "Databend 版本发布"
description = "Databend 的版本发布"
draft = false
weight = 670
sort_by = "weight"
template = "docs/page.html"

[extra]
giscus = true
+++

## 版本发布

目前 Databend 采用 nightly 发布模式，每天夜里都会打包二进制文件，并递增 patch 版本。

### targets

目前主要关注的 targets 包括：

- x86_64-unknown-linux-gnu
- x86_64-unknown-linux-musl
- x86_64-apple-darwin
- aarch64-unknown-linux-gnu
- aarch64-unknown-linux-musl
- aarch64-apple-darwin

### 内容物概要

为方便体验，release 中除了 meta 和 query 的二进制文件之外，还包含一份默认配置和用于快速启动的脚本。

```
✦ ❯ tree .
.
├── bin
│   ├── databend-meta
│   ├── databend-metabench
│   ├── databend-metactl
│   └── databend-query
├── configs
│   ├── databend-meta.toml
│   └── databend-query.toml
├── meta.log
├── query.log
├── readme.txt
└── scripts
    ├── start.sh
    └── stop.sh

3 directories, 11 files
```

`readme.txt` 中包含一些必要的提示信息，只需执行 `./scripts/start.sh` 即可快速启动 databend 。

## 路线图

尽管采用 nightly 发布模式，但 Databend 并非野蛮生长。除了年度路线图外，Databend 还会按开发阶段发布版本路线图，这也决定了当前 minor 版本的分配。

### 年度路线图

- [Issue #3706 - Roadmap 2022](https://github.com/datafuselabs/databend/issues/3706)
- [Issue #746 - Roadmap 2021](https://github.com/datafuselabs/databend/issues/746)

### 版本路线图

- [Release proposal: Nightly v0.8](https://github.com/datafuselabs/databend/issues/4591)
- [Release proposal: Nightly v0.7](https://github.com/datafuselabs/databend/issues/3428)
- [Release proposal: Nightly v0.6](https://github.com/datafuselabs/databend/issues/2525)
- [Release proposal: Nightly v0.5](https://github.com/datafuselabs/databend/issues/2257)
