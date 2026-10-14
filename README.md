# Typelet

A tiny, cross-platform typing companion to keep your fingers moving while AI agents work.

一个小巧的跨平台桌面悬浮打字工具。等待 Agent 完成任务时，随手打一段文字，保持打字的手感。

Typelet 目前处于早期开发阶段，已实现本机文本练习。Agent 自动唤起、单词模式和内容导入是后续计划，尚未实现。

## 当前功能

- 小巧悬浮窗：默认 380 × 450，可拖动、置顶。
- 单实例：Typelet 已在运行时再次启动它，会唤起并聚焦已有窗口，不会叠加新窗口。
- 中英文文本练习：点击正文直接跟打，支持退格纠错；完成后按回车换下一段。
- 字符反馈：当前字符使用淡蓝底色和蓝色下划线，错字使用红色底块。
- 练习统计：顶部显示速度、正确率和用时，文字区下方只保留一条细进度条。
- 外观与界面语言：明暗主题、中英文界面均默认跟随系统，也可手动选择。
- 字体选择：内置 Noto Sans Mono，可搜索、预览并选择系统已安装的字体。

右上角齿轮打开设置，外观、界面语言和字体偏好保存在本机。练习语言在主页面单独选择。离开输入焦点或打开设置会暂停计时，返回后保留当前练习。

练习正文不可选中，在练习区域拖动不会产生文本选区。在练习页内点击标题栏、留白等非输入区域后，焦点会自动回到输入框；从其他应用切回窗口时同样如此，不需要额外点一下才能打字，当前位置标记保持显示。

当前练习不需要账号、模型 API 或网络服务，尚未接入 Agent 事件或 hooks。内置字体及 SIL Open Font License 位于 [public/fonts/](public/fonts/)，中文字符由系统中文字体补足。

## 后续计划

- [ ] **Agent 自动唤起**：在设置中选择要响应的 Agent，在其运行时启动或显示 Typelet。支持哪些 Agent，以及通过何种事件、hooks 或其他能力接入，待后续调研。
- [ ] **句子 / 单词两种模式**：在设置中切换练习模式。单词模式支持打字、发音和翻译显示，交互参考 [Qwerty Learner](https://qwerty.kaiyi.cool/)。发音和翻译来源尚未确定。
- [ ] **内容导入**：导入自己的句子或词库，用于相应练习模式。文件格式、数据字段和导入流程尚未确定。

这三项是产品方向，不代表当前版本已支持，也没有预先承诺具体接入协议或文件格式。详细范围与待定问题见 [后续规划](docs/roadmap.md)。

## 技术栈

- **Tauri 2**：跨平台桌面窗口与系统能力。
- **Svelte + TypeScript**：界面、样式、输入交互及设置。
- **Rust**：打字匹配、统计与系统字体读取。
- **Vite**：网页开发预览和前端构建。

桌面练习以 Rust 核心结果为准；网页开发预览使用 TypeScript 回退逻辑，便于调试界面。

## 启动

需要 Node.js 24 或更新版本、Rust stable，以及 Tauri 2 在当前系统上的 WebView 依赖。Linux 字体读取还需要 Fontconfig 和 FreeType 的开发库；网页开发预览使用 `fc-list`。

在项目目录中运行：

~~~bash
npm install
npm run dev
~~~

`npm run dev` 打开 Tauri 小窗并启动热更新页面。

仅预览网页：

~~~bash
npm run web:dev
~~~

默认预览地址为 `http://127.0.0.1:5173/`。验证：`npm test`；打包：`npm run tauri:build`。

## 跨平台

同一套 Svelte + TypeScript 界面、Rust 打字核心和 Tauri 2 桌面外壳。Tauri 使用各平台系统 WebView，并负责窗口、拖动、置顶、最小化、关闭和打包；各平台需要在对应系统构建与验证。macOS 发布签名和 Windows 签名尚未配置。

“小巧”指窗口与功能范围。Tauri 不携带完整浏览器运行时，但运行仍依赖目标系统提供的 WebView。

[产品与技术范围](docs/design.md) · [后续规划](docs/roadmap.md) · [验证记录](docs/validation.md)

## 目录

- src/：Svelte 界面、样式和中英文文案。
- src-tauri/：Tauri 桌面外壳、Rust 打字核心、窗口权限和应用图标。
- packages/：内置练习文本及网页预览使用的 TypeScript 打字逻辑。
- public/：网页图标、内置字体和字体许可证。
- tests/：TypeScript 打字逻辑测试；Rust 单测位于 src-tauri/typing-core/。
- docs/：产品与技术范围、后续规划和验证记录。

DESIGN.md 是界面的 Apple 风格设计参考。node_modules/、dist/ 和 Rust target/ 均为依赖或生成产物。
