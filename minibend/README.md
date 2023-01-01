# minibend

minibend 是一款从零开始、使用 Rust 构建的查询引擎。

查询引擎是数据库系统的一个重要组件，需要具备以下几点能力：

- 访问数据
- 提供查询接口
- 返回查询结果

通常我们会使用 SQL 也就是结构化查询语言进行交互。

minibend 同时也是 *Databend Internals*，或者说 *Databend 内幕大揭秘* 这个手册的实战部分。*Databend 内幕大揭秘* 将会透过 Databend 的设计与实现，为你揭开面向云架构的现代数据库的面纱。

## 致谢

- [*How Query Engines Work*](https://leanpub.com/how-query-engines-work) ，Andy 的大作，也是这一系列的重要参考和基础。
- [Veeupup/naive-query-engine](https://github.com/Veeupup/naive-query-engine), Veeupup 的查询引擎实现。
- [timvw/simple-query-engine](https://github.com/timvw/simple-query-engine), Tim Van Wassenhove 的查询引擎实现。