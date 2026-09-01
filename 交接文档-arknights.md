# 交接文档:终末地启动器近期修复(供 Arknights agent 对照)

> 背景:终末地(Endfield)启动器在 1.5.3 预下载期间暴露了 3 个问题,均已修复并验证。
> 本仓库 fork 自 an-anime-team 的 anime-game-core / anime-launcher-sdk,Arknights agent 可对照检查自家模块是否命中同样的问题。

---

## 问题 1:预下载永远检测不到 —— API 请求缺少 version 参数

### 现象

游戏开放预下载(官方启动器出现预下载按钮),但本项目主界面**没有预下载按钮**。

### 根因

鹰角 `get_latest_game` API 的契约:**只有当请求体里的 `version` 字段等于当前正式版时,响应才带 `pre_patch`(预下载)数据**。实测:

| 请求 version | 响应 |
|---|---|
| `""`(代码写死) | `pre_patch: null` |
| `"1.4.4"`(当前正式版) | `pre_patch: {version: "1.5.3", patches: [6 个分卷]}` |

代码写死了 `version: String::new()`,导致 `try_get_diff` 永远拿不到 `pre_patch` → 状态机永远没有 `PredownloadAvailable` → UI 按钮(可见性由 state 驱动)永远不显示。

### 修复

**`games/endfield/api/mod.rs`** — `request()` 增加 `version: String` 参数,缓存 key 改为 `(edition, version)`:

```rust
#[cached(key = "(GameEdition, String)", convert = "{ (game_edition, version.clone()) }", result)]
pub fn request(game_edition: GameEdition, version: String) -> anyhow::Result<schema::GetLatestGameRsp> {
    // ... version 填入 get_latest_game_req
}
```

**`games/endfield/game.rs` `try_get_diff()`** — 已安装时把本地版本传进去:

```rust
let requested_version = if self.is_installed() {
    self.get_version().map(|v| v.to_string()).unwrap_or_default()
} else { String::new() };
let response = api::request(self.edition, requested_version)?;
```

其他调用方(`get_latest_version`、repairer 的 `get_resource_base_url`)传 `String::new()` 即可——它们只需要正式版信息,不受影响。

### ⚠️ Arknights 自查

[`games/arknights/api/mod.rs`](anime-game-core/src/games/arknights/api/mod.rs:24) **同样写死 `version: String::new()`**。如果方舟 API 有相同的 pre_patch 契约(很可能有),方舟 agent 需要做同样的改动。

---

## 问题 2:预下载只下载第一个分卷(1024MB)

### 现象

预下载按钮出现后,点击只下载了 `.zip.001`(正好 1 GiB / 一个分卷)就提示完成。1.5.3 预下载是 6 × 1 GiB 分卷。

### 根因

`VersionDiff::download_to` 的默认实现委托给 `download_as`,而 `download_as` 只取 `uris.first()`:

```rust
Self::Predownload { uris, .. } | Self::Diff { uris, .. } => match uris.first() {
    Some(uri) => uri,   // ← 只下第一个!
    None => return Err(Self::Error::MultipleSegments)
},
```

这是上游 anime-game-core 的通用设计(原神/星铁等单 URL 更新);但终末地方舟系的更新是 **ZIP 分卷**(`.zip.001 ~ .N`),`uris` 有多个,必须全部下载。

### 修复

**`games/endfield/version_diff.rs`** — 在 `VersionDiffExt` impl 里覆盖 `download_to`,遍历全部 segments:

```rust
fn download_to(&mut self, folder: impl AsRef<Path>, progress: impl Fn(u64, u64) + Send + 'static) -> Result<(), Self::Error> {
    // 已全部下载(检查 is_downloaded)→ 直接报满进度返回
    // 遍历 uris:
    //   let mut downloader = Downloader::new(uri)?.with_continue_downloading(true);
    //   let filename = downloader.get_filename(); // 已剥离 ?auth_key
    //   已完整下载的分卷跳过(续传)
    //   downloader.download(...)  // progress 用 Arc<Mutex<F>> 跨分卷共享,进度累计
    // 最后报 (downloaded_size, downloaded_size)
}
```

配套改动:

- **`traits/version_diff.rs`**:新增 trait 方法 `is_downloaded(folder) -> bool`(默认实现检查单文件),终末地覆盖为检查**所有 segments** 都存在。UI 按钮的完成态判断从"第一个分卷存在"改为调用 `is_downloaded`。`file_name()` 一并修掉:剥离 URL query 参数 `?auth_key=...`(此前只修了 `Downloader::get_filename`,trait 这个方法漏了——**旧版本下载的文件名会带 auth_key 尾巴**)。

- **旧文件名迁移**:磁盘上已存在的 `xxx.zip.001?auth_key=...` 文件,下载前自动 rename 成干净名 `xxx.zip.001`,避免已下载的分卷被重新下载(实测 1 GiB 白下)。

### ⚠️ Arknights 自查

[`games/arknights/version_diff.rs`](anime-game-core/src/games/arknights/version_diff.rs:415) **同样只有 `uris.first()`**,方舟分卷更新会遇到一模一样的问题。

---

## 问题 3:anime-game-core 默认 feature 编译失败

### 现象

`cargo build`(不带 `--features all`)失败:

```
error[E0432]: unresolved import `md5` --> src/version_detect.rs:7:5
```

### 根因

`md-5` 在 Cargo.toml 是 optional 依赖,只在 `install` feature 下启用;但 `version_detect.rs` 无条件 `use md5`(供 `file_md5` → `get_version_game_scan` / `get_version_sophon` 使用)。主项目用 `features=["all"]` 掩盖了问题;任何外部项目按默认 feature 引用该 crate 会编译失败。

### 修复

`version_detect.rs` 给 `use md5`、`file_md5`、`get_version_game_scan`、`get_version_sophon` 加精确 cfg:

```rust
#[cfg(all(any(feature = "genshin", feature = "star-rail", feature = "honkai", feature = "zzz", feature = "sophon"), feature = "install"))]
use md5::{Digest, Md5};
// file_md5 同 cfg;get_version_game_scan 用 all(any(4个游戏feature), install);get_version_sophon 用 all(sophon, install)
```

4 个游戏调用点(`genshin/star-rail/honkai/zzz` 的 `game.rs`)在调用 `get_version_game_scan` / `get_version_sophon` 处加 `#[cfg(feature = "install")]`(md5 不可用时静默跳过该检测路径)。

### 验证矩阵

| 组合 | 修复前 | 修复后 |
|---|---|---|
| 默认(无游戏 feature) | ❌ | ✅ |
| `star-rail,install` / `genshin,install` / `sophon` / `all` | — | ✅ |
| `genshin` / `zzz` 单编 | ❌(另有上游遗留错误:版本差分 trait 未按 install gate,与本修复无关) | 错误数减少 |

> 上游遗留:genshin/zzz 单编仍各有 `version_diff.rs` 的既有错误(非 install 时实现 install-gated 的 trait 方法)、genshin 缺 `use anyhow::Context`。未修,涉及面大,留给上游。

---

## 验证方法

### API 契约(需要网络,curl 直测)

```bash
curl -s -X POST "https://launcher.hypergryph.com/api/proxy/batch_proxy" \
  -H "Content-Type: application/json" -H "User-Agent: XelLauncher/0.2.5" \
  -d '{"seq":"5","proxy_reqs":[{"kind":"get_latest_game","get_latest_game_req":{"appcode":"<APP_CODE>","launcher_appcode":"<LAUNCHER_APP_CODE>","channel":"1","sub_channel":"1","version":"<当前正式版>"}}]}' \
  | python3 -c "import json,sys; r=json.load(sys.stdin)['proxy_rsps'][0]['get_latest_game_rsp']; print('pre_patch:', r.get('pre_patch',{}).get('version') if r.get('pre_patch') else None)"
```

version 传当前正式版 → 应返回预下载版本;传空 → null。

### 分卷完整性(用户安装前自查)

官方 API 返回每个分卷的 `md5` / `package_size`,本地:

```bash
md5sum <文件> | awk '{print $1}'  # 与官方 MD5 比对
# 合并验证: cat 所有分卷 | md5sum → 得到合并后 zip 的 MD5,解压后 7z t 校验
```

### 多分卷下载(不依赖外网的单元测试)

本地起支持 Range 的 HTTP 服务器模拟分卷,验证:全部下载 / 续传跳过已完成分卷 / 旧格式文件名迁移(见 `anime-game-core/tests/multi_segment_download.rs`,3 个测试)。

---

## 提交记录(本地 main 分支)

```
70e6f7d fix: 预下载支持多分卷:下载全部 segments、断点续传、旧文件名迁移
97a2de7 feat: 请求官方 API 时携带本地版本以获取终末地预下载(pre_patch)信息
e7c635d fix: 修复 anime-game-core 在默认 feature 下因 md5 可选依赖编译失败的问题
d3bce38 chore: 忽略参考项目文件夹
```

tag `v0.1.1`(本地,未推送)。`/tmp/efl-build/endfield-launcher-0.1.1-1-x86_64.pkg.tar.zst` 为打好的 Arch 包(打包用了 `--nodeps --nocheck`:`cargo`/`rust` 是 rustup 安装,不在 pacman 数据库;hdiff 测试依赖外部工具路径,见下)。

## 环境备注(本机)

- hdiff 测试 `tests/hdiff_apply.rs` 硬编码 `/home/sour/文档/YJ-Lunch/hdiffpatch_tools/linux64/hdiffz`(不存在),该测试必失败——环境问题,非代码缺陷
- `参考/` 目录放有上游 + 逆向参考项目(Xel-Launcher、Hi3Helper.Plugin.Hypergryph 等),已加入 .gitignore
- 预下载临时目录默认 `~/.local/share/endfield-launcher/`(config.json 的 `launcher.temp`)
