# AirBox 局域网快传 · 设计文档

> 日期：2026-08-26 · 状态：已批准（用户确认「开干」）

## 目标

macOS/Windows 桌面端 App：在局域网内启动 HTTP 服务，界面展示二维码（`http://192.168.x.x:port/t/<token>/`），
手机扫码后用浏览器打开网页，即可与电脑双向互传文件，无需安装手机 App、无需公网服务器。

## 架构

```
┌────────────────────────── 桌面端（Tauri 2 + Vue 3） ──────────────────────────┐
│  Rust 后端                                                                     │
│   ├─ axum HTTP 服务（监听 0.0.0.0:随机端口，启动时绑定 0 端口自动分配）          │
│   │    ├─ /t/<token>/              → 手机网页（内嵌静态 HTML，include_str!）    │
│   │    ├─ /t/<token>/api/files     → 文件列表 GET                               │
│   │    ├─ /t/<token>/api/upload    → 上传 POST（multipart 流式写盘）            │
│   │    ├─ /t/<token>/api/files/:name → 下载 GET / 删除 DELETE                   │
│   │    └─ CORS 全放开（桌面 UI 从 tauri:// 跨源访问 127.0.0.1）                  │
│   ├─ LAN IP 检测：getifaddrs 过滤（首选 192.168/16，排除 utun/awdl 等虚拟口）   │
│   └─ 配置持久化：收件目录保存到 app_config_dir/airbox.json                      │
│  Vue 桌面 UI：二维码卡片 + 复制链接 + 收件目录选择 + 文件列表 + 打开文件夹       │
└────────────────────────────────────────────────────────────────────────────────┘

手机浏览器 ──扫码──▶ http://<lan_ip>:<port>/t/<token>/  （纯静态页面，无需构建）
```

## 关键决策

1. **手机网页是独立静态 HTML**（`src-tauri/mobile/index.html`，随源码提交，Rust `include_str!` 内嵌）。
   不依赖 Vite 构建产物，`cargo test` 与发布构建零耦合；功能足够（上传/列表/下载/删除/进度）。
2. **随机 token 鉴权**：每次启动生成 8 位随机 token 拼进 URL，非 `/t/<token>/` 路径一律 404，
   只有扫码者能访问。token 通过环境变量 `AIRBOX_TOKEN` 可覆盖（供测试与高级用户）。
3. **收件目录默认** `~/Downloads/AirBox`，可用 `AIRBOX_DIR` 覆盖；桌面端可选目录并持久化。
4. **端口随机**：绑定 `0.0.0.0:0`，由系统分配，避免与既有服务冲突；`AIRBOX_PORT` 可覆盖。
5. **文件名安全**：仅取 basename，拒绝 `.`/`..`/路径分隔符；重名自动追加 ` (1)`。

## 数据流

- 启动：读配置 → 生成 token → 起服务 → 返回 `ServerInfo { url, lan_ip, port, token, dir }` 给前端。
- 桌面 UI 通过 `invoke('get_server_info')` 拿到信息，再以 `http://127.0.0.1:<port>/t/<token>/api/...` 拉取列表。
- 上传：手机 multipart 流式写 `dir/.tmp-<rand>`，完成后 rename；同名去重。
- 删除/下载：文件名 URL 编码传输，服务端解码后校验 basename。

## 测试策略

- Rust 单元/集成测试（`cargo test`，tower::ServiceExt::oneshot 驱动 axum）：
  空列表、多文件上传与内容校验、重名去重、路径穿越拒绝、下载字节一致、删除、
  错误 token 拒绝、手机页可访问。
- LAN IP 过滤逻辑用样例接口数据单测。
- E2E：`AIRBOX_PORT`/`AIRBOX_TOKEN`/`AIRBOX_DIR` 覆盖后启动真实 App，curl 走完整上传→列表→下载→删除流程。

## 范围外（YAGNI）

- 断点续传 / Range 请求
- 手机端浏览电脑任意目录（只暴露收件目录）
- 公网中继 / 账号系统 / 端到端加密
- 系统托盘常驻（关窗即退出）
