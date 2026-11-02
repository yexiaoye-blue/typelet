# Typelet 工作约束

<!-- codebase-memory-mcp:start -->
For structural codebase exploration, use the installed `codebase-memory` skill.
<!-- codebase-memory-mcp:end -->

## Linux 窗口点击与焦点回归：修改前必读

2026-10-30 已通过同一二进制的后端对照和最终 RPM 实测，确认本机 GNOME 的强制 XWayland 路径会出现点击失焦、窗口下沉；原生 Wayland 点击和输入正常。证据、失败尝试和最终包记录见 [docs/validation.md](docs/validation.md) 的“原生 Wayland 对照与启动后端修复”。

- 不要为了置顶在应用启动时强制设置 `GDK_BACKEND=x11`。让 GTK 选择当前桌面的后端，保留用户显式覆盖的能力。读取实际 GTK display 类型；不能仅看到 `WAYLAND_DISPLAY` 就认定程序运行在原生 Wayland。
- GTK 3 的 Wayland 后端没有实现 `set_keep_above`。通过原生能力状态禁用置顶并提示原因；不要通过切换到 XWayland、反复切换置顶或抢焦点掩盖限制。当前能力入口是 `src-tauri/src/window_platform.rs`。
- `pointerdown` 与 `click` 的聚焦时机曾被错误认定为根因。改成延迟聚焦、改回 pointerdown 以及仅验证 helper 调用均未解决此问题；不要再次把这些未经实机验证的推测称为已修复。
- 用户再次报告无法点击时，先核对运行中的二进制、安装包 payload、校验和及实际后端。没有证据，不要归因于旧包、分支基线或用户安装方式。
- 改动窗口、焦点、拖动、尺寸或启动路径后，必须用最终安装包中的程序在真实桌面测试普通窗口和小窗：开始/继续、键盘输入、连续点击内容区、切换应用后点回；再核对设置往返、拖动、最小化/恢复和置顶支持状态。保留用户数据，隔离诊断配置。
- 浏览器预览、源代码正则匹配、单测通过、构建通过、进程存活或启动时显示 focused，都不能证明桌面点击回归已解决。无法执行的验收项必须明确标为待验收，不能写成成功。
- 交付时区分包版本和 RPM release，记录最终校验和、实际运行后端及用户实测结果。此次已验收基准为 `Typelet-0.1.1-2.x86_64.rpm`；不要把更早的 `0.1.1-1` 包当作该修复。
