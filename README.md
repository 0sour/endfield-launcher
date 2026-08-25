# Endfield Launcher

明日方舟：终末地（国服）Linux 启动器（非官方）

基于 [an-anime-team](https://github.com/an-anime-team) 的 sleepy-launcher 模板（relm4 + GTK4）开发，为 Linux 玩家提供终末地 PC 端的安装、更新与启动。

## ✨ 功能

- **国服支持**：官服 / B服 双渠道，一键切换（自动部署渠道 SDK 文件）
- **游戏安装/更新**：官方 CDN 下载，增量差分（hdiff）更新，AES 加密包自动解密
- **账号管理**：多账号备份/恢复（sdk_data 目录）
- **Wine 管理**：Wine/DXVK 版本下载、prefix 创建、组件管理
- **首次运行向导**：引导配置 Wine、DXVK 与游戏路径
- **游戏修复**：文件完整性校验与修复
- **背景图**：从官方 API 获取游戏背景
- **多语言**：20 种语言（Fluent）

## 📦 安装

### Arch Linux（pacman）

从 [Releases](https://github.com/0sour/endfield-launcher/releases) 下载 `.pkg.tar.zst` 包：

```bash
sudo pacman -U endfield-launcher-*.pkg.tar.zst
```

### 从源码构建

```bash
git clone https://github.com/0sour/endfield-launcher
cd endfield-launcher

# 直接构建
cargo build --release

# 或打包（注意：makepkg 需在独立目录运行，避免 src/ 冲突）
mkdir -p ~/build && cp PKGBUILD ~/build/
cd ~/build && makepkg -si
```

### 依赖

- `gtk4`、`libadwaita`（GUI）
- `p7zip`（解压加密分卷包）
- `bubblewrap`（沙箱，可选）
- `gamemode`、`gamescope`（可选增强）

## 🚀 使用

```bash
endfield-launcher
```

首次运行会显示配置向导。数据目录：`~/.local/share/endfield-launcher/`

## 🛠️ 技术栈

- Rust + [relm4](https://github.com/Relm4/Relm4)（GTK4 声明式 UI）
- [anime-game-core](https://github.com/an-anime-team/anime-game-core)（游戏核心库，含终末地模块扩展）
- [anime-launcher-sdk](https://github.com/an-anime-team/anime-launcher-sdk)（启动器 SDK，含终末地模块扩展）

## 🙏 致谢

- [an-anime-team](https://github.com/an-anime-team) — 模板与核心库
- [Xel-Launcher](https://github.com/lTinchl/Xel-Launcher) — 国服 API 逆向参考
- [Hi3Helper.Plugin.Hypergryph](https://github.com/misaka10843/Hi3Helper.Plugin.Hypergryph) — 官方协议参考

## ⚠️ 声明

本项目为**非官方**启动器，与鹰角网络（Hypergryph）无关。游戏版权归鹰角网络所有。

## 📄 许可证

[GPL-3.0](LICENSE)
