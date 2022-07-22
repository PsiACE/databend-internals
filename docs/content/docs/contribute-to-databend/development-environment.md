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

如果遇到依赖缺失问题，可以参考「**分步安装 - 测试必备**」这一部分的内容安装。

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
# 包含目前功能测试和 lint 需要的所有 Python 依赖
$ cd tests
$ pip install poetry
$ poetry install
$ poetry shell
# sqllogic 测试需要（包含在上面步骤中，按需选用）
(tests) $ cd logictest
$ pip install -r requirements.txt
# fuzz 测试需要
(tests) $ cd fuzz
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

- 作者：*The Rust Programming Language*
- 为 Visual Studio Code 提供 Rust 语言支持。

**crates**

- 作者：*Seray Uzgur*
- 帮助 Rust 开发者管理 Cargo.toml 中的依赖。仅支持来源为 crates.io 的依赖。

**CodeLLDB**

- 作者：*Vadim Chugunov*
- 由 LLDB 驱动的原生调试工具。支持调试 C++ 、Rust 和其他编译语言。

**Remote - Containers**

- 作者：*Microsoft*
- 在 Docker 容器内打开任何文件夹或 Repo ，并利用 Visual Studio Code 的全部功能。

### 利用 Dev Containers 开发（For Linux）

安装「Remote - Containers」插件，打开 Databend 后会看到右下角弹出窗口并提示「Reopen in Container」。

**安装 Docker**

根据 [Docker Docs - Install](https://docs.docker.com/engine/install/#server) 安装并启动对应你发行版的 docker 。

以 `Fedora 36` 为例，步骤如下：

```bash
# 移除旧版本 docker
$ sudo dnf remove docker \
                  docker-client \
                  docker-client-latest \
                  docker-common \
                  docker-latest \
                  docker-latest-logrotate \
                  docker-logrotate \
                  docker-selinux \
                  docker-engine-selinux \
                  docker-engine
# 设置存储库
$ sudo dnf -y install dnf-plugins-core
$ sudo dnf config-manager \
    --add-repo \
    https://download.docker.com/linux/fedora/docker-ce.repo
# 安装 Docker Engine
$ sudo dnf install docker-ce docker-ce-cli containerd.io docker-compose-plugin
```

**将当前 User 添加到 'docker' group 中**

参考  [Docker Docs - PostInstall](https://docs.docker.com/engine/install/linux-postinstall/) 中 **Manage Docker as a non-root user** 一节配置，可能需要重启。

步骤如下：

```bash
# 添加 docker 用户组
$ sudo groupadd docker
# 将用户添加到 docker 这个组中
$ sudo usermod -aG docker $USER
# 激活更改
$ newgrp docker
# 更改权限以修复 permission denied
$ sudo chown "$USER":"$USER" /home/"$USER"/.docker -R
$ sudo chmod g+rwx "$HOME/.docker" -R
```

**其他步骤**

启用 Docker ：

```bash
$ sudo systemctl start docker
```

点击左下角「打开远程窗口」选中「Reopen in Container」即可体验。

## 其他实用工具推荐

这里列出一些可能有助于 Databend 开发的实用工具，根据实际情况按需选用。

### starship

轻量级、反应迅速、可无限定制的高颜值终端！

- <https://github.com/starship/starship>

![starship](https://raw.githubusercontent.com/starship/starship/master/media/demo.gif)

参考 [starship - installation](https://github.com/starship/starship#-installation) 进行安装。

```bash
curl -sS https://starship.rs/install.sh | sh
```

### hyperfine

命令行基准测试工具。

- <https://github.com/sharkdp/hyperfine>

![hyperfine](https://camo.githubusercontent.com/88a0cb35f42e02e28b0433d4b5e0029e52e723d8feb8df753e1ed06a5161db56/68747470733a2f2f692e696d6775722e636f6d2f7a31394f5978452e676966)

参考 [hyperfine - installation](https://github.com/sharkdp/hyperfine#installation) 进行安装。

```bash
cargo install hyperfine
```