# Windows 完整组件与离线预设

本分支保留上游完整功能、配置选项、平台源码和资源来源。英译中是可选新增路径；不删减拼音、双拼、注音、五笔、英文补全、表情、符号、学习、快捷键、状态条或设置程序。

## 构建与资源清单

| 组件 | 要求 |
| --- | --- |
| 中文基础词库 / 语言模型 | 使用官方锁定数据中的正式 dict.qj / lm.qj，不以 sample 词库代替 |
| 普通英文补全 | 保留正式 english.tsv、频率与来源声明 |
| 六种释义表 | 保留全部；使用当前转换器重建修改过的英汉表 |
| 领域词、辅码、笔画、五笔 | 保留正式资源及署名 |
| 离线模型 | 保留知微、通变等上游资源和加载选项 |
| 设置程序 | 保留 apps/settings 新版与 Windows legacy 设置、所需运行时 |
| Windows TSF | 保留 x64/x86 配套组件；位数和注册路径分别核验 |
| 配置 | 保留完整模板、注释与选项，英译中开关暂通过配置文件设置 |

正式数据来源由 `tools/release/data.lock` 锁定，按上游构建流程下载并验证哈希。模型、运行时或正式数据不因 Git 源码迁移而删减；原本作为发布依赖而非 Git 文件的资产仍按相同构建流程获取。本分支没有在 Git 中加入本机编译产物或安装包。

Windows legacy 设置程序的运行时清单包含 `Microsoft.Web.WebView2.Core.dll`，其余要求见 [Windows 构建说明](../../apps/windows/README.md)。构建目录不等同正式安装包；签名、Inno Setup 和 uiAccess 按上游流程处理。

## 完整离线预设

```powershell
cargo run --locked --release -p qingjian-platform --example offline_windows_config -- <new-config.toml>
```

生成器拒绝覆盖现有文件，从原生模板生成完整配置，只覆盖明确的离线设置和测试应用条目，并重新解析验证。它不启动服务或注册输入法。

`examples/english-to-chinese-windows.toml` 保留原生学习、表情、领域与离线模型等配置；云、更新检查、输入内容日志关闭，密钥环境变量名为空。原有联网、日志和自启能力仍在源码与设置中保留，不因提供此预设而默认启用。

选择排序复用上游 `record_choice`、`choice_weight` 与 `user-choices.tsv`；新英译中路径的成功提交已接入该机制，详见 [选择说明](english-translation-choices.md)。它不是另一套个人学习存储。

## 验证边界

资源哈希、模型加载、服务协议、设置离屏渲染和单元测试分别验证不同层面，不能代替真实 TSF 文本框与所有应用验收。安装升级、旧 DLL 卸载和回退需单独协调，详见 [Windows 验收规程](english-to-chinese-acceptance.md)。本页不声明任何用户当前安装、进程或个人数据状态。
