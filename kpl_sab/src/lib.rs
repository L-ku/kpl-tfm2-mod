//! KPL SAB mod — phase 1: per-region player naming + ages + ability, TEAM-AWARE.
//!
//! Renames China-region athletes to real KPL names matched to their TEAM +
//! LANE, and sets ability stats. Only the listed starters/rotation get KPL
//! names — free agents, K甲 teams, and extra bench athletes keep their default
//! names (avoids duplicates / wrong-team assignments).

use mod_api_stable::{
    declare_stable_mod, LogLevel, ManagementEventKindV1, RecordKindV1, StableHost, StableMod,
    StableServerCtx, StableServerExtension,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

static START_DONE: AtomicBool = AtomicBool::new(false);
static CURSOR: AtomicU64 = AtomicU64::new(0);

// (name, age, ovr)
type P = (&'static str, i64, i64);

fn roster_for(team: &str) -> Option<[&'static [P]; 5]> {
    match team {
        "成都AG超玩会" => Some([
            &[("轩染", 22, 86)],
            &[("钟意", 22, 82), ("染详", 21, 72)],
            &[("长生", 22, 87)],
            &[("小麦", 21, 72), ("一诺", 25, 89)],
            &[("大帅", 22, 85)],
        ]),
        "重庆狼队" => Some([
            &[("清清", 24, 85)],
            &[("皖皖", 21, 81)],
            &[("紫幻", 20, 82)],
            &[("道崽", 20, 82)],
            &[("信", 21, 80)],
        ]),
        "武汉eStarPro" => Some([
            &[("海棠", 21, 80)],
            &[("小楼", 20, 80)],
            &[("Ming", 21, 80)],
            &[("鸣鸣", 20, 80)],
            &[("小A", 21, 80)],
        ]),
        "北京WB" => Some([
            &[("樱花", 20, 79), ("梓墨", 23, 72)],
            &[("安凉", 21, 80), ("暖阳", 24, 82)],
            &[("果冻", 20, 80), ("听悦", 21, 72)],
            &[("昭珏", 20, 80)],
            &[("一笙", 21, 84)],
        ]),
        "广州TTG" => Some([
            &[("小泽", 21, 80), ("萝卜", 22, 89)],
            &[("小胖", 21, 87)],
            &[("鹤辞", 20, 83)],
            &[("小雪", 20, 87)],
            &[("涛", 21, 80), ("涵", 20, 72)],
        ]),
        "佛山DRG" => Some([
            &[("花缘", 21, 80)],
            &[("小小阳", 20, 80)],
            &[("早点", 21, 80)],
            &[("梦岚", 22, 82)],
            &[("子阳", 23, 82)],
        ]),
        "苏州KSG" => Some([
            &[("无言", 21, 80)],
            &[("句号", 20, 80)],
            &[("流浪", 20, 82)],
            &[("小屿", 21, 80)],
            &[("晚星", 22, 80), ("烈峰峰", 20, 72)],
        ]),
        "济南RW侠" => Some([
            &[("归期", 22, 82)],
            &[("今屿", 23, 82)],
            &[("挽墨", 21, 80)],
            &[("小俞", 21, 86)],
            &[("梦溪", 20, 80)],
        ]),
        "上海EDG.M" => Some([
            &[("流星", 21, 80)],
            &[("夏凌", 21, 80)],
            &[("花卷", 25, 82)],
            &[("风箫", 22, 84)],
            &[("绝心", 21, 80), ("小碎", 20, 72)],
        ]),
        "南通Hero久竞" => Some([
            &[("坦然", 22, 82)],
            &[("落尘", 21, 80)],
            &[("清弦", 20, 80)],
            &[("妖刀", 24, 82)],
            &[("紫渊", 20, 80)],
        ]),
        "深圳DYG" => Some([
            &[("小乐", 21, 80)],
            &[("方羽", 20, 80)],
            &[("向鱼", 23, 81)],
            &[("钎城", 23, 80)],
            &[("落空", 20, 80)],
        ]),
        "北京JDG" => Some([
            &[("光明", 21, 80)],
            &[("无双", 21, 80)],
            &[("清融", 23, 85), ("昊昊", 21, 72)],
            &[("小寒", 21, 80)],
            &[("无畏", 24, 82)],
        ]),
        "杭州LGD.NBW" => Some([
            &[("小落", 21, 81)],
            &[("米苏", 20, 80)],
            &[("九尾", 23, 80)],
            &[("星尘", 21, 80)],
            &[("小崽", 20, 80)],
        ]),
        "上海RNG.M" => Some([
            &[("小困", 21, 80)],
            &[("航航", 20, 80)],
            &[("庆安", 21, 80)],
            &[("润润", 20, 80)],
            &[("安尧", 20, 80)],
        ]),
        "长沙TES.A" => Some([
            &[("无幻", 22, 80)],
            &[("书源", 21, 80)],
            &[("小七", 21, 80)],
            &[("蓝桉", 21, 80)],
            &[("玖欣", 21, 80), ("帆帆", 23, 80)],
        ]),
        "西安WE" => Some([
            &[("小度", 21, 80)],
            &[("景云", 21, 80)],
            &[("开心", 20, 80)],
            &[("绝意", 21, 80)],
            &[("九幽", 20, 80)],
        ]),
        "深圳KLG" => Some([
            &[("沐风", 22, 80), ("阿泽", 20, 72)],
            &[("夜未", 21, 81)],
            &[("青崖", 21, 80)],
            &[("白茶", 21, 80)],
            &[("老白", 23, 79), ("阿黎", 20, 72)],
        ]),
        "桐乡情久" => Some([
            &[("小年", 21, 80)],
            &[("云澈", 20, 79)],
            &[("顾安", 22, 79), ("青禾", 20, 72)],
            &[("半暖", 21, 80)],
            &[("拾光", 20, 80)],
        ]),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum Lane {
    Top,
    Jungle,
    Mid,
    Bottom,
    Support,
}

impl Lane {
    fn from_usize(i: usize) -> Lane {
        match i {
            0 => Lane::Top,
            1 => Lane::Jungle,
            2 => Lane::Mid,
            3 => Lane::Bottom,
            _ => Lane::Support,
        }
    }
    fn stat_path(self) -> &'static str {
        match self {
            Lane::Top => "stat.top",
            Lane::Jungle => "stat.jungle",
            Lane::Mid => "stat.mid",
            Lane::Bottom => "stat.bottom",
            Lane::Support => "stat.support",
        }
    }
    fn style(self) -> [(&'static str, i64); 12] {
        match self {
            Lane::Top => [
                ("aggressive", 70), ("concentration", 75), ("control_speed", 70),
                ("ego", 70), ("judgement", 82), ("last_hit", 75),
                ("mental", 75), ("order", 70), ("positioning", 75),
                ("roaming", 45), ("skill_avoid", 70), ("skill_hit", 70),
            ],
            Lane::Jungle => [
                ("aggressive", 85), ("concentration", 70), ("control_speed", 80),
                ("ego", 75), ("judgement", 85), ("last_hit", 70),
                ("mental", 70), ("order", 65), ("positioning", 70),
                ("roaming", 90), ("skill_avoid", 65), ("skill_hit", 75),
            ],
            Lane::Mid => [
                ("aggressive", 75), ("concentration", 75), ("control_speed", 75),
                ("ego", 75), ("judgement", 80), ("last_hit", 75),
                ("mental", 75), ("order", 65), ("positioning", 70),
                ("roaming", 70), ("skill_avoid", 78), ("skill_hit", 82),
            ],
            Lane::Bottom => [
                ("aggressive", 72), ("concentration", 80), ("control_speed", 75),
                ("ego", 70), ("judgement", 75), ("last_hit", 90),
                ("mental", 75), ("order", 65), ("positioning", 86),
                ("roaming", 40), ("skill_avoid", 70), ("skill_hit", 82),
            ],
            Lane::Support => [
                ("aggressive", 45), ("concentration", 75), ("control_speed", 65),
                ("ego", 60), ("judgement", 82), ("last_hit", 50),
                ("mental", 82), ("order", 90), ("positioning", 75),
                ("roaming", 75), ("skill_avoid", 82), ("skill_hit", 60),
            ],
        }
    }
}

fn lane_of(ctx: &StableServerCtx<'_>, id: usize) -> Lane {
    let (top, jg, mid, bot, sup) = (
        ctx.athlete_get_i64(id, "stat.top").unwrap_or(0),
        ctx.athlete_get_i64(id, "stat.jungle").unwrap_or(0),
        ctx.athlete_get_i64(id, "stat.mid").unwrap_or(0),
        ctx.athlete_get_i64(id, "stat.bottom").unwrap_or(0),
        ctx.athlete_get_i64(id, "stat.support").unwrap_or(0),
    );
    let mut l = Lane::Top;
    let mut v = top;
    if jg > v { l = Lane::Jungle; v = jg; }
    if mid > v { l = Lane::Mid; v = mid; }
    if bot > v { l = Lane::Bottom; v = bot; }
    if sup > v { l = Lane::Support; }
    l
}

fn team_name_of(ctx: &StableServerCtx<'_>, id: usize) -> Option<String> {
    ctx.record_get_json(RecordKindV1::Athlete, id, "contract.InContract.team_id")
        .and_then(|s| s.parse::<usize>().ok())
        .and_then(|tid| ctx.record_get_string(RecordKindV1::Team, tid, "name"))
}

fn apply(ctx: &mut StableServerCtx<'_>, id: usize, name: &str, age: i64, ovr: i64, lane: Lane) {
    ctx.record_set_string(RecordKindV1::Athlete, id, "name", name);
    ctx.athlete_set_i64(id, "age", age);

    let scale = ovr as f64 / 80.0;
    ctx.athlete_set_i64(id, lane.stat_path(), ovr);
    ctx.athlete_set_i64(id, &format!("initial_stat.{}", lane.stat_path().split('.').nth(1).unwrap()), ovr);
    for (field, base) in lane.style() {
        let v = ((base as f64 * scale).round() as i64).clamp(0, 100);
        ctx.athlete_set_i64(id, &format!("stat.{}", field), v);
        ctx.athlete_set_i64(id, &format!("initial_stat.{}", field), v);
    }
}

/// 名单里的某个名字属于哪条分路 + 评分。
fn roster_lane_of(team: &str, name: &str) -> Option<(usize, i64)> {
    let r = roster_for(team)?;
    for (li, names) in r.iter().enumerate() {
        for &(n, _age, ovr) in names.iter() {
            if n == name {
                return Some((li, ovr));
            }
        }
    }
    None
}

fn rename_china(ctx: &mut StableServerCtx<'_>) {
    // v2：不再按 region_id==1 过滤。B 组的 6 支队（TACJ，region 5）是从原 JP
    // 联赛改名来的，v1 的 region 过滤把整队漏掉了 → 北京WB/重庆狼队等全是英文。
    // 判断改成「队名在 roster_for 里 + 名单里已经出现了哪些名字」，因此可以安全
    // 重跑：已经改过名的队 present 已满，不会出现「一个队两个轩染」。
    if ctx.save_get_string("rename_done_v2").as_deref() == Some("1") {
        return;
    }
    ctx.save_set_string("rename_done_v2", "1");

    // team -> Vec<(athlete_id, name, lane)>
    let mut by_team: HashMap<String, Vec<(usize, String, Lane)>> = HashMap::new();
    for id in ctx.record_ids(RecordKindV1::Athlete) {
        let Some(name) = ctx.record_get_string(RecordKindV1::Athlete, id, "name") else {
            continue;
        };
        let Some(t) = team_name_of(ctx, id) else { continue };
        if roster_for(&t).is_none() {
            continue;
        }
        by_team.entry(t).or_default().push((id, name, lane_of(ctx, id)));
    }

    // 每个队：先认已经改好的名字，再把缺的名单名分给英文默认名选手。
    for (team, athletes) in &by_team {
        let Some(r) = roster_for(team) else { continue };
        let total: usize = r.iter().map(|x| x.len()).sum();
        let present: HashSet<String> = athletes
            .iter()
            .map(|(_, n, _)| n.clone())
            .filter(|n| roster_lane_of(team, n).is_some())
            .collect();
        if present.len() >= total {
            continue;
        }
        // 候选 = 名字不在名单里的选手（英文默认名 / 其他替补）。
        let cands: Vec<(usize, Lane)> = athletes
            .iter()
            .filter(|(_, n, _)| !present.contains(n))
            .map(|(id, _, lane)| (*id, *lane))
            .collect();
        // 缺的名单名（分路, 名字, 年龄, 评分）。
        let mut missing: Vec<(usize, &'static str, i64, i64)> = Vec::new();
        for (li, names) in r.iter().enumerate() {
            for &(n, a, o) in names.iter() {
                if !present.contains(n) {
                    missing.push((li, n, a, o));
                }
            }
        }
        let mut taken = vec![false; cands.len()];
        let mut todo: Vec<(usize, &'static str, i64, i64)> = Vec::new();
        // 第一遍：同分路先配。
        for &(li, n, a, o) in &missing {
            let pos = cands
                .iter()
                .enumerate()
                .position(|(k, &(_, l))| !taken[k] && l as usize == li);
            match pos {
                Some(k) => {
                    taken[k] = true;
                    apply(ctx, cands[k].0, n, a, o, Lane::from_usize(li));
                }
                None => todo.push((li, n, a, o)),
            }
        }
        // 第二遍：剩下的名字给剩下的替补，并把他的分路统计改成名字对应的位置。
        for (li, n, a, o) in todo {
            let Some(k) = cands.iter().enumerate().position(|(k, _)| !taken[k]) else {
                break;
            };
            taken[k] = true;
            apply(ctx, cands[k].0, n, a, o, Lane::from_usize(li));
        }
    }
}

/// v2：把「名字对应的主分路」钉死。
/// v1 的 apply 只把目标分路设成评分，却没压低该选手原本的其他分路；如果原选手
/// 另一条分路更高，argmax 仍指向旧位置 → 名字和主要位置对不上（用户实测）。
/// 这里把目标分路 = 评分，其余四条压到评分以下，保证 argmax 一定对。
fn fix_roles(ctx: &mut StableServerCtx<'_>) {
    if ctx.save_get_string("rolefix_done_v2").as_deref() == Some("1") {
        return;
    }
    ctx.save_set_string("rolefix_done_v2", "1");
    let paths = [
        "stat.top",
        "stat.jungle",
        "stat.mid",
        "stat.bottom",
        "stat.support",
    ];
    for id in ctx.record_ids(RecordKindV1::Athlete) {
        let Some(name) = ctx.record_get_string(RecordKindV1::Athlete, id, "name") else {
            continue;
        };
        let Some(team) = team_name_of(ctx, id) else { continue };
        let Some((li, ovr)) = roster_lane_of(&team, &name) else {
            continue;
        };
        let off = (ovr - 12).clamp(30, 90);
        for (k, p) in paths.iter().enumerate() {
            let cur = ctx.athlete_get_i64(id, p).unwrap_or(0);
            let v = if k == li { ovr } else { cur.min(off) };
            ctx.athlete_set_i64(id, p, v);
            let field = p.split('.').nth(1).unwrap();
            ctx.athlete_set_i64(id, &format!("initial_stat.{}", field), v);
        }
        // 风格值也按分路统一，保证游戏内定位与名字一致。
        let scale = ovr as f64 / 80.0;
        for (field, base) in Lane::from_usize(li).style() {
            let v = ((base as f64 * scale).round() as i64).clamp(0, 100);
            ctx.athlete_set_i64(id, &format!("stat.{}", field), v);
            ctx.athlete_set_i64(id, &format!("initial_stat.{}", field), v);
        }
    }
}

/// v3：18 支真队里名单之外的多余选手（原 JP 选手）统一改成 KPL 风格的
/// 中文名，**能力值完全不动**（用户要求保留 90–100 的高能力）。
/// 名字取一个固定的 KPL 风格名字池，全局去重（跳过存档里已有的名字）。
const BENCH_NAMES: &[&str] = &[
    "小义", "阿豆", "一晨", "北诗", "小新", "一曲", "梦客", "清枫", "蓝柚", "星宇",
    "沉舟", "明澈", "初晨", "花海", "清欢", "云中", "阿改", "汐音", "白羽", "青雉",
    "沐白", "苏烈", "长歌", "望月", "孤城", "南风", "北辰", "明眸", "赤霄", "流云",
    "沧海", "星野", "风起", "寒江", "白露", "惊鸿", "落霞", "逐月", "千寻", "云深",
    "无痕", "长夜", "青灯", "归鸿", "听风", "临渊", "白泽", "玄策", "墨白", "云鹤",
    "小北", "阿澈", "星痕", "木子", "小满", "青岚", "山鬼", "折扇", "南栀", "北柠",
    "青梧", "秋笙", "冬临", "春野", "夏至", "白芷", "朱砂", "青冥", "紫电", "惊蛰",
];

fn rename_bench(ctx: &mut StableServerCtx<'_>) {
    if ctx.save_get_string("benchname_done_v3").as_deref() == Some("1") {
        return;
    }
    ctx.save_set_string("benchname_done_v3", "1");
    // 全局已用名字，避免和真名/其他队重复。
    let mut used: HashSet<String> = HashSet::new();
    for id in ctx.record_ids(RecordKindV1::Athlete) {
        if let Some(n) = ctx.record_get_string(RecordKindV1::Athlete, id, "name") {
            used.insert(n);
        }
    }
    let mut by_team: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    for id in ctx.record_ids(RecordKindV1::Athlete) {
        let Some(name) = ctx.record_get_string(RecordKindV1::Athlete, id, "name") else {
            continue;
        };
        let Some(t) = team_name_of(ctx, id) else { continue };
        if roster_for(&t).is_none() {
            continue;
        }
        by_team.entry(t).or_default().push((id, name));
    }
    let mut next = 0usize;
    for (team, players) in &by_team {
        for (id, name) in players {
            // 名单里的真名选手不动。
            if roster_lane_of(team, name).is_some() {
                continue;
            }
            let mut picked: Option<&'static str> = None;
            while next < BENCH_NAMES.len() {
                let cand = BENCH_NAMES[next];
                next += 1;
                if !used.contains(cand) {
                    picked = Some(cand);
                    break;
                }
            }
            // 名字池用尽（正常不会）时退回队名+序号，保证唯一。
            let sub = match picked {
                Some(n) => n.to_string(),
                None => format!("{}青训{}", team, next),
            };
            used.insert(sub.clone());
            ctx.record_set_string(RecordKindV1::Athlete, *id, "name", &sub);
        }
    }
}

/// v4：中国赛区（region 1/5）里所有还没换成中文名的选手——K 甲、青训队、
/// 没进 roster_for 名单的二队/挂名队，以及中国赛区的自由球员——统一改成
/// KPL 风格中文名。只改 name，年龄和能力值完全不动。
///
/// 名字是人工挑的 KPL 风格 ID（不是随机拼字），按诗意/武侠/职业选手常见
/// 风格分组；不足时才会复用名单外的现有名字。
const CN_NAMES: &[&str] = &[
    // 常见 KPL 风格 / 现有替补名池
    "小义", "阿豆", "一晨", "北诗", "小新", "一曲", "梦客", "清枫", "蓝柚", "星宇",
    "沉舟", "明澈", "初晨", "花海", "清欢", "云中", "阿改", "汐音", "白羽", "青雉",
    "沐白", "苏烈", "长歌", "望月", "孤城", "南风", "北辰", "明眸", "赤霄", "流云",
    "星野", "风起", "寒江", "白露", "惊鸿", "落霞", "逐月", "千寻", "云深", "长夜",
    "青灯", "归鸿", "听风", "临渊", "白泽", "玄策", "墨白", "云鹤", "小北", "阿澈",
    "星痕", "木子", "小满", "青岚", "山鬼", "折扇", "南栀", "北柠", "青梧", "秋笙",
    "冬临", "春野", "夏至", "白芷", "朱砂", "青冥", "紫电", "惊蛰",
    // 职业选手感
    "一诺", "暖阳", "清融", "梦泪", "猫神", "飞牛", "老帅", "无痕", "阿泰", "伪装",
    "兰息", "纵情", "爱思", "笑影", "啊泽", "子阳", "天赐",
    // 诗意 / 自然
    "月白", "星河", "云澈", "风隐", "雨墨", "雷泽", "光影", "梦岚", "夜北", "晨星",
    "夕颜", "东篱", "西洲", "白鹿", "金戈", "木棉", "水镜", "火舞", "河图", "海晏",
    "江离", "川野", "林深", "森屿", "石青", "岩松", "冰河", "雪见", "霜降", "露白",
    "秋分", "初雪", "离歌", "安澜", "宁远", "远山", "高歌", "飞白", "翔空", "潜龙",
    "渊明", "虎啸", "狼影", "鹰扬", "龙渊", "凤鸣", "麟角", "羽衣", "翼展", "刃心",
    "剑影", "弓弦", "弦歌", "书白", "墨染", "画卷", "琴心", "棋魂", "酒仙", "茶烟",
    "花间", "叶落", "尘白", "沙洲", "明镜", "暗影", "浊酒", "清酒", "长明", "初岚",
    "秋白", "夏禾", "春水", "冬阳", "南星", "西风", "东君", "云舟", "星阑", "月蚀",
    "霜华", "雪落", "雨眠", "风眠", "云眠", "星眠", "花眠", "竹影", "梅香", "兰亭",
    "菊隐", "松间", "月下", "江畔", "山间", "云间", "水间", "林间", "石间", "溪云",
    "溪风", "溪月", "溪桥", "溪谷", "青衫", "白衣", "素衣", "玄衣", "绛衣", "墨衣",
    "青锋", "白刃", "玄铁", "赤焰", "紫焰", "金鳞", "银翼", "铜戈", "玉衡", "玉京",
    "玉楼", "玉川", "玉山", "玉门", "玉关", "玉龙", "星辰", "星尘", "月光", "月影",
    "日出", "日落", "云开", "云起", "云落", "风吟", "雨落", "雨霖", "雷鸣", "闪电",
    "光年", "梦回", "梦境", "夜行", "夜色", "晨光", "晨雾", "夕阳", "夕照", "北风",
    "东风", "青云", "青空", "青石", "白霜", "白夜", "白雪", "黑曜", "赤瞳", "紫瞳",
    "金瞳", "木叶", "水月", "火影", "山岚", "河洛", "海潮", "江湖", "川流", "林海",
    "森罗", "石破", "岩心", "冰封", "雪原", "霜寒", "露华", "秋月", "春雷", "夏雨",
    "冬雪", "初阳", "末夏", "离火", "归云", "安歌", "宁川", "远航", "高远", "飞扬",
    "翔宇", "潜行", "渊海", "虎牙", "狼牙", "鹰眼", "龙牙", "凤羽", "麟羽", "羽林",
    "翼风", "刃影", "锋刃", "剑心", "刀锋", "弦月", "歌行", "舞月", "书行", "墨行",
    "画影", "琴音", "棋局", "酒歌", "茶语", "花影", "叶影", "果心", "尘缘", "沙影",
    "涯月", "明川", "暗川", "清川", "浊川", "重山", "轻舟", "疾风", "徐行", "静水",
    "动雷", "守月", "攻玉", "念川", "忘川", "知秋", "行舟", "言川", "默然", "笑川",
    "悲歌", "欢颜",
    // 三字 ID
    "玉麒麟", "云中鹤", "月中仙", "山中客", "海上花", "天涯客", "江湖客", "故人归",
    "少年游", "长安客", "洛阳花", "听雨轩", "望江楼", "踏雪行", "逐月者", "摘星人",
    "揽星河", "饮风尘", "醉清风", "笑红尘", "步青云", "渡星河", "观沧海", "问苍穹",
    "斩长空", "破晓时", "归途远", "流光转", "浮生梦", "半盏茶", "一壶酒", "半卷书",
    "三尺剑", "七弦琴", "九万里", "千山雪", "万重山",
    // 单字 ID
    "信", "涛", "涵", "泽", "煜", "焱", "淼", "磊", "骏", "逸", "骐", "骥", "熠",
    "昊", "晟", "睿", "骁", "野", "默", "岚",
    // 扩充池：中国赛区每赛季都会新生成选手，名字要留足余量（防止池子取空）。
    "白虹", "紫陌", "丹青", "碧落", "苍穹", "九霄", "八荒", "四野", "孤舟", "寒霜",
    "冷月", "残阳", "暮雪", "烟雨", "雾隐", "霜刃", "雪刃", "风刃", "雷刃", "月刃",
    "星刃", "云刃", "墨刃", "银月", "金乌", "玉兔", "赤乌", "白驹", "青骢", "绝影",
    "飞电", "追风", "逐日", "踏浪", "破浪", "问剑", "试剑", "藏剑", "出鞘", "归鞘",
    "寒锋", "冷锋", "秋声", "春声", "夏声", "山月", "江月", "海月", "湖月", "竹月",
    "梅月", "松月", "柳月", "枫月", "桐月", "荷月", "桂月", "桃月", "杏月", "听雨",
    "听雪", "听海", "听松", "听潮", "观海", "观星", "观月", "望云", "望海", "望山",
    "望川", "清风", "清潭", "清溪", "清源", "清野", "清秋", "清霜", "清影", "墨池",
    "墨轩", "墨客", "墨言", "墨语", "书客", "书童", "书剑", "书语", "琴客", "琴语",
    "琴川", "棋客", "棋语", "酒客", "茶客", "花客", "花仙", "花匠", "叶舟", "叶语",
    "云客", "云生", "云舒", "云卷", "云台", "云阙", "云泽", "云海", "云岭", "云峰",
    "云涛", "星客", "星语", "星穹", "星轨", "星海", "星洲", "星落", "星沉", "月客",
    "月语", "月华", "月泉", "月台", "月阙", "月舟", "风客", "风语", "风谷", "风岭",
    "风铃", "雨客", "雨声", "雨竹", "雨荷", "雨桐", "雷声", "雪客", "雪声", "雪松",
    "雪梅", "雪竹", "霜客", "霜月", "秋客", "秋雨", "秋风", "秋叶", "秋山", "秋江",
    "春山", "春江", "春雪", "夏风", "夏荷", "冬梅", "冬青", "冬至", "夏夜", "春夜",
    "秋夜", "冬夜", "南山", "北山", "东林", "西林", "南江", "北江", "白鹤", "青鸾",
    "朱雀", "玄武", "青龙", "白虎", "麒麟", "凤凰", "鲲鹏", "貔貅", "墨羽", "玄羽",
    "青羽", "金羽", "银羽", "紫羽", "赤羽", "黑羽", "霜羽", "雪羽", "剑客", "刀客",
    "弓客", "枪客", "拳客", "影客", "刃客", "弦客", "歌客", "舞客", "月神", "星神",
    "风神", "雨神", "雷神", "雪神", "霜神", "云神", "山神", "海神", "云歌", "月歌",
    "星歌", "风歌", "雨歌", "雪歌", "竹歌", "梅歌", "兰歌", "菊歌",
];

fn cn_name_pool() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for n in CN_NAMES {
        if seen.insert(n) {
            v.push((*n).to_string());
        }
    }
    v
}

/// 1.1.0 早期版本用「姓 + 名」随机拼出的名字（陈影、王电之类）。
/// 这些名字已经在一些存档里了，再跑一遍时要把它们也换成新名字池。
fn is_legacy_generated(name: &str) -> bool {
    let mut it = name.chars();
    let (Some(a), Some(b), None) = (it.next(), it.next(), it.next()) else {
        return false;
    };
    const S: &str = "陈林王李张刘黄吴徐孙胡朱高罗郑梁谢宋唐许韩冯邓曹彭曾肖田董袁潘于蒋蔡余杜叶程苏魏";
    const G: &str = "一三小子天月星云风雨雷电光影梦夜晨夕南北东西青白黑赤紫金木水火土山河海江川林森石岩冰雪霜露秋春夏冬初末离归安宁远高飞翔潜渊虎狼鹰龙凤麟羽翼刃锋剑刀弓弦歌舞书墨画琴棋酒茶花叶果尘沙涯角明暗清浊重轻疾徐静动守攻念忘知行言默笑哭悲欢";
    S.contains(a) && G.contains(b)
}

/// 从 `soloranks` JSON 里取第一个 `region_id`（0 韩国 / 1 中国 / 2 欧洲 /
/// 3 北美 / 4 南美 / 5 日本）。
fn solo_region(j: &str) -> Option<i64> {
    let pat = "\"region_id\":";
    let i = j.find(pat)? + pat.len();
    let rest = &j[i..];
    let end = rest
        .find(|c: char| c == ',' || c == '}' || c == ']')
        .unwrap_or(rest.len());
    rest[..end].trim().parse().ok()
}

/// League id -> region id.
fn league_regions(ctx: &mut StableServerCtx<'_>) -> HashMap<usize, i64> {
    let mut m = HashMap::new();
    for lid in ctx.record_ids(RecordKindV1::League) {
        if let Some(r) = ctx
            .record_get_json(RecordKindV1::League, lid, "region_id")
            .and_then(|s| s.parse::<i64>().ok())
        {
            m.insert(lid, r);
        }
    }
    m
}

/// 预扫描所有 Team：team_id -> Some((是否中国赛区, 队名))。
/// 一次 O(队伍数) 的读取，替代每个选手各查两遍 Team 记录。
fn cn_team_table(
    ctx: &StableServerCtx<'_>,
    lg: &HashMap<usize, i64>,
) -> HashMap<usize, Option<(bool, String)>> {
    let mut m = HashMap::new();
    for tid in ctx.record_ids(RecordKindV1::Team) {
        let entry = ctx
            .record_get_json(RecordKindV1::Team, tid, "league_id")
            .and_then(|s| s.parse::<usize>().ok())
            .and_then(|lid| lg.get(&lid).copied())
            .map(|region| {
                let name = ctx
                    .record_get_string(RecordKindV1::Team, tid, "name")
                    .unwrap_or_default();
                (region == 1 || region == 5, name)
            });
        m.insert(tid, entry);
    }
    m
}

/// 中国赛区归属：`None` = 不是中国赛区；`Some("")` = 中国赛区的自由球员；
/// `Some(队名)` = 中国赛区某支队伍的选手。region 1 = 中国，region 5 = 原日本
/// 赛区（本 mod 里被改成了 B 组 / K 甲）。
fn cn_team_of(
    ctx: &StableServerCtx<'_>,
    teams: &HashMap<usize, Option<(bool, String)>>,
    id: usize,
) -> Option<String> {
    let tid = ctx
        .record_get_json(RecordKindV1::Athlete, id, "contract.InContract.team_id")
        .and_then(|s| s.parse::<usize>().ok());
    match tid {
        Some(tid) => {
            let (in_cn, name) = teams.get(&tid)?.as_ref()?;
            in_cn.then(|| name.clone())
        }
        // 自由球员没有球队，用 Solo Rank 所在赛区判断是不是中国赛区。
        None => {
            let ranks = ctx
                .record_get_json(RecordKindV1::Athlete, id, "soloranks")
                .unwrap_or_default();
            if matches!(solo_region(&ranks), Some(1) | Some(5)) {
                Some(String::new())
            } else {
                None
            }
        }
    }
}

fn rename_cn_all(ctx: &mut StableServerCtx<'_>) {
    // 每次开服 / 每个管理事件都跑一遍：新生成的选手也会被换成中文名。
    // 顺带做一次全局查重：中国赛区里重名的选手，后出现的会改名，保证不重名。
    // 已经是唯一中文名的会保持原样，所以重复执行是安全的。

    let lg_region = league_regions(ctx);
    let teams = cn_team_table(ctx, &lg_region);
    let curated: HashSet<&str> = CN_NAMES.iter().copied().collect();

    struct Row {
        id: usize,
        name: String,
        prio: u8,
    }

    let mut ids: Vec<usize> = ctx.record_ids(RecordKindV1::Athlete);
    ids.sort_unstable();

    // 非中国赛区的名字直接占位，保证改出来的名字和他们不重。
    let mut kept: HashSet<String> = HashSet::new();
    let mut rows: Vec<Row> = Vec::new();

    for id in ids {
        let Some(name) = ctx.record_get_string(RecordKindV1::Athlete, id, "name") else {
            continue;
        };
        let Some(team) = cn_team_of(ctx, &teams, id) else {
            kept.insert(name);
            continue;
        };
        // roster_for 里的真实 KPL 名字（如 "Ming" / "小A"）永远保留。
        let real_roster = !team.is_empty() && roster_lane_of(&team, &name).is_some();
        let english = name.chars().any(|c| c.is_ascii_alphabetic());
        let legacy = !english && !curated.contains(name.as_str()) && is_legacy_generated(&name);
        // 优先级：真实名单名 > 现成中文名 > 名字池里的名字 > 英文 / 旧随机名。
        // 重名时优先级低的那个改名，真名永远保留。
        let prio = if real_roster {
            0
        } else if english || legacy {
            3
        } else if curated.contains(name.as_str()) {
            1
        } else {
            2
        };
        rows.push(Row { id, name, prio });
    }

    rows.sort_by(|a, b| (a.prio, a.id).cmp(&(b.prio, b.id)));

    // 按优先级占名；需要改名的（英文 / 旧随机名）或重名的排队重新取名。
    let mut to_rename: Vec<usize> = Vec::new();
    for r in &rows {
        if r.prio == 0 {
            kept.insert(r.name.clone());
        } else if r.prio >= 3 || kept.contains(&r.name) {
            to_rename.push(r.id);
        } else {
            kept.insert(r.name.clone());
        }
    }

    let mut pool = cn_name_pool()
        .into_iter()
        .filter(|n| !kept.contains(n))
        .collect::<Vec<_>>()
        .into_iter();
    for id in to_rename {
        let Some(next_name) = pool.next() else { break };
        kept.insert(next_name.clone());
        ctx.record_set_string(RecordKindV1::Athlete, id, "name", &next_name);
    }
}

fn parse_ids(json: &str) -> Vec<usize> {
    json.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<usize>().ok())
        .collect()
}

/// Set BO5 (need_win=3) for league regular matches and BO7 (need_win=4) for playoffs.
fn set_bo(ctx: &mut StableServerCtx<'_>) {
    for cid in ctx.record_ids(RecordKindV1::LeagueCompetition) {
        if let Some(m) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "matches") {
            for mid in parse_ids(&m) {
                set_need_win(ctx, mid, "3");
            }
        }
        if let Some(p) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "playoffs") {
            for mid in parse_ids(&p) {
                set_need_win(ctx, mid, "4");
            }
        }
    }
}

/// 只在值确实不同时写，避免每次读档把所有比赛记录都标脏。
fn set_need_win(ctx: &mut StableServerCtx<'_>, mid: usize, want: &str) {
    let cur = ctx
        .record_get_json(RecordKindV1::Match, mid, "need_win")
        .unwrap_or_default();
    if cur.trim() != want {
        ctx.record_set_json(RecordKindV1::Match, mid, "need_win", want);
    }
}

// ============================ SAB 分组 ============================
// 真实 KPL SAB：不抽签、按强弱分组。**分组靠 data_setting 顺序**——把 TACC 的
// 18 队按实力（资金）排好，引擎会自动按顺序拆成 3 个 competition（各 6 队）=
// S/A/B 三组。Rust 侧只做三件安全的事：
//   1. set_bo 设常规赛 BO5、季后赛 BO7（改 need_win，已验证安全）；
//   2. record_last_season 把上赛季 finalized 三组 standings 按名次存进存档；
//   3. dump_sab 把当前分组/名次写到桌面。
// ⚠️ 绝不重写 Match 的 team1/team2 或 competition 的 standings —— 会把引擎的
//    team→competition 索引搞坏、直接卡死（2026-10-12 踩过坑）。

#[derive(Clone, Copy)]
struct StandRow {
    team: usize,
    win: i64,
    lose: i64,
    set_win: i64,
    set_lose: i64,
}

/// 取 JSON 对象里某个整数字段，例如 `{"win":3}` 里取 "win"。
fn field_i64(obj: &str, key: &str) -> Option<i64> {
    let needle = format!("\"{}\":", key);
    let pos = obj.find(&needle)?;
    let rest = obj[pos + needle.len()..].trim_start();
    let mut chars = rest.chars().peekable();
    let mut neg = false;
    if chars.peek() == Some(&'-') {
        neg = true;
        chars.next();
    }
    let mut v: i64 = 0;
    let mut any = false;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            v = v * 10 + d as i64;
            any = true;
        } else {
            break;
        }
    }
    any.then_some(if neg { -v } else { v })
}

/// 解析 standings 形如 `{"10":{"win":0,"set_win":0,"set_lose":0,...},...}`。
fn parse_standings(json: &str) -> Vec<StandRow> {
    let mut rows = Vec::new();
    let b = json.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] != b'"' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        let mut team: Option<usize> = Some(0);
        while j < b.len() && b[j] != b'"' {
            if b[j].is_ascii_digit() {
                team = team
                    .and_then(|t| t.checked_mul(10))
                    .and_then(|t| t.checked_add((b[j] - b'0') as usize));
            } else {
                team = None;
            }
            j += 1;
        }
        if j >= b.len() || b[j] != b'"' {
            i += 1;
            continue;
        }
        let mut k = j + 1;
        while k < b.len() && (b[k] as char).is_whitespace() {
            k += 1;
        }
        if k >= b.len() || b[k] != b':' {
            i += 1;
            continue;
        }
        k += 1;
        while k < b.len() && (b[k] as char).is_whitespace() {
            k += 1;
        }
        if k >= b.len() || b[k] != b'{' {
            i += 1;
            continue;
        }
        let start = k;
        let mut depth = 0usize;
        while k < b.len() {
            match b[k] {
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            k += 1;
        }
        let obj = &json[start..=k.min(b.len() - 1)];
        if let Some(tid) = team {
            rows.push(StandRow {
                team: tid,
                win: field_i64(obj, "win").unwrap_or(0),
                lose: field_i64(obj, "lose").unwrap_or(0),
                set_win: field_i64(obj, "set_win").unwrap_or(0),
                set_lose: field_i64(obj, "set_lose").unwrap_or(0),
            });
        }
        i = k + 1;
    }
    rows
}

/// CN KPL（region_id=1，S/A/B 三个联赛）的 competition id，按 finalized 过滤。
fn cn_d1_comps(ctx: &StableServerCtx<'_>, want_finalized: bool) -> Vec<usize> {
    let mut comps = Vec::new();
    for lid in ctx.record_ids(RecordKindV1::League) {
        let region = ctx
            .record_get_json(RecordKindV1::League, lid, "region_id")
            .and_then(|s| s.parse::<i64>().ok());
        if region == Some(1) || region == Some(5) { // CN1(region1) + CN2(region5，原日本)
            if let Some(c) = ctx.record_get_json(RecordKindV1::League, lid, "competitions") {
                for cid in parse_ids(&c) {
                    let fin = ctx
                        .record_get_json(RecordKindV1::LeagueCompetition, cid, "finalized")
                        .map(|s| s.trim() == "true")
                        .unwrap_or(false);
                    if fin == want_finalized {
                        comps.push(cid);
                    }
                }
            }
        }
    }
    comps.sort_unstable();
    comps.dedup();
    comps
}

/// 当前活跃赛季（未 finalized）的 CN KPL competition id，按 id 升序（= S/A/B 顺序）。
fn cn_d1_competitions(ctx: &StableServerCtx<'_>) -> Vec<usize> {
    cn_d1_comps(ctx, false)
}

/// 上赛季（已 finalized）的 CN KPL competition id，按 id 升序（= S/A/B 顺序）。
fn cn_d1_finalized_competitions(ctx: &StableServerCtx<'_>) -> Vec<usize> {
    cn_d1_comps(ctx, true)
}

/// 单循环：把 LeagueCompetition 的 matches 从双循环砍成单循环（前 N*(N-1)/2 场）。
/// 10 队联赛双循环 90 场 → 45 场（9 轮）。
/// 幂等：只在 matches 仍长于单循环时砍，否则跳过。旧版是「每次砍半」，
/// on_server_start + 多次 SeasonRollover 反复触发 → 90→45→22→11→5→2，赛季只剩两场。
fn make_single_rr(ctx: &mut StableServerCtx<'_>) {
    let mut max_rounds = 0usize;
    for cid in ctx.record_ids(RecordKindV1::LeagueCompetition) {
        let Some(matches) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "matches") else {
            continue;
        };
        let ids = parse_ids(&matches);
        if ids.is_empty() {
            continue;
        }
        // 队伍数 N = standings 的键数（正常 10 队 → 10）。
        // ⚠️ SAB 的积分表会被 mod 缩成 6 真队，此时 N=6，不能再按 N 算单循环
        // （6 队只有 15 场，会把 45 场砍成 15 场）。改成「只有长度正好等于
        // N*(N-1) 的双循环才裁」，且裁到一半（90 → 45）；已经是 45/49 的跳过。
        let n = ctx
            .record_get_json(RecordKindV1::LeagueCompetition, cid, "standings")
            .map(|s| parse_standings(&s).len())
            .unwrap_or(0);
        if n < 2 {
            continue;
        }
        max_rounds = max_rounds.max(n - 1); // 10 队 → 9 轮
        let double_rr = n * (n - 1);
        if ids.len() != double_rr || ids.len() <= 45 {
            continue; // 已是单循环（45）/ 附加卡位赛（>45）或不是双循环：不动
        }
        let single_rr = ids.len() / 2;
        let json = format!(
            "[{}]",
            ids[..single_rr].iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
        );
        ctx.record_set_json(RecordKindV1::LeagueCompetition, cid, "matches", &json);
        // 新砍掉的 return-leg match 改到 2099-12-31（未来）：清幽灵轮 + 让引擎永不把它
        // 当「due」的过去赛程。⚠️ 之前错设成 2000-01-01（过去），引擎会把 540 个过去日期的
        // 孤儿当欠赛一格格补打，卡在同一天打不同队。
        for &mid in &ids[single_rr..] {
            ctx.record_set_string(RecordKindV1::Match, mid, "date", "2099-12-31T00:00:00");
        }
    }
    if max_rounds >= 2 {
        // 全局 YearSchedule 是所有 10 队联赛共用的：SAB 的积分表被缩成 6 队后
        // 不能再从它推轮数（否则会把全局日历裁到 5 轮）。固定按 10 队 = 9 轮。
        trim_year_schedule(ctx, max_rounds.max(9));
    }
    heal_orphan_dates(ctx);
}

/// 自愈：把所有 date 以 "2000-" 开头的 Match（之前被错设成 2000-01-01 的孤儿）改回
/// 2099-12-31。合法比赛日期都在 2026+，不存在 2000 年的，所以只精确匹配这个错值，
/// 不会误伤杯赛/季后赛/练习赛等其他 Match 记录。
fn heal_orphan_dates(ctx: &mut StableServerCtx<'_>) {
    for mid in ctx.record_ids(RecordKindV1::Match) {
        if ctx
            .record_get_string(RecordKindV1::Match, mid, "date")
            .map(|d| d.starts_with("2000-"))
            .unwrap_or(false)
        {
            ctx.record_set_string(RecordKindV1::Match, mid, "date", "2099-12-31T00:00:00");
        }
    }
}

/// 裁掉 YearSchedule 里超出单循环轮数的 LeagueMatch 事件（10 队单循环 9 轮 = round 0..8，
/// 删掉 round >= 9 的 9..17）。matches 砍到 45 后 YearSchedule 仍写 18 轮，实测引擎对 round >= 9
/// 的事件是「跳过不扣分」（不崩），但日历上会空转 9 轮。这里把整条
/// `[日期,{"LeagueMatch":{"index":N,"round":R}}]` 删掉，让日历和 matches 一致。幂等：删过一次后没得删。
fn trim_year_schedule(ctx: &mut StableServerCtx<'_>, keep_rounds: usize) {
    for sid in ctx.record_ids(RecordKindV1::YearSchedule) {
        let Some(sched) = ctx.record_get_json(RecordKindV1::YearSchedule, sid, "schedules") else {
            continue;
        };
        let needle = "{\"LeagueMatch\":";
        let mut removes: Vec<(usize, usize)> = Vec::new(); // (start_incl, end_excl)
        let mut from = 0usize;
        while let Some(rel) = sched[from..].find(needle) {
            let p = from + rel; // '{' of {"LeagueMatch":
            // 事件对象内无 ']'，故条目结尾 ']' = p 之后第一个 ']'
            let close = p + sched[p..].find(']').unwrap_or(sched.len() - p);
            if let Some(round) = field_i64(&sched[p..close], "round") {
                if round >= keep_rounds as i64 {
                    // 条目开头 '[' = p - 14（"[\"YYYY-MM-DD\"," 固定 14 字节）
                    let start = p.saturating_sub(14);
                    let mut s = start;
                    // 吞掉紧贴的前导逗号，避免留下 ",,"
                    if s > 0 && sched.as_bytes().get(s - 1) == Some(&b',') {
                        s -= 1;
                    }
                    removes.push((s, close + 1));
                }
            }
            from = close + 1;
        }
        if removes.is_empty() {
            continue;
        }
        removes.sort_by(|a, b| b.0.cmp(&a.0));
        let mut out = sched.clone();
        for (s, e) in removes {
            out.replace_range(s..e, "");
        }
        ctx.record_set_json(RecordKindV1::YearSchedule, sid, "schedules", &out);
    }
}

/// 根据 competition id 反查所属联赛名（TACC/TACC2/TACJ/TACJ2...）。
fn comp_league_name(ctx: &StableServerCtx<'_>, cid: usize) -> Option<String> {
    for lid in ctx.record_ids(RecordKindV1::League) {
        let Some(c) = ctx.record_get_json(RecordKindV1::League, lid, "competitions") else {
            continue;
        };
        if parse_ids(&c).contains(&cid) {
            return ctx.record_get_string(RecordKindV1::League, lid, "name");
        }
    }
    None
}

/// 把上赛季（finalized 的 CN D1 competition）最终名次存到 mod 存档命名空间，
/// 供下赛季分组用。名次 = S组排序 + A组排序 + B组排序（competition id 升序 = S/A/B）。
fn record_last_season(ctx: &mut StableServerCtx<'_>) {
    let final_comps = cn_d1_finalized_competitions(ctx);
    if final_comps.is_empty() {
        return;
    }
    let mut ranked: Vec<usize> = Vec::new();
    for &cid in &final_comps {
        let json = ctx
            .record_get_json(RecordKindV1::LeagueCompetition, cid, "standings")
            .unwrap_or_default();
        let mut rows = parse_standings(&json);
        rows.sort_by(|x, y| {
            y.win
                .cmp(&x.win)
                .then((y.set_win - y.set_lose).cmp(&(x.set_win - x.set_lose)))
                .then(y.set_win.cmp(&x.set_win))
                .then(x.team.cmp(&y.team))
        });
        for r in rows {
            ranked.push(r.team);
        }
    }
    if ranked.is_empty() {
        return;
    }
    let s = ranked
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(",");
    ctx.save_set_string("last_season_rank", &s);
}

fn team_name(ctx: &StableServerCtx<'_>, tid: usize) -> String {
    ctx.record_get_string(RecordKindV1::Team, tid, "name")
        .unwrap_or_else(|| format!("#{}", tid))
}

// ============================ 内存扫描：国际杯种子分配表 ============================
// 引擎抽国际杯时从 world+0x169a8 的 Vec<InternationalSeedAllocation> 读种子槽：
// 结构体 0x58 字节，+0x00/+0x18/+0x30 是三组 {cap,ptr,len}，+0x48 赛区，+0x50 赛事类型。
// 每个槽 = 24 字节 {联赛键, 名次基准, ...}。池子长度不是 4 的倍数就会在
// tournament.rs:1494 越界崩。稳定 SDK 不暴露 world，只能直接扫本进程堆内存找出这张表，
// 确认到底哪些赛区/名次没取到队伍。
mod seedscan {

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut core::ffi::c_void;
        fn ReadProcessMemory(
            h_process: *mut core::ffi::c_void,
            base: *const core::ffi::c_void,
            buf: *mut core::ffi::c_void,
            size: usize,
            read: *mut usize,
        ) -> i32;
    }


    /// 读本进程内存；失败返回 false（ReadProcessMemory 不会抛异常）。
    pub fn rpm(addr: usize, dst: &mut [u8]) -> bool {
        if dst.is_empty() {
            return true;
        }
        if addr < 0x10000 {
            return false;
        }
        let mut got = 0usize;
        let ok = unsafe {
            ReadProcessMemory(
                GetCurrentProcess(),
                addr as *const core::ffi::c_void,
                dst.as_mut_ptr() as *mut core::ffi::c_void,
                dst.len(),
                &mut got,
            )
        };
        ok != 0 && got == dst.len()
    }


    pub fn rdu64(addr: usize) -> Option<u64> {
        let mut b = [0u8; 8];
        if rpm(addr, &mut b) {
            Some(u64::from_le_bytes(b))
        } else {
            None
        }
    }

    pub fn rdu32(addr: usize) -> Option<u32> {
        let mut b = [0u8; 4];
        if rpm(addr, &mut b) {
            Some(u32::from_le_bytes(b))
        } else {
            None
        }
    }

}

// ============================ 国际杯种子表修正 ============================
// 5/4 国际杯抽签崩在 tournament.rs:1494（种子池最后一个 pot 只有 2 队，
// 回溯分配器仍按 4 个槽位写下标 → 越界）。种子分配表在 world 里，稳定 SDK
// 不暴露，只能从 host state 指针直接读/改（全部走 ReadProcessMemory，失败不抛异常）。
// WORLD_ADDR 缓存找到的 world 地址，避免每个 tick 重新扫描。
static WORLD_ADDR: AtomicUsize = AtomicUsize::new(0);

/// 判断 w+base 处是不是一个可读的 hashbrown 头（ctrl/mask/items）。
fn map_header_ok(w: usize, base: usize, items_off: usize) -> bool {
    let (Some(ctrl), Some(mask), Some(items)) = (
        seedscan::rdu64(w + base),
        seedscan::rdu64(w + base + 8),
        seedscan::rdu64(w + base + items_off),
    ) else {
        return false;
    };
    if ctrl < 0x10000 || mask >= 4096 || !(mask + 1).is_power_of_two() || items > mask + 1 {
        return false;
    }
    let mut probe = [0u8; 16];
    seedscan::rpm(ctrl as usize, &mut probe)
}

/// 在 state 头部找 world：要求 region 集合、排名表、比赛表都是 hashbrown 形状。
fn find_world(s: usize) -> Option<usize> {
    if s < 0x10000 {
        return None;
    }
    let mut cands: Vec<usize> = vec![s];
    for off in (0..0x400).step_by(8) {
        if let Some(v) = seedscan::rdu64(s + off) {
            if v > 0x10000 && v % 8 == 0 {
                cands.push(v as usize);
            }
        }
    }
    for &w in cands.iter().take(130) {
        let (Some(ctrl), Some(mask), Some(items)) = (
            seedscan::rdu64(w + 0x16938),
            seedscan::rdu64(w + 0x16940),
            seedscan::rdu64(w + 0x16950),
        ) else {
            continue;
        };
        if ctrl > 0x10000
            && mask < 4096
            && (mask + 1).is_power_of_two()
            && items <= mask + 1
            && map_header_ok(w, 0x17478, 0x18)
            && map_header_ok(w, 0x173e8, 0x18)
        {
            let mut probe = [0u8; 16];
            if seedscan::rpm(ctrl as usize, &mut probe) {
                return Some(w);
            }
        }
    }
    None
}

/// 5/4「Challengers E」抽签池：`0x1416db2d2` 里用 `[tournament+0x68]` 选 vec，
/// Challengers E 的类型是 2 → 用种子表 **vec1**（+0x20/+0x28），不是 vec0。
/// 引擎分配的 vec1 = (region0,6)(region1,6)(region5,4)。region 5 = TACJ 被本 mod
/// 用成了 KPL B 组，B 组赛季末 finalized、playoffs 清空，1-6 名取队返回 None，
/// 这 4 个槽位凑不出队 → 池子只剩 12 队 → 4 组 × 4 队的分组赛在
/// tournament.rs:2326 越界（len 3, index 3）。
///
/// 每个条目取队名次都从 1 开始，所以同一赛区不能拆成两个条目（会取到重复队），
/// 只能把凑不出队的赛区并到同 vec 里「赛区列表里有、且队够」的赛区上：
///   - vec0（Masters，region 0..5）：只有 region 5 供不出队 → 并到 region 0；
///   - vec1（Challengers E，赛区 0/1/5）：region 5 供不出，且上一版误改成 region 2
///     的条目也要收回（region 2 不在这项的赛区列表里，会被整条跳过）→ 并到 0/1；
///   - vec2（Challengers W，region 2/3/4）：只处理 region 5（正常不会出现）。
/// 每个赛区最多取 10 名（comp 0/1/2/3/4 的 1..10 名都实测可取）。
fn maybe_fix_cup_allocation(ctx: &StableServerCtx<'_>) {
    let w = match WORLD_ADDR.load(Ordering::Relaxed) {
        0 => match find_world(ctx.raw_state() as usize) {
            Some(w) => {
                WORLD_ADDR.store(w, Ordering::Relaxed);
                w
            }
            None => return,
        },
        w => w,
    };
    let (Some(cap), Some(ptr), Some(len)) = (
        seedscan::rdu64(w + 0x169a0),
        seedscan::rdu64(w + 0x169a8),
        seedscan::rdu64(w + 0x169b0),
    ) else {
        return;
    };
    if len == 0 || len > 64 || cap < len || ptr < 0x10000 {
        return;
    }
    // 只改当前年份、type=0 的那条分配表（和其它年份的条目区分开）。
    let year = seedscan::rdu32(w + 0x17a40).map(|v| (v as i32) >> 13).unwrap_or(0) as u64;
    for e in 0..len as usize {
        let base = ptr as usize + e * 0x58;
        if seedscan::rdu64(base + 0x48).unwrap_or(u64::MAX) != year {
            continue;
        }
        if seedscan::rdu64(base + 0x50).unwrap_or(u64::MAX) != 0 {
            continue;
        }
        for (vi, (poff, loff)) in [(0x08usize, 0x10usize), (0x20, 0x28), (0x38, 0x40)]
            .iter()
            .enumerate()
        {
            let (Some(vlen), Some(vptr)) = (
                seedscan::rdu64(base + loff),
                seedscan::rdu64(base + poff),
            ) else {
                continue;
            };
            if vlen == 0 || vlen > 32 || vptr < 0x10000 {
                continue;
            }
            // 引擎分组约束（0x1416dc3f7 起的重排循环）：抽签按种子表顺序每 4 队切
            // 一组，并记录「组内位置 → 联赛」出现次数，要求次数 < 阈值，否则重排
            // （最多 10000 次）后退化成不完整的小组，抽签函数随后 panic
            // （tournament.rs:2277 的 len 2 index 2 就是这么来的）。
            // 阈值 [rbp+0x510]：Challengers=2、Masters/Champions=1。
            // 一轮抽签里每个联赛最多占满「阈值」个完整小组：
            //   vec0（Masters/Champions）：4 队/赛区
            //   vec1/vec2（Challengers）：8 队/赛区
            // 例：vec1 塞成 (0,10)(1,6) 时，前两组把联赛 0 占满 4 个位置各 2 次，
            // 第三组无论怎么排都会撞 2 >= 2 → 必然失败。
            let cap: u64 = if vi == 0 { 4 } else { 8 };
            // 引擎这项赛事认的赛区；不在列表里的赛区供不出队，必须把名额让出来。
            let good: &[u64] = match vi {
                1 => &[0, 1],
                2 => &[2, 3, 4],
                _ => &[0, 1, 2, 3, 4],
            };
            let mut total = 0u64;
            let mut kept = 0u64;
            for k in 0..vlen as usize {
                let ea = vptr as usize + k * 0x18;
                let (Some(r), Some(c)) =
                    (seedscan::rdu64(ea), seedscan::rdu64(ea + 8))
                else {
                    continue;
                };
                total += c;
                let keep = if !good.contains(&r) {
                    0
                } else {
                    c.min(cap)
                };
                kept += keep;
                if keep != c {
                    unsafe { core::ptr::write_volatile((ea + 8) as *mut u64, keep) };
                }
            }
            // 去掉的队（超上限 / 赛区供不出）再按 cap 匀给同 vec 里还有空位的赛区。
            let mut left = total.saturating_sub(kept);
            for &gr in good {
                if left == 0 {
                    break;
                }
                let Some(tk) = (0..vlen as usize).find(|&k| {
                    seedscan::rdu64(vptr as usize + k * 0x18) == Some(gr)
                }) else {
                    continue;
                };
                let ta = vptr as usize + tk * 0x18;
                let tc = seedscan::rdu64(ta + 8).unwrap_or(0);
                let room = cap.saturating_sub(tc);
                let add = room.min(left);
                if add == 0 {
                    continue;
                }
                unsafe { core::ptr::write_volatile((ta + 8) as *mut u64, tc + add) };
                left -= add;
            }
        }
    }
}

// ======================== 杯赛席位替换（挂名队 / 重复队 -> 真队） ========================
//
// 背景：国际杯参赛队按「联赛最终名次」取——Masters 取各赛区 1-4，Challengers
// 东/西区取 5-10。CN 赛区第 7-10 名是 0-0 的挂名队（TCG/ZQ/DSM…），于是它们
// 会被抽进东区；另外名次编码之前 RW 在 S/A 两张表里都有名次，会同时出现在
// Masters 和东区。
//
// 改名额分配（少发 CN 席位）实测走不通：东区是 12 队 4 组 × 3，少 2 队就变成
// 4 组 × 2，抽签兜底分配越界崩溃（tournament.rs len 2 index 2）。
// 所以保留席位，**抽签之后**只改杯赛记录里的队伍：
//   - 挂名队 / 已经出现在前面杯赛里的真队 -> 没进任何杯赛的真 KPL 队
//     （候选顺序 = 官方最终名次，其次 team id）
//   - 只动 TournamentCompetition 的 team_seeds/groups/statistics 和组内比赛的
//     team1/team2；不碰积分表、名次、升降级。
//   - 幂等；任何一场小组/淘汰赛已 End（或杯赛已 finalize）就整届跳过。
const SWAP_CUP_FILLERS: bool = true;

/// 官方最终 1-10 名（统一季后赛 5 场排位赛的结果，和「KPL 最终名次」新闻同源）。
/// 季后赛没打完/存档里没有槽位 id 时返回空。
fn final_rank_teams(ctx: &StableServerCtx<'_>) -> Vec<usize> {
    let Some(c) = sab_comps(ctx).or_else(|| sab_comps_all(ctx)) else {
        return Vec::new();
    };
    let Some(s_mids) = comp_mids(ctx, c.s) else {
        return Vec::new();
    };
    if s_mids.len() < 45 {
        return Vec::new();
    }
    let tag = s_mids[0];
    let (Some(s_po), Some(a_po)) = (
        save_id_list(ctx, &format!("po_sids_{}", tag)),
        save_id_list(ctx, &format!("po_aids_{}", tag)),
    ) else {
        return Vec::new();
    };
    if s_po.len() < 8 || a_po.len() < 8 {
        return Vec::new();
    }
    let mut po_mids: Vec<usize> = Vec::with_capacity(16);
    for k in 0..8 {
        po_mids.push(s_po[k]);
        po_mids.push(a_po[k]);
    }
    let mut out = Vec::with_capacity(10);
    for mid in [po_mids[14], po_mids[15], po_mids[13], po_mids[11], po_mids[7]] {
        let (Some((w, _, _)), Some(l)) = (match_result(ctx, mid), po_loser_of(ctx, mid)) else {
            return Vec::new();
        };
        out.push(w);
        out.push(l);
    }
    out
}

/// 只取 `"<num>":` 形式的对象键（team_seeds 的 key 就是 team id；value 是种子号）。
fn json_num_keys(json: &str) -> Vec<usize> {
    let b = json.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 && j + 1 < b.len() && b[j] == b'"' && b[j + 1] == b':' {
                if let Ok(v) = json[i + 1..j].parse::<usize>() {
                    out.push(v);
                }
            }
        }
        i += 1;
    }
    out
}

/// CN 赛区的挂名队：名字不在 KPL 名单里、且属于 CN 的联赛（S/A/B = 1/7/5，青训 11）。
fn is_cn_filler(ctx: &StableServerCtx<'_>, t: usize) -> bool {
    if is_real_kpl(&team_name(ctx, t)) {
        return false;
    }
    let lid = ctx
        .record_get_json(RecordKindV1::Team, t, "league_id")
        .and_then(|s| s.trim().parse::<usize>().ok());
    matches!(lid, Some(1) | Some(5) | Some(7) | Some(11))
}

/// 一次性替换 JSON 对象里 `"<old>":` 的键（不会链式替换）。只替换深度 >= min_depth
/// 的键——groups 里第 1 层是小组下标（"0".."3"），第 2 层才是 team id。
fn json_swap_keys(json: &str, map: &[(usize, usize)], min_depth: usize) -> String {
    let mut out = String::with_capacity(json.len());
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < json.len() {
        let b = json.as_bytes()[i];
        if b == b'{' {
            depth += 1;
            out.push('{');
            i += 1;
            continue;
        }
        if b == b'}' {
            depth = depth.saturating_sub(1);
            out.push('}');
            i += 1;
            continue;
        }
        if b == b'"' {
            // 先尝试数字键
            let mut j = i + 1;
            while j < json.len() && json.as_bytes()[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1
                && j + 1 < json.len()
                && json.as_bytes()[j] == b'"'
                && json.as_bytes()[j + 1] == b':'
            {
                let old = json[i + 1..j].parse::<usize>().ok();
                if depth >= min_depth {
                    if let Some(old) = old {
                        if let Some((_, new)) = map.iter().find(|(o, _)| *o == old) {
                            out.push('"');
                            out.push_str(&new.to_string());
                            out.push('"');
                            i = j + 1; // 冒号留给下一轮原样拷贝
                            continue;
                        }
                    }
                }
                out.push_str(&json[i..j + 2]); // 含收尾引号和冒号
                i = j + 2;
                continue;
            }
            // 普通字符串（含中文值）：整段拷贝
            let mut j = i + 1;
            while j < json.len() && json.as_bytes()[j] != b'"' {
                j += 1;
            }
            if j < json.len() {
                out.push_str(&json[i..=j]);
                i = j + 1;
            } else {
                out.push_str(&json[i..]);
                i = json.len();
            }
            continue;
        }
        let ch_len = json[i..].chars().next().map_or(1, |c| c.len_utf8());
        out.push_str(&json[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// 一次性替换 `"Normal":<old>` 里的队 id（team1/team2）。
fn json_swap_teams(json: &str, map: &[(usize, usize)]) -> String {
    const NEEDLE: &str = "\"Normal\":";
    let mut out = String::with_capacity(json.len());
    let mut i = 0usize;
    while i < json.len() {
        if json[i..].starts_with(NEEDLE) {
            let start = i + NEEDLE.len();
            let mut j = start;
            while j < json.len() && json.as_bytes()[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                if let Ok(old) = json[start..j].parse::<usize>() {
                    out.push_str(NEEDLE);
                    match map.iter().find(|(o, _)| *o == old) {
                        Some((_, new)) => out.push_str(&new.to_string()),
                        None => out.push_str(&json[start..j]),
                    }
                    i = j;
                    continue;
                }
            }
        }
        let ch_len = json[i..].chars().next().map_or(1, |c| c.len_utf8());
        out.push_str(&json[i..i + ch_len]);
        i += ch_len;
    }
    out
}

fn maybe_swap_cup_fillers(ctx: &mut StableServerCtx<'_>) {
    if !SWAP_CUP_FILLERS {
        return;
    }
    let rank = final_rank_teams(ctx);
    if rank.len() != 10 {
        return;
    }
    let mut tids = ctx.record_ids(RecordKindV1::TournamentCompetition);
    tids.sort_unstable();
    if tids.is_empty() {
        return;
    }

    struct Seat {
        tid: usize,
        teams: Vec<usize>,
        gms: Vec<usize>,
        editable: bool,
    }
    let mut seats: Vec<Seat> = Vec::new();
    let mut placed: Vec<usize> = Vec::new();
    for &tid in &tids {
        let Some(seeds_json) =
            ctx.record_get_json(RecordKindV1::TournamentCompetition, tid, "team_seeds")
        else {
            continue;
        };
        let teams = json_num_keys(&seeds_json);
        if teams.is_empty() {
            continue;
        }
        let gms = ctx
            .record_get_json(RecordKindV1::TournamentCompetition, tid, "group_matches")
            .map(|j| parse_ids(&j))
            .unwrap_or_default();
        let tms = ctx
            .record_get_json(
                RecordKindV1::TournamentCompetition,
                tid,
                "tournament_matches",
            )
            .map(|j| parse_ids(&j))
            .unwrap_or_default();
        let fin = ctx
            .record_get_json(RecordKindV1::TournamentCompetition, tid, "finalized")
            .as_deref()
            == Some("true");
        let started = fin || gms.iter().chain(tms.iter()).any(|&m| is_played(ctx, m));
        placed.extend(teams.iter().copied());
        seats.push(Seat {
            tid,
            teams,
            gms,
            editable: !started,
        });
    }
    if seats.is_empty() {
        return;
    }
    placed.sort_unstable();
    placed.dedup();

    // 候选：没进任何杯赛的真 KPL 队；先按官方最终名次，再按 team id。
    let mut order: Vec<usize> = rank.clone();
    order.extend(all_real_kpl_team_ids(ctx));
    let mut used = placed.clone();
    let mut cands: Vec<usize> = Vec::new();
    for t in order {
        if used.contains(&t) || cands.contains(&t) {
            continue;
        }
        cands.push(t);
        used.push(t);
    }

    let mut ci = 0usize;
    let mut assigned: Vec<usize> = Vec::new();
    for s in &seats {
        if !s.editable {
            assigned.extend(s.teams.iter().copied());
            continue;
        }
        // 要换掉的席位：CN 挂名队，或已经出现在前面（更高优先级）杯赛里的真队。
        let mut map: Vec<(usize, usize)> = Vec::new();
        for &t in &s.teams {
            let dup = is_real_kpl(&team_name(ctx, t)) && assigned.contains(&t);
            if !(is_cn_filler(ctx, t) || dup) {
                continue;
            }
            let Some(&new) = cands.get(ci) else {
                break;
            };
            ci += 1;
            map.push((t, new));
        }
        if map.is_empty() {
            assigned.extend(s.teams.iter().copied());
            continue;
        }

        // groups 是数组：[{team_id:{stat}}, ...]，队 id 键在第 1 层（没有小组下标键）。
        for (field, min_depth) in [
            ("team_seeds", 1usize),
            ("groups", 1usize),
            ("statistics", 1usize),
        ] {
            let Some(cur) =
                ctx.record_get_json(RecordKindV1::TournamentCompetition, s.tid, field)
            else {
                continue;
            };
            let want = json_swap_keys(&cur, &map, min_depth);
            if want != cur {
                ctx.record_set_json(RecordKindV1::TournamentCompetition, s.tid, field, &want);
            }
        }
        for &m in &s.gms {
            for field in ["team1", "team2"] {
                let Some(cur) = ctx.record_get_json(RecordKindV1::Match, m, field) else {
                    continue;
                };
                let want = json_swap_teams(&cur, &map);
                if want != cur {
                    ctx.record_set_json(RecordKindV1::Match, m, field, &want);
                }
            }
        }

        let mut now = s.teams.clone();
        for x in now.iter_mut() {
            if let Some((_, n)) = map.iter().find(|(o, _)| *o == *x) {
                *x = *n;
            }
        }
        assigned.extend(now);
    }
}

// ============================ 统一季后赛（S+A 12 队，B 组淘汰） ============================
// 第三轮（45 场）全部打完后，按第三轮的 S/A 名次排季后赛：
//   S1-S4 → 胜者组；S5/S6 → 败者组第二轮；A1-A4 → 败者组第一轮；A5/A6 淘汰。
// 胜者组第一轮败者 → 败者组第三轮；胜者组决赛败者 → 败者组决赛；胜者组冠军进总决赛。
// 引擎给每个 LeagueCompetition 固定 8 个季后赛 Match 槽（日期 = 4/16..5/3 共 8 天），
// 本 mod 接管 S/A 两个容器的 16 个槽（槽 k 固定是第 k 个季后赛日，一天一个容器一场）：
//   S 槽 0..7 = 胜者组R1a / 胜者组R1b / 败者组R2a / 胜者组R2 / 败者组R3a /
//               败者组R4 / 败者组决赛 / 总决赛
//   A 槽 0..7 = 败者组R1a / 败者组R1b / 败者组R2b / 9-10名 / 败者组R3b /
//               7-8名 / 5-6名 / 3-4名
// 依赖链保证上一轮总是早一天。A5/A6 不进季后赛（只排 11-12 名次，没有比赛）。
// 队伍引用：已知的真队写死 {"Normal":id}；上一轮没打完时写引擎自己的
// {"WinnerOf":mid} / {"LoserOf":mid}，每天 tick 都会重新算一遍（打完就换成真队）。
const UNIFIED_PLAYOFFS: bool = true;
/// 统一季后赛 16 场全部打完后，把 S 容器(comp 1)的 `playoffs` 数组重排成
/// 引擎 6 队双败模板能正确读出 1-6 名的顺序（只改数组顺序，不改任何比分）。
/// 目标槽序：[任意, 任意, #189(负者狼队), #188(负者EDG.M), 任意,
///            #193(负者JDG), #194(负者TTG), #195(RW胜/AG负)]
/// 引擎按 1=slot7胜者、2=slot7负者、3=slot6负者、4=slot5负者、
/// 5/6=slot2/slot3负者(按积分排序) 计算，因此 5/6 会变成狼队/EDG.M。
const ENCODE_S_RANKING: bool = true;
/// A组(TACC2, league 7) 和 B组(TACJ, league 5) 不再单独发奖。
/// 统一名次只在 S组(league 1) 的 prize_money 发一次，避免同一支队
/// 在两张表里各拿一份（实测 TTG 3+5、JDG 1.5+2.5、RW 10+0.8）。
/// B组 playoffs 被清空、1-6 名取不到，本来也发不出。
const ZERO_AB_PRIZES: bool = true;
/// 季后赛 16 场全部打完后，按真实比赛结果（含 7-8、9-10 名排位赛）推一条
/// 「KPL 最终名次」新闻。引擎自带的冠军新闻 7-10 名读的是积分榜，会列出
/// 挂名队；这条新闻给出权威的完整 1-10 名（1-6 双败、7-10 排位赛）。
const PUSH_RANK_NEWS: bool = true;

#[derive(Clone, Copy)]
enum PoSrc {
    Team(usize),
    Win(usize),
    Lose(usize),
}

/// 16 场季后赛蓝图（索引 = 槽位顺序：S0,A0,S1,A1,...），Win/Lose 指向蓝图索引。
fn po_plan(s: &[usize], a: &[usize]) -> Option<Vec<(PoSrc, PoSrc, &'static str)>> {
    if s.len() < 6 || a.len() < 6 {
        return None;
    }
    // 槽位顺序 = S0,A0,S1,A1,...；前四场刻意排成和引擎原生播种完全一致
    // （槽0=1v4、槽1=2v3），这样就算引擎在季后赛第一天重新按 CompetitionRank
    // 播种，改出来的对阵也和我们的相同。
    Some(vec![
        (PoSrc::Team(s[0]), PoSrc::Team(s[3]), "胜者组R1a S1vS4"),
        (PoSrc::Team(a[0]), PoSrc::Team(a[3]), "败者组R1a A1vA4"),
        (PoSrc::Team(s[1]), PoSrc::Team(s[2]), "胜者组R1b S2vS3"),
        (PoSrc::Team(a[1]), PoSrc::Team(a[2]), "败者组R1b A2vA3"),
        (PoSrc::Team(s[4]), PoSrc::Win(1), "败者组R2a S5vW(A1A4)"),
        (PoSrc::Team(s[5]), PoSrc::Win(3), "败者组R2b S6vW(A2A3)"),
        (PoSrc::Win(0), PoSrc::Win(2), "胜者组R2"),
        (PoSrc::Lose(1), PoSrc::Lose(3), "9-10名"),
        (PoSrc::Lose(0), PoSrc::Win(4), "败者组R3a"),
        (PoSrc::Lose(2), PoSrc::Win(5), "败者组R3b"),
        (PoSrc::Win(8), PoSrc::Win(9), "败者组R4"),
        (PoSrc::Lose(4), PoSrc::Lose(5), "7-8名"),
        (PoSrc::Win(10), PoSrc::Lose(6), "败者组决赛"),
        (PoSrc::Lose(8), PoSrc::Lose(9), "5-6名"),
        (PoSrc::Win(6), PoSrc::Win(12), "总决赛"),
        (PoSrc::Lose(12), PoSrc::Lose(10), "3-4名"),
    ])
}

fn comp_playoff_ids(ctx: &StableServerCtx<'_>, cid: usize) -> Option<Vec<usize>> {
    ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "playoffs")
        .map(|j| parse_ids(&j))
}

/// 改写某个 competition 的 playoffs 数组（只在内容不同时写）。
fn set_comp_playoffs(ctx: &mut StableServerCtx<'_>, cid: usize, ids: &[usize]) -> bool {
    let want = format!(
        "[{}]",
        ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
    );
    let cur = ctx
        .record_get_json(RecordKindV1::LeagueCompetition, cid, "playoffs")
        .unwrap_or_default();
    if cur.trim() == want {
        return false;
    }
    ctx.record_set_json(RecordKindV1::LeagueCompetition, cid, "playoffs", &want)
}

fn po_loser_of(ctx: &StableServerCtx<'_>, mid: usize) -> Option<usize> {
    let (w, _, _) = match_result(ctx, mid)?;
    let x = match_team(ctx, mid, "team1")?;
    let y = match_team(ctx, mid, "team2")?;
    if w == x {
        Some(y)
    } else if w == y {
        Some(x)
    } else {
        None
    }
}

fn po_ref_json(ctx: &StableServerCtx<'_>, src: PoSrc, mids: &[usize]) -> String {
    match src {
        PoSrc::Team(t) => format!("{{\"Normal\":{}}}", t),
        PoSrc::Win(i) => match match_result(ctx, mids[i]) {
            Some((w, _, _)) => format!("{{\"Normal\":{}}}", w),
            None => format!("{{\"WinnerOf\":{}}}", mids[i]),
        },
        PoSrc::Lose(i) => match po_loser_of(ctx, mids[i]) {
            Some(l) => format!("{{\"Normal\":{}}}", l),
            None => format!("{{\"LoserOf\":{}}}", mids[i]),
        },
    }
}

fn save_id_list(ctx: &StableServerCtx<'_>, key: &str) -> Option<Vec<usize>> {
    let s = ctx.save_get_string(key)?;
    let v: Vec<usize> = s
        .split(',')
        .filter_map(|x| x.trim().parse::<usize>().ok())
        .collect();
    (!v.is_empty()).then_some(v)
}

/// S组 / A组 / B组 是同一级联赛（KPL 的三个分组，不是一二级）。
/// `League.division` 决定引擎怎么看一个联赛：
///   0 = 一级联赛 → 有「一级联赛」赞助加成（first_division_bonus_bp）、
///       可签 first_division_only 的赞助商、经理董事会按保级评估；
///   1 = 二级联赛 → 与同赛区 div0 组成升降级对，赛季末打 BO5 升降级赛，
///       输的一方打「降级」标签、拿不到一级加成。
/// 原始 data_setting 里 TACC(一组/S) = div0、TACC2(二组/A) = div1，于是引擎把
/// S→A 当成真降级，给出升降级赛新闻、降级履历和赞助惩罚。
/// 这里把 TACC2 强制改回 div0（A 组也是顶级联赛），S/A/B 三组同级；
/// K甲(TACJ2) 保持 div1，B组(TACJ) 保持 div0，B↔K甲 的真升降级保留。
/// 幂等：只在不一致时写。旧存档读进来也会在第一个 tick 被纠正。
const SAME_TIER_SAB: bool = true;

/// S/A/B 同级：把各组联赛的 `division` 维持在期望值（A 组=0）。
fn enforce_same_tier(ctx: &mut StableServerCtx<'_>) {
    if !SAME_TIER_SAB {
        return;
    }
    for name in ["TACC", "TACC2", "TACJ"] {
        let Some(lid) = league_id_by_name(ctx, name) else {
            continue;
        };
        let want = "0";
        let cur = ctx
            .record_get_json(RecordKindV1::League, lid, "division")
            .unwrap_or_default();
        if cur.trim() != want {
            ctx.record_set_json(RecordKindV1::League, lid, "division", want);
        }
    }
    // 旧存档里可能已经排好了「S组后2 vs A组前2」的升降级赛。已经开打的保留
    // （不能抹掉战绩），一场未打的直接撤掉 —— 两组同级后本来就不该有这一轮。
    cancel_stale_promotion(ctx);
}

/// 清掉还没开打的 `promotion_series`（只清 S / A 两个容器）。
/// B 组(TACJ, div0) 和 K甲(TACJ2, div1) 是真正的 B↔K甲 升降级，必须保留。
/// 幂等：空数组就跳过；已开打的也跳过（不能抹掉战绩）。
/// 清空的同时把两场附加赛的日期推到 2099 并标 `promotion_series_finalized`：
/// 数组空了以后引擎不该再去索引它，日期推走则避免它们被当成欠赛补打。
fn cancel_stale_promotion(ctx: &mut StableServerCtx<'_>) {
    let Some(c) = sab_comps(ctx).or_else(|| sab_comps_all(ctx)) else {
        return;
    };
    for cid in [c.s, c.a] {
        let Some(j) =
            ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "promotion_series")
        else {
            continue;
        };
        let ids = parse_ids(&j);
        if ids.is_empty() {
            continue;
        }
        if ids.iter().any(|m| is_played(ctx, *m)) {
            continue;
        }
        ctx.record_set_json(RecordKindV1::LeagueCompetition, cid, "promotion_series", "[]");
        ctx.record_set_json(
            RecordKindV1::LeagueCompetition,
            cid,
            "promotion_series_finalized",
            "true",
        );
        for mid in ids {
            let d = ctx
                .record_get_string(RecordKindV1::Match, mid, "date")
                .unwrap_or_default();
            if !d.starts_with("2099") && !d.is_empty() {
                ctx.record_set_string(
                    RecordKindV1::Match,
                    mid,
                    "date",
                    "2099-12-31T00:00:00",
                );
            }
        }
    }
}

/// 统一季后赛：45 场打完后锁定 12 队、写 16 个槽位的对阵，之后每天重新对账。
/// A组(TACC2, league 7) / B组(TACJ, league 5) 的奖金表清零。
/// 统一名次只在 S组(league 1) 发一次，避免 S 队在 A 组表里重复拿钱。
/// 幂等：已经是 0 就不重复写。
fn zero_ab_prizes(ctx: &mut StableServerCtx<'_>) {
    if !ZERO_AB_PRIZES {
        return;
    }
    for lid in [5usize, 7usize] {
        let cur = ctx
            .record_get_json(RecordKindV1::League, lid, "prize_money")
            .unwrap_or_default();
        let t = cur.trim();
        if t == "[0.0,0.0,0.0,0.0,0.0]" || t == "[0,0,0,0,0]" || t == "[]" {
            continue;
        }
        ctx.record_set_json(
            RecordKindV1::League,
            lid,
            "prize_money",
            "[0.0,0.0,0.0,0.0,0.0]",
        );
    }
}

/// 完整 16 场季后赛赛程的落盘键（mod 存档命名空间，客户端面板读它）。
/// 每行一场，'\t' 分隔：`日期 \t 轮次名 \t team1 \t 比分1 \t 比分2 \t team2`。
/// 顺序 = 8 个季后赛日 × 2 场（S 容器槽 k + A 容器槽 k），和比赛日期一一对应。
/// 未确定的一方写 0，未打完的比分写 -1（都是客户端能识别的哨兵）。
const PO_BOARD_KEY: &str = "kpl_po_board";

/// 16 个槽位在面板/新闻里显示的轮次名（和 po_plan 的槽序一一对应）。
const PO_ROUND_CN: [&str; 16] = [
    "胜者组 R1", "败者组 R1", "胜者组 R1", "败者组 R1",
    "败者组 R2", "败者组 R2", "胜者组 R2", "9-10 名",
    "败者组 R3", "败者组 R3", "败者组 R4", "7-8 名",
    "败者组决赛", "5-6 名", "总决赛", "3-4 名",
];

fn po_day(ctx: &StableServerCtx<'_>, mid: usize) -> String {
    ctx.record_get_string(RecordKindV1::Match, mid, "date")
        .unwrap_or_default()
}

/// 比赛日期显示成 `M/D`；带时刻的（面板用）显示成 `M/D HH:MM`。
fn po_md(d: &str) -> String {
    if d.len() < 10 {
        return d.to_string();
    }
    let day = format!(
        "{}/{}",
        d[5..7].trim_start_matches('0'),
        d[8..10].trim_start_matches('0')
    );
    match d.get(11..16) {
        Some(t) if t.len() == 5 && t.as_bytes()[2] == b':' => format!("{day} {t}"),
        _ => day,
    }
}

/// 只有写成 `{"Normal":队id}` 才算已确定的队伍。`{"WinnerOf":N}` /
/// `{"LoserOf":N}` 里的 N 是**比赛 id**，`match_team`（用宽松的 parse_ids）
/// 会把它们当成队 id，显示出来就是错的，这里必须排除。
fn po_team(ctx: &StableServerCtx<'_>, mid: usize, field: &str) -> Option<usize> {
    let j = ctx.record_get_json(RecordKindV1::Match, mid, field)?;
    let p = j.find("\"Normal\"")?;
    parse_ids(&j[p..]).first().copied()
}

/// 生成「日期 + 轮次 + 双方 + 比分」的一行。未定 = 待定，未打 = 不写比分。
fn po_line(ctx: &StableServerCtx<'_>, mid: usize, label: &str, with_score: bool) -> String {
    let side = |f: &str| match po_team(ctx, mid, f) {
        Some(id) => team_name(ctx, id),
        None => "待定".to_string(),
    };
    let (a, b) = (side("team1"), side("team2"));
    let md = po_md(&po_day(ctx, mid));
    match (with_score, match_result(ctx, mid)) {
        (true, Some((_, s1, s2))) => format!("{} {} {} {}:{} {}", md, label, a, s1, s2, b),
        _ => format!("{} {} {} vs {}", md, label, a, b),
    }
}

/// 把完整 16 场赛程写进存档（客户端面板的数据源）。幂等：内容没变就不写。
fn publish_po_board(
    ctx: &mut StableServerCtx<'_>,
    po_mids: &[usize],
    plan: &[(PoSrc, PoSrc, &'static str)],
) {
    let mut out = String::from("v1\n");
    for slot in 0..16 {
        let mid = po_mids[slot];
        // 顺序：日期 \t 轮次 \t team1 \t team2 \t 比分1 \t 比分2
        let t = |f: &str| {
            po_team(ctx, mid, f)
                .map(|id| id.to_string())
                .unwrap_or_else(|| "0".to_string())
        };
        let (s1, s2) = match match_result(ctx, mid) {
            Some((_, a, b)) => (a, b),
            None => (-1, -1),
        };
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            po_day(ctx, mid),
            plan[slot].2,
            t("team1"),
            t("team2"),
            s1,
            s2
        ));
    }
    if ctx.save_get_string(PO_BOARD_KEY).as_deref() != Some(out.as_str()) {
        ctx.save_set_string(PO_BOARD_KEY, &out);
    }
}

/// 每个季后赛日 2 场都打完后推一条战报（8 天最多 8 条）。
/// 只在装好本版之后新打完的日子推送；旧存档里已经打完的历史日不补发。
fn push_po_day_news(
    ctx: &mut StableServerCtx<'_>,
    tag: usize,
    po_mids: &[usize],
    plan: &[(PoSrc, PoSrc, &'static str)],
) {
    // 第一次跑（基线程/老存档）时，把已经打完的日子直接标记成「已报」但**不推送**，
    // 否则一次读档会补发 8 条历史战报。
    let init_key = format!("po_day_init_{}", tag);
    let baseline = ctx.save_get_string(&init_key).is_none();
    if baseline {
        for k in 0..8 {
            if is_played(ctx, po_mids[2 * k]) && is_played(ctx, po_mids[2 * k + 1]) {
                ctx.save_set_string(&format!("po_day_{}_{}", tag, k), "old");
            }
        }
        ctx.save_set_string(&init_key, "1");
    }
    for k in 0..8 {
        let (m1, m2) = (po_mids[2 * k], po_mids[2 * k + 1]);
        if !is_played(ctx, m1) || !is_played(ctx, m2) {
            break; // 日期是顺序推进的，本日没打完，后面的更没打完
        }
        let key = format!("po_day_{}_{}", tag, k);
        if ctx.save_get_string(&key).is_some() {
            continue;
        }
        let content = format!(
            "KPL 季后赛战报（{}）：\n{}\n{}\n完整 16 场对阵与实时比分：底部栏「季后赛」按钮。",
            po_day(ctx, m1),
            po_line(ctx, m1, plan[2 * k].2, true),
            po_line(ctx, m2, plan[2 * k + 1].2, true),
        );
        for tid in all_real_kpl_team_ids(ctx) {
            ctx.news_push(tid, "KPL 季后赛战报", &content, "KPL");
        }
        ctx.save_set_string(&key, "1");
    }
}

fn maybe_playoffs(ctx: &mut StableServerCtx<'_>) {
    if !UNIFIED_PLAYOFFS {
        return;
    }
    // 赛季末 A/B 容器会被本函数强制 finalize，之后 sab_comps（只看未 finalize
    // 的容器）会返回 None——不能因此就整段跳过，否则名次编码和最终名次新闻
    // 永远推不出去。
    let Some(c) = sab_comps(ctx).or_else(|| sab_comps_all(ctx)) else {
        return;
    };
    let (Some(s_mids), Some(a_mids)) = (comp_mids(ctx, c.s), comp_mids(ctx, c.a)) else {
        return;
    };
    if s_mids.len() < 45 || a_mids.len() < 45 {
        return;
    }
    if !s_mids[..45].iter().all(|m| is_played(ctx, *m))
        || !a_mids[..45].iter().all(|m| is_played(ctx, *m))
    {
        return;
    }
    let tag = s_mids[0];
    // 槽位 id 第一次读到就落盘：合并后 A 容器的 playoffs 会被清空，
    // 之后再读就是 0 个，必须靠这里保存的原始 8 个 id。
    let sids_key = format!("po_sids_{}", tag);
    let aids_key = format!("po_aids_{}", tag);
    let (s_po, a_po) = match (save_id_list(ctx, &sids_key), save_id_list(ctx, &aids_key)) {
        (Some(s), Some(a)) if s.len() >= 8 && a.len() >= 8 => (s, a),
        _ => {
            let (Some(s), Some(a)) =
                (comp_playoff_ids(ctx, c.s), comp_playoff_ids(ctx, c.a))
            else {
                return;
            };
            if s.len() < 8 || a.len() < 8 {
                return;
            }
            let join = |v: &[usize]| {
                v.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            };
            ctx.save_set_string(&sids_key, &join(&s));
            ctx.save_set_string(&aids_key, &join(&a));
            (s, a)
        }
    };
    let sk = format!("po_s_{}", tag);
    let ak = format!("po_a_{}", tag);
    let snapshot = match (save_id_list(ctx, &sk), save_id_list(ctx, &ak)) {
        (Some(s), Some(a)) if s.len() >= 6 && a.len() >= 6 => (s, a),
        _ => {
            let s: Vec<usize> = standings_ranked(ctx, c.s).iter().map(|r| r.team).collect();
            let a: Vec<usize> = standings_ranked(ctx, c.a).iter().map(|r| r.team).collect();
            if s.len() < 6 || a.len() < 6 {
                return;
            }
            let join = |v: &[usize]| {
                v.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            };
            ctx.save_set_string(&sk, &join(&s));
            ctx.save_set_string(&ak, &join(&a));
            (s, a)
        }
    };
    let (s_rank, a_rank) = snapshot;
    let Some(plan) = po_plan(&s_rank, &a_rank) else {
        return;
    };
    let mut po_mids: Vec<usize> = Vec::with_capacity(16);
    for k in 0..8 {
        po_mids.push(s_po[k]);
        po_mids.push(a_po[k]);
    }

    // 注意：引擎每个比赛容器每天只打「playoffs[round]」一场（YearSchedule 的
    // LeaguePlayoff round 0..7），把一个容器塞 16 场只有前 8 场会被排进赛程。
    // 所以 S/A 两个容器各保持原生 8 个槽（一共 16 场）——这是 12:06 版实测跑通
    // 的结构。只在文本层把两个容器的季后赛都显示成「KPL 季后赛」。
    // B 组第二轮后全淘汰：它的 8 个季后赛槽清空，不再打自己的季后赛。
    set_comp_playoffs(ctx, c.b, &[]);

    // 完整 16 场赛程落盘（客户端「季后赛」面板的数据源）+ 每日战报新闻。
    // 排在改写对阵之前：先发布当前状态，下一步写对阵后下一 tick 自然刷新。
    publish_po_board(ctx, &po_mids, &plan);
    push_po_day_news(ctx, tag, &po_mids, &plan);

    let built_key = format!("po_built_{}", tag);
    let first = ctx.save_get_string(&built_key).is_none();
    // 完整赛程新闻单独一个键：升级到本版后当前赛季也会补推一次。
    let news_key = format!("po_full_news_{}", tag);
    let first_news = ctx.save_get_string(&news_key).is_none();
    for i in 0..16 {
        let mid = po_mids[i];
        let (x, y, _label) = plan[i];
        if is_played(ctx, mid) {
            continue;
        }
        let jx = po_ref_json(ctx, x, &po_mids);
        let jy = po_ref_json(ctx, y, &po_mids);
        for (field, want) in [("team1", &jx), ("team2", &jy)] {
            let cur = ctx
                .record_get_json(RecordKindV1::Match, mid, field)
                .unwrap_or_default();
            if cur.trim() != want.trim() {
                ctx.record_set_json(RecordKindV1::Match, mid, field, want);
            }
        }
    }
    // 16 场全部结束后 A/B 容器已经没有季后赛了，强制 finalize，
    // 否则赛季可能卡在「联赛未结束」不往下走。
    if po_mids.iter().all(|m| is_played(ctx, *m)) {
        for cid in [c.a, c.b] {
            let fin = ctx
                .record_get_json(RecordKindV1::LeagueCompetition, cid, "finalized")
                .unwrap_or_default();
            if fin.trim() != "true" {
                ctx.record_set_json(RecordKindV1::LeagueCompetition, cid, "finalized", "true");
            }
        }
        // 名次编码：引擎每个容器按 6 队双败模板算名次，我们的槽序从 5 名
        // 开始对不上。16 场都 End 后把 S 容器的 8 个槽重排成引擎模板
        // 的语义顺序（全部是已打完的真实比赛，比分不动）：
        //   slot0/1/4 = #190/#191/#192（不影响名次）
        //   slot2 = #189（负者狼队，rank5）
        //   slot3 = #188（负者EDG.M，rank6）
        //   slot5 = #193（负者JDG，rank4）
        //   slot6 = #194（负者TTG，rank3）
        //   slot7 = #195（RW胜AG，rank1/2）
        // 引擎实际读法：1=slot7胜者、2=slot7负者、3=slot6负者、
        // 4=slot5负者、5/6=slot2/slot3负者按积分排序。
        if ENCODE_S_RANKING {
            let want = [
                po_mids[4], po_mids[6], po_mids[2], po_mids[0],
                po_mids[8], po_mids[10], po_mids[12], po_mids[14],
            ];
            set_comp_playoffs(ctx, c.s, &want);
        }
        // 最终名次新闻：全部 16 场已 End，按真实结果取 1-10 名。
        // 顺序 = 总决赛(1/2)、3-4名赛、5-6名赛、7-8名赛、9-10名赛。
        if PUSH_RANK_NEWS {
            let news_key = format!("rank_news_{}", tag);
            if ctx.save_get_string(&news_key).is_none() {
                let pairs = [
                    po_mids[14], po_mids[15], po_mids[13], po_mids[11], po_mids[7],
                ];
                let mut lines = String::new();
                let mut rank = 1usize;
                let mut complete = true;
                for mid in pairs {
                    let (Some((w, _, _)), Some(l)) =
                        (match_result(ctx, mid), po_loser_of(ctx, mid))
                    else {
                        complete = false;
                        break;
                    };
                    lines.push_str(&format!("{}. {}\n", rank, team_name(ctx, w)));
                    lines.push_str(&format!("{}. {}\n", rank + 1, team_name(ctx, l)));
                    rank += 2;
                }
                if complete && rank == 11 {
                    let content = format!(
                        "KPL 2026 春季赛统一季后赛最终名次：\n{}1-6 名由双败季后赛决定，7-10 名由排位赛决定。\n以上为 KPL 官方最终名次。",
                        lines
                    );
                    let recipients = all_real_kpl_team_ids(ctx);
                    for &tid in &recipients {
                        ctx.news_push(tid, "KPL 最终名次", &content, "KPL");
                    }
                    ctx.save_set_string(&news_key, "1");
                }
            }
        }
    }

    if first {
        ctx.save_set_string(&built_key, "1");
    }
    if first_news {
        ctx.save_set_string(&news_key, "1");
        // 完整 16 场对阵（全部轮次 + 日期），不是摘要。第一轮对阵已知，
        // 后续轮次写「胜者/负者」，打完由每日战报补齐。
        let mut bracket = String::new();
        for slot in 0..16 {
            bracket.push_str(&po_line(ctx, po_mids[slot], PO_ROUND_CN[slot], false));
            bracket.push('\n');
        }
        let content = format!(
            "KPL 统一季后赛完整赛程（16 场，全部 BO7，S/A 合流，A5/A6 淘汰）：\n\
             胜者组：S1 {}、S2 {}、S3 {}、S4 {}\n\
             败者组：S5 {}、S6 {}、A1 {}、A2 {}、A3 {}、A4 {}\n\
             \n{}",
            team_name(ctx, s_rank[0]),
            team_name(ctx, s_rank[1]),
            team_name(ctx, s_rank[2]),
            team_name(ctx, s_rank[3]),
            team_name(ctx, s_rank[4]),
            team_name(ctx, s_rank[5]),
            team_name(ctx, a_rank[0]),
            team_name(ctx, a_rank[1]),
            team_name(ctx, a_rank[2]),
            team_name(ctx, a_rank[3]),
            bracket
        );
        for tid in all_real_kpl_team_ids(ctx) {
            ctx.news_push(tid, "KPL 统一季后赛完整赛程", &content, "KPL");
        }
    }
}

// ============================ SAB 实验：6 真队三轮 ============================
// 结构：9 轮 × 5 槽 = 45 场 = 6 队的 3 个完整单循环（每循环 15 场）。
// YearSchedule 是全局一份、每轮固定 5 个 LeagueMatch 槽，Spring 裁到 9 轮后正好 45 个
// 槽位 —— 和「3 个单循环」严丝合缝。所以日历一格都不用动：把 45 个槽位全部改写成
// 6 真队之间的配对，4 个挂名队就一场不打（0-0），「借挂名比赛」这一招不再需要。
//
// 本版是**实验版**，要验证三件事：
//   ① 同 competition 内改写 team1/team2 会不会卡死（坑#7 是否只发生在跨 competition）；
//   ② 一个队在某轮打 2 场（6 队填 5 槽必然发生）引擎认不认；
//   ③ 0-0 挂名队赛季末会不会被 promotion_series 拉进去。
// 结果：kpl_sab_r6_log.txt（改写结果）、kpl_sab_r6.txt（改写后赛程 + 场次统计）、
//       kpl_sab_groups.txt（积分，含胜负）。
const R6_EXPERIMENT: bool = true;

/// 真 KPL 队名白名单 = roster_for 里有名单的 18 队。
fn is_real_kpl(name: &str) -> bool {
    roster_for(name).is_some()
}

/// 6 队圆桌法单循环：5 轮 × 3 场 = 15 组配对（"list[0] 固定、其余右旋"）。
fn rr_rounds6(teams: &[usize]) -> Vec<Vec<(usize, usize)>> {
    let n = teams.len();
    let mut list = teams.to_vec();
    let mut rounds = Vec::with_capacity(n - 1);
    for _ in 0..n - 1 {
        let mut pairs = Vec::with_capacity(n / 2);
        for i in 0..n / 2 {
            pairs.push((list[i], list[n - 1 - i]));
        }
        rounds.push(pairs);
        let last = list.pop().unwrap();
        list.insert(1, last);
    }
    rounds
}

/// 3 个单循环拍平成 45 组配对。圆桌法里 list[0] 每轮都排在第 1 对，所以拍平后它落在
/// index 0,3,6,...，每 5 槽最多出现 2 次 —— 不会出现一个队一轮打 3 场。
fn three_rr_pairs(teams: &[usize]) -> Vec<(usize, usize)> {
    let rounds = rr_rounds6(teams);
    let mut flat = Vec::new();
    for _ in 0..3 {
        for r in &rounds {
            for &p in r {
                flat.push(p);
            }
        }
    }
    flat
}

/// 联赛里有名单的真 KPL 队（从 standings 的队 id 里按队名白名单挑）。
fn real_teams_in(ctx: &StableServerCtx<'_>, cid: usize) -> Vec<usize> {
    let mut out = Vec::new();
    if let Some(j) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "standings") {
        for r in parse_standings(&j) {
            if is_real_kpl(&team_name(ctx, r.team)) {
                out.push(r.team);
            }
        }
    }
    out.sort_unstable();
    out
}

fn match_team(ctx: &StableServerCtx<'_>, mid: usize, field: &str) -> Option<usize> {
    ctx.record_get_json(RecordKindV1::Match, mid, field)
        .and_then(|j| parse_ids(&j).first().copied())
}

fn is_played(ctx: &StableServerCtx<'_>, mid: usize) -> bool {
    ctx.record_get_json(RecordKindV1::Match, mid, "running_state")
        .map(|s| s.contains("End"))
        .unwrap_or(false)
}

fn set_match_teams(ctx: &mut StableServerCtx<'_>, mid: usize, a: usize, b: usize) -> bool {
    let ok1 = ctx.record_set_json(RecordKindV1::Match, mid, "team1", &format!("{{\"Normal\":{}}}", a));
    let ok2 = ctx.record_set_json(RecordKindV1::Match, mid, "team2", &format!("{{\"Normal\":{}}}", b));
    ok1 && ok2
}

/// 把 CN S/A/B 三个联赛的 45 场全部改写成「6 真队 × 3 个单循环」。幂等：配对由
/// standings 里的 6 真队决定，重复调用结果相同；前 45 场里已有打完的（旧档）就整联赛跳过。
fn make_sab_r6(ctx: &mut StableServerCtx<'_>) {
    if !R6_EXPERIMENT {
        return;
    }
    for cid in cn_d1_competitions(ctx) {
        if comp_league_name(ctx, cid).as_deref() == Some("TACJ2") {
            continue; // K甲 不动，保持 10 队单循环
        }
        let Some(mj) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "matches") else {
            continue;
        };
        let mids = parse_ids(&mj);
        let real = real_teams_in(ctx, cid);
        if mids.len() < 45 || real.len() != 6 {
            continue;
        }
        if mids[..45].iter().any(|m| is_played(ctx, *m)) {
            continue;
        }
        for (i, &(a, b)) in three_rr_pairs(&real).iter().enumerate() {
            set_match_teams(ctx, mids[i], a, b);
        }
    }
}

// ============================ 每轮积分清零（轮次边界） ============================
// KPL SAB 的语义是「每轮战绩重算」：一轮打完，名次被记录，然后所有队回到 0-0
// 进入下一轮（第三轮的最终名次才是赛季名次）。
// 引擎的 `LeagueCompetition.standings` 是整季累加的（3 个单循环 = 每队 15 场，
// 实测 S组 6 队各 6-9/13-2 这种），所以必须在轮次边界把它清零。
// 边界 = 场上打完的场次数跨过 15 的整数倍（一个 6 队单循环 = 15 场，9 轮 × 5 槽
// 里前 15 场就是第一轮）。清零前先把那张表**快照**下来（= 该轮最终名次，
// 重分组和季后赛要用），快照存在 mod 存档命名空间里，跨读档不丢。
//
// ⚠️ 这同时是一个**安全性实验**：坑#7 记的是「改 standings 会卡死」，但那次是
// 跨 competition 的假说。这里只在同一个 competition 自己的 standings 上写，
// 且只在轮次边界写一次（幂等）。若卡死，把 ROUND_RESET 改成 false 重编即可。
const ROUND_RESET: bool = true;
/// 一个 6 队单循环 = 15 场。
const RR_PER_ROUND: usize = 15;
/// 45 场 / 15 = 3 轮。
const RR_TOTAL_ROUNDS: usize = 3;

/// 该 competition 前 45 场里已打完（running_state 含 "End"）的场次数。
fn rr_played(ctx: &StableServerCtx<'_>, mids: &[usize]) -> usize {
    mids.iter().filter(|m| is_played(ctx, **m)).count()
}

/// standings 按名次排序（胜场 → set 差 → set 胜 → team id）。
fn standings_ranked(ctx: &StableServerCtx<'_>, cid: usize) -> Vec<StandRow> {
    let json = ctx
        .record_get_json(RecordKindV1::LeagueCompetition, cid, "standings")
        .unwrap_or_default();
    let mut rows = parse_standings(&json);
    rows.sort_by(|x, y| {
        y.win
            .cmp(&x.win)
            .then((y.set_win - y.set_lose).cmp(&(x.set_win - x.set_lose)))
            .then(y.set_win.cmp(&x.set_win))
            .then(x.team.cmp(&y.team))
    });
    rows
}

/// 把这个 competition 的 standings 全部清零（保留原 key 集合 = 不动参赛队）。
fn zero_standings(ctx: &mut StableServerCtx<'_>, cid: usize) -> bool {
    let json = ctx
        .record_get_json(RecordKindV1::LeagueCompetition, cid, "standings")
        .unwrap_or_default();
    let mut rows = parse_standings(&json);
    if rows.is_empty() {
        return false;
    }
    rows.sort_by_key(|r| r.team);
    let body = rows
        .iter()
        .map(|r| {
            format!(
                "\"{}\":{{\"assist\":0,\"death\":0,\"kill\":0,\"lose\":0,\"set_lose\":0,\"set_win\":0,\"win\":0}}",
                r.team
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    ctx.record_set_json(
        RecordKindV1::LeagueCompetition,
        cid,
        "standings",
        &format!("{{{}}}", body),
    )
}

/// 轮次边界检查：每打满 15 场进入下一轮 → 快照旧表 + 清零新轮。
/// 幂等：当前轮号记在 mod 存档 `rr_round_{league_type}_{cid}`，重复调用/反复读档
/// 都不会重复清（只在 round 号变大时动作一次）。首次见到（无 key）只记基线、不清，
/// 这样旧档/半截档的累加积分不会被误清。
fn maybe_reset_rounds(ctx: &mut StableServerCtx<'_>) {
    if !ROUND_RESET {
        return;
    }
    for cid in cn_d1_competitions(ctx) {
        let name = comp_league_name(ctx, cid).unwrap_or_else(|| "?".into());
        if name == "TACJ2" {
            continue; // K甲 是 10 队单循环，没有轮次
        }
        // 重分组后容器里会有「外组队」的 standings 行，真队数会 >6；只要不少于 6
        // 就仍是 R6 版式。旧档 8/3/7 版式（真队 8/3/7）只有 3~8，靠 <6 挡掉 3 队那组。
        if real_teams_in(ctx, cid).len() < 6 {
            continue;
        }
        let Some(mj) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "matches") else {
            continue;
        };
        let mids: Vec<usize> = parse_ids(&mj).into_iter().take(45).collect();
        if mids.len() < 45 {
            continue;
        }
        // 赛季标记 = 本容器第一场比赛 id（每赛季唯一）。用它做 key，避免下赛季
        // 复用同一个 cid 时 rr_round 已是 2、新赛季永远不清零。
        let season_tag = mids[0];
        let round = rr_played(ctx, &mids) / RR_PER_ROUND; // 0=第一轮中, 1=第二轮中...
        let key = format!("rr_round_{}", season_tag);
        let Some(prev_s) = ctx.save_get_string(&key) else {
            ctx.save_set_string(&key, &round.to_string()); // 基线：不清
            continue;
        };
        let prev: usize = prev_s.trim().parse().unwrap_or(round);
        if round <= prev {
            continue;
        }
        // ① 快照刚打完的第 prev+1 轮最终名次（清零前的表就是它）
        let ranked = standings_ranked(ctx, cid);
        let snap = ranked
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}.{}({}-{})", i + 1, team_name(ctx, r.team), r.win, r.lose))
            .collect::<Vec<_>>()
            .join(" >> ");
        ctx.save_set_string(&format!("rr_snap_{}_{}", season_tag, prev + 1), &snap);
        // ①b 机器可读的本轮名次（只含真正打了这轮的 6 队：15 场 = 每队 5 场）。
        // 重分组要用它 —— 清零之后 standings 就没了，且容器里还有挂名/外组 0-0 行。
        let rank_ids = ranked
            .iter()
            .filter(|r| r.win + r.lose >= 5)
            .map(|r| r.team.to_string())
            .collect::<Vec<_>>()
            .join(",");
        ctx.save_set_string(&format!("rr_rank_{}_{}", season_tag, prev + 1), &rank_ids);
        // ② 第三轮打完 = 常规赛结束：名次要留给赛季末升降级，**不能清零**。
        //    第 1、2 轮边界（15/30 场）才清零进入下一轮；45 场只快照。
        if round >= RR_TOTAL_ROUNDS {
            ctx.save_set_string(&format!("rr_done_{}", season_tag), "1");
            ctx.save_set_string(&key, &round.to_string());
            continue;
        }
        // ③ 清零进入新一轮
        let ok = zero_standings(ctx, cid);
        ctx.save_set_string(
            &format!("rr_zero_{}_{}", season_tag, round),
            if ok { "ok" } else { "FAIL" },
        );
        ctx.save_set_string(&key, &round.to_string());
    }
}

// ============================ 重分组（实验） ============================
// 三轮之间的两次重分组：
//   ① 15 场后（第 1 轮结束）：每组前2→新S、3-4→新A、5-6→新B（纯名次，全量重排）
//   ② 30 场后（第 2 轮结束）：S后2↔A前2、A后2↔B前2（卡位赛，只动 4 队）
// 新组的 15 场写进对应角色容器的下一段槽位（15-29 / 30-44）。
//
// ⚠️ 这是本 mod 第一次把**别的 competition 的队**放进一个容器的 matches 数组。
// 坑#7 的卡死只验证过跨 competition 改 standings；这里只改 Match.team1/team2
// （同容器内改写已验证安全，R6 45/45），standings 交给引擎自己按比赛结果生长。
// 真正要观察的是：引擎给外组队记分时会不会卡死 / 会不会新增 standings 行。
//
// 三组进度不同步（B 组日历比 S/A 晚 4 天），而重分组需要三组同轮名次，所以
// 赛季初会把 A/B 日期对齐到 S、并把第 2/3 段后移 REGROUP_GAP_DAYS 天，
// 保证「三组都过边界 → 有缓冲时间重写下一段 → 下一段才开打」。
const REG_GROUP_EXPERIMENT: bool = true;
const REGROUP_GAP_DAYS: i64 = 5;

/// 第二轮结束用**真打的 BO7 卡位赛**决定第三轮分组（而不是直接按名次搬）。
///   ① 30 场边界先按名次写一套「临时第三轮分组」（保证第三轮 3/17 能正常开打），
///   ② 同时从 2099 孤儿 Match 里借 4 场，改成 BO7（need_win=4）排在 3/12–3/15，
///      追加到 S 容器的 matches 数组尾部（索引 45..48，不影响 45 场正赛），
///   ③ 4 场都 End 后从 Match.running_state.End 里读 winner/比分，
///      若第三轮一场未打就重写 30-44 槽位 + 搬队 + 重写积分表；否则保留临时分组。
const CARD_MATCHES: bool = true;
const CARD_NEED_WIN: i64 = 4;

/// 已打完的轮次 Match 标记 is_practice=true，尝试让联赛/队伍赛程页不再显示
/// 旧组对阵（若引擎不过滤，至少不影响比赛结果）。
const HIDE_PLAYED_PHASES: bool = true;

/// 积分表只保留 6 真队（删掉 4 个挂名队行）。若引擎因缺行出问题，改成 false
/// 并把旧 DLL 换回来即可。
const STRIP_FILLERS: bool = true;

/// 常规赛 45 场打完后，把 4 个挂名队补回积分表（引擎季后赛 8 队签表需要 8 行）。
/// 赛季中（含第三轮）积分表始终只有 6 真队。
const RESTORE_FILLERS_AFTER_REGULAR: bool = true;

/// 重分组时是否把队伍**真正搬进新联赛**（路 B）：
///   - 改 `Team.league_id`（队页面/联赛归属跟着变）
///   - 重写三个容器的 `standings` key 集合（积分表显示新组的 6 真队 + 原 4 挂名）
/// 只改 Match 的「路 A」UI 会一直显示老分组，所以默认关着，等跨 competition
/// 比赛实测通过后再开。开了之后每轮分组会保留到下赛季（make_sab_r6 按 league_id 走）。
const MOVE_TEAMS_EXPERIMENT: bool = true;

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(y) { 29 } else { 28 },
        _ => 30,
    }
}

fn add_days(mut y: i64, mut m: i64, mut d: i64, mut n: i64) -> (i64, i64, i64) {
    while n > 0 {
        d += 1;
        if d > days_in_month(y, m) {
            d = 1;
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
        n -= 1;
    }
    // 负数必须支持：卡位赛日期 = 第三轮首场 - 4 天。旧实现 while n>0 直接跳过，
    // 导致 -4 天变成当天 → 卡位赛排到第三轮同一天（3/17），只打了 1 场就被放弃。
    while n < 0 {
        d -= 1;
        if d < 1 {
            m -= 1;
            if m < 1 {
                m = 12;
                y -= 1;
            }
            d = days_in_month(y, m);
        }
        n += 1;
    }
    (y, m, d)
}

fn day_ymd(date: &str) -> Option<(i64, i64, i64)> {
    let dp = date.split('T').next()?;
    let mut it = dp.split('-');
    Some((
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
    ))
}

fn shift_day(date: &str, n: i64) -> String {
    match day_ymd(date) {
        Some((y, m, d)) => {
            let (y2, m2, d2) = add_days(y, m, d, n);
            format!("{:04}-{:02}-{:02}", y2, m2, d2)
        }
        None => date.to_string(),
    }
}

struct SabComps {
    s: usize,
    a: usize,
    b: usize,
}

/// 当前赛季活跃的 S/A/B 三个 competition id（TACC/TACC2/TACJ）。
fn sab_comps(ctx: &StableServerCtx<'_>) -> Option<SabComps> {
    let (mut s, mut a, mut b) = (None, None, None);
    for cid in cn_d1_competitions(ctx) {
        match comp_league_name(ctx, cid).as_deref() {
            Some("TACC") => s = Some(cid),
            Some("TACC2") => a = Some(cid),
            Some("TACJ") => b = Some(cid),
            _ => {}
        }
    }
    Some(SabComps { s: s?, a: a?, b: b? })
}

/// 同上，但不看 finalized。16 场季后赛打完后本 mod 会强制 finalize A/B 容器，
/// 之后 `cn_d1_competitions`（只收未 finalize 的）里就没有 A/B 了，
/// `sab_comps` 随即返回 None。同名的只取 id 最大的（最新一届）。
fn sab_comps_all(ctx: &StableServerCtx<'_>) -> Option<SabComps> {
    let (mut s, mut a, mut b) = (None, None, None);
    for cid in ctx.record_ids(RecordKindV1::LeagueCompetition) {
        let slot = match comp_league_name(ctx, cid).as_deref() {
            Some("TACC") => &mut s,
            Some("TACC2") => &mut a,
            Some("TACJ") => &mut b,
            _ => continue,
        };
        *slot = Some((*slot).map_or(cid, |old: usize| old.max(cid)));
    }
    Some(SabComps { s: s?, a: a?, b: b? })
}

fn comp_mids(ctx: &StableServerCtx<'_>, cid: usize) -> Option<Vec<usize>> {
    let mj = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "matches")?;
    Some(parse_ids(&mj))
}

/// 赛季初（三组一场未打）重排三组日期：
///   - 每个容器每天只打 1 场（原来 5 槽挤在 3 天里，会出现同队一天两赛）
///   - 第 1/2/3 段之间各休 REGROUP_GAP_DAYS 天，给重分组留缓冲 + 给队伍休息
///   - 三个容器用同一天（S 组第 1 场的日期为基准），只是开赛时刻错开
/// 幂等：日期直接从 S 组第 1 场的日期推出来，重复执行结果相同。
fn maybe_normalize_dates(ctx: &mut StableServerCtx<'_>) {
    if !REG_GROUP_EXPERIMENT {
        return;
    }
    let Some(c) = sab_comps(ctx) else { return };
    let mut mids: Vec<Vec<usize>> = Vec::new();
    for cid in [c.s, c.a, c.b] {
        let Some(v) = comp_mids(ctx, cid) else { return };
        if v.len() < 45 {
            return;
        }
        // 只对新赛季（一场未打）做，避免把进行中的日程打乱
        if v[..45].iter().any(|m| is_played(ctx, *m)) {
            return;
        }
        mids.push(v);
    }
    let Some(base_date) = ctx.record_get_string(RecordKindV1::Match, mids[0][0], "date") else {
        return;
    };
    if base_date.is_empty() {
        return;
    }
    let base_day = base_date.split('T').next().unwrap_or(&base_date).to_string();

    // 每个容器的开赛时刻（保持原版的黄金档：A 下午、S 傍晚、B 晚上）
    let times = ["17:00:00", "14:00:00", "21:00:00"];
    for (idx, _label) in [(0usize, "S"), (1, "A"), (2, "B")] {
        for i in 0..45 {
            let seg = (i / 15) as i64;
            let offset = i as i64 + seg * REGROUP_GAP_DAYS;
            let want = format!("{}T{}", shift_day(&base_day, offset), times[idx]);
            let mid = mids[idx][i];
            let cur = ctx
                .record_get_string(RecordKindV1::Match, mid, "date")
                .unwrap_or_default();
            if want != cur {
                ctx.record_set_string(RecordKindV1::Match, mid, "date", &want);
            }
        }
    }
}

/// 读某容器第 n 轮保存下来的「真实参赛 6 队名次」（逗号分隔 team id）。
fn saved_rank(ctx: &StableServerCtx<'_>, cid: usize, round: usize) -> Option<Vec<usize>> {
    let mids = comp_mids(ctx, cid)?;
    let tag = mids.first()?;
    let s = ctx.save_get_string(&format!("rr_rank_{}_{}", tag, round))?;
    let v: Vec<usize> = s
        .split(',')
        .filter_map(|x| x.trim().parse::<usize>().ok())
        .collect();
    (v.len() >= 6).then_some(v)
}

/// 按规则算出新 S/A/B 各 6 队（ranks 的输入顺序 = [S容器, A容器, B容器的名次]）。
fn new_groups(
    round: usize,
    s: &[usize],
    a: &[usize],
    b: &[usize],
) -> Option<([usize; 6], [usize; 6], [usize; 6])> {
    if s.len() < 6 || a.len() < 6 || b.len() < 6 {
        return None;
    }
    Some(if round == 1 {
        // 前三组的前2/中2/后2 → 新 S/A/B
        (
            [s[0], s[1], a[0], a[1], b[0], b[1]],
            [s[2], s[3], a[2], a[3], b[2], b[3]],
            [s[4], s[5], a[4], a[5], b[4], b[5]],
        )
    } else {
        // S后2↔A前2、A后2↔B前2
        (
            [s[0], s[1], s[2], s[3], a[0], a[1]],
            [a[2], a[3], s[4], s[5], b[0], b[1]],
            [b[2], b[3], b[4], b[5], a[4], a[5]],
        )
    })
}

/// 把 6 队单循环的 15 场按顺序写进 mids[start .. start+15]。
fn write_segment(
    ctx: &mut StableServerCtx<'_>,
    mids: &[usize],
    start: usize,
    teams: &[usize; 6],
) -> (usize, usize) {
    let flat: Vec<(usize, usize)> = rr_rounds6(teams).into_iter().flatten().collect();
    let mut ok = 0usize;
    for (k, &(x, y)) in flat.iter().enumerate().take(15) {
        if start + k < mids.len() && set_match_teams(ctx, mids[start + k], x, y) {
            ok += 1;
        }
    }
    (ok, flat.len().min(15))
}

/// 联赛名 → League id（TACC=1 / TACC2=7 / TACJ=5 ...），从 League 记录动态查，
/// 不硬编码，免得 data_setting 改顺序后搬错。
fn league_id_by_name(ctx: &StableServerCtx<'_>, name: &str) -> Option<usize> {
    for lid in ctx.record_ids(RecordKindV1::League) {
        if ctx
            .record_get_string(RecordKindV1::League, lid, "name")
            .as_deref()
            == Some(name)
        {
            return Some(lid);
        }
    }
    None
}

// ============================ 每档第一轮抽签 ============================
// 旧做法：分组在 data_setting 生成时就固定（gen_kpl_mod.js shuffle 一次），
// 每一档开赛后再也不会重抽。新做法：每档联赛刚建好、一场未打时，在存档里
// 把 18 支真队随机抽成 3 组 × 6（12 个挂名队 4×3 跟着走），并 push 新闻。
//   - 新档进档（首个 Spring）抽一次；
//   - 夏季赛开赛前（6/22 LeagueScheduleCreate{Summer} + StartRecruit 转会期
//     开始）再抽一次。
// 幂等：按赛季首场比赛 id 存 `draw_done_*`，只在「三容器前 45 场全都没打」时动作。
const DRAW_GROUPS: bool = true;

/// 12 个 CN 挂名队（非 KPL 真队），每组固定 4 个，不参与抽签。
const FILLER_TEAM_NAMES: &[&str] = &[
    "武汉GK", "深圳TCG", "杭州ZQ", "上海DSM",
    "南昌RF", "苏州LSG", "厦门VG", "重庆YTG",
    "成都MT", "南京HZ", "北京SDG", "天津TE",
];

fn is_filler_team(name: &str) -> bool {
    FILLER_TEAM_NAMES.contains(&name)
}

/// 抽签用的小 RNG（xorshift64*）。没有外部 rand 依赖，用系统时间 + 进程 id 播种。
struct DrawRng(u64);

impl DrawRng {
    fn new() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self((nanos ^ ((std::process::id() as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

/// 10 队单循环（9 轮 × 5 场 = 45 场），K甲 在抽签归位后重排赛程用。
fn rr_all_pairs(teams: &[usize]) -> Vec<(usize, usize)> {
    let n = teams.len();
    if n < 2 || n % 2 != 0 {
        return Vec::new();
    }
    let mut list = teams.to_vec();
    let mut out = Vec::with_capacity(n * (n - 1) / 2);
    for _ in 0..n - 1 {
        for i in 0..n / 2 {
            out.push((list[i], list[n - 1 - i]));
        }
        let last = list.pop().unwrap();
        list.insert(1, last);
    }
    out
}

/// 改 League.full_name 指向的 i18n key（只动显示名，不动 League.name —— 内部
/// 识别/杯赛种子都靠 name/region_id）。
fn set_league_display(ctx: &mut StableServerCtx<'_>, league_name: &str, text_key: &str) {
    if let Some(lid) = league_id_by_name(ctx, league_name) {
        let _ = ctx.record_set_string(
            RecordKindV1::League,
            lid,
            "full_name",
            &format!("#asset/base/text/ui?league.{}", text_key),
        );
    }
}

/// 第一轮：三个容器显示「一组 / 二组 / 三组」。
fn set_round1_display(ctx: &mut StableServerCtx<'_>) {
    set_league_display(ctx, "TACC", "kplg1");
    set_league_display(ctx, "TACC2", "kplg2");
    set_league_display(ctx, "TACJ", "kplg3");
}

/// 第二轮起：显示「S组 / A组 / B组」。季后赛名由 i18n 的
/// competition_name.league_*_playoff = "KPL 季后赛" 统一提供。
fn set_sab_display(ctx: &mut StableServerCtx<'_>) {
    set_league_display(ctx, "TACC", "tacc");
    set_league_display(ctx, "TACC2", "tacc2");
    set_league_display(ctx, "TACJ", "tacj");
}

/// 纯显示层同步：按当前轮次把三容器的 `League.full_name` 对齐成
/// 「一组/二组/三组」（第一轮，<15 场）或「S组/A组/B组」（第二轮起）。
/// 不改 League.name / 赛区 / 比赛 / 积分，只改 UI 文本引用。
/// 在开服/读档、每档开赛时调用 —— 即使引擎只在读档时解析一次文本，
/// 重进存档后显示的也一定是当前轮次该有的名字。
fn sync_display_names(ctx: &mut StableServerCtx<'_>) {
    let Some(c) = sab_comps(ctx) else { return };
    let mut min_played = usize::MAX;
    for cid in [c.s, c.a, c.b] {
        let Some(v) = comp_mids(ctx, cid) else { return };
        let end = v.len().min(45);
        min_played = min_played.min(rr_played(ctx, &v[..end]));
    }
    if min_played >= RR_PER_ROUND {
        set_sab_display(ctx);
    } else {
        set_round1_display(ctx);
    }
}

/// 每档联赛刚开始（三容器前 45 场全未打）时抽签：18 真队随机分 3 组 × 6，
/// 12 挂名队 4×3；写 standings / Team.league_id，重排 K甲（防升降级留下的
/// 错位），push 新闻。实际上场的 45 场由后面的 make_sab_r6 按新分组重写。
fn maybe_draw_groups(ctx: &mut StableServerCtx<'_>) {
    if !DRAW_GROUPS {
        return;
    }
    let Some(c) = sab_comps(ctx) else { return };
    let cids = [c.s, c.a, c.b];
    let league_names = ["TACC", "TACC2", "TACJ"];
    let labels = ["一组", "二组", "三组"];
    let mut mids: Vec<Vec<usize>> = Vec::new();
    for &cid in &cids {
        let Some(v) = comp_mids(ctx, cid) else { return };
        if v.len() < 45 {
            return;
        }
        if v[..45].iter().any(|m| is_played(ctx, *m)) {
            return; // 本档已经开打，绝不重抽
        }
        mids.push(v);
    }
    let season_tag = mids[0][0];
    let done_key = format!("draw_done_{}", season_tag);
    if ctx.save_get_string(&done_key).is_some() {
        return;
    }

    let mut reals = all_real_kpl_team_ids(ctx);
    let mut fillers: Vec<usize> = ctx
        .record_ids(RecordKindV1::Team)
        .into_iter()
        .filter(|&t| is_filler_team(&team_name(ctx, t)))
        .collect();
    if reals.len() != 18 || fillers.len() != 12 {
        return;
    }

    let mut rng = DrawRng::new();
    rng.shuffle(&mut reals);
    rng.shuffle(&mut fillers);

    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut fgroups: Vec<Vec<usize>> = Vec::new();
    for i in 0..3 {
        groups.push(reals[i * 6..i * 6 + 6].to_vec());
        fgroups.push(fillers[i * 4..i * 4 + 4].to_vec());
    }

    // ① 每个容器 standings = 6 真 + 4 挂，并更新 Team.league_id。
    for i in 0..3 {
        let mut rows = groups[i].clone();
        rows.extend_from_slice(&fgroups[i]);
        write_standings_raw(ctx, cids[i], &rows);
        if let Some(lid) = league_id_by_name(ctx, league_names[i]) {
            let lid = lid.to_string();
            for &t in &rows {
                ctx.record_set_json(RecordKindV1::Team, t, "league_id", &lid);
            }
        }
    }

    // ② K甲 归位：引擎原生升降级可能把真队降进 K甲、把青训队升进 S/A/B。
    //    凡是既不是 18 真队、也不是 12 挂名队的 CN 队，一律放回 K甲。
    let cn_lids: Vec<usize> = ["TACC", "TACC2", "TACJ", "TACJ2"]
        .iter()
        .filter_map(|n| league_id_by_name(ctx, n))
        .collect();
    let kgl_lid = league_id_by_name(ctx, "TACJ2");
    let assigned: HashSet<usize> = reals.iter().chain(fillers.iter()).copied().collect();
    if let Some(kgl_lid) = kgl_lid {
        for t in ctx.record_ids(RecordKindV1::Team) {
            if assigned.contains(&t) {
                continue;
            }
            let lid = ctx
                .record_get_json(RecordKindV1::Team, t, "league_id")
                .and_then(|j| j.trim().parse::<usize>().ok());
            if lid.map(|l| cn_lids.contains(&l)).unwrap_or(false) {
                let name = team_name(ctx, t);
                if !is_real_kpl(&name) && !is_filler_team(&name) {
                    let _ = ctx
                        .record_set_json(RecordKindV1::Team, t, "league_id", &kgl_lid.to_string());
                }
            }
        }
    }

    // ③ K甲 的 standings / 赛程也按归位后的 10 队重写（若发生过升降级，原
    //    competition 里可能混进真队/丢了青训队）。
    for cid in cn_d1_competitions(ctx) {
        if comp_league_name(ctx, cid).as_deref() != Some("TACJ2") {
            continue;
        }
        let Some(kgl_lid) = kgl_lid else { break };
        let mut kgl: Vec<usize> = ctx
            .record_ids(RecordKindV1::Team)
            .into_iter()
            .filter(|&t| {
                !is_real_kpl(&team_name(ctx, t))
                    && !is_filler_team(&team_name(ctx, t))
                    && ctx
                        .record_get_json(RecordKindV1::Team, t, "league_id")
                        .and_then(|j| j.trim().parse::<usize>().ok())
                        == Some(kgl_lid)
            })
            .collect();
        kgl.sort_unstable();
        if kgl.len() == 10 {
            write_standings_raw(ctx, cid, &kgl);
            if let Some(m) = comp_mids(ctx, cid) {
                for (k, &(a, b)) in rr_all_pairs(&kgl).iter().enumerate().take(45) {
                    if k < m.len() {
                        set_match_teams(ctx, m[k], a, b);
                    }
                }
            }
        }
        break;
    }

    // ④ 第一轮显示名 + 新闻（全体 18 真队）。
    set_round1_display(ctx);
    let ty = ctx
        .record_get_string(RecordKindV1::LeagueCompetition, c.s, "league_type")
        .unwrap_or_default();
    let split = if ty.to_lowercase().contains("summer") {
        "夏季赛"
    } else {
        "春季赛"
    };
    let title = format!("KPL {}第一轮抽签", split);
    let mut content = format!("KPL {}第一轮抽签结果（每组 6 队，组内单循环）：\n", split);
    for i in 0..3 {
        let names: Vec<String> = groups[i].iter().map(|&t| team_name(ctx, t)).collect();
        content.push_str(&format!("{}：{}\n", labels[i], names.join("、")));
    }
    content.push_str("第一轮结束后按名次重新分为 S组 / A组 / B组。\n");
    for tid in all_real_kpl_team_ids(ctx) {
        ctx.news_push(tid, &title, &content, "KPL");
    }
    ctx.save_set_string(&done_key, "1");
}

/// 取 standings 里某队的**原始成绩对象**（保留 kill/death/assist 等所有字段，
/// 重新写回时不会丢统计）。
fn standings_entry_json(json: &str, tid: usize) -> Option<String> {
    let needle = format!("\"{}\":", tid);
    let pos = json.find(&needle)?;
    let after_key = pos + needle.len();
    let rest = &json[after_key..];
    let brace = rest.find('{')?;
    let start = after_key + brace;
    let b = json.as_bytes();
    let mut depth = 0usize;
    let mut end = start;
    while end < b.len() {
        match b[end] {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        end += 1;
    }
    (end < b.len()).then(|| json[start..=end].to_string())
}

/// 按给定的 team id 顺序重写 standings（成绩对象原样保留；新队用 0-0）。
fn write_standings_raw(ctx: &mut StableServerCtx<'_>, cid: usize, ids: &[usize]) -> bool {
    if ids.is_empty() {
        return false;
    }
    let old = ctx
        .record_get_json(RecordKindV1::LeagueCompetition, cid, "standings")
        .unwrap_or_default();
    let zero = "{\"assist\":0,\"death\":0,\"kill\":0,\"lose\":0,\"set_lose\":0,\"set_win\":0,\"win\":0}";
    let body = ids
        .iter()
        .map(|&t| {
            let obj = standings_entry_json(&old, t).unwrap_or_else(|| zero.to_string());
            format!("\"{}\":{}", t, obj)
        })
        .collect::<Vec<_>>()
        .join(",");
    ctx.record_set_json(
        RecordKindV1::LeagueCompetition,
        cid,
        "standings",
        &format!("{{{}}}", body),
    )
}

/// 容器里现有的挂名队（非 KPL 真队）。
fn filler_teams_in(ctx: &StableServerCtx<'_>, cid: usize) -> Vec<usize> {
    let mut out = Vec::new();
    if let Some(j) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "standings") {
        for r in parse_standings(&j) {
            if !is_real_kpl(&team_name(ctx, r.team)) {
                out.push(r.team);
            }
        }
    }
    out.sort_unstable();
    out
}

/// 本容器的 4 个挂名队：优先读赛季初存下的 `rr_fillers_*`；没有就按
/// 「非真队 + Team.league_id == 本容器联赛」动态找。
fn filler_ids_for(
    ctx: &StableServerCtx<'_>,
    cid: usize,
    label: &str,
    tag: usize,
) -> Vec<usize> {
    if let Some(s) = ctx.save_get_string(&format!("rr_fillers_{}_{}", tag, label)) {
        let ids: Vec<usize> = s
            .split(',')
            .filter_map(|x| x.trim().parse().ok())
            .collect();
        if ids.len() >= 4 {
            return ids;
        }
    }
    let from_rows = filler_teams_in(ctx, cid);
    if from_rows.len() >= 4 {
        return from_rows;
    }
    let Some(lid) = comp_league_name(ctx, cid).and_then(|n| league_id_by_name(ctx, &n)) else {
        return Vec::new();
    };
    let mut out: Vec<usize> = ctx
        .record_ids(RecordKindV1::Team)
        .into_iter()
        .filter(|&t| {
            !is_real_kpl(&team_name(ctx, t))
                && ctx
                    .record_get_json(RecordKindV1::Team, t, "league_id")
                    .and_then(|j| j.trim().parse::<usize>().ok())
                    == Some(lid)
        })
        .collect();
    out.sort_unstable();
    out
}

/// 把 S/A/B 积分表里的挂名队行删掉（只留 6 真队）。幂等；第一次删时把 4 个
/// 挂名队 id 存进 `rr_fillers_*`，赛季末补回季后赛签表用。
fn strip_fillers(ctx: &mut StableServerCtx<'_>, cid: usize, label: &str, tag: usize) -> usize {
    if !STRIP_FILLERS {
        return 0;
    }
    let json = ctx
        .record_get_json(RecordKindV1::LeagueCompetition, cid, "standings")
        .unwrap_or_default();
    let rows = parse_standings(&json);
    if rows.is_empty() {
        return 0;
    }
    let mut real: Vec<usize> = Vec::new();
    let mut fill: Vec<usize> = Vec::new();
    for r in &rows {
        if is_real_kpl(&team_name(ctx, r.team)) {
            real.push(r.team);
        } else {
            fill.push(r.team);
        }
    }
    if !fill.is_empty() {
        ctx.save_set_string(
            &format!("rr_fillers_{}_{}", tag, label),
            &fill.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(","),
        );
    }
    if real.len() == 6 && rows.len() > 6 {
        write_standings_raw(ctx, cid, &real);
        return rows.len() - 6;
    }
    0
}

fn strip_fillers_all(ctx: &mut StableServerCtx<'_>) {
    let Some(c) = sab_comps(ctx) else { return };
    let Some(tag) = comp_mids(ctx, c.s).and_then(|v| v.first().copied()) else {
        return;
    };
    for (label, cid) in [("S", c.s), ("A", c.a), ("B", c.b)] {
        strip_fillers(ctx, cid, label, tag);
    }
}

/// 45 场常规赛全部打完后，把 4 个挂名队 0-0 补回积分表。
/// 引擎季后赛是 8 队签表，只有 6 行时可能取不到第 7/8 名。
fn maybe_restore_fillers(ctx: &mut StableServerCtx<'_>) {
    if !RESTORE_FILLERS_AFTER_REGULAR {
        return;
    }
    let Some(c) = sab_comps(ctx) else { return };
    let Some(tag) = comp_mids(ctx, c.s).and_then(|v| v.first().copied()) else {
        return;
    };
    for (label, cid) in [("S", c.s), ("A", c.a), ("B", c.b)] {
        let Some(v) = comp_mids(ctx, cid) else { continue };
        if v.len() < 45 || v[..45].iter().any(|m| !is_played(ctx, *m)) {
            continue;
        }
        let rows = ctx
            .record_get_json(RecordKindV1::LeagueCompetition, cid, "standings")
            .map(|j| parse_standings(&j))
            .unwrap_or_default();
        if rows.len() >= 10 {
            continue;
        }
        // 引擎 finalize 时可能把积分表压回只剩季后赛球队（S/A 混编的容器甚至只剩
        // 两三行）。国际杯抽签按「每个联赛前 4」凑 16 队种子池，行数不足会让某个
        // 池子凑不满 4 队，抽签的兜底分配就会越界崩溃。所以只要常规赛 45 场打完，
        // 每天都把该联赛的 10 队补齐（原有行和战绩原样保留，缺的队补 0-0）。
        let mut ids: Vec<usize> = standings_ranked(ctx, cid)
            .iter()
            .map(|r| r.team)
            .collect();
        if let Some(lid) = comp_league_name(ctx, cid).and_then(|n| league_id_by_name(ctx, &n)) {
            let mut real: Vec<usize> = ctx
                .record_ids(RecordKindV1::Team)
                .into_iter()
                .filter(|&t| {
                    is_real_kpl(&team_name(ctx, t))
                        && ctx
                            .record_get_json(RecordKindV1::Team, t, "league_id")
                            .and_then(|j| j.trim().parse::<usize>().ok())
                            == Some(lid)
                })
                .collect();
            real.sort_unstable();
            for t in real {
                if !ids.contains(&t) {
                    ids.push(t);
                }
            }
        }
        for f in filler_ids_for(ctx, cid, label, tag) {
            if !ids.contains(&f) {
                ids.push(f);
            }
        }
        if ids.len() > rows.len() {
            ids.truncate(10);
            write_standings_raw(ctx, cid, &ids);
        }
    }
}

/// 只认 `{"Normal":id}` 这种已经落地的队伍引用。`CompetitionRank` / `LeagueRank`
/// 里也带数字，但那是容器 id/名次，不是队伍 id，不能混用。
fn concrete_team(ctx: &StableServerCtx<'_>, mid: usize, field: &str) -> Option<usize> {
    let j = ctx.record_get_json(RecordKindV1::Match, mid, field)?;
    let p = j.find("\"Normal\"")?;
    parse_ids(&j[p..]).first().copied()
}

/// 国际杯抽签前，把 B 组被清空的 8 个季后赛槽位补成 8 场**已打完**的比赛。
///
/// 背景：B 组第二轮后全淘汰，我们把它本来的 8 个季后赛槽清空了（见 maybe_playoffs）。
/// 引擎在联赛 finalize 时按 playoffs 算最终名次；空数组算不出名次，JP（TACJ）赛区
/// 在国际杯种子分配里就一个队都取不到。5/4 的抽签把三个赛区的种子拼成 16 队池，
/// 少 2 队时最后一池只有 2 队，兜底分配越界崩溃（index out of bounds: len 2 index 2）。
///
/// 修法：借用 B 组常规赛里已经打完的比赛当季后赛槽位。这些比赛本来就在积分表里，
/// 只是多挂一个引用；8 个季后赛日（4/16..5/3）都已经过去，引擎不会重打 End 的比赛，
/// 也不会把日期在过去的比赛排进赛程。槽位优先覆盖 B 组 6 支真队（挂名队本来
/// 一场没打，引擎会按积分表把它们排在名次表后面），让 finalize 至少能算出
/// 「季后赛参赛队」这一段名次，不再返回空名次表。
fn maybe_fill_b_playoffs(ctx: &mut StableServerCtx<'_>) {
    let Some(c) = sab_comps(ctx) else { return };
    let Some(mids) = comp_mids(ctx, c.b) else { return };
    if mids.len() < 45 || mids[..45].iter().any(|m| !is_played(ctx, *m)) {
        return;
    }
    // 赛季末再补：至少 4 个季后赛日打完之后。太早补会让 B 组的季后赛页
    // 在季后赛期间显示 8 场常规赛。
    let sp = comp_playoff_ids(ctx, c.s).unwrap_or_default();
    if sp.iter().filter(|m| is_played(ctx, **m)).count() < 4 {
        return;
    }
    let Some(name) = comp_league_name(ctx, c.b) else { return };
    let Some(lid) = league_id_by_name(ctx, &name) else { return };
    let teams: Vec<usize> = ctx
        .record_ids(RecordKindV1::Team)
        .into_iter()
        .filter(|&t| {
            ctx.record_get_json(RecordKindV1::Team, t, "league_id")
                .and_then(|j| j.trim().parse::<usize>().ok())
                == Some(lid)
        })
        .collect();
    if teams.len() < 10 {
        return;
    }
    let mut po = comp_playoff_ids(ctx, c.b).unwrap_or_default();
    // 引擎若已经塞了槽位，只补到 8 个，不动原有的。
    let target = 8usize;
    if po.len() >= target {
        return;
    }
    let candidates: Vec<usize> = mids[..45]
        .iter()
        .copied()
        .filter(|m| !po.contains(m) && is_played(ctx, *m))
        .filter(|m| {
            matches!(
                (concrete_team(ctx, *m, "team1"), concrete_team(ctx, *m, "team2")),
                (Some(x), Some(y)) if teams.contains(&x) && teams.contains(&y)
            )
        })
        .collect();
    let mut added: Vec<usize> = Vec::new();
    let mut covered: Vec<usize> = po
        .iter()
        .flat_map(|m| {
            [concrete_team(ctx, *m, "team1"), concrete_team(ctx, *m, "team2")]
                .into_iter()
                .flatten()
        })
        .collect();
    // 第一遍：优先覆盖还没进过季后赛的队伍；第二遍：纯补长度。
    for &mid in &candidates {
        if po.len() + added.len() >= target {
            break;
        }
        let (Some(x), Some(y)) = (
            concrete_team(ctx, mid, "team1"),
            concrete_team(ctx, mid, "team2"),
        ) else {
            continue;
        };
        if !covered.contains(&x) || !covered.contains(&y) {
            added.push(mid);
            if !covered.contains(&x) {
                covered.push(x);
            }
            if !covered.contains(&y) {
                covered.push(y);
            }
        }
    }
    for &mid in &candidates {
        if po.len() + added.len() >= target {
            break;
        }
        if !added.contains(&mid) {
            added.push(mid);
        }
    }
    if added.is_empty() {
        return;
    }
    po.extend(added.iter().copied());
    set_comp_playoffs(ctx, c.b, &po);
}

/// 把某容器的 standings key 集合整体换成 teams（全部 0-0）。
/// 注意：这是**换 key 集合**，和每轮清零（保留原 key）不同 —— 引擎的 team→competition
/// 索引如果和 standings key 不一致就可能卡死。所以调用前必须同步改 Team.league_id，
/// 两边一起改才不会脱节。
fn rewrite_standings_keys(ctx: &mut StableServerCtx<'_>, cid: usize, teams: &[usize]) -> bool {
    let mut ids = teams.to_vec();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return false;
    }
    let body = ids
        .iter()
        .map(|t| {
            format!(
                "\"{}\":{{\"assist\":0,\"death\":0,\"kill\":0,\"lose\":0,\"set_lose\":0,\"set_win\":0,\"win\":0}}",
                t
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    ctx.record_set_json(
        RecordKindV1::LeagueCompetition,
        cid,
        "standings",
        &format!("{{{}}}", body),
    )
}

/// 路 B：把新组的 6 队真正搬进承办容器。
///   - `Team.league_id` 改成承办容器的 League id
///   - 容器 standings 换成**只有新组 6 真队**（挂名队不再进积分表）
/// 返回 (搬了几队, standings 写入是否成功, 该容器 League id)。
fn move_group_to_container(
    ctx: &mut StableServerCtx<'_>,
    cid: usize,
    league_name: &str,
    teams: &[usize; 6],
) -> (usize, bool, Option<usize>) {
    let lid = league_id_by_name(ctx, league_name);
    let mut moved = 0usize;
    if let Some(lid) = lid {
        for &t in teams {
            if ctx.record_set_json(RecordKindV1::Team, t, "league_id", &lid.to_string()) {
                moved += 1;
            }
        }
    }
    let ok = if STRIP_FILLERS {
        rewrite_standings_keys(ctx, cid, teams)
    } else {
        let mut rows = teams.to_vec();
        rows.extend(filler_teams_in(ctx, cid));
        rewrite_standings_keys(ctx, cid, &rows)
    };
    (moved, ok, lid)
}

// ============================ 第二轮卡位赛（真打 BO7） ============================

/// 从 Match.running_state 读比赛结果：`{"End":{"winner":12,"team1_score":4,...}}`。
fn match_result(ctx: &StableServerCtx<'_>, mid: usize) -> Option<(usize, i64, i64)> {
    let j = ctx.record_get_json(RecordKindV1::Match, mid, "running_state")?;
    if !j.contains("\"End\"") {
        return None;
    }
    let winner = field_i64(&j, "winner")?;
    if winner < 0 {
        return None;
    }
    Some((
        winner as usize,
        field_i64(&j, "team1_score").unwrap_or(0),
        field_i64(&j, "team2_score").unwrap_or(0),
    ))
}

/// 找 n 个未被任何 competition 引用、日期还在 2099 的等待中 Match。
fn find_orphan_matches(ctx: &StableServerCtx<'_>, n: usize) -> Vec<usize> {
    let mut used: HashSet<usize> = HashSet::new();
    for cid in ctx.record_ids(RecordKindV1::LeagueCompetition) {
        if let Some(j) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "matches") {
            used.extend(parse_ids(&j));
        }
        if let Some(j) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "playoffs") {
            used.extend(parse_ids(&j));
        }
    }
    for cid in ctx.record_ids(RecordKindV1::TournamentCompetition) {
        if let Some(j) = ctx.record_get_json(RecordKindV1::TournamentCompetition, cid, "matches") {
            used.extend(parse_ids(&j));
        }
    }
    let mut out = Vec::new();
    for mid in ctx.record_ids(RecordKindV1::Match) {
        if out.len() >= n {
            break;
        }
        if used.contains(&mid) || is_played(ctx, mid) {
            continue;
        }
        let d = ctx
            .record_get_string(RecordKindV1::Match, mid, "date")
            .unwrap_or_default();
        if d.starts_with("2099") {
            out.push(mid);
        }
    }
    out
}

fn append_matches_to_comp(ctx: &mut StableServerCtx<'_>, cid: usize, extra: &[usize]) -> bool {
    let Some(mj) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "matches") else {
        return false;
    };
    let mut ids = parse_ids(&mj);
    for &m in extra {
        if !ids.contains(&m) {
            ids.push(m);
        }
    }
    let json = format!(
        "[{}]",
        ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
    );
    ctx.record_set_json(RecordKindV1::LeagueCompetition, cid, "matches", &json)
}

/// 把追加的卡位赛 Match 从 competition.matches 里摘掉，并送回 2099 孤儿区。
fn remove_card_matches(ctx: &mut StableServerCtx<'_>, cid: usize, extra: &[usize]) {
    if let Some(mj) = ctx.record_get_json(RecordKindV1::LeagueCompetition, cid, "matches") {
        let ids: Vec<usize> = parse_ids(&mj)
            .into_iter()
            .filter(|m| !extra.contains(m))
            .take(45)
            .collect();
        let json = format!(
            "[{}]",
            ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
        );
        ctx.record_set_json(RecordKindV1::LeagueCompetition, cid, "matches", &json);
    }
    for &m in extra {
        ctx.record_set_string(
            RecordKindV1::Match,
            m,
            "date",
            "2099-12-31T00:00:00",
        );
    }
}

/// 把已打完的槽位标成练习赛（尝试让赛程页不再列旧组对阵）。
fn mark_practice_slots(ctx: &mut StableServerCtx<'_>, mids: &[Vec<usize>], end: usize) -> usize {
    let mut n = 0usize;
    if !HIDE_PLAYED_PHASES {
        return 0;
    }
    for v in mids {
        for &mid in v.iter().take(end.min(45)) {
            if is_played(ctx, mid)
                && ctx.record_set_json(RecordKindV1::Match, mid, "is_practice", "true")
            {
                n += 1;
            }
        }
    }
    n
}

/// 某队在一组槽位里的最新比赛日期（取字符串最大，ISO 日期可直接比）。
fn latest_played_day(ctx: &StableServerCtx<'_>, mids: &[Vec<usize>], end: usize) -> Option<String> {
    let mut best: Option<String> = None;
    for v in mids {
        for &mid in v.iter().take(end.min(45)) {
            if !is_played(ctx, mid) {
                continue;
            }
            let d = ctx
                .record_get_string(RecordKindV1::Match, mid, "date")
                .unwrap_or_default();
            if d.is_empty() {
                continue;
            }
            if best.as_ref().map_or(true, |b| d > *b) {
                best = Some(d);
            }
        }
    }
    best.map(|d| d.split('T').next().unwrap_or(&d).to_string())
}

fn match_day(ctx: &StableServerCtx<'_>, mid: usize) -> Option<String> {
    ctx.record_get_string(RecordKindV1::Match, mid, "date")
        .map(|d| d.split('T').next().unwrap_or(&d).to_string())
}

/// 按名次写第三轮分组（可重复调用；只在槽位未开打时调用）。
fn apply_card_groups(
    ctx: &mut StableServerCtx<'_>,
    cids: &[usize; 3],
    mids: &[Vec<usize>],
    groups: &[[usize; 6]; 3],
    season_tag: usize,
) -> bool {
    let labels = ["S", "A", "B"];
    let league_names = ["TACC", "TACC2", "TACJ"];
    let mut all_ok = true;
    for i in 0..3 {
        let (ok, total) = write_segment(ctx, &mids[i], 2 * RR_PER_ROUND, &groups[i]);
        let (_moved, st_ok, _lid) =
            move_group_to_container(ctx, cids[i], league_names[i], &groups[i]);
        ctx.save_set_string(
            &format!("rr_group_{}_{}_2", season_tag, labels[i]),
            &groups[i]
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        if ok != total || !st_ok {
            all_ok = false;
        }
    }
    ctx.save_set_string(&format!("move_done_{}_2", season_tag), "1");
    all_ok
}

/// 卡位赛结果 → 第三轮分组。
///   S5 vs A2、S6 vs A1：胜者留 S，败者去 A
///   A5 vs B2、A6 vs B1：胜者升 A，败者去 B
fn groups_from_cards(
    s: &[usize],
    a: &[usize],
    b: &[usize],
    pairs: &[(usize, usize)],
    winners: &[usize],
) -> Option<[[usize; 6]; 3]> {
    if s.len() < 6 || a.len() < 6 || b.len() < 6 || pairs.len() != 4 || winners.len() != 4 {
        return None;
    }
    let mut losers = Vec::with_capacity(4);
    for (i, &(x, y)) in pairs.iter().enumerate() {
        losers.push(if winners[i] == x { y } else { x });
    }
    Some([
        [s[0], s[1], s[2], s[3], winners[0], winners[1]],
        [a[2], a[3], losers[0], losers[1], winners[2], winners[3]],
        [b[2], b[3], b[4], b[5], losers[2], losers[3]],
    ])
}

fn notify_cards_scheduled(
    ctx: &mut StableServerCtx<'_>,
    pairs: &[(usize, usize)],
    dates: &[String],
) {
    let mut content = String::from("KPL 第二轮卡位赛（BO7）对阵：\n");
    for (i, &(x, y)) in pairs.iter().enumerate() {
        let d = dates.get(i).cloned().unwrap_or_default();
        let up = if i < 2 { "S/A" } else { "A/B" };
        content.push_str(&format!(
            "{} {} vs {}（{} 卡位）\n",
            d,
            team_name(ctx, x),
            team_name(ctx, y),
            up
        ));
    }
    content.push_str("胜者进入上一组，败者落入下一组。");
    for tid in all_real_kpl_team_ids(ctx) {
        ctx.news_push(tid, "KPL 第二轮卡位赛对阵", &content, "KPL");
    }
}

fn notify_card_results(
    ctx: &mut StableServerCtx<'_>,
    pairs: &[(usize, usize)],
    results: &[(usize, i64, i64)],
    groups: Option<&[[usize; 6]; 3]>,
) {
    if results.is_empty() {
        let content =
            String::from("KPL 第二轮卡位赛未能在第三轮开打前完成，第三轮按第二轮名次分组。\n");
        for tid in all_real_kpl_team_ids(ctx) {
            ctx.news_push(tid, "KPL 卡位赛结果", &content, "KPL");
        }
        return;
    }
    let mut content = String::from("KPL 第二轮卡位赛（BO7）结果：\n");
    for (i, &(x, y)) in pairs.iter().enumerate() {
        let Some(&(winner, s1, s2)) = results.get(i) else {
            continue;
        };
        let (w, l) = if winner == x { (x, y) } else { (y, x) };
        content.push_str(&format!(
            "{} {} - {} {}\n",
            team_name(ctx, x),
            s1,
            s2,
            team_name(ctx, y)
        ));
        content.push_str(&format!(
            "  {} 胜，{} 负\n",
            team_name(ctx, w),
            team_name(ctx, l)
        ));
    }
    if let Some(g) = groups {
        content.push_str("第三轮分组：\n");
        for (label, t) in ["S", "A", "B"].iter().zip(g.iter()) {
            let names: Vec<String> = t.iter().map(|&x| team_name(ctx, x)).collect();
            content.push_str(&format!("{}组：{}\n", label, names.join("、")));
        }
    }
    for tid in all_real_kpl_team_ids(ctx) {
        ctx.news_push(tid, "KPL 卡位赛结果", &content, "KPL");
    }
}

/// 全部真 KPL 队（roster 里的 18 队）——新闻收件人用这个。
/// 旧写法只扫「未 finalize」容器的积分表：16 场季后赛打完后 A/B 被强制
/// finalize，就只剩 S 组，新闻只推给一个联赛（玩家实测反馈「只退给一个联赛里」）。
fn all_real_kpl_team_ids(ctx: &StableServerCtx<'_>) -> Vec<usize> {
    let mut ids: Vec<usize> = ctx
        .record_ids(RecordKindV1::Team)
        .into_iter()
        .filter(|&t| is_real_kpl(&team_name(ctx, t)))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// 第二轮卡位赛主流程：30 场边界布置 → 4 场 BO7 → 读结果 → 重写第三轮分组。
fn maybe_cards(ctx: &mut StableServerCtx<'_>) {
    if !CARD_MATCHES || !REG_GROUP_EXPERIMENT {
        return;
    }
    let Some(c) = sab_comps(ctx) else { return };
    let cids = [c.s, c.a, c.b];
    let mut mids: Vec<Vec<usize>> = Vec::new();
    let mut played: Vec<usize> = Vec::new();
    for &cid in &cids {
        let Some(v) = comp_mids(ctx, cid) else { return };
        if v.len() < 45 {
            return;
        }
        played.push(v[..45].iter().filter(|m| is_played(ctx, **m)).count());
        mids.push(v);
    }
    let season_tag = mids[0][0];
    let done_key = format!("card_done_{}", season_tag);
    if ctx.save_get_string(&done_key).is_some() {
        return;
    }
    let setup_key = format!("card_setup_{}", season_tag);
    let round3_started = mids
        .iter()
        .any(|v| v[30..45].iter().any(|m| is_played(ctx, *m)));

    if ctx.save_get_string(&setup_key).is_none() {
        // 安全：没有布置记录却有多余 Match 引用（上次写到一半崩溃）→ 先摘掉
        for &cid in &cids {
            if let Some(v) = comp_mids(ctx, cid) {
                if v.len() > 45 {
                    let extra: Vec<usize> = v[45..].to_vec();
                    remove_card_matches(ctx, cid, &extra);
                }
            }
        }
        // 还没到 30 场，或第三轮已经开打（来不及插卡位赛）→ 不动
        if played.iter().any(|&p| p < 2 * RR_PER_ROUND) || round3_started {
            return;
        }
        // 老档可能已经写了第三轮分组；新档由这里写。只要有任一槽位已打就不动。
        let Some(s_rank) = saved_rank(ctx, cids[0], 2) else {
            return;
        };
        let Some(a_rank) = saved_rank(ctx, cids[1], 2) else {
            return;
        };
        let Some(b_rank) = saved_rank(ctx, cids[2], 2) else {
            return;
        };
        let Some((prov_s, prov_a, prov_b)) = new_groups(2, &s_rank, &a_rank, &b_rank) else {
            return;
        };
        let provisional = [prov_s, prov_a, prov_b];
        // ① 先把临时第三轮分组落盘（保证 3/17 第三轮能开打）
        apply_card_groups(ctx, &cids, &mids, &provisional, season_tag);
        // ② 旧的两轮标练习赛（赛程页尽量不再显示旧组对阵）
        mark_practice_slots(ctx, &mids, 2 * RR_PER_ROUND);
        // ③ 借 4 场孤儿 Match 改成 BO7
        let pairs = [
            (s_rank[4], a_rank[1]), // S5 vs A2
            (s_rank[5], a_rank[0]), // S6 vs A1
            (a_rank[4], b_rank[1]), // A5 vs B2
            (a_rank[5], b_rank[0]), // A6 vs B1
        ];
        let card_mids = find_orphan_matches(ctx, 4);
        if card_mids.len() != 4 {
            ctx.save_set_string(&done_key, "1");
            return;
        }
        // 日期：第三轮首场前 4 天开始，但不能早于当前最新已打日期 +1。
        // 三组取最早的那个第三轮首场日，保证三个容器开打前 4 场都结束。
        let r3_day = mids
            .iter()
            .filter_map(|v| {
                match_day(ctx, v[30]).and_then(|d| {
                    d.split('T').next().map(|s| s.to_string())
                })
            })
            .min();
        let r3_first = r3_day.as_ref().map(|d| shift_day(d, -4));
        let last_played = latest_played_day(ctx, &mids, 45);
        let start_day = match (r3_first, last_played) {
            (Some(r3), Some(lp)) => {
                let lp_next = shift_day(&lp, 1);
                if lp_next > r3 {
                    lp_next
                } else {
                    r3
                }
            }
            (Some(r3), None) => r3,
            (None, Some(lp)) => shift_day(&lp, 1),
            (None, None) => String::from("2026-03-12"),
        };
        let mut dates: Vec<String> = Vec::new();
        for k in 0..4 {
            dates.push(shift_day(&start_day, k as i64));
        }
        // 安全阀：4 场必须全部排在第三轮首场之前，否则又会出现「打 1 场第三轮
        // 就开打、卡位赛被放弃」的情况。没有空档就跳过卡位赛（保留临时分组）。
        let no_room = r3_day
            .as_ref()
            .map(|r3| dates.last().map_or(true, |d| d.as_str() >= r3.as_str()))
            .unwrap_or(false);
        if no_room {
            ctx.save_set_string(&done_key, "1");
            return;
        }
        for (i, &mid) in card_mids.iter().enumerate() {
            let (x, y) = pairs[i];
            set_match_teams(ctx, mid, x, y);
            ctx.record_set_json(RecordKindV1::Match, mid, "is_practice", "false");
            ctx.record_set_json(RecordKindV1::Match, mid, "game_rule", "\"Game5V5\"");
            ctx.record_set_json(RecordKindV1::Match, mid, "need_win", &CARD_NEED_WIN.to_string());
            ctx.record_set_string(
                RecordKindV1::Match,
                mid,
                "date",
                &format!("{}T19:00:00", dates[i]),
            );
        }
        append_matches_to_comp(ctx, cids[0], &card_mids);
        ctx.save_set_string(
            &format!("card_mids_{}", season_tag),
            &card_mids
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        ctx.save_set_string(
            &format!("card_pairs_{}", season_tag),
            &pairs
                .iter()
                .map(|(x, y)| format!("{},{}", x, y))
                .collect::<Vec<_>>()
                .join(";"),
        );
        ctx.save_set_string(&setup_key, "1");
        notify_cards_scheduled(ctx, &pairs, &dates);
        return;
    }

    // ---- 已布置，检查结果 ----
    let card_mids: Vec<usize> = ctx
        .save_get_string(&format!("card_mids_{}", season_tag))
        .unwrap_or_default()
        .split(',')
        .filter_map(|x| x.trim().parse().ok())
        .collect();
    let pair_ids: Vec<usize> = ctx
        .save_get_string(&format!("card_pairs_{}", season_tag))
        .unwrap_or_default()
        .split(';')
        .flat_map(|p| p.split(',').filter_map(|x| x.trim().parse::<usize>().ok()))
        .collect();
    if card_mids.len() != 4 || pair_ids.len() != 8 {
        ctx.save_set_string(&done_key, "1");
        return;
    }
    let pairs: Vec<(usize, usize)> = pair_ids.chunks(2).map(|c| (c[0], c[1])).collect();
    let results: Vec<Option<(usize, i64, i64)>> =
        card_mids.iter().map(|&m| match_result(ctx, m)).collect();
    let all_end = results.iter().all(Option::is_some);

    if all_end {
        let res: Vec<(usize, i64, i64)> = results.into_iter().flatten().collect();
        let winners: Vec<usize> = res.iter().map(|r| r.0).collect();
        let Some(s_rank) = saved_rank(ctx, cids[0], 2) else {
            return;
        };
        let Some(a_rank) = saved_rank(ctx, cids[1], 2) else {
            return;
        };
        let Some(b_rank) = saved_rank(ctx, cids[2], 2) else {
            return;
        };
        let final_groups = groups_from_cards(&s_rank, &a_rank, &b_rank, &pairs, &winners);
        let can_rewrite = !round3_started
            && mids
                .iter()
                .all(|v| v[30..45].iter().all(|m| !is_played(ctx, *m)));
        if can_rewrite {
            if let Some(g) = &final_groups {
                apply_card_groups(ctx, &cids, &mids, g, season_tag);
            }
        }
        notify_card_results(ctx, &pairs, &res, if can_rewrite { final_groups.as_ref() } else { None });
        remove_card_matches(ctx, cids[0], &card_mids);
        ctx.save_set_string(&done_key, "1");
        ctx.save_set_string(&format!("regroup_done_{}_2", season_tag), "1");
        return;
    }

    if round3_started {
        // 4 场没打完第三轮就开了：放弃卡位赛，保留临时分组
        remove_card_matches(ctx, cids[0], &card_mids);
        notify_card_results(ctx, &pairs, &[], None);
        ctx.save_set_string(&done_key, "1");
        ctx.save_set_string(&format!("regroup_done_{}_2", season_tag), "1");
    }
}

/// 边界检查 + 执行重分组。只有三组都跨过边界才动作；每轮只做一次
/// （key 用「S 容器首场 id + 轮号」，每赛季唯一）。
fn maybe_regroup(ctx: &mut StableServerCtx<'_>) {
    if !REG_GROUP_EXPERIMENT {
        return;
    }
    let Some(c) = sab_comps(ctx) else { return };
    let cids = [c.s, c.a, c.b];
    let labels = ["S", "A", "B"];
    let mut mids: Vec<Vec<usize>> = Vec::new();
    let mut played: Vec<usize> = Vec::new();
    for &cid in &cids {
        let Some(v) = comp_mids(ctx, cid) else { return };
        if v.len() < 45 {
            return;
        }
        played.push(rr_played(ctx, &v[..45]));
        mids.push(v);
    }
    let season_tag = mids[0][0];

    for (round, boundary) in [(1usize, RR_PER_ROUND), (2usize, 2 * RR_PER_ROUND)] {
        // 第二轮的第三轮分组由卡位赛流程（maybe_cards）负责：先写临时分组，
        // 4 场 BO7 打完后再改成最终分组。这里不要直接按名次定死。
        if round == 2 && CARD_MATCHES {
            continue;
        }
        let done_key = format!("regroup_done_{}_{}", season_tag, round);
        if ctx.save_get_string(&done_key).is_some() {
            continue;
        }
        if played.iter().any(|&p| p < boundary) {
            continue;
        }
        // 名次快照必须三组都在（每组的 15 场边界触发 maybe_reset_rounds 时写入）
        let Some(s_rank) = saved_rank(ctx, cids[0], round) else {
            continue;
        };
        let Some(a_rank) = saved_rank(ctx, cids[1], round) else {
            continue;
        };
        let Some(b_rank) = saved_rank(ctx, cids[2], round) else {
            continue;
        };
        let Some((ns, na, nb)) = new_groups(round, &s_rank, &a_rank, &b_rank) else {
            continue;
        };
        let start = boundary;
        let blocked = mids
            .iter()
            .any(|m| m[start..start + 15].iter().any(|x| is_played(ctx, *x)));
        if blocked {
            continue;
        }
        let groups = [&ns, &na, &nb];
        let league_names = ["TACC", "TACC2", "TACJ"];
        for i in 0..3 {
            write_segment(ctx, &mids[i], start, groups[i]);
            // 路 B（实验）：把队伍真正搬进新联赛，并换掉 standings 的 key 集合，
            // 让 UI 的积分表/队伍归属跟实际分组一致。
            if MOVE_TEAMS_EXPERIMENT {
                move_group_to_container(ctx, cids[i], league_names[i], groups[i]);
            }
            // 记录本容器这一轮的 6 队（下一轮名次用）
            ctx.save_set_string(
                &format!("rr_group_{}_{}_{}", season_tag, labels[i], round),
                &groups[i].iter().map(|t| t.to_string()).collect::<Vec<_>>().join(","),
            );
        }
        // 第一轮打完：把 0-14 槽位标成练习赛，尽量让赛程页不再列旧组对阵
        if round == 1 {
            mark_practice_slots(ctx, &mids, RR_PER_ROUND);
            // 第一轮结束 → 显示名从「一组/二组/三组」切回「S组/A组/B组」。
            set_sab_display(ctx);
        }
        // 每轮新分组 push 成新闻（跟赛季初抽签一样，所有 CN 队伍都收到）
        notify_regroup(ctx, round, &groups);
        if MOVE_TEAMS_EXPERIMENT {
            ctx.save_set_string(&format!("move_done_{}_{}", season_tag, round), "1");
        }
        ctx.save_set_string(&done_key, "1");
    }
}

/// 路 B 的补搬：老档里重分组已经只改了 Match（路 A），没有搬队。
/// 读每轮存下来的 `rr_group_*`，如果该轮下一段还没开打，就把队伍补搬过去
/// （league_id + standings key 集合）。新档在 `maybe_regroup` 里已经搬过、
/// 会写 `move_done_*` 标记，这里直接跳过。
fn maybe_repair_moves(ctx: &mut StableServerCtx<'_>) {
    if !MOVE_TEAMS_EXPERIMENT {
        return;
    }
    let Some(c) = sab_comps(ctx) else { return };
    let cids = [c.s, c.a, c.b];
    let labels = ["S", "A", "B"];
    let league_names = ["TACC", "TACC2", "TACJ"];
    let Some(mids0) = comp_mids(ctx, cids[0]) else { return };
    let Some(&season_tag) = mids0.first() else { return };
    for round in 1usize..=2 {
        let done_key = format!("move_done_{}_{}", season_tag, round);
        if ctx.save_get_string(&done_key).is_some() {
            continue;
        }
        // 该轮对应的下一段（round*15 .. round*15+15）必须一场未打，否则补搬会
        // 把已打出来的积分表一起重置，损失战绩。
        let start = round * RR_PER_ROUND;
        let mut safe = true;
        for &cid in &cids {
            let Some(v) = comp_mids(ctx, cid) else {
                safe = false;
                break;
            };
            if v.len() < start + 15 || v[start..start + 15].iter().any(|m| is_played(ctx, *m)) {
                safe = false;
                break;
            }
        }
        if !safe {
            continue;
        }
        let mut parsed: Vec<[usize; 6]> = Vec::new();
        for label in labels {
            let key = format!("rr_group_{}_{}_{}", season_tag, label, round);
            let Some(s) = ctx.save_get_string(&key) else {
                parsed.clear();
                break;
            };
            let ids: Vec<usize> = s
                .split(',')
                .filter_map(|x| x.trim().parse().ok())
                .collect();
            if ids.len() != 6 {
                parsed.clear();
                break;
            }
            parsed.push([ids[0], ids[1], ids[2], ids[3], ids[4], ids[5]]);
        }
        if parsed.len() != 3 {
            continue;
        }
        for i in 0..3 {
            move_group_to_container(ctx, cids[i], league_names[i], &parsed[i]);
        }
        let refs = [&parsed[0], &parsed[1], &parsed[2]];
        notify_regroup(ctx, round, &refs);
        ctx.save_set_string(&done_key, "1");
    }
}

/// 重分组结果 push 成新闻（每轮一次）。标题写清是**下一轮**的分组，
/// 这样旧新闻不会被误当成当前分组。推给所有 CN 真队。
fn notify_regroup(
    ctx: &mut StableServerCtx<'_>,
    round: usize,
    groups: &[&[usize; 6]; 3],
) {
    let next = round + 1;
    let stage = match next {
        2 => "第二",
        3 => "第三",
        _ => "下一",
    };
    let title = format!("KPL 第{}轮分组", stage);
    let mut content = format!("KPL 第{}轮结束，第{}轮分组如下：\n", round, stage);
    for (label, g) in ["S", "A", "B"].iter().zip(groups.iter()) {
        let names: Vec<String> = g.iter().map(|&t| team_name(ctx, t)).collect();
        content.push_str(&format!("{}组：{}\n", label, names.join("、")));
    }
    for tid in all_real_kpl_team_ids(ctx) {
        ctx.news_push(tid, &title, &content, "KPL");
    }
}

/// 每赛季轮换时：记上赛季名次 + dump 当前分组（按 league_type 每季一次）。
fn maybe_sab_seed(ctx: &mut StableServerCtx<'_>) {
    let comps = cn_d1_competitions(ctx);
    if comps.is_empty() {
        return;
    }
    let ty = comps
        .iter()
        .find_map(|&cid| ctx.record_get_string(RecordKindV1::LeagueCompetition, cid, "league_type"))
        .unwrap_or_else(|| String::from("?"));
    if ctx.save_get_string("sab_seed_type").as_deref() == Some(ty.as_str()) {
        return;
    }
    record_last_season(ctx);
    // 第一轮分组的新闻改由 maybe_draw_groups 用「一组/二组/三组」推送。
    ctx.save_set_string("sab_seed_type", &ty);
}

struct KplNaming;

impl StableServerExtension for KplNaming {
    fn on_server_start(&self, ctx: &mut StableServerCtx<'_>) {
        if START_DONE.swap(true, Ordering::SeqCst) {
            return;
        }
        rename_china(ctx);
        fix_roles(ctx);
        rename_bench(ctx);
        rename_cn_all(ctx);
        set_bo(ctx);
        // S/A/B 同级（A 组不再是「二级联赛」），赶在任何升降级判定之前。
        enforce_same_tier(ctx);
        zero_ab_prizes(ctx);
        // 新档进档：本档还没开打就抽签（18 真队随机分一组/二组/三组）。
        maybe_draw_groups(ctx);
        // 显示层：读档时按当前轮次对齐「一组/二组/三组」或「S/A/B」。
        sync_display_names(ctx);
        make_single_rr(ctx);
        make_sab_r6(ctx);
        strip_fillers_all(ctx);
        maybe_normalize_dates(ctx);
        maybe_reset_rounds(ctx);
        maybe_regroup(ctx);
        maybe_cards(ctx);
        maybe_playoffs(ctx);
        // 抽签之后把杯赛里的挂名队/重复队换成没进杯赛的真队（只改杯赛记录）。
        maybe_swap_cup_fillers(ctx);
        maybe_repair_moves(ctx);
        maybe_restore_fillers(ctx);
    }

    /// 每个管理 tick 开始前重写一遍季后赛对阵：引擎若在季后赛轮次事件里重新播种
    /// （CN 容器原始引用是 CompetitionRank），这里会立刻改回来，保证当天开打的
    /// 一定是我们的对阵。
    fn before_management_tick(&self, ctx: &mut StableServerCtx<'_>) {
        // S/A/B 同级：每个 tick 都确认 A 组还是 div0（引擎若在赛季末 rebuild
        // 联赛时写回 div1，这里马上改回去，避免下一次升降级判定读到旧值）。
        enforce_same_tier(ctx);
        // 先把 45 场打完的 S/A/B 积分表补回 10 队，再改季后赛对阵。5/4 国际杯
        // 抽签紧跟在 tick 开始后，这一步是抽签前最后一次修正数据的机会。
        maybe_restore_fillers(ctx);
        maybe_playoffs(ctx);
        // A/B 组奖金表清零（只留 S 组一张表），放在抽签/发奖之前。
        zero_ab_prizes(ctx);
        // maybe_playoffs 每次都会把 B 的季后赛清空，补槽必须排在它后面。
        maybe_fill_b_playoffs(ctx);
        // B 组 finalized 之后 1-6 名取不到队，国际杯要抽 B 组的话种子池会凑不满。
        maybe_fix_cup_allocation(ctx);
        // 抽签在 tick 里完成；下一次 tick 开始时就能把挂名队换成真队。
        maybe_swap_cup_fillers(ctx);
    }

    fn after_management_tick(&self, ctx: &mut StableServerCtx<'_>) {
        let cursor = CURSOR.load(Ordering::SeqCst);
        let mut nc = cursor;
        for ev in ctx.management_events_after(cursor) {
            nc = ev.seq;
            match &ev.kind {
                Some(ManagementEventKindV1::SeasonRollover) => {
                    rename_china(ctx);
                    fix_roles(ctx);
                    rename_bench(ctx);
                    rename_cn_all(ctx);
                    maybe_sab_seed(ctx);
                    set_bo(ctx);
                    // 新赛季联赛记录重建后立刻恢复 S/A/B 同级。
                    enforce_same_tier(ctx);
                    zero_ab_prizes(ctx);
                    // 每档联赛刚建好（含 6/22 夏季赛 = 转会期开始）：抽签 + push。
                    maybe_draw_groups(ctx);
                    sync_display_names(ctx);
                    make_single_rr(ctx);
                    make_sab_r6(ctx);
                    strip_fillers_all(ctx);
                    maybe_normalize_dates(ctx);
                    maybe_reset_rounds(ctx);
                    maybe_regroup(ctx);
                    maybe_cards(ctx);
                    maybe_playoffs(ctx);
                    maybe_repair_moves(ctx);
                }
                Some(ManagementEventKindV1::MatchFinished) => {
                    maybe_reset_rounds(ctx);
                    maybe_regroup(ctx);
                    maybe_cards(ctx);
                    maybe_playoffs(ctx);
                    maybe_restore_fillers(ctx);
                }
                // 转会完成后可能有新选手进入 CN 赛区，补一次中文名/查重。
                Some(ManagementEventKindV1::TransferCompleted) => {
                    rename_cn_all(ctx);
                }
                _ => {}
            }
        }
        CURSOR.store(nc, Ordering::SeqCst);
    }
}

// ======================= 客户端：完整 16 场季后赛面板 =======================
//
// 原生联赛页的 `#league_playoff` 只有 8 个卡位（引擎每个 LeagueCompetition
// 固定 8 个季后赛槽），而本 mod 的 16 场分在 S/A 两个容器里，所以原生页和
// 日历最多只显示一半。这里在底部栏加一个「季后赛」按钮，展开一个照原版对
// 阵卡样式（327×107 深色圆角卡、40px 队徽槽、队名 + 比分、分隔线）排版
// 的完整 16 场面板，数据全部来自服务端落盘的 `kpl_po_board`。
//
// 纯客户端只读：不写存档、不改比赛记录，因此不影响判定。

use mod_api_stable::{ClientSceneKindV1, SceneKindV1, StableClient, StableExtension, UiEventKindV1};
use std::sync::{Arc, Mutex};

const PO_BTN_PATH: &str = "main.bottom.po_btn";
const PO_UI_PATH: &str = "main.po_ui";

const PO_BTN_SRC: &str = r#"
po_btn:color_icon_button {
  @"asset/base/style/main#secondary_button";
  x: 1111px;
  y: 0px;
  width: 112px;
  height: 40px;
  text: {
    text: "季后赛";
    font: "asset/base/font/set/bold";
    size: 16;
    align_x: Center;
    align_y: Center;
  }
}
"#;

const PO_ROOT_SRC: &str = r#"
po_ui:empty {
  width: 100%;
  height: 100%;
  z: 2000;
  visible: false;
}
"#;

/// 面板外壳：和原生 `rank` 页同尺寸（1600×968 居中），背板用原版
/// `#161721` + 12 圆角。
const PO_SHELL_SRC: &str = r#"
panel:color {
  x: 32px;
  y: 24px;
  width: 1600px;
  height: 968px;
  color: #161721ff;
  rounding: Uniform { rounding: 12; }
}
"#;

const PO_ROWS: usize = 16;

/// 一场一张卡，照原版 `#league_playoff` 的对阵卡排版：
///   327×107 深色卡（#1d1f2c，12 圆角），上下两行各 53px：
///   40×40 队徽槽 + 队名 + 右对齐比分，中间 1px 分隔线。
/// 卡片顶部各多一行小字（轮次 + 日期），所以整卡高 134px。
/// 布局照原版对阵页：4 列 × 4 行，列 = 轮次推进（R1 → R2 → R3/R4 → 决赛周），
/// 行 = 当轮第几场。
fn po_row_src(slot: usize) -> String {
    let col = slot / 4;
    let row = slot % 4;
    // 面板 1600 宽：327 * 4 张卡 + 3 道 77 间距 + 左右各 16 = 1563，居中后每卡
    // 再收 4px ~ 9px 的余量。纵向 4 行 × (20 标题 + 107 卡 + 78 间隙) = 820。
    let x = 16 + 392 * col;
    let y = 80 + 205 * row;
    format!(
        r#"
#r{slot}:empty {{
  x: {x}px;
  y: {y}px;
  width: 327px;
  height: 196px;

  #card:color {{
    x: 0px;
    y: 24px;
    width: 327px;
    height: 107px;
    color: #1d1f2cff;
    rounding: Uniform {{ rounding: 12; }}
  }}

  #tag:label {{
    @"asset/base/style/main#label";
    x: 2px;
    y: 0px;
    width: 200px;
    height: 20px;
    size: 14;
    color: #8a93a6ff;
    align_x: Left;
    align_y: Center;
    text: "";
  }}

  #date:label {{
    @"asset/base/style/main#label";
    anchor_x: 1;
    pivot_x: 1;
    x: -2px;
    y: 0px;
    width: 96px;
    height: 20px;
    size: 14;
    color: #8a93a6ff;
    align_x: Right;
    align_y: Center;
    text: "";
  }}

  #team0:empty {{
    x: 0px;
    y: 24px;
    width: 327px;
    height: 53px;

    #logo_slot:color {{
      width: 40px;
      height: 40px;
      x: 10px;
      anchor_y: 0.5;
      pivot_y: 0.5;
      color: #161721ff;
      rounding: Uniform {{ rounding: 8; }}
      ignore_event: true;
    }}

    #logo:image {{
      width: 32px;
      height: 32px;
      anchor_x: 0.5;
      pivot_x: 0.5;
      anchor_y: 0.5;
      pivot_y: 0.5;
      ignore_event: true;
    }}

    #name:label {{
      @"asset/base/style/main#label";
      x: 56px;
      width: 210px;
      height: 53px;
      size: 18;
      color: #e8e8e8ff;
      align_x: Left;
      align_y: Center;
      text: "";
    }}

    #score:label {{
      @"asset/base/style/main#label";
      anchor_x: 1;
      pivot_x: 1;
      x: -10px;
      width: 40px;
      height: 53px;
      size: 18;
      color: #e8e8e8ff;
      align_x: Center;
      align_y: Center;
      text: "-";
    }}
  }}

  #divider:image {{
    source: "asset/base/sprite/white";
    width: 307px;
    height: 1px;
    x: 10px;
    y: 77px;
    color: #161721ff;
  }}

  #team1:empty {{
    x: 0px;
    y: 78px;
    width: 327px;
    height: 53px;

    #logo_slot:color {{
      width: 40px;
      height: 40px;
      x: 10px;
      anchor_y: 0.5;
      pivot_y: 0.5;
      color: #161721ff;
      rounding: Uniform {{ rounding: 8; }}
      ignore_event: true;
    }}

    #logo:image {{
      width: 32px;
      height: 32px;
      anchor_x: 0.5;
      pivot_x: 0.5;
      anchor_y: 0.5;
      pivot_y: 0.5;
      ignore_event: true;
    }}

    #name:label {{
      @"asset/base/style/main#label";
      x: 56px;
      width: 210px;
      height: 53px;
      size: 18;
      color: #e8e8e8ff;
      align_x: Left;
      align_y: Center;
      text: "";
    }}

    #score:label {{
      @"asset/base/style/main#label";
      anchor_x: 1;
      pivot_x: 1;
      x: -10px;
      width: 40px;
      height: 53px;
      size: 18;
      color: #e8e8e8ff;
      align_x: Center;
      align_y: Center;
      text: "-";
    }}
  }}
}}
"#
    )
}

fn po_header_src() -> String {
    let mut s = String::from(
        r#"
#header:empty {
  x: 0px;
  y: 0px;
  width: 1600px;
  height: 72px;
"#,
    );
    s.push_str(
        r#"
  #title:label {
    @"asset/base/style/main#bold_label";
    x: 32px;
    y: 14px;
    width: 520px;
    height: 32px;
    size: 23;
    color: #ffffffff;
    align_x: Left;
    align_y: Center;
    text: "KPL 季后赛 · 完整 16 场";
  }

  #sub:label {
    @"asset/base/style/main#label";
    x: 34px;
    y: 46px;
    width: 900px;
    height: 20px;
    size: 12;
    color: #8a93a6ff;
    align_x: Left;
    align_y: Center;
    text: "全部 BO7 · S/A 合流 · 自左向右四个比赛周：R1（4/16-4/17）→ R2（4/18-4/19）→ R3/R4（4/25-4/26）→ 决赛周（5/1-5/3）";
  }

  #state:label {
    @"asset/base/style/main#label";
    anchor_x: 1;
    pivot_x: 1;
    x: -80px;
    y: 14px;
    width: 520px;
    height: 40px;
    size: 14;
    color: #8a93a6ff;
    align_x: Right;
    align_y: Center;
    text: "";
  }

  #close:color_icon_button {
    @"asset/base/style/main#secondary_button";
    anchor_x: 1;
    pivot_x: 1;
    x: -24px;
    y: 14px;
    width: 40px;
    height: 40px;
    text: {
      text: "X";
      font: "asset/base/font/set/bold";
      size: 18;
      align_x: Center;
      align_y: Center;
    }
  }
"#,
    );
    s.push_str("}\n");
    s
}

struct PoState {
    in_game: bool,
    open: bool,
    frames: u64,
    dirty: bool,
    fail: u32,
    handlers: bool,
}

struct PoExt {
    st: Arc<Mutex<PoState>>,
}

impl StableExtension for PoExt {
    fn post_update(&self, ctx: &mut StableClient<'_>, _dt: u64) {
        po_tick(ctx, &self.st);
    }
}

fn po_tick(ctx: &mut StableClient<'_>, st: &Arc<Mutex<PoState>>) {
    let in_game = ctx.scene_kind() == Some(SceneKindV1::InGame);
    {
        let mut s = st.lock().unwrap();
        s.frames = s.frames.wrapping_add(1);
        if !in_game {
            if s.in_game {
                s.in_game = false;
                s.open = false;
                s.fail = 0;
            }
            return;
        }
        s.in_game = true;
    }
    if !ctx.ui_exists("main") || !ctx.ui_exists("main.bottom") {
        return;
    }
    // 比赛流程界面（BP、对局、赛后）会盖住管理 UI，按钮跟着原版底部栏一起藏。
    let in_match = matches!(
        ctx.client_scene_kind(),
        Some(ClientSceneKindV1::Match)
            | Some(ClientSceneKindV1::MatchResult)
            | Some(ClientSceneKindV1::InGame)
            | Some(ClientSceneKindV1::SetFeedback)
            | Some(ClientSceneKindV1::LockerRoom)
    );
    if in_match {
        let was = {
            let mut s = st.lock().unwrap();
            let was = s.open;
            s.open = false;
            was
        };
        if was && ctx.ui_exists(PO_UI_PATH) {
            ctx.ui_set_visible(PO_UI_PATH, false);
        }
        if ctx.ui_exists(PO_BTN_PATH) {
            ctx.ui_set_visible(PO_BTN_PATH, false);
        }
        return;
    }
    if ctx.ui_visible(PO_BTN_PATH) == Some(false) {
        ctx.ui_set_visible(PO_BTN_PATH, true);
    }
    if ctx.key_pressed("F8") {
        let open = {
            let mut s = st.lock().unwrap();
            s.open = !s.open;
            s.dirty = true;
            s.open
        };
        if ctx.ui_exists(PO_UI_PATH) {
            ctx.ui_set_visible(PO_UI_PATH, open);
        }
    }
    po_ensure_ui(ctx, st);
    let dirty = {
        let mut s = st.lock().unwrap();
        // 面板开着时每秒重读一次赛程：比赛打完/对阵确定后比分和队名会自动更新。
        if s.open && s.frames % 60 == 0 {
            s.dirty = true;
        }
        s.dirty && s.open
    };
    if dirty {
        po_redraw(ctx);
        st.lock().unwrap().dirty = false;
    }
}

fn po_ensure_ui(ctx: &mut StableClient<'_>, st: &Arc<Mutex<PoState>>) {
    if !ctx.ui_exists(PO_BTN_PATH) {
        if ctx.ui_spawn_source("main.bottom", PO_BTN_SRC) {
            po_bind(ctx, st, PO_BTN_PATH, false);
        }
    }
    if !ctx.ui_exists(PO_UI_PATH) {
        if st.lock().unwrap().fail >= 3 {
            return;
        }
        if !po_spawn_tree(ctx, "main", PO_ROOT_SRC) {
            st.lock().unwrap().fail += 1;
            return;
        }
    }
    let shell = format!("{PO_UI_PATH}.panel");
    if !ctx.ui_exists(&shell) {
        if !po_spawn_tree(ctx, PO_UI_PATH, PO_SHELL_SRC) {
            return;
        }
    }
    if !ctx.ui_exists(&format!("{shell}.header")) {
        if !po_spawn_tree(ctx, &shell, &po_header_src()) {
            return;
        }
    }
    for slot in 0..PO_ROWS {
        let path = format!("{shell}.r{slot}");
        if ctx.ui_exists(&path) {
            continue;
        }
        if !po_spawn_tree(ctx, &shell, &po_row_src(slot)) {
            return;
        }
    }
    let close_path = format!("{shell}.header.close");
    if !st.lock().unwrap().handlers && ctx.ui_exists(&close_path) {
        po_bind(ctx, st, &close_path, true);
        st.lock().unwrap().handlers = true;
    }
    let open = st.lock().unwrap().open;
    ctx.ui_set_visible(PO_UI_PATH, open);
}

// -- 单节点 spawn（host 拒绝一次声明多个节点的 source） --

struct PoNode {
    name: String,
    src: String,
    kids: Vec<PoNode>,
}

fn po_spawn_tree(ctx: &mut StableClient<'_>, parent: &str, src: &str) -> bool {
    match po_parse_node(src) {
        Some(n) => po_spawn_node(ctx, parent, &n),
        None => false,
    }
}

fn po_spawn_node(ctx: &mut StableClient<'_>, parent: &str, node: &PoNode) -> bool {
    let path = format!("{parent}.{}", node.name);
    if !ctx.ui_exists(&path) && !ctx.ui_spawn_source(parent, &node.src) {
        return false;
    }
    node.kids.iter().all(|k| po_spawn_node(ctx, &path, k))
}

fn po_find_open(b: &[u8], mut i: usize) -> Option<usize> {
    let mut in_str = false;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'{' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn po_match_close(b: &[u8], open: usize) -> Option<usize> {
    let mut depth = 1i32;
    let mut i = open + 1;
    let mut in_str = false;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn po_stmt_start(b: &[u8], mut i: usize) -> bool {
    while i > 0 {
        i -= 1;
        let c = b[i];
        if (c as char).is_whitespace() {
            continue;
        }
        return c == b'{' || c == b'}' || c == b';';
    }
    true
}

fn po_parse_node(src: &str) -> Option<PoNode> {
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] as char).is_whitespace() {
        i += 1;
    }
    let start = i;
    let open = po_find_open(b, i)?;
    let head = src[start..open].trim();
    let colon = head.find(':')?;
    let name = head[..colon].trim().trim_start_matches('#').to_string();
    let close = po_match_close(b, open)?;
    let (own, kids) = po_split_body(&src[open + 1..close]);
    Some(PoNode {
        name,
        src: format!("{} {{\n{}}}\n", head.trim_start_matches('#'), own),
        kids,
    })
}

fn po_split_body(body: &str) -> (String, Vec<PoNode>) {
    let b = body.as_bytes();
    let mut own = String::new();
    let mut kids = Vec::new();
    let (mut i, mut last) = (0usize, 0usize);
    while i < b.len() {
        let c = b[i];
        if c == b'"' {
            i += 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if b[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if c == b'{' {
            i = po_match_close(b, i).map_or(i + 1, |x| x + 1);
            continue;
        }
        if c == b'#' && po_stmt_start(b, i) {
            if let Some(open) = po_find_open(b, i) {
                if let Some(close) = po_match_close(b, open) {
                    own.push_str(&body[last..i]);
                    if let Some(kid) = po_parse_node(&body[i..close + 1]) {
                        kids.push(kid);
                    }
                    last = close + 1;
                    i = close + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    own.push_str(&body[last..]);
    (own, kids)
}

// -- 点击绑定（host 会同时发 click 和 path-event，按帧去重） --

fn po_bind(ctx: &mut StableClient<'_>, st: &Arc<Mutex<PoState>>, path: &str, close: bool) {
    let last = Arc::new(AtomicU64::new(u64::MAX));
    let run = {
        let st = st.clone();
        let last = last.clone();
        move |c: &mut StableClient<'_>| {
            let f = st.lock().unwrap().frames;
            if last.swap(f, Ordering::SeqCst) == f {
                return;
            }
            let mut s = st.lock().unwrap();
            if close {
                s.open = false;
            } else {
                s.open = !s.open;
                s.dirty = true;
            }
            let open = s.open;
            drop(s);
            if c.ui_exists(PO_UI_PATH) {
                c.ui_set_visible(PO_UI_PATH, open);
            }
        }
    };
    let r1 = run.clone();
    ctx.ui_register_click(path, "", move |c| r1(c));
    let r2 = run;
    ctx.ui_register_path_events(path, move |c| {
        if c.ui_current_event().map_or(true, |e| e.kind == Some(UiEventKindV1::Click)) {
            r2(c);
        }
    });
}

// -- 绘制：读服务端落盘的 16 行，填进原版样式的卡片 --

fn po_name(ctx: &StableClient<'_>, v: &str) -> String {
    v.parse::<usize>()
        .ok()
        .filter(|&id| id > 0)
        .and_then(|id| ctx.team_name(id))
        .unwrap_or_else(|| "待定".to_string())
}

fn po_logo(ctx: &StableClient<'_>, v: &str) -> Option<String> {
    v.parse::<usize>()
        .ok()
        .filter(|&id| id > 0)
        .and_then(|id| ctx.record_get_string(RecordKindV1::Team, id, "logo"))
}

/// 比分：-1 = 未打，写 "-"（和原版对阵卡一样）。
fn po_score(v: &str) -> String {
    match v.parse::<i64>() {
        Ok(n) if n >= 0 => n.to_string(),
        _ => "-".to_string(),
    }
}

/// 一行 = 一场：`(轮次, 日期, 队1名, 队2名, 比分1, 比分2, 队徽1, 队徽2)`。
struct PoCard {
    tag: String,
    date: String,
    t: [String; 2],
    s: [String; 2],
    logo: [Option<String>; 2],
}

fn po_read_card(ctx: &StableClient<'_>, line: &str) -> Option<PoCard> {
    let f: Vec<&str> = line.split('\t').collect();
    if f.len() < 6 {
        return None;
    }
    Some(PoCard {
        tag: f[1].to_string(),
        date: po_md(f[0]),
        t: [po_name(ctx, f[2]), po_name(ctx, f[3])],
        s: [po_score(f[4]), po_score(f[5])],
        logo: [po_logo(ctx, f[2]), po_logo(ctx, f[3])],
    })
}

fn po_redraw(ctx: &mut StableClient<'_>) {
    let shell = format!("{PO_UI_PATH}.panel");
    let board = ctx.save_get_string(PO_BOARD_KEY).unwrap_or_default();
    let done = board
        .lines()
        .skip(1)
        .filter(|l| l.split('\t').nth(4).and_then(|v| v.parse::<i64>().ok()).map_or(false, |v| v >= 0))
        .count();
    let state = if board.is_empty() {
        "季后赛尚未开始（45 场常规赛打完后生成 16 场对阵）".to_string()
    } else {
        format!("共 16 场 · 已结束 {done} 场 · 全部 BO7")
    };
    let _ = ctx.ui_set_text(&format!("{shell}.header.state"), &state);
    let cards: Vec<PoCard> = board
        .lines()
        .skip(1)
        .take(PO_ROWS)
        .filter_map(|l| po_read_card(ctx, l))
        .collect();
    for slot in 0..PO_ROWS {
        let base = format!("{shell}.r{slot}");
        let Some(c) = cards.get(slot) else {
            // 新赛季还没生成对阵：不留上一届的旧卡片。
            let _ = ctx.ui_set_visible(&base, false);
            continue;
        };
        let _ = ctx.ui_set_visible(&base, true);
        let _ = ctx.ui_set_text(&format!("{base}.tag"), &c.tag);
        let _ = ctx.ui_set_text(&format!("{base}.date"), &c.date);
        for i in 0..2 {
            let side = format!("{base}.team{i}");
            let _ = ctx.ui_set_text(&format!("{side}.name"), &c.t[i]);
            let _ = ctx.ui_set_text(&format!("{side}.score"), &c.s[i]);
            // 原生卡片里队徽是 logo_slot 的 image 子节点（槽提供深色圆角底）。
            let lpath = format!("{side}.logo_slot.logo");
            match &c.logo[i] {
                Some(l) => {
                    let _ = ctx.ui_set_team_logo(&lpath, l);
                    let _ = ctx.ui_set_visible(&lpath, true);
                }
                None => {
                    let _ = ctx.ui_set_visible(&lpath, false);
                }
            }
        }
    }
}

fn init(host: &StableHost) -> StableMod {
    host.log(LogLevel::Info, "[kpl_sab] naming build loaded");
    let mut decl = StableMod::new("kpl_sab");
    decl.set_server_extension(KplNaming);
    // 客户端面板：完整 16 场季后赛（原生页/日历只有 8 个卡位）。
    decl.set_extension(PoExt {
        st: Arc::new(Mutex::new(PoState {
            in_game: false,
            open: false,
            frames: 0,
            dirty: false,
            fail: 0,
            handlers: false,
        })),
    });
    decl
}

declare_stable_mod!(init);
