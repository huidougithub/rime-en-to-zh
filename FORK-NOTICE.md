# 青简英译中分支说明

此仓库现为基于青简的英译中开发分支，不是官方发行版。仓库沿用 `huidougithub/rime-en-to-zh` 名称；当前源码属于青简项目，不是 Rime 扩展。

- 官方上游：<https://github.com/qingjian-team/qingjian>
- 上游基线：`db5d6fe325580fe49adf30b7bc8a7a786aafd4f3`
- 完整保留该基线及其祖先的 Git 历史、许可证与资产来源。分支修改由 Codex AI 协助完成，日期为 2026-10-21 至 2026-10-22。
- 新增可选 Windows 精确英译中、成功上屏后的个人译词排序接入、逐义词性与来源显示、多义项保留和兼容检查，并修正 Windows 诊断日志中的输入正文记录。
- 保留上游全部平台源码、功能、配置选项和资产来源；离线配置是可选预设，不删除联网等原有能力。

分支代码及英汉表局部修订按 GPL-3.0-or-later，全文见 [LICENSE](LICENSE)。英汉表为有限、含 LLM 生成内容的资源；局部修订不代表全库经过人工审校或具有已测准确率。

资产保留各自许可，不以 GPL 统一替代：

- 英汉表及修订来源：[assets/glossary/README.md](assets/glossary/README.md)、[修订清单](tests/english_to_chinese/glossary_changes.json)。
- 英文字表：ESDB 的 MIT-like / WordNet 声明、CSpell 的 MIT、typos 的 MIT，见 [英文来源文件](assets/lexicon/05_english/sources/)。
- 中文词表、领域词与 THUOCL 的 MIT 声明：[字词库来源](assets/lexicon/README.md)。
- 其他资产以相邻 README / LICENSE 为准；渲染器及 bamboo-slip 的双许可文件原样保留。
- 上游名称与 logo 的授权范围以原有声明为准，不推定商标或商业使用许可。

本仓库不发布本机配置、学习数据、输入日志或私人部署证据。安装包、模型与正式数据的构建来源仍按上游构建说明处理；没有因源码迁移删除这些能力。

入口：[英译中实现](docs/notes/english-to-chinese-prototype.md)、[多义项兼容](docs/notes/multiple-senses.md)、[Windows 验收步骤](docs/notes/english-to-chinese-acceptance.md)。
