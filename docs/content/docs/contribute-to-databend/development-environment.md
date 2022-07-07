+++
title = "如何设置 Databend 开发环境"
description = "工欲善其事，必先利其器。在开启 Databend 贡献之旅前，一起来配置适合自己的开发环境吧。"
draft = false
weight = 610
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "工欲善其事，必先利其器。在开启 Databend 贡献之旅前，一起来配置适合自己的开发环境吧。"
toc = true
top = false
giscus = true
+++

## 快速安装

为方便开发者快速建立开发环境，Databend 维护了一个复杂的 shell 脚本，位于 `scripts/setup/dev_setup.sh` 。

只需执行一条指令即可完成开发环境配置：

```bash
$ make setup -d
```

> 注意：此过程会辅助安装部分 python 环境，可能会对本地原开发环境造成影响，建议预先执行以下命令以创建并启用专属虚拟环境。
> 
> ```bash
> $ python -m venv .databend
> $ source .databend/bin/activate
> ```

该环境主要关注构建和 lint ，测试需要的部分依赖在这里是缺失的，可以参考「**分步安装 - 测试必备**」这一部分的内容安装。

## 分步安装

这里以 `Fedora 36` 为例，考虑到不同系统和发行版之间的差异，你可能需要自行安装 `gcc`，`python` 和 `openssl` 。

### 安装 Rust toolchain

推荐使用 rustup 来管理 Rust toolchain ，参考 <https://rustup.rs/> 进行安装。

对于 MacOS 和 Linux 用户，执行：

```bash
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Databend 通常使用最新发布的 nightly 工具链进行开发，相关信息记录在 `rust-toolchain.toml` 中。

Rustup 会在使用时对工具链进行自动重载，安装时只需默认配置。

```bash
$ cargo build
info: syncing channel updates for 'nightly-2022-05-19-x86_64-unknown-linux-gnu'
info: latest update on 2022-05-19, rust version 1.63.0-nightly (cd282d7f7 2022-05-18)
```

### 安装必备依赖

以下列出了一些安装构建和测试必备依赖的关键步骤，说明及报错信息以注释形式呈现。

**构建必备**

```bash
# common-hive-meta-store 需要，thrift not found
$ sudo dnf install thrift
# openssl-sys 需要，Can't locate FindBin.pm, File/Compare.pm in @INC
$ sudo dnf install perl-FindBin perl-File-Compare
# prost-build 需要，is `cmake` not installed?
# The CMAKE_CXX_COMPILER: c++ is not a full path and was not found in the PATH.，安装 clang 时也会安装 gcc-c++ 和 llvm
$ sudo dnf install cmake clang
```

**测试必备**

```bash
# 功能测试和后续体验需要
$ sudo dnf install mysql
# 包含目前测试和 lint 需要的所有 Python 依赖
$ cd tests
$ pip install poetry
$ poetry install
$ poetry shell
# sqllogic 测试需要（包含在上面步骤中，按需选用）
$ cd logictest
$ pip install -r requirements.txt
```

**Lint 必备**

```bash
# taplo fmt 需要
$ cargo install taplo-cli
```

## 编辑器 - Visual Studio Code

![Visual Studio Code](https://code.visualstudio.com/assets/home/home-screenshot-linux-lg.png)

- 访问 <https://code.visualstudio.com/> ，安装 Visual Studio Code 。

### 插件推荐

**rust-analyzer**

- The Rust Programming Language
- Rust language support for Visual Studio Code

**CodeLLDB**

- Vadim Chugunov
- A native debugger powered by LLDB. Debug C++, Rust and other compiled languages.

**Remote - Containers**

- Microsoft
- Open any folder or repository inside a Docker container and take advantage of Visual Studio Code's full feature set.

**crates**

- Seray Uzgur
- Helps Rust developers managing dependencies with Cargo.toml. Only works with dependencies from crates.io.

### 利用 Dev Containers 开发（For Linux）

安装「Remote - Containers」插件，打开 Databend 后会看到右下角弹出窗口并提示「Reopen in Container」。

**安装 Docker**

根据 [Docker Docs - Install](https://docs.docker.com/engine/install/#server) 安装并启动对应你发行版的 docker 。

**将当前 User 添加到 'docker' group 中**

对于 Linux 用户，参考  [Docker Docs - PostInstall](https://docs.docker.com/engine/install/linux-postinstall/) 中 **Manage Docker as a non-root user** 一节配置，可能需要重启。

**其他步骤**

点击左下角「打开远程窗口」选中「Reopen in Container」即可体验。
