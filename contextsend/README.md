# ContextSend

从市场上的开源本地 Chat AI 应用（ChatBox、Jan 等）提取对话上下文，并在局域网内向配对设备安全分享的跨平台桌面工具。

> 架构：**Adapters + Core Engine + Network layer + UI**
> 技术栈：Rust（核心）+ TypeScript / Vue3（前端）+ Tauri v2（桌面外壳）

## 当前状态：Phase 0（项目骨架）

空白带托盘的可运行桌面应用 + 完整开发工作流 + 三平台 CI/CD。业务功能（mDNS 发现、配对、加密传输、适配器解析）在后续阶段实现。


## 环境要求

- [Rust](https://rustup.rs/) 稳定版（含 `clippy`、`rustfmt` 组件）
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/) 11+（`npm install -g pnpm`）
- Linux 额外需要：`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libgtk-3-dev`

## 本地开发

```bash
cd contextsend
pnpm install          # 安装前端依赖
pnpm tauri dev        # 启动应用（修改 .vue 热重载，修改 .rs 自动重编译）
```

## 构建安装包

```bash
pnpm tauri build      # 产出当前平台安装包
# Windows: .msi / .exe(nsis)
# macOS:   .app / .dmg
# Linux:   .AppImage / .deb
```

