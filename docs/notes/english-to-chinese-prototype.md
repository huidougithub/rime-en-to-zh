# 可选英译中：实现与验证

本分支基于官方青简 `db5d6fe325580fe49adf30b7bc8a7a786aafd4f3`。来源与许可见 [FORK-NOTICE.md](../../FORK-NOTICE.md)。英译中是新增的可选路径，不替代原有输入方案或资源。

## 行为

Windows 配置 `[general] english_to_chinese = true`，缺省关闭，设置页暂未添加该开关。它沿用青简内部英文模式，并受原生英文候选开关及应用规则控制；中文拼音、双拼与其他模式保持原逻辑。

对完整当前输入作 ASCII 大小写折叠，再精确查原生 `Glossary`。每个不同中文译词成为独立主候选，逐义保留词性，原文候选始终最后。辅助注记只显示一次英文来源；原有 `translation_pos` 开关控制词性显示。

Space 提交高亮候选，不追加英文词间空格；Enter 保留实际原文。含空格短语用半角双引号包围，如 `"set a record"`；引号内空格留在组词中，闭合后才能查询短语。未知词、未闭合引号、错拼及不完整前缀只有原文路径，不用补全或纠错词的译义冒充当前输入。

成功提交的中文译词复用原有选词记录排序，不建立新存储。只重排当前词表已有义项；失败、取消、Enter 原文和私密输入不增加译词计数。详见 [选词接入](english-translation-choices.md)。

## 实现

- Core 的 `engine/query/translation.rs` 负责完整词条查询、去重和稳定排序；`CandidateKind::Translation` 保留来源与词性。
- `engine/commit/translation.rs` 校验当前英文及释义成员，复用原生提交和组词消耗。
- Windows Router 只处理配置及字符事件；TSF 在实际文档写入成功后返回提交确认。Server、DLL 的协议 11 需配套使用。
- TSV、内存和 `.qj` 保留全部有效义项；v1/v2 兼容规则及容量边界见 [多义项说明](multiple-senses.md)。
- Windows TSF 和相关 Server 正常诊断不记录按键、组词或提交正文；上游显式输入日志与详细调试选项保留。

## 词表范围

上游英汉源表为 44,924 条，本分支为 44,931 条。局部修改与来源逐条记录于 [glossary_changes.json](../../tests/english_to_chinese/glossary_changes.json)。包括 bank 河岸义、right 词性对应、tear 眼泪义词性、may 的可能/许可/月名三义，以及七个短语。

[固定语义样例](../../tests/english_to_chinese/cases.tsv) 记录预期、禁止错译与已知漏义。大小写仍折叠，未做 Apple 等专名消歧；保留全部已收录义项不等于补齐全词典。生成提示仍建议常用 1–2 义，解析和保存不再截断返回的有效义项。本分支没有自动生成全库新释义。

## 可复现检查

```powershell
python tools/qa/english_to_chinese.py --report target/english-to-chinese/data-audit.json
cargo fmt --all -- --check
cargo test --locked -p qingjian-core -p qingjian-translate -p qingjian-platform -p qingjian-format
cargo test --locked -p qingjian-windows-server -p qingjian-windows-tsf
```

Windows 包名及完整构建步骤以 [Windows 构建说明](../../apps/windows/README.md) 和 `tools/qa/build-english-to-chinese.ps1` 为准。脚本可用 `-ToolchainRoot` / `-BuildRoot` 指定自己的工具与缓存目录，只影响脚本进程环境，不安装工具、不注册输入法或启动 Server。

自动验证覆盖精确词义、Space/Enter、短语限定、未知原文、词性开关、来源去重、选择确认与持久化、多义 TSV/.qj/候选往返及格式边界。已在 Windows 执行相关 Rust 测试、fmt、严格 Clippy 及 x64/x86 相关构建；公共源码保留测试入口，不发布私人运行记录。

自动测试不替代真实 Windows 文本框、所有应用、不同 DPI、权限桌面、键盘布局或跨平台验收。完整资源检查见 [组件说明](windows-full-restoration.md)，手工步骤见 [验收规程](english-to-chinese-acceptance.md)。
