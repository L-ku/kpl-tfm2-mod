//! kpl_scout — Chief Scout style player finder for Teamfight Manager 2.
//!
//! Adds a **Chief Scout** button to the free middle area of the bottom bar
//! (between the centred clock and the native right-anchored buttons, so it
//! never covers an original control) and opens a panel modelled on the
//! original "Chief Scout: Best Player Finder" window: tabbed candidate lists,
//! role-aware scores, filters on the second row, a ranked table, paging and a
//! selected-player card with reasons.
//!
//! Pure client-side stable-ABI mod: builds its own UI at runtime and only
//! reads management records. No executable patching, no save mutation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mod_api_stable::{
    declare_stable_mod, ClientSceneKindV1, LogLevel, RecordKindV1, SceneKindV1, StableClient,
    StableExtension, StableHost, StableMod, UiEventKindV1,
};

const MOD_ID: &str = "kpl_scout";
const BTN_PATH: &str = "main.bottom.fp_btn";
const PANEL_PATH: &str = "main.fp_ui";
const ROWS: usize = 10;

const ROLES: [&str; 5] = ["top", "jungle", "mid", "bottom", "support"];
const ROLE_CN: [&str; 5] = ["对抗路", "打野", "中路", "发育路", "游走"];

const EXTRA: [&str; 12] = [
    "aggressive",
    "concentration",
    "control_speed",
    "ego",
    "judgement",
    "last_hit",
    "mental",
    "order",
    "positioning",
    "roaming",
    "skill_avoid",
    "skill_hit",
];
const EX_AGGR: usize = 0;
const EX_EGO: usize = 3;
const EX_POS: usize = 8;
const EX_ROAM: usize = 9;
const EX_HIT: usize = 11;

const TEAM_PATHS: [&str; 6] = [
    "contract.InContract.team_id",
    "contract.team_id",
    "team_id",
    "contract.InContract.team",
    "contract.team",
    "team",
];

const ST_OTHER: i32 = 0;
const ST_OWN: i32 = 1;
const ST_FREE: i32 = 2;

const TABS: [&str; 5] = ["全部", "自由球员", "转会", "升级", "本队"];

#[derive(Clone, Default)]
struct Row {
    id: usize,
    name: String,
    age: i64,
    pot: i64,
    pot_hi: i64,
    team_id: usize,
    team: String,
    scores: [f64; 5],
    extra: [f64; 12],
    status: i32,
}

impl Row {
    fn best_role(&self) -> usize {
        let mut best = 0;
        for i in 1..5 {
            if self.scores[i] > self.scores[best] {
                best = i;
            }
        }
        best
    }

    fn score_for(&self, role: i32) -> f64 {
        if (0..5).contains(&role) {
            self.scores[role as usize]
        } else {
            self.scores[self.best_role()]
        }
    }

    fn role_for(&self, role: i32) -> usize {
        if (0..5).contains(&role) {
            role as usize
        } else {
            self.best_role()
        }
    }

    fn pot_text(&self) -> String {
        if self.pot <= 0 {
            "-".to_string()
        } else if self.pot_hi > self.pot {
            format!("{}-{}", self.pot, self.pot_hi)
        } else {
            self.pot.to_string()
        }
    }
}

struct State {
    in_game: bool,
    open: bool,
    tab: i32,  // 0 all, 1 free, 2 transfer, 3 upgrade, 4 own
    role: i32, // -1 all
    pot_min: i64,
    age_max: i64,
    upgrade_only: bool,
    sort: i32, // 0 score, 1 potential, 2 age
    page: usize,
    sel: usize, // index into the current filtered list
    rows: Vec<Row>,
    my_best: [f64; 5],
    my_team: usize,
    reload: bool,
    dirty: bool,
    spawn_fails: u32,
    handlers_done: bool,
    probe_done: bool,
    frames: u64,
    nav_frame: u64,
    /// Solo Rank info card: click sets `sr_want`, the next full-context tick
    /// fills the card (record reads are not available inside click callbacks).
    sr_want: bool,
    sr_open: bool,
}

impl State {
    fn new() -> Self {
        Self {
            in_game: false,
            open: false,
            tab: 0,
            role: -1,
            pot_min: 0,
            age_max: 0,
            upgrade_only: false,
            sort: 0,
            page: 0,
            sel: 0,
            rows: Vec::new(),
            my_best: [0.0; 5],
            my_team: 0,
            reload: false,
            dirty: false,
            spawn_fails: 0,
            handlers_done: false,
            probe_done: false,
            frames: 0,
            nav_frame: u64::MAX,
            sr_want: false,
            sr_open: false,
        }
    }

    fn preset(&mut self) {
        self.role = -1;
        self.pot_min = 0;
        self.age_max = 0;
        self.upgrade_only = false;
        self.sort = 0;
        self.page = 0;
        self.sel = 0;
    }
}

struct ScoutExt {
    st: Arc<Mutex<State>>,
}

impl StableExtension for ScoutExt {
    fn post_update(&self, ctx: &mut StableClient<'_>, _dt_micros: u64) {
        tick(ctx, &self.st);
    }
}

// ---------------------------------------------------------------------------
// main loop
// ---------------------------------------------------------------------------

fn tick(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let in_game = ctx.scene_kind() == Some(SceneKindV1::InGame);
    {
        let mut s = st.lock().unwrap();
        s.frames = s.frames.wrapping_add(1);
        if !in_game {
            if s.in_game {
                s.in_game = false;
                s.open = false;
                s.rows.clear();
                s.reload = true;
                s.page = 0;
                s.sel = 0;
                s.spawn_fails = 0;
                s.handlers_done = false;
            }
            return;
        }
        s.in_game = true;
    }

    if !ctx.ui_exists("main") || !ctx.ui_exists("main.bottom") {
        return;
    }

    // The management UI (including our button) stays alive behind the match
    // screens, so the button used to be visible while playing a match. Hide it
    // and close the panel on every match-flow screen.
    let in_match = matches!(
        ctx.client_scene_kind(),
        Some(ClientSceneKindV1::Match)
            | Some(ClientSceneKindV1::MatchResult)
            | Some(ClientSceneKindV1::InGame)
            | Some(ClientSceneKindV1::SetFeedback)
            | Some(ClientSceneKindV1::LockerRoom)
    );
    if in_match {
        let was_open = {
            let mut s = st.lock().unwrap();
            let was_open = s.open;
            s.open = false;
            s.sr_open = false;
            s.sr_want = false;
            was_open
        };
        if was_open && ctx.ui_exists(PANEL_PATH) {
            ctx.ui_set_visible(PANEL_PATH, false);
        }
        if ctx.ui_exists(BTN_PATH) {
            ctx.ui_set_visible(BTN_PATH, false);
        }
        return;
    }

    ensure_ui(ctx, st);

    // Back on a management screen: make sure a previously hidden button shows.
    if ctx.ui_visible(BTN_PATH) == Some(false) {
        ctx.ui_set_visible(BTN_PATH, true);
    }

    // Keyboard fallback: the panel can always be toggled with F7 even if a
    // hover/click slot is unavailable on the host build.
    if ctx.key_pressed("F7") {
        let open = {
            let mut s = st.lock().unwrap();
            s.open = !s.open;
            if s.open {
                s.reload = true;
                s.dirty = true;
            }
            s.open
        };
        write_log(&format!("F7 -> open={open}"));
        if ctx.ui_exists(PANEL_PATH) {
            ctx.ui_set_visible(PANEL_PATH, open);
        }
    }

    let (open, reload, dirty, probe) = {
        let s = st.lock().unwrap();
        (s.open, s.reload, s.dirty, !s.probe_done && s.frames > 90)
    };

    if open && reload {
        let (rows, my_best, my_team) = load_all(ctx);
        let mut s = st.lock().unwrap();
        s.rows = rows;
        s.my_best = my_best;
        s.my_team = my_team;
        s.reload = false;
        s.dirty = true;
        let mp = max_page(&s);
        s.page = s.page.min(mp);
    }

    if open && st.lock().unwrap().sr_want {
        fill_solo_rank(ctx, st);
    }

    if open && dirty {
        redraw(ctx, st);
        st.lock().unwrap().dirty = false;
    }

    if probe {
        probe_once(ctx, st);
    }
}

fn ensure_ui(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    if !ctx.ui_exists(BTN_PATH) {
        let ok = ctx.ui_spawn_source("main.bottom", BUTTON_SRC);
        write_log(if ok { "button spawned" } else { "button spawn FAILED" });
        if ok {
            bind(ctx, BTN_PATH, st, Action::Open);
        }
    }

    // `spawn_source` only accepts a single node with properties: a source that
    // also declares child nodes is rejected (that is why the one-piece panel
    // and every multi-node section failed). So every section source is parsed
    // into a tree and spawned one node per call.
    if !ctx.ui_exists(PANEL_PATH) {
        if st.lock().unwrap().spawn_fails >= 3 {
            return;
        }
        if spawn_source_tree(ctx, "main", ROOT_SRC, "root") {
            write_log("root spawned");
            st.lock().unwrap().handlers_done = false;
        } else {
            let mut s = st.lock().unwrap();
            s.spawn_fails += 1;
            write_log("root spawn FAILED");
            return;
        }
    }

    if !ctx.ui_exists(PANEL_PATH) {
        return;
    }

    let shell = format!("{PANEL_PATH}.panel");
    if !ctx.ui_exists(&shell) {
        let ok = spawn_source_tree(ctx, PANEL_PATH, SHELL_SRC, "shell");
        write_log(if ok { "shell spawned" } else { "shell spawn FAILED" });
        if !ok {
            return;
        }
    }

    let mut all_ok = true;
    for (name, src) in &section_sources() {
        if st.lock().unwrap().spawn_fails >= 6 {
            all_ok = false;
            continue;
        }
        let root = format!("{shell}.{name}");
        let had_root = ctx.ui_exists(&root);
        let ok = spawn_source_tree(ctx, &shell, src, name);
        if ok {
            if !had_root {
                write_log(&format!("section {name}: ok"));
            }
        } else {
            all_ok = false;
            // Only a missing section root counts as a hard failure; children
            // can lag one frame behind their parent and are retried.
            if !ctx.ui_exists(&root) {
                st.lock().unwrap().spawn_fails += 1;
                write_log(&format!("section {name}: root FAILED"));
            } else if !had_root {
                write_log(&format!("section {name}: children pending"));
            }
        }
    }

    let list = format!("{shell}.list");
    if ctx.ui_exists(&list) {
        for i in 0..ROWS {
            if st.lock().unwrap().spawn_fails >= 6 {
                all_ok = false;
                break;
            }
            let path = format!("{list}.r{i}");
            let had = ctx.ui_exists(&path);
            if !spawn_source_tree(ctx, &list, &row_source(i), &format!("row{i}")) {
                all_ok = false;
                if !ctx.ui_exists(&path) && !had {
                    st.lock().unwrap().spawn_fails += 1;
                    write_log(&format!("row{i}: root FAILED"));
                }
            }
        }
    }

    if all_ok && !st.lock().unwrap().handlers_done {
        register_panel_handlers(ctx, st);
        st.lock().unwrap().handlers_done = true;
        write_log("handlers registered");
    }

    // Keep the root in sync with the state: a click that arrives before the
    // panel finished spawning still opens it on the next frame.
    let open = st.lock().unwrap().open;
    ctx.ui_set_visible(PANEL_PATH, open);
}

// ---------------------------------------------------------------------------
// one-node-at-a-time UI spawner
// ---------------------------------------------------------------------------
//
// `ui_spawn_source` rejects a source that declares child nodes, so a section
// source is split into its node tree and each node is spawned on its own under
// its parent path.

struct UNode {
    name: String,
    /// This node's own source, with every child declaration removed.
    src: String,
    kids: Vec<UNode>,
}

fn spawn_source_tree(ctx: &mut StableClient<'_>, parent: &str, src: &str, tag: &str) -> bool {
    match parse_node(src) {
        Some(node) => spawn_node(ctx, parent, &node, tag),
        None => {
            write_log(&format!("parse FAILED {tag}"));
            false
        }
    }
}

fn spawn_node(ctx: &mut StableClient<'_>, parent: &str, node: &UNode, tag: &str) -> bool {
    let path = format!("{parent}.{}", node.name);
    let mut ok = true;
    if !ctx.ui_exists(&path) && !ctx.ui_spawn_source(parent, &node.src) {
        write_log(&format!(
            "spawn FAILED {tag} -> {} ({} B)",
            node.name,
            node.src.len()
        ));
        return false;
    }
    for kid in &node.kids {
        if !spawn_node(ctx, &path, kid, tag) {
            ok = false;
        }
    }
    ok
}

fn find_open(b: &[u8], mut i: usize) -> Option<usize> {
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

fn match_close(b: &[u8], open: usize) -> Option<usize> {
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

fn is_stmt_start(b: &[u8], mut i: usize) -> bool {
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

fn parse_node(src: &str) -> Option<UNode> {
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] as char).is_whitespace() {
        i += 1;
    }
    let head_start = i;
    let open = find_open(b, i)?;
    let head = src[head_start..open].trim();
    let colon = head.find(':')?;
    let name = head[..colon].trim().trim_start_matches('#').to_string();
    let close = match_close(b, open)?;
    let (own, kids) = split_body(&src[open + 1..close]);
    let root = format!("{} {{\n{}}}\n", head.trim_start_matches('#'), own);
    Some(UNode { name, src: root, kids })
}

/// Splits a node body into its own property text and the child node trees.
fn split_body(body: &str) -> (String, Vec<UNode>) {
    let b = body.as_bytes();
    let mut own = String::new();
    let mut kids = Vec::new();
    let mut i = 0usize;
    let mut last = 0usize;
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
            i = match_close(b, i).map_or(i + 1, |close| close + 1);
            continue;
        }
        if c == b'#' && is_stmt_start(b, i) {
            if let Some(open) = find_open(b, i) {
                if let Some(close) = match_close(b, open) {
                    own.push_str(&body[last..i]);
                    if let Some(kid) = parse_node(&body[i..close + 1]) {
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

// ---------------------------------------------------------------------------
// actions
// ---------------------------------------------------------------------------

fn open_panel(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    {
        let mut s = st.lock().unwrap();
        s.open = true;
        s.reload = true;
        s.dirty = true;
        s.page = 0;
        s.sel = 0;
    }
    if ctx.ui_exists(PANEL_PATH) {
        ctx.ui_set_visible(PANEL_PATH, true);
    }
}

fn close_panel(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    {
        let mut s = st.lock().unwrap();
        s.open = false;
        s.sr_open = false;
        s.sr_want = false;
    }
    if ctx.ui_exists(PANEL_PATH) {
        ctx.ui_set_visible(PANEL_PATH, false);
    }
}

fn set_tab(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>, tab: i32) {
    let mut s = st.lock().unwrap();
    s.tab = tab;
    s.page = 0;
    s.sel = 0;
    s.dirty = true;
}

fn toggle_role(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>, role: i32) {
    let mut s = st.lock().unwrap();
    s.role = if s.role == role { -1 } else { role };
    s.page = 0;
    s.sel = 0;
    s.dirty = true;
}

fn cycle_pot(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let mut s = st.lock().unwrap();
    s.pot_min = match s.pot_min {
        0 => 80,
        80 => 90,
        90 => 95,
        _ => 0,
    };
    s.page = 0;
    s.sel = 0;
    s.dirty = true;
}

fn cycle_age(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let mut s = st.lock().unwrap();
    s.age_max = match s.age_max {
        0 => 20,
        20 => 23,
        23 => 26,
        _ => 0,
    };
    s.page = 0;
    s.sel = 0;
    s.dirty = true;
}

fn toggle_upgrade(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let mut s = st.lock().unwrap();
    s.upgrade_only = !s.upgrade_only;
    s.page = 0;
    s.sel = 0;
    s.dirty = true;
}

fn cycle_sort(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let mut s = st.lock().unwrap();
    s.sort = (s.sort + 1) % 3;
    s.page = 0;
    s.sel = 0;
    s.dirty = true;
}

fn reset_filters(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let mut s = st.lock().unwrap();
    s.preset();
    s.dirty = true;
}

fn refresh_data(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let mut s = st.lock().unwrap();
    s.reload = true;
    s.page = 0;
    s.sel = 0;
}

fn page_step(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>, delta: i32) {
    let mut s = st.lock().unwrap();
    if s.nav_frame == s.frames {
        return;
    }
    s.nav_frame = s.frames;
    let max = max_page(&s);
    let p = s.page as i32 + delta;
    s.page = p.clamp(0, max as i32) as usize;
    s.sel = 0;
    s.dirty = true;
}

fn page_edge(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>, last: bool) {
    let mut s = st.lock().unwrap();
    if s.nav_frame == s.frames {
        return;
    }
    s.nav_frame = s.frames;
    let mp = max_page(&s);
    s.page = if last { mp } else { 0 };
    s.sel = 0;
    s.dirty = true;
}

fn select_row(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>, slot: usize) {
    let mut s = st.lock().unwrap();
    s.sel = s.page * ROWS + slot;
    s.dirty = true;
}

/// Queued from the click callback (which only has a ui/asset context); the
/// card is filled on the next full-context tick by [`fill_solo_rank`].
fn open_solo_rank(_ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let mut s = st.lock().unwrap();
    s.sr_want = true;
    s.dirty = true;
}

/// Solo Rank region ids, in the order the game's own `region` i18n array uses.
const RGN_CN: [&str; 6] = ["韩国", "中国", "欧洲", "北美", "南美", "日本"];

fn region_name(id: i64) -> String {
    if id < 0 {
        return "未知".to_string();
    }
    RGN_CN
        .get(id as usize)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("#{id}"))
}

/// Champion key -> Chinese display name (`text/champion.i18n`, zh-hans).
const CHAMP_CN: &[(&str, &str)] = &[
    ("astrologer", "占星术师"),
    ("archer", "弓箭手"),
    ("bard", "吟游诗人"),
    ("barrier_magician", "结界术师"),
    ("berserker", "狂战士"),
    ("boomerang_hunter", "回旋镖手"),
    ("chef", "厨师"),
    ("clown", "小丑"),
    ("dancer", "舞姬"),
    ("dark_mage", "黑魔法师"),
    ("demon", "恶魔"),
    ("dual_blader", "双刀客"),
    ("executioner", "处刑人"),
    ("exorcist", "驱魔师"),
    ("fighter", "格斗家"),
    ("gambler", "赌徒"),
    ("ghost", "幽灵"),
    ("gunner", "快枪手"),
    ("ice_mage", "寒冰法师"),
    ("illusionist", "幻术师"),
    ("jiangshi", "僵尸"),
    ("knight", "骑士"),
    ("lancer", "长枪兵"),
    ("lightning_mage", "雷电法师"),
    ("magic_knight", "魔剑士"),
    ("monk", "僧侣"),
    ("necromancer", "死灵法师"),
    ("ninja", "忍者"),
    ("ogre", "食人魔"),
    ("plague_doctor", "瘟疫医生"),
    ("poison_dart_hunter", "毒针猎手"),
    ("priest", "牧师"),
    ("pyromancer", "火焰法师"),
    ("pythoness", "巫女"),
    ("shadowmancer", "暗影法师"),
    ("shield_bearer", "巨盾兵"),
    ("soldier", "步枪手"),
    ("swordman", "剑客"),
    ("taoist", "道士"),
    ("vampire", "吸血鬼"),
    ("werewolf", "狼人"),
    ("dokkaebi", "赤魇"),
    ("hammerer", "重锤手"),
    ("inquisitor", "审判官"),
    ("pole_warrior", "棒行者"),
    ("spirit_caller", "精灵使"),
    ("whip_master", "鞭术师"),
    ("enchanter", "附魔师"),
    ("siege_breaker", "破城者"),
    ("prisoner", "囚徒"),
    ("android", "机器人"),
    ("white_mage", "白魔法师"),
    ("wind_mage", "飓风法师"),
    ("bomber", "爆破手"),
    ("cavalry_knight", "骑兵"),
    ("druid", "德鲁伊"),
    ("voodoo_shaman", "巫毒萨满"),
    ("hitman", "杀手"),
    ("guardian_spirit", "戍魂灵"),
    ("hunter", "追猎者"),
    ("circus_blade", "戏刃师"),
    ("sand_mage", "沙术师"),
    ("alchemist", "炼金术师"),
    ("crossbowman", "弩手"),
    ("nightmare", "梦魇"),
    ("strongman", "大力士"),
    ("harpooner", "鱼叉手"),
    ("spellbreaker", "破法者"),
    ("gladiator", "角斗士"),
    ("ferryman", "摆渡人"),
    ("bombardier", "炮击手"),
    ("hypnotist", "催眠师"),
];

fn champ_name(key: &str) -> &str {
    CHAMP_CN
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, n)| *n)
        .unwrap_or(key)
}

fn json_i64_after(j: &str, key: &str) -> Option<i64> {
    let pat = format!("\"{key}\":");
    let i = j.find(&pat)? + pat.len();
    let rest = &j[i..];
    let end = rest.find(|c: char| c == ',' || c == '}').unwrap_or(rest.len());
    rest[..end].trim().parse::<i64>().ok()
}

fn top_champions(j: &str, n: usize) -> Vec<(String, i64)> {
    let mut out: Vec<(String, i64)> = Vec::new();
    let mut pos = 0usize;
    while let Some(off) = j[pos..].find("{\"floor\":") {
        let i = pos + off;
        let before = &j[..i];
        if let Some(q) = before.rfind('"') {
            if let Some(q2) = before[..q].rfind('"') {
                let name = j[q2 + 1..q].to_string();
                if let Some(voff) = j[i..].find("\"value\":") {
                    let vs = i + voff + 8;
                    let rest = &j[vs..];
                    let end = rest.find(|c: char| c == ',' || c == '}').unwrap_or(rest.len());
                    if let Ok(v) = rest[..end].trim().parse::<i64>() {
                        out.push((name, v));
                    }
                }
            }
        }
        pos = i + 1;
    }
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out.truncate(n);
    out
}

fn fill_solo_rank(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let row = {
        let s = st.lock().unwrap();
        let list = filtered(&s);
        if list.is_empty() {
            None
        } else {
            Some(s.rows[list[s.sel.min(list.len() - 1)]].clone())
        }
    };
    let Some(row) = row else {
        st.lock().unwrap().sr_want = false;
        return;
    };
    let ranks = ctx
        .record_get_json(RecordKindV1::Athlete, row.id, "soloranks")
        .unwrap_or_default();
    let rating = json_i64_after(&ranks, "rating").unwrap_or(0);
    let wins = json_i64_after(&ranks, "wins").unwrap_or(0);
    let losses = json_i64_after(&ranks, "losses").unwrap_or(0);
    let region = region_name(json_i64_after(&ranks, "region_id").unwrap_or(-1));
    let prof = ctx
        .record_get_json(RecordKindV1::Athlete, row.id, "champion_proficiency")
        .unwrap_or_default();
    let champs = top_champions(&prof, 3);

    let base = format!("{PANEL_PATH}.panel.sr");
    let role = row.role_for(-1);
    let _ = ctx.ui_set_text(&format!("{base}.sr_l0"), &format!("{}  ·  {}  ·  {}", row.name, ROLE_CN[role], if row.team.is_empty() { "自由球员" } else { row.team.as_str() }));
    let _ = ctx.ui_set_text(&format!("{base}.sr_l1"), &format!("Solo Rank 评分: {rating}   战绩: {wins} 胜 {losses} 负   区域: {region}"));
    let _ = ctx.ui_set_text(&format!("{base}.sr_l2"), &format!("隐藏潜力: {}   年龄: {}", row.pot_text(), row.age));
    let _ = ctx.ui_set_text(&format!("{base}.sr_l3"), "擅长英雄 (图标 + 熟练度):");
    for i in 0..3 {
        let icon = format!("{base}.sr_i{i}");
        match champs.get(i) {
            Some((key, v)) => {
                let ok = ctx.ui_set_champion_icon(&icon, key, 72.0, 72.0, 2.0);
                write_log(&format!("champion icon {key}: {ok}"));
                let _ = ctx.ui_set_visible(&icon, true);
                let _ = ctx.ui_set_text(
                    &format!("{base}.sr_l{}", 4 + i),
                    &format!("{}  {}", champ_name(key), v),
                );
            }
            None => {
                let _ = ctx.ui_set_visible(&icon, false);
                let _ = ctx.ui_set_text(&format!("{base}.sr_l{}", 4 + i), "");
            }
        }
    }
    let _ = ctx.ui_set_text(
        &format!("{base}.sr_l7"),
        "数据来自球员记录；游戏原生 Solo Rank 界面无法由 SDK 切换。",
    );
    let _ = ctx.ui_set_visible(&base, true);
    let mut s = st.lock().unwrap();
    s.sr_want = false;
    s.sr_open = true;
    s.dirty = true;
}

fn close_solo_rank(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let mut s = st.lock().unwrap();
    s.sr_open = false;
    s.sr_want = false;
    s.dirty = true;
    drop(s);
    let _ = ctx.ui_set_visible(&format!("{PANEL_PATH}.panel.sr"), false);
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Open,
    Close,
    Tab(i32),
    Role(i32),
    Pot,
    Age,
    Upgrade,
    Sort,
    Reset,
    Refresh,
    Prev,
    Next,
    First,
    Last,
    Row(usize),
    SoloRank,
    SrClose,
}

fn apply_action(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>, action: Action) {
    write_log(&format!("action {action:?}"));
    match action {
        Action::Open => open_panel(ctx, st),
        Action::Close => close_panel(ctx, st),
        Action::Tab(i) => set_tab(ctx, st, i),
        Action::Role(r) => toggle_role(ctx, st, r),
        Action::Pot => cycle_pot(ctx, st),
        Action::Age => cycle_age(ctx, st),
        Action::Upgrade => toggle_upgrade(ctx, st),
        Action::Sort => cycle_sort(ctx, st),
        Action::Reset => reset_filters(ctx, st),
        Action::Refresh => refresh_data(ctx, st),
        Action::Prev => page_step(ctx, st, -1),
        Action::Next => page_step(ctx, st, 1),
        Action::First => page_edge(ctx, st, false),
        Action::Last => page_edge(ctx, st, true),
        Action::Row(i) => select_row(ctx, st, i),
        Action::SoloRank => open_solo_rank(ctx, st),
        Action::SrClose => close_solo_rank(ctx, st),
    }
}

/// Register both the click slot and the generic path-event slot: hosts differ
/// in which of the two fires for a plain button. The host delivers BOTH on
/// this build, so a per-binding frame guard collapses the duplicate — without
/// it every toggle (role/upgrade) fires twice and cancels itself out.
fn bind(ctx: &mut StableClient<'_>, path: &str, st: &Arc<Mutex<State>>, action: Action) {
    let last = Arc::new(AtomicU64::new(u64::MAX));
    let a = st.clone();
    let la = last.clone();
    let clicked = ctx.ui_register_click(path, "", move |c| {
        let f = a.lock().unwrap().frames;
        if la.swap(f, Ordering::SeqCst) != f {
            apply_action(c, &a, action);
        }
    });
    let b = st.clone();
    let lb = last.clone();
    let events = ctx.ui_register_path_events(path, move |c| {
        let click = c
            .ui_current_event()
            .map_or(true, |e| e.kind == Some(UiEventKindV1::Click));
        if click {
            let f = b.lock().unwrap().frames;
            if lb.swap(f, Ordering::SeqCst) != f {
                apply_action(c, &b, action);
            }
        }
    });
    write_log(&format!(
        "bind {path}: click={clicked} events={events} exists={}",
        ctx.ui_exists(path)
    ));
}

// ---------------------------------------------------------------------------
// data
// ---------------------------------------------------------------------------

fn read_i64(ctx: &StableClient<'_>, id: usize, paths: &[&str]) -> Option<i64> {
    for p in paths {
        if let Some(v) = ctx.record_get_i64(RecordKindV1::Athlete, id, p) {
            return Some(v);
        }
    }
    None
}

fn read_f64(ctx: &StableClient<'_>, id: usize, paths: &[&str]) -> Option<f64> {
    for p in paths {
        if let Some(j) = ctx.record_get_json(RecordKindV1::Athlete, id, p) {
            if let Ok(v) = j.trim().parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

fn load_all(ctx: &mut StableClient<'_>) -> (Vec<Row>, [f64; 5], usize) {
    let my_team = ctx.player_team_id().unwrap_or(0);
    let ids = ctx.athlete_ids();
    let mut rows = Vec::with_capacity(ids.len());

    for id in ids {
        let name = ctx.athlete_name(id).unwrap_or_default();
        if name.trim().is_empty() {
            continue;
        }
        let mut scores = [0.0f64; 5];
        for (i, role) in ROLES.iter().enumerate() {
            let a = format!("stat.{role}");
            let b = format!("initial_stat.{role}");
            scores[i] = read_f64(ctx, id, &[&a, &b]).unwrap_or(0.0);
        }
        let mut extra = [0.0f64; 12];
        for (i, key) in EXTRA.iter().enumerate() {
            let a = format!("stat.{key}");
            let b = format!("initial_stat.{key}");
            extra[i] = read_f64(ctx, id, &[&a, &b]).unwrap_or(0.0);
        }
        let age = read_i64(ctx, id, &["age"]).unwrap_or(0);
        // The real ceiling is `hidden.potential`; the estimate fields only fill
        // in once the athlete has been scouted.
        let pot = read_i64(
            ctx,
            id,
            &["hidden.potential", "potential_estimate_high", "potential_estimate_low"],
        )
        .unwrap_or(0);
        let pot_hi =
            read_i64(ctx, id, &["potential_estimate_high", "hidden.potential"]).unwrap_or(pot);
        // `contract` is an enum: FreeAgent has no team, InContract does (team id
        // 0 is a real team — the player's club in this save).
        let contract = ctx
            .record_get_json(RecordKindV1::Athlete, id, "contract")
            .unwrap_or_default();
        let is_free = contract.contains("\"FreeAgent\"");
        let team_id = if is_free {
            0
        } else {
            read_i64(ctx, id, &TEAM_PATHS).unwrap_or(0).max(0) as usize
        };
        let team = if is_free {
            String::new()
        } else {
            ctx.team_name(team_id).unwrap_or_default()
        };
        let status = if is_free {
            ST_FREE
        } else if team_id == my_team {
            ST_OWN
        } else {
            ST_OTHER
        };
        rows.push(Row { id, name, age, pot, pot_hi, team_id, team, scores, extra, status });
    }

    let mut best = [0.0f64; 5];
    for r in &rows {
        if r.status == ST_OWN {
            for i in 0..5 {
                if r.scores[i] > best[i] {
                    best[i] = r.scores[i];
                }
            }
        }
    }
    (rows, best, my_team)
}

fn delta(s: &State, r: &Row) -> f64 {
    let role = r.role_for(s.role);
    r.scores[role] - s.my_best[role]
}

fn filtered(s: &State) -> Vec<usize> {
    let mut list: Vec<usize> = s
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            let tab_ok = match s.tab {
                1 => r.status == ST_FREE,
                2 => r.status == ST_OTHER,
                3 => delta(s, r) > 0.0 && r.status != ST_OWN,
                4 => r.status == ST_OWN,
                _ => true,
            };
            let pot_ok = s.pot_min == 0 || r.pot >= s.pot_min;
            let age_ok = s.age_max == 0 || (r.age > 0 && r.age <= s.age_max);
            let up_ok = !s.upgrade_only || delta(s, r) > 0.0;
            tab_ok && pot_ok && age_ok && up_ok
        })
        .map(|(i, _)| i)
        .collect();

    let role = s.role;
    let sort = s.sort;
    list.sort_by(|&a, &b| {
        let ra = &s.rows[a];
        let rb = &s.rows[b];
        let ord = match sort {
            1 => rb.pot_hi.cmp(&ra.pot_hi),
            2 => ra.age.cmp(&rb.age),
            _ => rb
                .score_for(role)
                .partial_cmp(&ra.score_for(role))
                .unwrap_or(std::cmp::Ordering::Equal),
        };
        ord.then_with(|| ra.name.cmp(&rb.name))
    });
    list
}

fn max_page(s: &State) -> usize {
    let n = filtered(s).len();
    n.saturating_sub(1) / ROWS
}

// ---------------------------------------------------------------------------
// rendering
// ---------------------------------------------------------------------------

fn redraw(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let s = st.lock().unwrap();
    let list = filtered(&s);
    let pages = list.len().saturating_sub(1) / ROWS + 1;
    let page = s.page.min(pages.saturating_sub(1));
    let start = page * ROWS;

    for slot in 0..ROWS {
        let base = format!("{PANEL_PATH}.panel.list.r{slot}");
        let idx = start + slot;
        if idx < list.len() {
            let r = &s.rows[list[idx]];
            let role = r.role_for(s.role);
            let _ = ctx.ui_set_visible(&base, true);
            let _ = ctx.ui_set_text(&format!("{base}.rk"), &format!("#{:02}", idx + 1));
            let _ = ctx.ui_set_text(&format!("{base}.nm"), &r.name);
            let _ = ctx.ui_set_text(&format!("{base}.sc"), &format!("{:.2}", r.scores[role]));
            let _ = ctx.ui_set_text(&format!("{base}.role"), ROLE_CN[role]);
            let _ = ctx.ui_set_text(&format!("{base}.stt"), status_cn(r.status));
            let _ = ctx.ui_set_text(
                &format!("{base}.ap"),
                &format!(
                    "{} / {}",
                    if r.age > 0 { r.age.to_string() } else { "-".into() },
                    r.pot_text()
                ),
            );
            let _ = ctx.ui_set_text(&format!("{base}.tags"), &tags_for(r, role));
            let sel = idx == s.sel.min(list.len().saturating_sub(1));
            let color = if sel { "#1d4a3eff" } else { "#11131bff" };
            let _ = ctx.ui_set_properties(&base, &format!("color: {color};"));
        } else {
            let _ = ctx.ui_set_visible(&base, false);
        }
    }

    // Status strip.
    let view = match s.tab {
        1 => "自由球员",
        2 => "转会",
        3 => "升级",
        4 => "本队",
        _ => "全部",
    };
    let pos = if s.role >= 0 { ROLE_CN[s.role as usize] } else { "全部" };
    let sort = match s.sort {
        1 => "潜力",
        2 => "年龄",
        _ => "评分",
    };
    let pot = match s.pot_min {
        0 => "全部".to_string(),
        v => format!("{v}+"),
    };
    let age = match s.age_max {
        0 => "全部".to_string(),
        v => format!("≤{v}"),
    };
    let up = if s.upgrade_only { "仅升级" } else { "全部" };
    let preset = if s.role < 0 && s.pot_min == 0 && s.age_max == 0 && !s.upgrade_only
        && s.sort == 0
    {
        "默认"
    } else {
        "自定义"
    };
    let _ = ctx.ui_set_text(&format!("{PANEL_PATH}.panel.status.v_view"), &format!("视图: {view}"));
    let _ = ctx.ui_set_text(&format!("{PANEL_PATH}.panel.status.v_pos"), &format!("位置: {pos}"));
    let _ = ctx.ui_set_text(&format!("{PANEL_PATH}.panel.status.v_rows"), &format!("行: {}", list.len()));
    let _ = ctx.ui_set_text(&format!("{PANEL_PATH}.panel.status.v_sort"), &format!("排序: {sort}"));
    let _ = ctx.ui_set_text(&format!("{PANEL_PATH}.panel.status.v_pot"), &format!("潜力: {pot}"));
    let _ = ctx.ui_set_text(&format!("{PANEL_PATH}.panel.status.v_age"), &format!("年龄: {age}"));
    let _ = ctx.ui_set_text(&format!("{PANEL_PATH}.panel.status.v_up"), &format!("升级: {up}"));
    let _ = ctx.ui_set_text(&format!("{PANEL_PATH}.panel.status.v_preset"), &format!("预设: {preset}"));
    let _ = ctx.ui_set_text(
        &format!("{PANEL_PATH}.panel.pager.pg_lbl"),
        &format!("{} / {}", page + 1, pages),
    );
    let shown_end = (start + ROWS).min(list.len());
    let _ = ctx.ui_set_text(
        &format!("{PANEL_PATH}.panel.pager.pg_info"),
        &format!(
            "{} / {} · {}-{} / {} 名候选",
            page + 1,
            pages,
            if list.is_empty() { 0 } else { start + 1 },
            shown_end,
            list.len()
        ),
    );

    // Tabs and role buttons.
    for i in 0..5 {
        paint_tab(ctx, &format!("tab{i}"), s.tab == i as i32);
        paint_role(ctx, &format!("role{i}"), s.role == i as i32);
    }
    paint_small(ctx, "b_pot", s.pot_min != 0);
    paint_small(ctx, "b_age", s.age_max != 0);
    paint_small(ctx, "b_up", s.upgrade_only);

    // Selected-player card.
    if !list.is_empty() {
        let idx = s.sel.min(list.len() - 1);
        let r = &s.rows[list[idx]];
        let role = r.role_for(s.role);
        let d = r.scores[role] - s.my_best[role];
        let _ = ctx.ui_set_text(&format!("{PANEL_PATH}.panel.dcard.d_name"), &r.name);
        let _ = ctx.ui_set_text(
            &format!("{PANEL_PATH}.panel.dcard.d_meta"),
            &format!(
                "{} · {} · {} 岁 · 潜力 {} · {}",
                ROLE_CN[role],
                if r.team.is_empty() { "自由球员" } else { r.team.as_str() },
                r.age,
                r.pot_text(),
                status_cn(r.status)
            ),
        );
        let _ = ctx.ui_set_text(
            &format!("{PANEL_PATH}.panel.dcard.d_score"),
            &format!("评分 {:.2} · 提升 {:+.2}", r.scores[role], d),
        );
        let reasons = reasons_for(&s, r, role);
        for i in 0..4 {
            let _ = ctx.ui_set_text(
                &format!("{PANEL_PATH}.panel.dcard.d_r{i}"),
                reasons.get(i).map(|x| x.as_str()).unwrap_or(""),
            );
        }
        let _ = ctx.ui_set_text(
            &format!("{PANEL_PATH}.panel.dcard.d_hint1"),
            "查看评分、战绩与擅长英雄。",
        );
        let _ = ctx.ui_set_text(
            &format!("{PANEL_PATH}.panel.dcard.d_hint2"),
            "Solo Rank 资料卡",
        );
    }
    let _ = ctx.ui_set_visible(&format!("{PANEL_PATH}.panel.sr"), s.sr_open);
}

fn tags_for(r: &Row, role: usize) -> String {
    let mut t: Vec<&str> = Vec::new();
    if r.pot >= 95 {
        t.push("精英潜力");
    } else if r.pot >= 85 {
        t.push("高潜力");
    }
    if r.age > 0 && r.age <= 20 {
        t.push("年轻");
    } else if r.age <= 24 {
        t.push("黄金年龄");
    }
    let score = r.scores[role];
    if score >= 90.0 {
        t.push(match role {
            0 => "顶级对抗路",
            1 => "顶级打野",
            2 => "顶级中路",
            3 => "顶级发育路",
            _ => "顶级游走",
        });
    } else if score >= 82.0 {
        t.push(match role {
            0 => "团战型对抗路",
            1 => "节奏型打野",
            2 => "操作型中路",
            3 => "发育核心",
            _ => "稳健游走",
        });
    }
    if r.extra[EX_AGGR] >= 65.0 {
        t.push("侵略性强");
    }
    if t.is_empty() {
        "均衡".to_string()
    } else {
        t.join("  ·  ")
    }
}

fn reasons_for(s: &State, r: &Row, role: usize) -> Vec<String> {
    let mut out = Vec::new();
    if r.pot >= 95 {
        out.push("精英潜力".to_string());
    } else if r.pot >= 85 {
        out.push("高潜力".to_string());
    }
    let score = r.scores[role];
    if score >= 90.0 {
        out.push(format!("精英{}", ROLE_CN[role]));
    } else if score >= 82.0 {
        out.push(format!("强力{}", ROLE_CN[role]));
    }
    if r.age > 0 && r.age <= 20 {
        out.push("年轻".to_string());
    } else if r.age <= 24 {
        out.push("黄金年龄".to_string());
    }
    if r.extra[EX_AGGR] >= 65.0 {
        out.push("侵略性良好".to_string());
    }
    if r.extra[EX_EGO] >= 65.0 {
        out.push("自我风险".to_string());
    }
    if r.extra[EX_POS] >= 65.0 {
        out.push("团战站位强".to_string());
    }
    if r.extra[EX_ROAM] >= 65.0 {
        out.push("游走支援强".to_string());
    }
    if r.extra[EX_HIT] >= 65.0 {
        out.push("技能命中强".to_string());
    }
    if r.status == ST_OWN {
        out.push("本队球员".to_string());
    } else if delta(s, r) > 0.0 {
        out.push(format!("比本队同位置高 {:+.1}", delta(s, r)));
    }
    out.truncate(4);
    if out.is_empty() {
        out.push("评分均衡".to_string());
    }
    out
}

fn paint_tab(ctx: &mut StableClient<'_>, name: &str, on: bool) {
    let path = format!("{PANEL_PATH}.panel.tabs.{name}");
    let src = if on {
        "btn: { color: #2a9d83ff; } text: { color: #ffffffff; }"
    } else {
        "btn: { color: #d9f4efff; } text: { color: #0f5b4dff; }"
    };
    let _ = ctx.ui_set_properties(&path, src);
}

fn paint_role(ctx: &mut StableClient<'_>, name: &str, on: bool) {
    let path = format!("{PANEL_PATH}.panel.roles.{name}");
    let src = if on {
        "btn: { color: #2a9d83ff; } text: { color: #ffffffff; }"
    } else {
        "btn: { color: #1d2130ff; } text: { color: #cdd6e6ff; }"
    };
    let _ = ctx.ui_set_properties(&path, src);
}

fn paint_small(ctx: &mut StableClient<'_>, name: &str, on: bool) {
    let path = format!("{PANEL_PATH}.panel.filters.{name}");
    let src = if on {
        "btn: { color: #2a9d83ff; } text: { color: #ffffffff; }"
    } else {
        "btn: { color: #1d2130ff; } text: { color: #cdd6e6ff; }"
    };
    let _ = ctx.ui_set_properties(&path, src);
}

fn status_cn(status: i32) -> &'static str {
    match status {
        ST_OWN => "本队",
        ST_FREE => "自由球员",
        ST_OTHER => "转会",
        _ => "-",
    }
}

// ---------------------------------------------------------------------------
// diagnostics
// ---------------------------------------------------------------------------

fn probe_dir() -> std::path::PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(base)
        .join("TeamSamoyed")
        .join("TeamfightManager2")
        .join("kpl_scout");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn write_log(msg: &str) {
    let path = probe_dir().join("log.txt");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = writeln!(f, "[kpl_scout] {msg}");
    }
}

/// Keys directly inside the outermost JSON object, in order of appearance.
fn top_keys(j: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = j.as_bytes();
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let start = i + 1;
                let mut k = start;
                while k < bytes.len() {
                    if bytes[k] == b'\\' {
                        k += 2;
                        continue;
                    }
                    if bytes[k] == b'"' {
                        break;
                    }
                    k += 1;
                }
                if k >= bytes.len() {
                    break;
                }
                let mut m = k + 1;
                while m < bytes.len() && (bytes[m] as char).is_whitespace() {
                    m += 1;
                }
                if depth == 1 && m < bytes.len() && bytes[m] == b':' {
                    out.push(j[start..k].to_string());
                }
                i = k + 1;
            }
            b'{' | b'[' => {
                depth += 1;
                i += 1;
            }
            b'}' | b']' => {
                depth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    out
}

fn probe_once(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let mut out = String::new();
    out.push_str("== kpl_scout probe ==\n");
    let ids = ctx.athlete_ids();
    let teams = ctx.team_ids();
    out.push_str(&format!(
        "athletes={} teams={} player_team={:?}\n",
        ids.len(),
        teams.len(),
        ctx.player_team_id()
    ));
    for p in ["main", "main.bottom", "main.bottom.time", "main.bottom.proceed", BTN_PATH, PANEL_PATH] {
        out.push_str(&format!(
            "node {p}: exists={} rect={:?}\n",
            ctx.ui_exists(p),
            ctx.ui_node_rect(p)
        ));
    }
    out.push_str(&format!("bottom children: {:?}\n", ctx.ui_child_names("main.bottom")));
    for p in [
        PANEL_PATH,
        "main.fp_ui.panel",
        "main.fp_ui.panel.tabs",
        "main.fp_ui.panel.list",
        "main.fp_ui.panel.list.r0",
    ] {
        out.push_str(&format!("children {p}: {:?}\n", ctx.ui_child_names(p)));
    }
    for p in [
        "main.fp_ui.panel.tabs.tab0",
        "main.fp_ui.panel.roles.role0",
        "main.fp_ui.panel.header.close",
        "main.fp_ui.panel.filters.b_pot",
        "main.fp_ui.panel.pager.next",
        "main.fp_ui.panel.dcard.d_open",
        "main.fp_ui.panel.list.r0",
        "main.fp_ui.panel.list.r0.nm",
    ] {
        out.push_str(&format!(
            "handler path {p}: exists={} rect={:?}\n",
            ctx.ui_exists(p),
            ctx.ui_node_rect(p)
        ));
    }

    let sample: Vec<usize> = ids.iter().copied().take(4).collect();
    let mut paths: Vec<String> = vec![
        "age".into(),
        "potential".into(),
        "potential_estimate_low".into(),
        "potential_estimate_high".into(),
        "hidden.potential".into(),
        "hidden".into(),
        "trait_values".into(),
        "stat.potential".into(),
        "initial_stat.potential".into(),
        "team_id".into(),
        "team".into(),
        "contract".into(),
        "contract.InContract.team_id".into(),
        "contract.team_id".into(),
        "contract.InContract".into(),
        "salary".into(),
        "stat.top".into(),
        "stat.jungle".into(),
        "stat.mid".into(),
        "stat.bottom".into(),
        "stat.support".into(),
        "initial_stat.top".into(),
    ];
    for key in EXTRA {
        paths.push(format!("stat.{key}"));
    }
    for id in sample {
        out.push_str(&format!("\n-- athlete {id} --\n"));
        for p in ["", ".", "contract"] {
            let j = ctx.record_get_json(RecordKindV1::Athlete, id, p).unwrap_or_default();
            let shown: String = j.chars().take(3000).collect();
            out.push_str(&format!("json[{p:?}] = {shown}\n"));
        }
        for p in &paths {
            let j = ctx.record_get_json(RecordKindV1::Athlete, id, p).unwrap_or_default();
            let shown: String = j.chars().take(300).collect();
            out.push_str(&format!("  {p} = {shown}\n"));
        }
    }

    // Full, untruncated athlete JSON so the real field names can be read.
    if let Some(id) = ids.first() {
        let j = ctx.record_get_json(RecordKindV1::Athlete, *id, "").unwrap_or_default();
        let _ = std::fs::write(probe_dir().join("athlete_full.json"), &j);
        out.push_str(&format!("\ntop-level keys: {:?}\n", top_keys(&j)));
    }

    // Contract variants and team ids across the whole athlete table.
    let mut variants: Vec<(String, usize)> = Vec::new();
    let mut teams_used: Vec<(i64, usize)> = Vec::new();
    for id in &ids {
        let c = ctx
            .record_get_json(RecordKindV1::Athlete, *id, "contract")
            .unwrap_or_default();
        let v = top_keys(&c).into_iter().next().unwrap_or_else(|| "<none>".into());
        match variants.iter_mut().find(|(n, _)| *n == v) {
            Some((_, n)) => *n += 1,
            None => variants.push((v, 1)),
        }
        let t = read_i64(ctx, *id, &TEAM_PATHS);
        match t {
            Some(t) => match teams_used.iter_mut().find(|(k, _)| *k == t) {
                Some((_, n)) => *n += 1,
                None => teams_used.push((t, 1)),
            },
            None => {}
        }
    }
    variants.sort_by(|a, b| b.1.cmp(&a.1));
    teams_used.sort_by(|a, b| b.1.cmp(&a.1));
    out.push_str(&format!("\ncontract variants: {:?}\n", variants));
    out.push_str(&format!("teams used (top 15): {:?}\n", &teams_used[..teams_used.len().min(15)]));
    for t in 0..8i64 {
        out.push_str(&format!("team_name({t}) = {:?}\n", ctx.team_name(t as usize)));
    }
    for id in ids.iter().take(6) {
        out.push_str(&format!(
            "first athlete {id}: name={:?} variant={:?} team={:?}\n",
            ctx.athlete_name(*id),
            ctx.record_get_json(RecordKindV1::Athlete, *id, "contract")
                .and_then(|c| top_keys(&c).into_iter().next()),
            read_i64(ctx, *id, &TEAM_PATHS)
        ));
    }
    out.push_str(&format!(
        "first athlete full json len={}\n",
        ctx.record_get_json(RecordKindV1::Athlete, ids.first().copied().unwrap_or(0), "")
            .map(|j| j.len())
            .unwrap_or(0)
    ));

    let _ = std::fs::write(probe_dir().join("probe.txt"), out);
    st.lock().unwrap().probe_done = true;
    write_log("probe written");
}

// ---------------------------------------------------------------------------
// UI source
// ---------------------------------------------------------------------------

const BUTTON_SRC: &str = r#"
fp_btn:color_icon_button {
  @"asset/base/style/main#secondary_button";
  x: 945px;
  y: 0px;
  width: 160px;
  height: 40px;
  text: {
    text: "Chief Scout";
    font: "asset/base/font/set/bold";
    size: 16;
    align_x: Center;
    align_y: Center;
  }
}
"#;

fn label(
    name: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    text: &str,
    size: i32,
    style: &str,
    align: &str,
    color: &str,
) -> String {
    format!(
        r#"
    #{name}:label {{
      @"asset/base/style/main#{style}";
      x: {x}px;
      y: {y}px;
      width: {w}px;
      height: {h}px;
      size: {size};
      color: {color};
      align_x: {align};
      align_y: Center;
      text: "{text}";
    }}
"#
    )
}

fn tab_button(name: &str, x: i32, text: &str) -> String {
    format!(
        r#"
    #{name}:color_icon_button {{
      @"asset/base/style/main#secondary_button";
      x: {x}px;
      y: 66px;
      width: 176px;
      height: 40px;
      text: {{
        text: "{text}";
        font: "asset/base/font/set/bold";
        size: 16;
        align_x: Center;
        align_y: Center;
      }}
    }}
"#
    )
}

fn small_button(name: &str, x: i32, text: &str, w: i32) -> String {
    format!(
        r#"
    #{name}:color_icon_button {{
      @"asset/base/style/main#secondary_button";
      x: {x}px;
      y: 182px;
      width: {w}px;
      height: 34px;
      text: {{
        text: "{text}";
        font: "asset/base/font/set/bold";
        size: 15;
        align_x: Center;
        align_y: Center;
      }}
    }}
"#
    )
}

fn row_source(slot: usize) -> String {
    let col = |name: &str, w: i32, align: &str, style: &str, size: i32, color: &str| {
        format!(
            r#"
      #{name}:label {{
        @"asset/base/style/main#{style}";
        ignore_event: true;
        width: {w}px;
        height: 100%;
        size: {size};
        color: {color};
        align_x: {align};
        align_y: Center;
        text: "";
      }}
"#
        )
    };
    let mut s = String::new();
    s.push_str(&format!(
        r#"
    #r{slot}:button {{
      width: 1592px;
      height: 44px;
      source: "asset/base/sprite/white";
      color: #11131bff;
      hover: {{
        color: #1b1f2cff;
      }}
      active: {{
        color: #232838ff;
      }}
      child_type: LeftToRight {{
        spacing: 0px;
      }}
      click_sound: "asset/base/sound/sfx/UI_mouse_click";
"#
    ));
    s.push_str(&col("rk", 64, "Center", "label", 14, "#8a93a6ff"));
    s.push_str(&col("nm", 280, "Left", "bold_label", 15, "#ffffffff"));
    s.push_str(&col("sc", 90, "Left", "bold_label", 15, "#e8e8e8ff"));
    s.push_str(&col("role", 90, "Left", "label", 14, "#cdd6e6ff"));
    s.push_str(&col("stt", 120, "Left", "label", 14, "#9ba5b8ff"));
    s.push_str(&col("ap", 120, "Left", "label", 14, "#cdd6e6ff"));
    s.push_str(&col("tags", 828, "Left", "label", 13, "#9ba5b8ff"));
    s.push_str("    }\n");
    s
}

fn header_label(name: &str, x: i32, w: i32, text: &str) -> String {
    format!(
        r#"
      #{name}:label {{
        @"asset/base/style/main#bold_label";
        x: {x}px;
        y: 0px;
        width: {w}px;
        height: 40px;
        size: 12;
        color: #cdd6e6ff;
        align_x: Left;
        align_y: Center;
        text: "{text}";
      }}
"#
    )
}

const ROOT_SRC: &str = r#"
fp_ui:empty {
  width: 100%;
  height: 100%;
  z: 2000;
  visible: false;
}
"#;

const SHELL_SRC: &str = r#"
panel:color {
  x: 264px;
  y: 0px;
  width: 1656px;
  height: 1016px;
  color: #0b0d12ff;
}
"#;

fn section_root(name: &str, open: &mut String) {
    open.push_str(&format!(
        r#"
{name}:empty {{
  x: 0px;
  y: 0px;
  width: 1656px;
  height: 1016px;
"#
    ));
}

fn build_header() -> String {
    let mut s = String::new();
    section_root("header", &mut s);
    s.push_str(&label("title", 32, 10, 600, 32, "Chief Scout", 23, "bold_label", "Left", "#ffffffff"));
    s.push_str(&label(
        "subtitle",
        34,
        42,
        760,
        20,
        "按角色评分的选人面板 · 缓存数据库 / 实时筛选",
        12,
        "label",
        "Left",
        "#8a93a6ff",
    ));
    s.push_str(
        r#"
    #close:color_icon_button {
      @"asset/base/style/main#secondary_button";
      anchor_x: 1;
      pivot_x: 1;
      x: -24px;
      y: 12px;
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

    #refresh:color_icon_button {
      @"asset/base/style/main#secondary_button";
      anchor_x: 1;
      pivot_x: 1;
      x: -24px;
      y: 66px;
      width: 176px;
      height: 40px;
      text: {
        text: "刷新数据库";
        font: "asset/base/font/set/bold";
        size: 14;
        align_x: Center;
        align_y: Center;
      }
    }
"#,
    );
    s.push_str(&label("db_state", 1456, 112, 176, 20, "缓存数据库", 11, "label", "Right", "#8a93a6ff"));
    s.push_str("  }\n}\n");
    s
}

fn build_tabs() -> String {
    let mut s = String::new();
    section_root("tabs", &mut s);
    for (i, t) in TABS.iter().enumerate() {
        s.push_str(&tab_button(&format!("tab{i}"), 32 + 190 * i as i32, t));
    }
    s.push_str("  }\n}\n");
    s
}

fn build_status() -> String {
    let mut s = String::new();
    section_root("status", &mut s);
    let key = "#8a93a6ff";
    let val = "#dbe2eeff";
    let col = |name: &str, x: i32, w: i32, text: &str, color: &str| {
        label(name, x, 106, w, 20, text, 12, "label", "Left", color)
    };
    s.push_str(&col("k_scope", 32, 44, "范围:", key));
    s.push_str(&col("v_view", 78, 96, "视图: 全部", val));
    s.push_str(&col("v_pos", 190, 96, "位置: 全部", val));
    s.push_str(&col("v_region", 302, 96, "地区: 全部", val));
    s.push_str(&col("v_rows", 416, 110, "行: 0", val));
    s.push_str(&col("k_rules", 32, 44, "条件:", key));
    s.push_str(&label("v_sort", 78, 128, 96, 20, "排序: 评分", 12, "label", "Left", val));
    s.push_str(&label("v_pot", 190, 128, 96, 20, "潜力: 全部", 12, "label", "Left", val));
    s.push_str(&label("v_age", 302, 128, 96, 20, "年龄: 全部", 12, "label", "Left", val));
    s.push_str(&label("v_up", 416, 128, 96, 20, "升级: 全部", 12, "label", "Left", val));
    s.push_str(&label("v_preset", 530, 128, 120, 20, "预设: 默认", 12, "label", "Left", val));
    s.push_str("  }\n}\n");
    s
}

fn build_roles() -> String {
    let mut s = String::new();
    section_root("roles", &mut s);
    s.push_str(&label("l_pos", 32, 158, 90, 20, "位置", 12, "bold_label", "Left", "#8a93a6ff"));
    for i in 0..5usize {
        s.push_str(&small_button(&format!("role{i}"), 32 + 100 * i as i32, ROLE_CN[i], 88));
    }
    s.push_str("  }\n}\n");
    s
}

fn build_filters() -> String {
    let mut s = String::new();
    section_root("filters", &mut s);
    s.push_str(&label("l_filter", 708, 158, 90, 20, "筛选", 12, "bold_label", "Left", "#8a93a6ff"));
    s.push_str(&small_button("b_region", 708, "地区", 96));
    s.push_str(&small_button("b_pot", 816, "潜力", 96));
    s.push_str(&small_button("b_age", 924, "年龄", 96));
    s.push_str(&small_button("b_up", 1032, "升级+", 96));
    s.push_str(&small_button("b_preset", 1148, "预设", 96));
    s.push_str(&small_button("b_sort", 1262, "排序", 96));
    s.push_str(&small_button("b_reset", 1366, "重置", 96));
    s.push_str("  }\n}\n");
    s
}

fn build_thead() -> String {
    let mut s = String::new();
    s.push_str(
        r#"
thead:empty {
  x: 32px;
  y: 226px;
  width: 1592px;
  height: 40px;
"#,
    );
    s.push_str(
        r#"
  #thead_bg:color {
    x: 0px;
    y: 0px;
    width: 1592px;
    height: 40px;
    color: #161a26ff;
  }
"#,
    );
    s.push_str(&header_label("h_rk", 20, 44, "排名"));
    s.push_str(&header_label("h_nm", 64, 280, "名字"));
    s.push_str(&header_label("h_sc", 344, 90, "评分"));
    s.push_str(&header_label("h_role", 434, 90, "位置"));
    s.push_str(&header_label("h_stt", 524, 120, "状态"));
    s.push_str(&header_label("h_ap", 644, 120, "年龄 / 潜力"));
    s.push_str(&header_label("h_tags", 764, 660, "标签"));
    s.push_str("  }\n}\n");
    s
}

fn build_list() -> String {
    let mut s = String::new();
    s.push_str(
        r#"
list:empty {
  x: 32px;
  y: 268px;
  width: 1592px;
  height: 460px;
  child_type: TopToBottom {
    spacing: 0px;
  }
"#,
    );
    // Rows are spawned one at a time by `ensure_ui`: ten rows in a single
    // source string is over 20 KB, which the engine's source parser rejects.
    s.push_str("  }\n}\n");
    s
}

fn build_pager() -> String {
    let mut s = String::new();
    section_root("pager", &mut s);
    s.push_str(
        r#"
    #first:color_icon_button {
      @"asset/base/style/main#secondary_button";
      anchor_x: 1;
      pivot_x: 1;
      x: -286px;
      y: 742px;
      width: 40px;
      height: 30px;
      text: {
        text: "|<";
        font: "asset/base/font/set/bold";
        size: 13;
        align_x: Center;
        align_y: Center;
      }
    }

    #prev:color_icon_button {
      @"asset/base/style/main#secondary_button";
      anchor_x: 1;
      pivot_x: 1;
      x: -238px;
      y: 742px;
      width: 40px;
      height: 30px;
      text: {
        text: "<";
        font: "asset/base/font/set/bold";
        size: 13;
        align_x: Center;
        align_y: Center;
      }
    }

    #next:color_icon_button {
      @"asset/base/style/main#secondary_button";
      anchor_x: 1;
      pivot_x: 1;
      x: -72px;
      y: 742px;
      width: 40px;
      height: 30px;
      text: {
        text: ">";
        font: "asset/base/font/set/bold";
        size: 13;
        align_x: Center;
        align_y: Center;
      }
    }

    #last:color_icon_button {
      @"asset/base/style/main#secondary_button";
      anchor_x: 1;
      pivot_x: 1;
      x: -24px;
      y: 742px;
      width: 40px;
      height: 30px;
      text: {
        text: ">|";
        font: "asset/base/font/set/bold";
        size: 13;
        align_x: Center;
        align_y: Center;
      }
    }
"#,
    );
    s.push_str(&label("pg_info", 32, 742, 620, 30, "1 / 1 · 0-0 / 0 名候选", 13, "label", "Left", "#9ba5b8ff"));
    s.push_str(&label("pg_lbl", 1424, 742, 110, 30, "1 / 1", 14, "bold_label", "Center", "#37d5b3ff"));
    s.push_str("  }\n}\n");
    s
}

fn build_dcard() -> String {
    let mut s = String::new();
    s.push_str(
        r#"
dcard:color {
  x: 32px;
  y: 784px;
  width: 1592px;
  height: 152px;
  color: #141722ff;
  rounding: Uniform {
    rounding: 10;
  }
"#,
    );
    s.push_str(&label("d_k", 20, 12, 200, 20, "已选球员", 12, "label", "Left", "#8a93a6ff"));
    s.push_str(&label("d_name", 20, 36, 320, 32, "", 21, "bold_label", "Left", "#ffffffff"));
    s.push_str(&label("d_meta", 250, 14, 700, 22, "", 13, "label", "Left", "#dbe2eeff"));
    s.push_str(&label("d_score", 250, 40, 700, 22, "", 13, "label", "Left", "#cdd6e6ff"));
    s.push_str(&label("d_why", 720, 12, 200, 20, "原因", 12, "label", "Left", "#8a93a6ff"));
    for i in 0..4 {
        s.push_str(&label(
            &format!("d_r{i}"),
            720,
            34 + 20 * i,
            420,
            20,
            "",
            13,
            "label",
            "Left",
            "#cdd6e6ff",
        ));
    }
    s.push_str(
        r#"
    #d_open:color_icon_button {
      @"asset/base/style/main#secondary_button";
      x: 1291px;
      y: 20px;
      width: 236px;
      height: 40px;
      text: {
        text: "打开 Solo Rank";
        font: "asset/base/font/set/bold";
        size: 15;
        align_x: Center;
        align_y: Center;
      }
    }
"#,
    );
    s.push_str(&label("d_hint1", 1291, 66, 236, 18, "", 10, "label", "Center", "#8a93a6ff"));
    s.push_str(&label("d_hint2", 1291, 82, 236, 18, "", 10, "label", "Center", "#8a93a6ff"));
    s.push_str("  }\n}\n");
    s
}

/// In-panel Solo Rank card. The stable SDK cannot switch the game's native
/// screens, so the button shows the record data instead.
fn build_sr() -> String {
    let mut s = String::new();
    s.push_str(
        r#"
sr:color {
  x: 460px;
  y: 196px;
  width: 720px;
  height: 400px;
  color: #141722ff;
  visible: false;
  rounding: Uniform {
    rounding: 12;
  }
"#,
    );
    s.push_str(&label("sr_title", 28, 18, 520, 30, "Solo Rank 资料", 20, "bold_label", "Left", "#ffffffff"));
    s.push_str(
        r#"
    #sr_close:color_icon_button {
      @"asset/base/style/main#secondary_button";
      anchor_x: 1;
      pivot_x: 1;
      x: -20px;
      y: 16px;
      width: 36px;
      height: 36px;
      text: {
        text: "X";
        font: "asset/base/font/set/bold";
        size: 16;
        align_x: Center;
        align_y: Center;
      }
    }
"#,
    );
    // Text lines: name / solo-rank stats / potential+age / section header.
    for (i, y) in [56, 86, 116, 150].iter().enumerate() {
        s.push_str(&label(
            &format!("sr_l{i}"),
            32,
            *y,
            650,
            28,
            "",
            15,
            "label",
            "Left",
            "#cdd6e6ff",
        ));
    }
    // Three champion columns: face icon above, Chinese name + proficiency below.
    for i in 0..3 {
        s.push_str(&format!(
            r#"
    #sr_i{i}:image {{
      x: {x}px;
      y: 182px;
      width: 72px;
      height: 72px;
      ignore_event: true;
    }}
"#,
            x = 44 + 224 * i
        ));
        s.push_str(&label(
            &format!("sr_l{}", 4 + i),
            16 + 224 * i,
            260,
            176,
            26,
            "",
            14,
            "bold_label",
            "Center",
            "#ffffffff",
        ));
    }
    s.push_str(&label(
        "sr_l7",
        32,
        336,
        650,
        24,
        "数据来自球员记录；游戏原生 Solo Rank 界面无法由 SDK 切换。",
        12,
        "label",
        "Left",
        "#8a93a6ff",
    ));
    s.push_str("  }\n}\n");
    s
}

fn section_sources() -> Vec<(&'static str, String)> {
    vec![
        ("header", build_header()),
        ("tabs", build_tabs()),
        ("status", build_status()),
        ("roles", build_roles()),
        ("filters", build_filters()),
        ("thead", build_thead()),
        ("list", build_list()),
        ("pager", build_pager()),
        ("dcard", build_dcard()),
        ("sr", build_sr()),
    ]
}

// ---------------------------------------------------------------------------
// click wiring
// ---------------------------------------------------------------------------

fn register_panel_handlers(ctx: &mut StableClient<'_>, st: &Arc<Mutex<State>>) {
    let p = |section: &str, n: &str| format!("{PANEL_PATH}.panel.{section}.{n}");

    bind(ctx, &p("header", "close"), st, Action::Close);
    bind(ctx, &p("header", "refresh"), st, Action::Refresh);
    for i in 0..5 {
        bind(ctx, &p("tabs", &format!("tab{i}")), st, Action::Tab(i));
        bind(ctx, &p("roles", &format!("role{i}")), st, Action::Role(i));
    }
    bind(ctx, &p("filters", "b_pot"), st, Action::Pot);
    bind(ctx, &p("filters", "b_age"), st, Action::Age);
    bind(ctx, &p("filters", "b_up"), st, Action::Upgrade);
    bind(ctx, &p("filters", "b_preset"), st, Action::Reset);
    bind(ctx, &p("filters", "b_sort"), st, Action::Sort);
    bind(ctx, &p("filters", "b_reset"), st, Action::Reset);
    bind(ctx, &p("pager", "first"), st, Action::First);
    bind(ctx, &p("pager", "prev"), st, Action::Prev);
    bind(ctx, &p("pager", "next"), st, Action::Next);
    bind(ctx, &p("pager", "last"), st, Action::Last);
    bind(ctx, &p("dcard", "d_open"), st, Action::SoloRank);
    bind(ctx, &p("sr", "sr_close"), st, Action::SrClose);
    for i in 0..ROWS {
        bind(ctx, &p("list", &format!("r{i}")), st, Action::Row(i));
    }
}

// ---------------------------------------------------------------------------
// entry
// ---------------------------------------------------------------------------

fn init(host: &StableHost) -> StableMod {
    host.log(LogLevel::Info, "[kpl_scout] Chief Scout panel loaded");
    write_log("loaded");
    let mut decl = StableMod::new(MOD_ID);
    decl.set_extension(ScoutExt { st: Arc::new(Mutex::new(State::new())) });
    decl
}

declare_stable_mod!(init);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_sources() {
        let dir = std::env::temp_dir().join("kpl_scout_src");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, src) in section_sources() {
            std::fs::write(dir.join(format!("{name}.ui")), &src).unwrap();
            let n = parse_node(&src).unwrap_or_else(|| panic!("parse {name}"));
            std::fs::write(dir.join(format!("{name}.root.ui")), &n.src).unwrap();
            for k in &n.kids {
                std::fs::write(dir.join(format!("{name}.{}.ui", k.name)), &k.src).unwrap();
            }
        }
        for i in 0..ROWS {
            let src = row_source(i);
            let n = parse_node(&src).unwrap_or_else(|| panic!("parse row {i}"));
            std::fs::write(dir.join(format!("row{i}.root.ui")), &n.src).unwrap();
            for k in &n.kids {
                std::fs::write(dir.join(format!("row{i}.{}.ui", k.name)), &k.src).unwrap();
            }
        }
    }
}
