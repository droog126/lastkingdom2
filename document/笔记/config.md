
```angular2html
$env:RUSTUP_DIST_SERVER = "https://rsproxy.cn"
$env:RUSTUP_UPDATE_ROOT = "https://rsproxy.cn/rustup"

```

```angular2html
[source.crates-io]
replace-with = 'mirror'

[source.mirror]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"

```



打开环境变量设置：
按 Win + S，搜索 “编辑系统环境变量” 或 “环境变量”，点击打开。

或者：右键 “此电脑” > “属性” > “高级系统设置” > “环境变量”。

添加用户变量：
在 “用户变量” 区域，点击 “新建”：
变量名：RUSTUP_DIST_SERVER

变量值：https://rsproxy.cn

再点击 “新建”：
变量名：RUSTUP_UPDATE_ROOT

变量值：https://rsproxy.cn/rustup



Visual Studio 2022 生成工具
