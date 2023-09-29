+++
title = "如何参与 Databend 开源协作"
description = "Databend 是一个开源的云数仓项目，这意味着你可以轻松参与“设计->研发->使用->反馈”的整个链路。这篇文章总结了参与 Databend 开源协作时需要注意的一些事项，以使贡献流程更加清晰和透明。"
draft = false
weight = 630
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "<blockquote>Databend 是一个开源的云数仓项目，这意味着你可以轻松参与“设计->研发->使用->反馈”的整个链路。这篇文章总结了参与 Databend 开源协作时需要注意的一些事项，以使贡献流程更加清晰和透明。</blockquote>"
toc = true
top = false
giscus = true
+++

在这篇文章中，主要从“沟通”和“实施”两个方面介绍 Databend 的开源协作。

## 沟通

沟通是参与开源的重要环节，正是得益于开源世界中沟通的公开与透明，才能迸发出如此生机与活力。

参与 Databend 开源协作的主要沟通方式有 Issues 、RFCs 以及 Channels 三种。  

### Issues

Issues 通常用于 Bug 反馈和新特性请求，Databend 使用 GitHub Issues 来跟踪和管理这些反馈。

<https://github.com/datafuselabs/databend/issues/new/choose>

**Bug 反馈**

Databend 提供了一个基本的 Bug 反馈模板以确保沟通的顺畅进行。

在进行 Bug 反馈之前，请检索是否存在已知的解决方案，并确定你正在运行的版本，最好包含 `commit id` 。

当然，提供清晰的问题描述和可复现步骤也是非常重要的环节。

好例子：<https://github.com/datafuselabs/databend/issues/6564>

> Databend 的迭代速度非常快，每天都会发布新的 nightly 供用户尝鲜，建议尝试新版本以确定能否复现。

**新特性请求**

对于新特性请求，请尽可能提供详细的描述或是预期的行为，如果有可以参考的文档就更好了。

好例子：https://github.com/datafuselabs/databend/issues/5979

### RFCs

对于小的功能点，打开 Issues 进行沟通就足够了。而大的功能、设计上的变动或者是需要充分讨论和同步的想法，请以 RFC 的形式提交。

在设计和沟通的早期阶段，推荐使用 Discusssions 进行讨论。

一旦确认实施和落地，提交 RFC 文档并建立用于跟踪的 Issues 则是更为合适的做法。

好例子：https://github.com/datafuselabs/databend/discussions/5438

### Channels

Channels 通常用于一般的交流和讨论，Databend 团队鼓励使用 GitHub Discusssions 进行一般问题的咨询和讨论，形成一个“知识库”，方便检索和参与。

当然，Databend 也提供 Slack 频道和微信群用于日常交流与讨论。

Slack：<https://link.databend.rs/join-slack>

## 实施

进入到实施环节，你将亲自动手改进 Databend 的代码或者文档，并踏上成为 Databend 维护者的道路，接下来让我们一起看一下有哪些环节需要重视。

### 编码

**前置环境**

在「[Databend 贡献之路 | 如何设置 Databend 开发环境](https://databend-internals.psiace.me/docs/contribute-to-databend/development-environment/)」一文中，已经详细介绍过如何配置 Databend 开发环境。

需要注意的是，考虑到不同系统和发行版之间的差异，你可能需要自行安装 `gcc`，`python` 和 `openssl` 等相关基础程序。 

**代码文档**

任何公共字段、函数和方法都应该用 [Rustdoc](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#making-useful-documentation-comments) 进行文档化。

必要的时候，请使用 [Diagon](https://arthursonzogni.com/Diagon/) 或其他 ASCII 图像生成器生成示意图以对设计进行充分描述。

下面给出一个简单的例子：

```rust
/// Represents (x, y) of a 2-dimensional grid
///
/// A line is defined by 2 instances.
/// A plane is defined by 3 instances.
#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}
```

### 文档

**前置环境**

本机上需要安装有用于 `node` 环境管理的 `nvm` ，以及用于 `node` 依赖管理的 `yarn` 。

文档的开发环节需要进入 `website` 目录后根据 `README.md` 中的描述进行配置。

通常情况下，包含以下步骤：

- 使用指定的 `node` 版本：`nvm use`
- 安装依赖：`yarn install`
- 本地预览：`yarn start`

**重要提示**

文档应当正确放置在 `docs` 目录下，请本地预览确认无误后再进行提交。

最终文档会托管到 <https://databend.rs> 。

### 准备

**License 检查**

如果引入了新文件，建议执行 License 以确认是否添加了合适的许可信息。对于非代码文件，可能需要编辑 `.licenserc.yaml` 以跳过检查。

```bash
make check-license
```

**代码风格**

请运行下述命令以完成代码风格的统一：

```bash
make lint
```

**依赖审计**

如果你引入了一些新的依赖项，可以使用：

```bash
cargo udeps --workspace
cargo audit
```

**测试**

在「[Databend 贡献之路 | 如何为 Databend 添加新的测试](https://databend-internals.psiace.me/docs/contribute-to-databend/write-and-run-tests/)」中，已经对测试做了详细的描述。

通常情况下，使用 `make test` 一次性执行 `单元测试` 和 `功能测试` 就可以。

但是，也建议执行 `集群` 相关的测试，以确保分布式执行不会出现差错。

### 拉请求（Pull Request）

**一般流程**

- 分叉 Databend 的 repo 并从 main 创建分支。
- 如果不存在跟踪的问题，请打开一个对应的问题并提供上下文
- 提交一份「草稿请求（Draft Pull Request）」，以标记你正在进行相关工作。
- 如果涉及编写代码，请添加对应的单元测试。
- 进行测试，验证并确保测试套件通过。
- 确保代码能够通过风格审计，`make lint` 。
- 将状态更改为「准备检查（Ready for review）」。
- 当心 `@mergify` 的回复，它会提供一些指导。

**PR 标题填写**

PR 标题需要符合 `<类型>(<范围>): <描述>` 的约束。

```plain text
fix(query): fix group by string bug
^--^  ^------------^
|     |
|     +-> Summary in present tense.
|
+-------> Type: feat, fix, refactor, ci, build, docs, website, chore
```

**PR 模板填写**

Databend 提供了一个基本的 PR 模板，请不要修改模板上下文，并填充对本次 PR 的总结信息，包括是否修复/修复了哪个已知的 Issue 。

```plain text
I hereby agree to the terms of the CLA available at: https://databend.rs/dev/policies/cla/

## Summary

Summary about this PR

Fixes #issue
```

好例子：https://github.com/datafuselabs/databend/pull/6665

### 持续集成

持续集成相关的文件位于 `.github` 目录中的 `actions` 和 `workflows` 目录下。

**文档**

文档相关的持续集成会通过 Vercel 进行，需要关注 `Status` ，并点击 `Visit Preview` 查看渲染情况。

**检查**

包括 License 检查、代码风格检查、依赖关系审计等内容。

- Dev Linux / check (pull_request)

**构建**

主要是测试跨平台构建，主要是针对 x86_64 和 aarch64 架构，对 Linux 的 GNU 和 MUSL 支持处于第一优先级别。MacOS 虽然标记为 optional ，但是需要尽量保证。

- Dev MacOS / build_x86_64_macos(optional)
- Dev MacOS / build_aarch64_macos(optional)
- Dev Linux / build_x86_64_gnu (pull_request)
- Dev Linux / build_aarch64_gnu (pull_request)
- Dev Linux / build_x86_64_musl (pull_request)
- Dev Linux / build_aarch64_musl (pull_request)
- Dev Linux / build_hive (pull_request)

**测试**

主要是执行各种测试确保代码和功能都符合要求，包括单元测试、功能测试、分布式测试、模糊测试等：

- Dev Linux / test_unit (pull_request)
- Dev Linux / test_metactl (pull_request)
- Dev Linux / test_compat (pull_request)
- Dev Linux / test_meta_cluster (pull_request)
- Dev Linux / test_stateless_standalone_linux (pull_request)
- Dev Linux / test_stateless_cluster_linux (pull_request)
- Dev Linux / test_sqllogic_standalone_linux (pull_request)
- Dev Linux / test_stateful_standalone_linux (pull_request)
- Dev Linux / test_fuzz_standalone_linux (pull_request)
- Dev Linux / test_stateful_hive_standalone (pull_request)
- Dev MacOS / test_stateless_cluster_macos(optional)

### 合并

有两位或两位以上维护者投下赞同票，并满足下述条件，Mergify 将会帮助我们完成代码合并工作：

- 所有测试都通过
- 所有审核意见都已经解决
- 没有代码冲突

在合并之后，你的 git name 将收集在 Databend 的 `system.contributors` 表中，在新版本 release 之后，执行 `SELECT * FROM system.contributors` 即可查看。
