# KPL SAB 赛制 / KPL SAB League Format

《Teamfight Manager 2》的 KPL（王者荣耀职业联赛）mod，包含三个包：

| 包 | 类型 | 内容 |
| --- | --- | --- |
| `kpl_sab` | 原生 DLL（stable mod SDK） | S/A/B 分组赛制、每档开始随机抽签、每轮清零重分组、统一双败季后赛、最终名次与新闻、国际杯席位修正；中国赛区统一 KPL 风格中文名（持续查重） |
| `kpl_real` | 数据包 | 中国赛区 → KPL 18 支真队 + K 甲；其他赛区 → 真实 LoL 战队；选队列表按实力排序、官方队徽的 64×64 像素风版本、中文 UI |
| `kpl_scout` | 原生 DLL（stable mod SDK） | 替代失效的 Chief Scout：底部栏「Chief Scout」按钮 + 找球员面板（分路评分、潜力/年龄/升级筛选、Solo Rank 资料卡含英雄头像） |

`kpl_sab` 与 `kpl_real` 建议同时启用。只装 `kpl_real` 是换皮；只装 `kpl_sab` 会得到 KPL 赛制但队名仍是原版。`kpl_scout` 可单独使用。

## Steam 创意工坊

- KPL SAB 赛制（原生 mod）：https://steamcommunity.com/sharedfiles/filedetails/?id=3822163110
- KPL + Real LoL Teams（数据包）：https://steamcommunity.com/sharedfiles/filedetails/?id=3822160023
- Chief Scout 找球员（原生 mod）：https://steamcommunity.com/sharedfiles/filedetails/?id=3822204418

## 赛制

- **开档选队按实力排序**：中国赛区每个联赛里 6 支真 KPL 队按五个位置最强评分之和从高到低排在前面，挂名队排在后面，选队界面一眼能看出强弱。
- **每个赛段开局随机抽签**：春季赛在开档时、夏季赛在 6/22 转会期开启时，把手里的 18 支真队重新随机分成三组（每组 6 队 + 4 支挂名队），并向 18 支真队推送「KPL 春季赛/夏季赛第一轮抽签」新闻。抽签只做一次，第一轮有比赛开打后不会重抽。
- **命名规则**：第一轮显示「KPL 一组 / 二组 / 三组」，第二轮起显示「KPL S组 / A组 / B组」，季后赛统一显示「KPL 季后赛」。只改显示层文本，联赛 ID、赛区、积分都不动。
- **S/A/B 三组**，每组 6 队；第一轮按抽签结果分组，之后按每轮名次重排。
- **每个单循环 15 场**（5 轮 × 3 场，圆桌法），一组打 **3 个单循环 = 45 场**。
- **每轮结束积分清零并重新分组**：第 1 轮打完记名次 → 清零 → 按名次重排 S/A/B → 第 2 轮……第 3 轮结束的名次才是赛季名次。
- **统一季后赛**：S + A 共 12 队，跨两个比赛容器打 **16 场 BO7 双败淘汰**（总决赛、3-4 名赛、5-6 名赛），B 组第二轮后淘汰、不再打自己的季后赛。
- **7-10 名排位赛**：两场 BO7 决定 7-10 名，最终 1-10 名全部由真实比赛结果决定。
- **官方名次新闻**：季后赛结束后向全部 18 支 KPL 队推送「KPL 最终名次」。
- **国际杯**：抽签后会修正参赛席位——把挂名队（TCG/ZQ/DSM 等）和重复出现的真队换成没进杯赛的真 KPL 队，只改杯赛记录，不影响积分与升降级。
- **像素风队徽**：18 支 KPL 真队换成官方队徽重绘的 64×64 像素版（KLG、情久用 K 甲官方队徽）；挂名队保留原来的字母块。

## 安装

Steam 创意工坊订阅后由游戏自动安装。手动安装：

1. 把 `kpl_sab/` 和 `kpl_real/` 放进游戏的 `mods/` 目录；
2. `kpl_sab/` 里需要有 `kpl_sab.dll` 和 `mod.mod_info`，`kpl_real/` 里是数据文件和 `mod.mod_info`；
3. 启动游戏，在 Mod 列表里启用两个包。

## 从源码构建

`kpl_sab` 是 stable-ABI 原生 mod，依赖游戏自带的 `mod-api-stable`：

```
Teamfight Manager 2/
└── mod-sdk-stable/
    ├── mod-api-stable/     <- 游戏自带
    ├── kpl_sab/            <- 本仓库的 kpl_sab 目录放到这里
    └── kpl_scout/          <- kpl_scout 同理
```

```sh
cargo build --release
cp target/release/kpl_sab.dll ../../mods/kpl_sab/kpl_sab.dll
```

也可以直接用游戏目录里的 `TFM2ModUploader.exe`：选择 `kpl_sab` 文件夹，
它会自动找到 `mod-api-stable`、构建并上传创意工坊。

## 兼容性

- 基于游戏 base 版本 **0.7.0**，stable ABI（游戏更新后一般不需要重编）。
- 只改存档内的联赛/比赛/新闻记录，不改可执行文件。

## 许可

源代码以 MIT 许可发布（见 `LICENSE`）。游戏数据、战队名称与队徽归各自权利人所有，
本仓库仅以 mod 形式分发修改后的游戏数据。
