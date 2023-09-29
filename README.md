<div align="center">

<h1 align="center">Databend 内幕大揭秘</h1>
<h3 align="center">与 Databend 一同探秘数据库系统</h3>
 
<h4 align="center">
  <a href="https://databend-internals.psiace.me">在线阅读</a>  |
  <a href="https://github.com/psiace/databend-internals">查看源码</a>  |
  <a href="https://github.com/datafuselabs/databend">前往 Databend</a>
</h4>

</div>

## 文档构建

本项目文档位于 [docs](./docs/) 目录下，由静态站点生成器 [Zola](https://getzola.org/) 和 [AdiDoks](https://github.com/aaranxu/adidoks) 主题强力驱动。

欲要在本地构建此项目所含文档，请参考 [Installation | Zola](https://www.getzola.org/documentation/getting-started/installation/) 预先安装 *Zola* 。

```bash
# 同步主题文件
git submodule update --init --recursive
# 进入 docs 目录
cd docs
# 构建并托管
zola serve
```

## 代码构建

本项目代码位于 [minibend](./minibend/) 目录下，由 Rust 编程语言开发。

欲要在本地构建此项目所含代码，请预先安装 *Rust 工具链* ，这里推荐使用 <https://rustup.rs/> 。

```bash
# 构建代码
cargo build
# 使用 Clippy 工具审计
cargo clippy -- -D warnings
# 使用 rustfmt 格式化
cargo fmt
```

## 许可协议

本项目的文本和代码均使用下述协议进行双重许可：

- Creative Commons Attribution 4.0 International ([LICENSE-CC-BY](./LICENSE-CC-BY) 或 https://creativecommons.org/licenses/by/4.0)
- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE) 或 http://apache.org/licenses/LICENSE-2.0)

任何人均可根据任一或二者的条款，自由重用本项目中的任何材料。
