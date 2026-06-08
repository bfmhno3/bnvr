# BNVR 系统设计方案

## 核心定位

**bnvr** (*BNVR is Not Verge Rev*) 是一个专为键盘流与硬核开发者设计的 All-in-One 终端网络瑞士军刀 。它剥离了传统 GUI 的肥胖层（如大内存占用的 Webview 视图），采用纯 Rust 构建高性能 TUI 与后台守护进程 ，并将网络流的清洗与高级调度控制权通过标准的 Python 模块彻底交还给用户 。

## 一、 统一目录结构与技术栈

为了实现彻底的前后端分离，项目在源码层级将 Client（TUI/CLI 表现层）与 Server（Daemon 内核调度层）解耦 。

### 1.1 推荐目录布局

```
bnvr/
├── Cargo.toml             # 核心依赖: clap, ratatui, crossterm, tokio, reqwest, sled, serde
├── .mise.toml             # 锁定局部开发环境
├── ext_scripts/           # 默认提供的全局 Python 核心拦截脚本地
└── src/
    ├── main.rs            # 统一入口：解析全局 2 级参数，分流至 Client 或 Server 逻辑
    ├── cli.rs             # 2 级全功能命令行矩阵定义 (基于 clap)
    ├── daemon/            # ⚙️ Server 内核守护层 (核心常驻进程)
    │   ├── mod.rs
    │   ├── core.rs        # Sidecar 进程生命周期管理 (Mihomo 监控进程树)
    │   ├── network.rs     # TUN 网卡接管、路由表下发、防止路由环路炸弹
    │   ├── db.rs          # Sled 嵌入式键值数据库 (审计日志与历史流量统计)
    │   └── py_bridge.rs   # 🐍 Python 胶水层管道 (安全、带超时的 stdin/stdout 通信)
    └── tui/               # 📺 Client 表现层 (无状态轻量附着端)
        ├── mod.rs
        ├── app.rs         # 内存状态机与网络数据帧解析
        ├── event.rs       # 键盘流捕获 (支持 vim 方向键 j/k)
        └── view.rs        # UI 像素级绘制 (Layout 布局、网速曲线、节点卡片矩阵)
```

### 1.2 核心技术栈

- **CLI 解析器**：`clap`（基于 Derive 宏模式，实现规范、严谨的二级子命令矩阵）。
- **TUI 表现层**：`ratatui` + `crossterm`（提供跨平台终端像素级高频帧渲染与原生的全键盘事件劫持）。
- **异步运行时**：`tokio`（单进程多任务调度的绝对基石：处理高并发拨测、内核日志流监听、Socket 数据推送）。
- **持久化存储**：`sled` 或 `sqlite3`（轻量级嵌入式单文件数据库，存储高频网络审计元数据）。
- **日志系统**：`tracing` + `tracing-subscriber`（天生支持异步追踪与结构化键值日志，提供卓越的工业级可观测性）。
- **进程间通信（IPC）**：跨平台使用 Unix Domain Sockets（Linux）及 Named Pipes（Windows）传输轻量 JSON 。

## 二、 核心机制：TUI 的挂起与恢复 (Attach & Detach)

在守护进程架构中，TUI 彻底降级为一个**无状态的“遥控器显示屏”** 。

```
[用户终端] 运行 bnvr ──> [Client: TUI] (开启 Raw Mode / 备用屏幕)
                              │  ▲
           (Unix Socket/管道) │  │ (实时推送网速、日志帧)
                              ▼  │
                     [Server: bnvr daemon] ───控制───> [Mihomo 内核]
```

1. **后台常驻 (Daemon)**：真正的核心网络接管、Mihomo 进程树、Sled 数据库读写、Python 自动化钩子，全部由 `bnvr daemon start` 在后台的 Tokio 运行时中不间断调度 。
2. **TUI 唤醒 (Attach)**：当用户在终端输入 `bnvr` 时，启动一个短暂的客户端进程 ：
   - 开启 `crossterm` 的 Raw Mode（接管键盘流）并切换到 Alternate Screen（备用屏幕，防止污染用户终端历史记录）。
   - 通过 Socket 附着到后台的 Daemon 上，拉取当前的节点状态、流量数据进行帧渲染 。
3. **TUI 退出 (Detach/Pause)**：当用户在 TUI 中按下 `q` 键退出时 ：
   - 向 Daemon 发送断开信号，Daemon 随即停止向该客户端推送流量日志（节省内核 CPU 开销）。
   - 客户端安全执行 `disable_raw_mode()` 和 `LeaveAlternateScreen`，完美复原终端控制权 。
   - **此时，网络代理、路由劫持、底层的 Python 断网自愈逻辑在后台不受任何影响，继续无缝运行** 。

## 三、 2 级全功能命令行矩阵 (CLI Commands)

利用 一级命令（领域分类） + 二级命令（核心动作） 的严格范式，使用户完全脱离交互界面也能在后台完成所有高级网络调度 。

### 1. 基础与服务生命周期管理

- `bnvr`（无参数）：尝试连接后台 Daemon 并渲染 TUI。若检测到 Daemon 未启动，则自动将其拉起后再 attach 进去 。
- `bnvr tldr`：快速在当前终端打印常见命令的极简备忘录，供健忘时速查 。
- `bnvr daemon <start / stop / status>`：独立操纵后台核心守护进程的启停与健康监测 。

### 2. bnvr kernel（内核版本与生命周期管理）

- `bnvr kernel list`：查看本地已装内核与远端 GitHub Releases 官方最新版本 。
- `bnvr kernel install`：自动识别当前的系统架构，从远端静默拉取并解压 Mihomo 内核二进制文件 。
- `bnvr kernel use`：一键热重载切换当前激活的内核版本，无需重启整个守护进程 。
- `bnvr kernel status`：查看内核运行状态、物理 PID 及瞬时系统资源占用 。

### 3. bnvr profile（多订阅矩阵配置管理）

- `bnvr profile <add / del / list>`：订阅源的基础增删改查 。
- `bnvr profile sync [name]`：静默拉取指定源，自动格式化并拦截，触发 Python 覆写清洗，最终覆盖内核配置 。
- `bnvr profile merge <sub_a> <sub_b>... --out`：级联交织合并多个订阅，消除重名节点，一键生成超大临时节点池供后置清洗 。
- `bnvr profile view [json_path]`：支持点对点路径导航，免打开直接快速查看长配置文件中的某一段（如：`proxies.0`） 。
- `bnvr profile diff`：对比原始服务商拉取的 Raw YAML 与经过 Python 洗节点处理后的 Target YAML 的属性行差异（支持终端红绿高亮） 。

### 4. bnvr overwrite（Python 覆写插件管理与 Git 缝合）

- `bnvr overwrite init <module_name>`：**【高阶环境隔离】** 新增模块时，Rust 自动调用 `uv venv` 在该插件目录下生成**独立且完全隔离的 Python 虚拟环境**，并生成 `requirements.txt` 。彻底杜绝用户的第三方库污染系统级环境 。
- `bnvr overwrite <list / use>`：管理与激活不同的 Python 策略插件 。
- `bnvr overwrite git <args...>`：**【极客透传机制】** 自动切入该模块的物理 `.git` 目录，将后续所有参数原地透传给系统的 Git 工具（例如：运行 `bnvr overwrite git pull` 即可无感拉取开源社区中别人的高级清洗规则仓库） 。

### 5. bnvr network（网络层与系统级接管）

- `bnvr network tun setup`：自动处理跨平台优雅提权（Linux/macOS 调用 `sudo -k` 或检测 `setcap`，Windows 申请 UAC 提权），为操作系统创建虚拟 TUN 网卡，强行接管整机所有终端及全局流量 。
- `bnvr network tun clear`：优雅还原系统路由表与 DNS 转发，在内核被杀或遭遇 panic 崩溃时充当看门狗防御线，杜绝断网遗留 。
- `bnvr network bypass <ip/cidr>`：运行时动态追加物理层直连路由，实现高阶的手动直连流量分流 。

### 6. 拨测、审计与路由诊断

- `bnvr bench [group]`：由 Rust 后端发起真正的多线程网络拨测。不仅测试 HTTP 响应，还强制审计 TCP+TLS 握手时延与 Jitter（延迟抖动率），检测结果作为元数据写入 Sled 数据库，供 Python 过滤脚本做排序依据，彻底粉碎欺诈节点 。
- `bnvr stats <top / summary / nodes>`：历史流量审计。按时间线排行消耗流量最高的域名前十名，或在交互终端绘制过去一周的直连/代理流量趋势图表 。
- `bnvr query rule <domain/ip>`：实时查询指定域名在当前分流规则下的匹配路径（例：`Domain ➔ Match ➔ Proxy ➔ 🇭🇰香港01`）。
- `bnvr query dns`：直接调用 Mihomo 内置的 DNS 引擎，打印其代理环境下的真实解析 IP、TTL 以及上游 DNS 源 。

## 四、 Python 插件规范与 IPC 防挂死安全熔断

为了实现对用户环境的“零依赖”污染，整个数据管道的设计精妙地融合了格式转换 ：

### 4.1 无感数据管道设计

1. **瞒天过海的格式对齐**：Mihomo 的订阅虽然 100% 是 YAML 格式，但 Python 标准库不自带 YAML 解析器（需要用户安装 `pyyaml`，易引发环境崩溃） 。
2. **零依赖处理**：Rust 后端在抓取到原始 YAML 后，在内存中将其转换为 **JSON 字符串** 。通过 `std::process::Command` 调起该模块对应的 `.venv/bin/python`，将 JSON 通过标准输入（stdin）灌入 。
3. **用户层调用**：Python 脚本利用原生的 `sys.stdin.read()` 接收解析，用户可以直接用最纯粹的字典推导式（List/Dict Comprehension）完成高效清洗，处理完成后通过标准输出（stdout）把 JSON 吐回给 Rust 。

> **四类全生命周期钩子规范：** 用户自定义的插件必须暴露一个标准的 `overwrite` 包名，且 `__init__.py` 必须实现或选择性覆盖以下四类全生命周期的钩子函数 ： * `preprocess(config: dict) -> dict`：前置配置钩子。订阅刚下载、内核尚未解析时触发 。 * `postprocess(config: dict) -> dict`：后置裁剪钩子。节点池拼装完毕后，利用 Python 字典推导式高效过滤垃圾节点 。 * `on_node_switch(old_node: dict, new_node: dict)`：事件响应。当用户在 TUI 界面手动切节点、或自动化测速切节点时异步触发（例如可在这里调用系统通知组件） 。 * `on_network_dropped()`：异常感知。当检测到当前激活节点彻底断网，或延迟连续多次炸裂时触发（可用于驱动自愈脚本） 。

### 4.2 Rust 侧 IPC Timeout 熔断保护

```rust
// 伪代码逻辑：py_ext/bridge.rs
let mut child = Command::new(venv_python_path)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;

// 限制 postprocess 必须在 3 秒内返回，否则强制熔断
match tokio::time::timeout(Duration::from_secs(3), child.wait_with_output()).await {
    Ok(Ok(output)) => { /* 解析 stdout 并写入 config.yaml */ },
    _ => {
        // 触发熔断保护：kill 掉该死循环或超时的 Python 进程
        child.kill().await?;
        tracing::warn!("Python 脚本执行超时或死循环，已被强行拔管！系统回退至原始配置。");
        // 回退安全策略，保证守护进程与 TUI 绝不卡死
    }
}
```

所有通过 stdin/stdout 调用的 Python 钩子（如 `postprocess` 洗节点），在 Rust 端均被 `tokio::time::timeout` 严格限制（设为 3 秒） 。如果 Python 脚本由于死循环、或者外部网络请求阻塞在 3 秒内未返回，Rust 主进程直接“强行拔管熔断” 。系统自动回退到未处理的原始配置，并在 Sled 数据库中记入一条警告日志，**确保守护进程与 TUI 永远不会发生永久性死锁或卡顿** 。