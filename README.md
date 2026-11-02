# Typelet

一个小巧的桌面悬浮打字工具。等待 Agent 完成任务时，随手练一段句子或几个单词。

A small cross-platform typing window for the time you spend waiting on an agent.

## 看看界面

下面四张图来自当前版本的**网页预览**，使用隔离浏览器中的内置演示内容。网页和桌面共用界面；截图不代表已验证原生窗口置顶、拖动或系统集成能力。

| 句子练习 | 单词练习 |
| --- | --- |
| <img src="docs/images/practice.png" width="320" alt="网页预览：句子练习，显示英文原句、中文释义和本章统计"> | <img src="docs/images/word-practice.png" width="320" alt="网页预览：单词练习，显示单词、音标、释义和输入反馈"> |

- **句子与单词**：内置电影台词和 CET4_3，也可在桌面端导入自己的文章或词表。
- **即时反馈**：输入时标出正确和错误字符；显示本章 WPM、正确率和用时。
- **按章练习**：拼对自动进入下一项，章末暂停并显示成绩。可调整每章数量，也可打乱当前章的练习顺序；已有电影章节保持原划分。
- **适合工作间隙**：默认窗口为 380 × 450，支持小窗模式；失去焦点时自动暂停。

## 安装并开始

到 [Releases](https://github.com/yexiaoye-blue/typelet/releases) 选择与系统和架构对应的包。标有 Preview 的版本来自 `develop`，文件名带来源提交；正式版本由语义化版本标签发布。

| 系统 | 安装包 |
| --- | --- |
| Fedora | `.rpm`，选择 `x86_64` 或 `aarch64` |
| Debian / Ubuntu | `.deb`，选择对应架构 |
| 其他 Linux 桌面 | `.AppImage`；仍需满足运行库和桌面环境要求 |
| Windows | `.msi` 或 `-setup.exe`，选择对应架构 |
| macOS | 当前发布流程没有提供安装包，可在本机从源码构建；尚未配置发布签名 |

Fedora 可以用 DNF 安装或升级本地包。把下例的 `typelet.rpm` 替换为实际下载的文件名：

```bash
sudo dnf install ./typelet.rpm
```

升级到新版本时使用 `install` 或 `upgrade`。`reinstall` 用于重装已安装的同一版本，不能据此判断新版本已装好。

打开应用后，在设置中选择「句子」或「单词」及要练习的词本，点击开始或按任意键开始输入。拼对自动前进；本章完成后按 `Enter` 开始下一章。练习中按 `Esc` 暂停，再按任意键继续。应用再次启动时会唤起已有窗口。

## 设置和快捷键

右上角齿轮或默认快捷键 `Ctrl + ,` 打开设置；macOS 使用 `⌘ + ,`。向下滚动到「快捷键」，点击入口进入独立配置页。

| 一级设置中的快捷键入口 | 快捷键二级配置页 |
| --- | --- |
| <img src="docs/images/settings-shortcuts-entry.png" width="320" alt="网页预览：一级设置向下滚动后显示快捷键入口"> | <img src="docs/images/shortcuts.png" width="320" alt="网页预览：快捷键二级页，包含搜索、六项操作和恢复默认"> |

点击组合键后直接按下新快捷键，按 `Esc` 取消录制。支持搜索、冲突提示、单项恢复及全部恢复默认；修改立即保存到本机。返回上一级时恢复原来的滚动位置和入口焦点。

| 操作 | 默认快捷键 |
| --- | --- |
| 打开设置 | `Ctrl/⌘ + ,` |
| 进入 / 退出小窗 | `Ctrl/⌘ + Shift + M` |
| 上一项 / 下一项 | `←` / `→` |
| 上一章 / 下一章 | `Ctrl/⌘ + ←` / `Ctrl/⌘ + →` |

练习中 `Esc` 暂停、章末 `Enter` 继续为固定操作。自定义快捷键仍需遵守当前系统和输入场景的限制。

其他设置包括明暗外观、中英文界面、练习字体、章节数量与顺序、英美音、自动朗读、朗读与按键音的独立音量及音色。静音会保留原音量；朗读优先使用有道读音，失败时使用系统语音。

桌面端可开启 Codex、Cursor 提交任务时自动唤起 Typelet，并查看应用写入的 hook 配置。Codex 首次可能要求确认 hook。DeepSeek Harness 仍待确认接入方式，当前不可用。

## 导入和保存

在「设置 → 词本」里管理预置和导入内容。桌面端先预览导入结果，确认后写入本机：

- 文章：UTF-8 `.txt`，按句末标点切分。
- 词表：UTF-8 `.tsv` 或 `.txt`，以制表符分隔，必须包含 `name` 表头。

字段、示例、重复词处理和格式边界见 [导入说明](docs/importing.md)。当前不直接导入 EPUB、PDF、DOCX、HTML 或 Anki `.apkg`。

桌面端把导入内容、词本选择和练习位置保存在应用数据目录的 `content.json`；外观、声音、快捷键等偏好保存在本机 WebView 存储中，不保存打到一半的按键。Linux 默认数据目录为 `~/.local/share/dev.typelet.typing/`（遵循 `XDG_DATA_HOME`）。备份或迁移时先正常退出应用，再备份整个数据目录，而不只是 `content.json`。

## 本地运行和开发

网页预览需要 Node.js 24 或更新版本：

```bash
npm ci
npm run web:dev
```

打开 **http://127.0.0.1:6670/**。预览使用同一份内置数据的 TypeScript 回退；浏览器中的进度与桌面应用数据相互独立。文件导入、Agent 配置写入和原生窗口能力不在网页预览中模拟。

运行桌面开发版还需要 Rust stable 和对应系统的 WebView 依赖；Linux 字体读取依赖 Fontconfig / FreeType，音频构建还涉及 PipeWire 等系统库。Linux 构建依赖可参考 [CI 构建脚本](.github/scripts/build-rpm.sh) 和 [发布工作流](.github/workflows/release.yml)。

```bash
npm run dev
```

| 命令 | 用途 |
| --- | --- |
| `npm test` | Rust 练习核心及 TypeScript 测试 |
| `npm run build` | TypeScript 检查和网页生产构建 |
| `npm run tauri:build -- --bundles rpm` | 本机构建 RPM，输出到 `src-tauri/target/release/bundle/rpm/` |

桌面 UI 使用 Svelte + TypeScript，Tauri 2 提供窗口和本机能力，Rust 核心负责练习匹配、统计和内容解析。应在对应系统上构建和验收；当前 Windows 安装包未配置发布签名。

## 平台边界

Linux 保留 GTK 为当前桌面选择的后端，也遵循显式的 `GDK_BACKEND`。GNOME Wayland 使用原生 Wayland；GTK 3 在该后端不支持置顶，因此置顶按钮会禁用并说明原因。支持置顶的桌面环境中可正常使用该按钮。

网页截图、测试通过或构建成功，都不等同于各系统的真实桌面、输入法、多显示器与窗口交互验收。当前边界和已有验证范围见 [产品与技术范围](docs/design.md) 与 [验证记录](docs/validation.md)。

## 内容来源和文档

- CET4_3 来自 [kajweb/dict](https://github.com/kajweb/dict)，原始数据来自有道 / 新东方，共 2607 个词，仅用于本机练习，不原样再发布词书。
- 内置电影台词及中文释义见 [`movie_lines.json`](src-tauri/typing-core/data/movie_lines.json)，来源记录在数据文件中。
- 按键音使用 Qwerty Learner 的 kbsim 录音。

[文档索引](docs/README.md) · [导入说明](docs/importing.md) · [当前范围](docs/design.md) · [后续规划](docs/roadmap.md) · [验证记录](docs/validation.md)
