+++
title = "Databend 的日常维护是怎么进行的"
description = "日常维护工作虽然简单，但却是保证项目活力和竞争力的有效手段。本文将会介绍 Databend 是如何与最新的工具链/依赖关系协同的。"
draft = false
weight = 680
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "日常维护工作虽然简单，但却是保证项目活力和竞争力的有效手段。本文将会介绍 Databend 是如何与最新的工具链/依赖关系协同的。"
toc = true
top = false
giscus = true
+++

Databend 的日常维护工作主要分为两大类：一类是工具链更新，另一类则是依赖关系更新。

## 工具链更新

工具链更新会随着 Rust 版本更新进行。在新的 stable 版本发布后，Databend 就会升级到对应日期附近的 nightly 工具链。

### 必备工作

更新工具链的必要步骤是编辑 `scripts/setup/rust-toolchain.toml` 的 `channel` 字段，并提交 Pull Request 。PR 合并后，会构建新的 build-tool 镜像，以确保 Databend 的 CI workflow 可以正常运行。

### 一般步骤

在镜像构建完成后，还需要完成以下工作：

- 编辑 `rust-toolchain.toml`，确保它和 `scripts/setup/rust-toolchain.toml` 一致。
- 运行 `scripts/setup/run_build_tool.sh make lint`，确保 clippy 无警告。
    - 通常情况下，clippy 会给出一些中肯的建议，但并非完全正确。
    - 如果有必要，可以使用 `#[allow(clippy::xxx)]` 来跳过部分 clippy 规则。
    - 请保留必备的注释以解释为什么需要当漏网之鱼。
- 运行 `scripts/setup/run_build_tool.sh make test`，确保测试通过。

### 注意事项

- build-tool 的使用依赖 docker，请确保已经安装并开启 docker 服务。

## 依赖关系更新

依赖关系更新以大约 30 天一次的频率进行，需要在应用上游最新成果和维持项目稳定构建之间进行权衡。

当前 Databend 有数以千计的第三方依赖，除了 crates.io 上的依赖外，还有部分源自 github 上的某次提交或者是分叉的上游项目。

### 一般步骤

这里列出一套相对普适的更新步骤：

- 运行 `cargo upgrade --workspace` 以更新来源为 crates.io 的依赖。
    - `upgrade` 子命令依赖 [cargo-edit](https://crates.io/crates/cargo-edit)，在使用前需要安装。
- 检查源自 github 的依赖，并根据实际情况进行更新，需要固定到提交对应的 `rev` 。
    - 请尽量避免只引入 `version` 或 `branch` 字段，这不利于后续更新维护。
- 更新需要单独更新的依赖项，比如：引入代码变更、暗含版本冲突等。
    - 如果该依赖项引发大范围的代码变更，请在日常维护工作结束后再进行更新。
- 分别运行 `make lint` 和 `make test`，以确保更新顺利进行。
    - 在此过程中可能会遇到一些需要单独更新的依赖项，请将其回退到之前版本，并重新执行这一步。
    - 请尽量确保流程结束后可以通过所有检查。

### 注意事项

- 如果 `cargo upgrade --workspace` 无法更新依赖，可以尝试先执行一遍 `cargo update` 。
- 在日常维护过程中，可能需要对 `Cargo.lock` 做一些手脚，请确保一切检查都可以顺利通过，并在 PR 中进行解释。


