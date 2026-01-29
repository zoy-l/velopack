# Velopack 项目结构与组件指南

这份文档旨在帮助你快速了解 Velopack 仓库中除 SDK (`lib-xxx`) 以外的核心组件及其功能。

## 1. 核心二进制文件 (Rust) - `src/bins`

这是 Velopack 的底层核心，负责性能敏感和需要直接与操作系统交互的任务。

| 组件源码                            | 目标文件名   | 功能描述                                                                                                                |
| :---------------------------------- | :----------- | :---------------------------------------------------------------------------------------------------------------------- |
| [setup.rs](src/bins/src/setup.rs)   | `Setup.exe`  | **安装程序**。负责将你的项目释放到目标目录，处理初始安装逻辑。                                                          |
| [update.rs](src/bins/src/update.rs) | `Update.exe` | **更新管理器**。负责下载、解压、校验增量包以及替换旧文件。                                                              |
| [stub.rs](src/bins/src/stub.rs)     | `stub.exe`   | **应用入口代理**。它是一个非常小的程序，放在你的 App 旁边，负责正确启动你的 C# 或 Chromium 主程序并处理相关的环境变量。 |

> [!TIP]
> 当你在构建项目时，这些 Rust 程序的输出通常位于 `target/release` (或 `debug`) 目录下。

---

## 2. 打包与分发工具 (.NET) - `src/vpk`

这是你平时在命令行中使用的 `vpk` 工具的源码所在地。

- **[Velopack.Vpk](src/vpk/Velopack.Vpk)**: `vpk` 命令行工具的主入口。它负责接收参数，并调度其他后端模块。
- **[Velopack.Packaging](src/vpk/Velopack.Packaging)**: 通用的打包逻辑。
  - **[.Windows](src/vpk/Velopack.Packaging.Windows)**: 处理 Windows 上的 MSI/EXE 生成。
  - **[.Unix](src/vpk/Velopack.Packaging.Unix)**: 处理 Linux 上的 AppImage 生成。
- **[Velopack.Deployment](src/vpk/Velopack.Vpk/Deployment)**: 负责将构建好的包上传到 GitHub, S3, Azure 等云端。

---

## 3. 其他重要目录

- **[vendor](vendor)**: 包含第三方依赖工具。例如，打包 Windows 安装包所需的 `wix` 编译器以及压缩工具 `zstd` 都在这里。
- **[artwork](artwork)**: 存储 Velopack 默认的图标和图形素材。
- **[samples](samples)**: 各种语言的示例项目。你可以参考其中的 C++ 示例来了解如何对接你的 Chromium 项目。
- **[src/code-generator](src/code-generator)**: 一个 .NET 工具，用于自动生成 SDK 之间的互操作代码（确保 C# 和 C++ 的结构定义保持同步）。

---

## 总结：你的开发关注点

既然你想做 **Chromium 升级项目**，你主要会用到：

1. **`src/vpk`**: 用它来把你编译好的 Chromium 目录打包成 Velopack 安装包。
2. **`src/lib-cpp`**: (或者直接调用 Rust 定义的 C API)，在你的 Chromium 源码中集成检查更新的按钮。
3. **`src/bins` (Update.exe)**: 它是你在运行打包命令后，会被自动包含在安装包中的“搬运工”。
