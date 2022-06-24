+++
title = "常见问题解答"
description = "开发和调试 Databend 时，难免会遇到一些小问题，这里列出一些解决方案供大家参考。"
draft = false
weight = 690
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "开发和调试 Databend 时，难免会遇到一些小问题，这里列出一些解决方案供大家参考。"
toc = true
top = false
giscus = true
+++

### OOM（链接时 kill -9）

下面是 16g 内存的一台设备编译 Databend 时的 Swap 情况。

```bash
$> swapon -s
Filename                                Type            Size            Used            Priority
/dev/nvme0n1p3                          partition       134217724       32632196        50
/dev/zram0                              partition       8388604         8388264         100
```

通常情况下，OOM 会在链接阶段发生。一些可能有效的解决方案包括：增加内存/Swap，更换 Linker，启用新的符号修饰方案。可以参考下面内容：

- <https://wiki.archlinux.org/title/Swap#Swap_file_creation>
- <https://psiace.me/escape-oom-at-build-time/>

### protocol_version < metasrv min-compatible

用于判断兼容性的代码会检查当前的 tag，可能是 fork 的 tags 落后于 datafuselabs/databend 。

```bash
$> git fetch git@github.com:datafuselabs/databend.git --tags
```

### protoc failed: Unknown flag: --experimental_allow_proto3_optional

`protoc` 现在可以随源码一起构建，考虑到发行版中的 `protoc` 版本不好统一，建议删除并重新构建项目源码。

### Undefined symbols "_lzma_auto_decoder"

提示需要 `lzma`，安装 `xz` 或者 `lzip` 可以解决。

```bash
Undefined symbols for architecture x86_64:
    "_lzma_auto_decoder", referenced from:
           xz2::stream::Stream::new_auto_decoder::hc1bac2a8128d00b2 in databend_query-6ac85c55ade712f3.xz2
ld: symbol(s) not found for architecture x86_64
clang: error: linker command failed with exit code 1 (use -v to see invocation)
```