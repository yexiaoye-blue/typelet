# 后续规划

2026-10-16。句子模式、单词模式和内容导入已经完成，当前行为见 [README](../README.md) 和 [产品与技术范围](design.md)。下面只保留还没做完的方向。

## 1. DeepSeek Harness 自动唤起

Codex 和 Cursor 已经可以在设置里开关，并查看、编辑 Typelet 写入的 hook。Codex 使用 `~/.codex/config.toml` 的 UserPromptSubmit，Cursor 使用 `~/.cursor/hooks.json` 的 beforeSubmitPrompt。提交任务时运行 `typelet`，单实例负责显示已有窗口。Codex 第一次可能要求确认新 hook。

DeepSeek Harness（dsh）仍列在设置里，但稳定的插件或任务 hook 还没有确认，所以开关不可用。确认接口之后再单独适配，不假设三家使用同一种配置格式。

## 2. 还没做的练习能力

这些不在当前版本里：

- 错词本、听写隐藏和学习算法。
- 单词默写、句子默写。如果以后要加，放在现有的单词模式和句子模式下面，而不是再做一套模式切换。
- 指法练习。没有这块内容时，相关设置保持隐藏。
- EPUB、PDF、DOCX、HTML 和 Anki `.apkg` 导入。

实现记录里曾经写过「拼对后不自动跳到下一项」。后来改成了拼对自动进入下一项，一章结束才暂停。以现在的 README 为准。
