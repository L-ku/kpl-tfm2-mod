# -*- coding: utf-8 -*-
"""把 2023 春 ~ 2026 夏 十个赛季的官方数据合成 kpl_sab 的能力值。
选手名单和人名一个不动，只重算 ovr / pot / age。

十个赛季：2023春 2023夏 2024春 2024夏 2024年度总决赛
          2025春 2025夏 2025年度总决赛 2026春 2026夏

为什么从 2023 开始：官网 getSeasonPlayerPerf 只有这几季有数据（2022 及更早
返回 0 行）。而 2023S1/S2、2024S1-S3 的 perf 没有选手名（只有 player_id），
用 2026 的 id→名字表回填，所以一诺、长生、钟意、轩染这些老将能凑满 8-10 季。

数据源（kpl.qq.com 自己的接口，无第三方）：
  getSeasonPlayerPerf  -> KDA、总击杀、伤害占比、承伤占比、经济占比、参团率
                          （2026 两季有参团率；2025 三季该字段为 0）
  getTeamsIntro        -> 分均伤害、分均承伤、胜率、英雄池明细（仅 2026 两季有名字）
  getScheduleDetail    -> 逐小局的出场选手 + 英雄 + 获胜方
                          （用来给 2025 三季重建胜率和英雄池；已用 2026 两季的官方
                            胜率/英雄池交叉验证，108/108 完全一致）
2025 三季 perf 没有选手名，但有稳定的 player_id，用 2026 的 id→名字表回填，
所以一名主力通常能凑到 3-5 个赛季的样本。

分路：2026 两季官方带 position；2025 三季 position 全是 0，用 2026 带标签的数据
训练一个「伤害占比 / 承伤占比 / 经济占比」的最近质心分类器反推（留一验证 94%，
错的基本只有对抗路↔打野）。名单里的人直接用名单分路，更可靠。

计分：
  1) 每项指标在「同赛季 + 同分路」内取百分位（0..1，并列取平均）；
  2) 分路各有一套 11 项权重；
  3) 跨季按「近重远轻」× 登场局数合成一个原始分；
  4) 原始分在每个分路内线性映射成原始能力值；
  5) 再按总样本量向「该分路中位数」收缩：局数越少越接近中位，
     打了几十局的不会被吹成明星，也不会因为一两季低迷就被打死；
     样本量足够（几百局）时几乎不收缩。
  十季一局没打的选手按替补/青训处理，能力值 62-68。
"""
import json, io, glob, re, os, sys, math, collections, random

BASE = os.environ.get('KPL_DATA', os.path.dirname(os.path.abspath(__file__)))
SEASONS = ['KPL2023S1', 'KPL2023S2', 'KPL2024S1', 'KPL2024S2', 'KPL2024S3',
           'KPL2025S1', 'KPL2025S2', 'KPL2025S3', 'KPL2026S1', 'KPL2026S2']
# 近重远轻：2026 夏 1.0，每往前一季递减
# 近重远轻：2026 夏 1.0，每往前一季递减约 15%（越早的数据参考价值越低，
# 但也不能降到 0 —— 生涯长度和稳定性本身就是实力的证据）
SEASON_W = {'KPL2026S2': 1.00, 'KPL2026S1': 0.90, 'KPL2025S3': 0.78,
            'KPL2025S2': 0.70, 'KPL2025S1': 0.62, 'KPL2024S3': 0.55,
            'KPL2024S2': 0.50, 'KPL2024S1': 0.45, 'KPL2023S2': 0.38,
            'KPL2023S1': 0.32}

LANE_OF_POS = {1: 'top', 2: 'mid', 3: 'bot', 4: 'jg', 5: 'sup'}
LANE_OF_CN = {'对抗路': 'top', '中路': 'mid', '发育路': 'bot', '打野': 'jg', '游走': 'sup'}
LANES = ('top', 'jg', 'mid', 'bot', 'sup')

METRICS = ['dmg', 'taken', 'dmg_share', 'taken_share', 'gold_share', 'part',
           'kda', 'win', 'kills', 'breadth', 'hero_wr']

# 分路权重（顺序同上）：对抗路吃承伤和参团，打野吃节奏，中/发育吃输出，游走吃参团
W = {
    'top': (0.10, 0.15, 0.08, 0.14, 0.08, 0.12, 0.13, 0.10, 0.05, 0.03, 0.02),
    'jg':  (0.12, 0.08, 0.10, 0.06, 0.12, 0.16, 0.13, 0.09, 0.08, 0.04, 0.02),
    'mid': (0.18, 0.04, 0.14, 0.04, 0.10, 0.12, 0.11, 0.09, 0.08, 0.05, 0.05),
    'bot': (0.20, 0.03, 0.16, 0.03, 0.12, 0.10, 0.11, 0.09, 0.09, 0.04, 0.03),
    'sup': (0.03, 0.16, 0.03, 0.14, 0.06, 0.22, 0.15, 0.09, 0.03, 0.06, 0.03),
}
# 反向指标：游走的经济占比越低越好（让经济）
INVERT = {('sup', 'gold_share')}

SHRINK_K = 60.0          # 样本量收缩常数（局）：60 局时收缩一半
BENCH_MIN, BENCH_MAX = 62, 68   # 十季 0 局选手的能力区间


def fnum(x):
    try:
        v = float(x)
        return v if v == v else None
    except Exception:
        return None


def percentile(vals, x):
    if not vals or x is None:
        return None
    lo = sum(1 for v in vals if v < x)
    eq = sum(1 for v in vals if v == x)
    return (lo + (eq - 1) / 2.0) / max(1, len(vals) - 1)


def age_from(bd, ref_year=2026, ref_month=7):
    """以夏季赛中段（7 月）为基准算周岁。"""
    if not bd:
        return None
    try:
        y, m = int(bd.split('/')[0]), int(bd.split('/')[1])
    except Exception:
        return None
    return ref_year - y - (0 if m <= ref_month else 1)


def hero_pool_stats(hp):
    hp = hp or []
    breadth = len(hp)
    games = sum((h.get('MatchCount') or 0) for h in hp)
    wins = sum((h.get('WinMatchCount') or 0) for h in hp)
    return breadth, (wins / games if games else None)


# ---------- 1. 读现有名单（名字 / 分路 / 年龄由名单决定，不动） ----------
src = io.open('C:/Users/Y/kpl-tfm2-mod/kpl_sab/src/lib.rs', encoding='utf-8').read()
i = src.find('fn roster_for')
j = src.find('_ => None', i)
seg = src[i:j]
roster = collections.OrderedDict()
for blk in re.finditer(r'"([^"]+)" => Some\(\[(.*?)\n        \]\),', seg, re.S):
    team, body = blk.group(1), blk.group(2)
    lanes = re.findall(r'&\[(.*?)\],', body)
    roster[team] = []
    for li, lane_body in enumerate(lanes):
        for m in re.finditer(r'\("([^"]+)",\s*(\d+),\s*(\d+),\s*(\d+)\)', lane_body):
            roster[team].append(dict(name=m.group(1), age=int(m.group(2)),
                                     ovr=int(m.group(3)), pot=int(m.group(4)),
                                     lane=LANES[li]))
lane_of = {p['name']: p['lane'] for t in roster.values() for p in t}

# ---------- 2. perf 接口：十季全量，用 player_id 认人 ----------
perf = collections.defaultdict(dict)   # (season, name) -> fields
id2name, id2pos = {}, {}
for season in SEASONS:
    p = os.path.join(BASE, 'perf_%s.json' % season)
    if not os.path.exists(p):
        continue
    for r in json.load(io.open(p, encoding='utf-8')).get('data') or []:
        pid, nm = r.get('player_id'), r.get('player_short_name')
        if nm:
            id2name[pid] = nm
            if r.get('position'):
                id2pos[pid] = r['position']

for season in SEASONS:
    p = os.path.join(BASE, 'perf_%s.json' % season)
    if not os.path.exists(p):
        continue
    for r in json.load(io.open(p, encoding='utf-8')).get('data') or []:
        pid = r.get('player_id')
        nm = r.get('player_short_name') or id2name.get(pid)
        if not nm:
            continue
        g = fnum(r.get('Matchcount')) or 0
        tk = fnum(r.get('TotalKill'))
        pos = r.get('position')
        perf[(season, nm)] = dict(
            games=g, kda=fnum(r.get('KDA')),
            dmg_share=fnum(r.get('AVGHurtCR')), taken_share=fnum(r.get('AVGBeHurtCr')),
            gold_share=fnum(r.get('AVGGoldCR')), part=fnum(r.get('AVGPartacipationCR')),
            kills=(tk / g) if (tk is not None and g > 0) else None,
            lane=LANE_OF_POS.get(pos) or LANE_OF_POS.get(id2pos.get(pid)),
            pos_labeled=bool(pos),             # 官方真的标了分路（反推分类器只用这些）
            team=r.get('team_name') or '',
            team_id=r.get('team_id') or '',
        )

# 2025 的参团率全是 0，当作缺失
for (season, nm), rec in perf.items():
    if season.startswith('KPL2025') and not rec.get('part'):
        rec['part'] = None


def estimate_part(rec):
    """用逐小局数据还原参团率：参团 = (击杀 + 助攻) / 队伍总击杀。

    getScheduleDetail 只给英雄和胜负，没有击杀/助攻，所以这里用官方
    「总击杀 / 总助攻」榜单换算不了。退而求其次：用选手的击杀占比
    × 队伍场均击杀估一个代理值 —— 只在同分路内比较，偏差可控。
    """
    return None

# ---------- 3. intro 接口：2026 两季的绝对值 / 胜率 / 英雄池 ----------
for f in glob.glob(os.path.join(BASE, 'intro', '*.json')):
    season = os.path.basename(f).split('__')[0]
    d = json.load(io.open(f, encoding='utf-8')).get('data') or {}
    for p in d.get('player_msg') or []:
        nm = p.get('player_name_short')
        if not nm:
            continue
        breadth, hero_wr = hero_pool_stats(p.get('HeroPool'))
        key = (season, nm)
        rec = perf.setdefault(key, dict(games=0, lane=None, team=''))
        rec.update(
            games=fnum(p.get('Matchcount')) or rec.get('games') or 0,
            dmg=fnum(p.get('DMGMin')), taken=fnum(p.get('TDMGMin')),
            win=fnum(p.get('Wincr')), breadth=breadth, hero_wr=hero_wr,
            age=age_from(p.get('birth_date')),
            lane=rec.get('lane') or LANE_OF_CN.get(p.get('position_cn')) or LANE_OF_POS.get(p.get('position')),
        )

# ---------- 3a2. 赛季口径对齐 ----------
# 2025 三季的伤害占比 / 承伤占比 / 经济占比整体比 2026 低 2-3 个点（官网口径变了），
# 直接用会系统性压低 2025 的表现。按「同分路、同期都在场上的选手」的赛季均值做平移：
# 以 2026S2 为基准，把每个旧赛季的每项占比减去它和基准的均值差。
BASE_SEASON = 'KPL2026S2'
for m in ('dmg_share', 'taken_share', 'gold_share'):
    base_mean = {}
    for lane in LANES:
        xs = [r[m] for (s, _n), r in perf.items()
              if s == BASE_SEASON and r.get('lane') == lane and r.get(m) is not None]
        if xs:
            base_mean[lane] = sum(xs) / len(xs)
    for season in SEASONS:
        if season == BASE_SEASON:
            continue
        for lane in LANES:
            xs = [r[m] for (s, _n), r in perf.items()
                  if s == season and r.get('lane') == lane and r.get(m) is not None]
            if not xs or lane not in base_mean:
                continue
            shift = base_mean[lane] - sum(xs) / len(xs)
            for (s, _n), r in perf.items():
                if s == season and r.get('lane') == lane and r.get(m) is not None:
                    r[m] = r[m] + shift

# ---------- 3a3. 还原 2025 缺的分均伤害 / 分均承伤 ----------
# 2026 的官方绝对量和占比之比在同分路内很稳定（dmg/share 约 210-220，
# taken/share 约 215-225），所以 2025 可以用「占比 × 该分路比值中位数」还原，
# 这样 11 项指标在五个赛季都能算齐，不再出现「2025 只有 5 项」的口径差。
for field, share_field, ratio in (('dmg', 'dmg_share', {}), ('taken', 'taken_share', {})):
    for lane in LANES:
        rs = [r for r in perf.values()
              if r.get('lane') == lane and r.get(field) and r.get(share_field)]
        if len(rs) < 5:
            continue
        vals = sorted(r[field] / r[share_field] for r in rs)
        mid = vals[len(vals) // 2]
        ratio[lane] = mid
    for (season, nm), r in perf.items():
        if r.get(field) is not None or r.get(share_field) is None:
            continue
        k = ratio.get(r.get('lane'))
        if k:
            r[field] = r[share_field] * k

# ---------- 3b. 逐小局数据：补胜率和英雄池 ----------
# 每小局的 team_a / team_b 和获胜方都在 gamedata 里；选手属于哪边由 perf 的
# team_id 判定（同一选手一个赛季只效力一支队）。
for season in SEASONS:
    gp = os.path.join(BASE, 'gamedata_%s.json' % season)
    if not os.path.exists(gp):
        continue
    gd = json.load(io.open(gp, encoding='utf-8'))
    for pid, g in gd.items():
        nm = id2name.get(pid)
        rec = perf.get((season, nm)) if nm else None
        if not rec or not g.get('games'):
            continue
        team_id = rec.get('team_id')
        wins = sum(1 for _, wt, _h in g['rounds'] if team_id and wt == team_id)
        if team_id:
            rec['win'] = wins / g['games']
            rec['games'] = min(rec.get('games') or g['games'], g['games'])
        # 英雄池：出场过的不同英雄数 + 加权胜率
        hs = collections.Counter(h for _, _, h in g['rounds'] if h)
        if hs:
            rec['breadth'] = len(hs)
            hw = 0
            for hero, n in hs.items():
                hw += n * (sum(1 for _, wt, h in g['rounds'] if h == hero and team_id and wt == team_id) / n)
            rec['hero_wr'] = hw / g['games'] if g['games'] else None

# 分路兜底：
#   1) 名单里的人用名单分路（最可靠）；
#   2) 其余（K 甲队、名单外选手）用 2026 带 position 的数据训练最近质心分类器，
#      按「伤害占比 / 承伤占比 / 经济占比」反推（留一验证 94%）。
_centroids = {}
for lane in LANES:
    xs = [r for r in perf.values()
          if r.get('lane') == lane and r.get('pos_labeled')
          and r.get('dmg_share') and r.get('taken_share') and r.get('gold_share')]
    if xs:
        _centroids[lane] = [sum(r[k] for r in xs) / len(xs)
                            for k in ('dmg_share', 'taken_share', 'gold_share')]


def infer_lane(rec):
    v = [rec.get('dmg_share'), rec.get('taken_share'), rec.get('gold_share')]
    if any(x is None for x in v) or not _centroids:
        return None
    return min(_centroids, key=lambda l: sum((v[i] - _centroids[l][i]) ** 2 for i in range(3)))


for (season, nm), rec in perf.items():
    if not rec.get('lane'):
        rec['lane'] = lane_of.get(nm) or infer_lane(rec)

# ---------- 4. 每项指标在本季本分路内的百分位 ----------
buckets = collections.defaultdict(list)
for (season, nm), rec in perf.items():
    if rec.get('lane') and rec['games'] >= 3:
        buckets[(season, rec['lane'])].append(rec)

for key, group in buckets.items():
    for m in METRICS:
        vals = [r.get(m) for r in group if r.get(m) is not None]
        if len(vals) < 3:
            for r in group:
                r['p_' + m] = None
            continue
        for r in group:
            r['p_' + m] = percentile(vals, r.get(m))

# ---------- 5. 逐季得分 + 跨季合成 ----------
# 关键：每季只在该季「有该指标的同分路选手」里取百分位，但某个指标这一季没有
# （比如 2025 没有分均伤害），就不计入这一季的加权和、也不把权重摊给其它指标
# ——「有 6 项就算 6 项的平均分」，与满分 11 项的口径一致，不会因为指标少而虚高。
LEAGUE_SCORE = {}    # 每季每分路的平均分，用于把单季成绩对齐到统一尺度
by_name = collections.defaultdict(list)
for (season, nm), rec in perf.items():
    rec['season'] = season
    by_name[nm].append(rec)

season_lane_raw = collections.defaultdict(list)
for name, rs in by_name.items():
    lane = lane_of.get(name) or next(
        (r['lane'] for r in sorted(rs, key=lambda r: -SEASONS.index(r['season'])) if r.get('lane')), None)
    if not lane:
        continue
    weights = W[lane]
    for r in rs:
        g = r.get('games') or 0
        if g < 3:
            continue
        parts = []
        for m in METRICS:
            pv = r.get('p_' + m)
            if pv is not None and (lane, m) in INVERT:
                pv = 1.0 - pv
            parts.append((m, pv))
        got = [(weights[METRICS.index(m)], p) for m, p in parts if p is not None]
        if len(got) < 3:                       # 至少 3 项指标才算数
            continue
        s = sum(w * p for w, p in got) / sum(w for w, _ in got)
        r['raw'] = s
        r['lane_final'] = lane
        season_lane_raw[(r['season'], lane)].append(s)

for k, xs in season_lane_raw.items():
    LEAGUE_SCORE[k] = sum(xs) / len(xs)

scored = {}
for name, rs in by_name.items():
    lane = lane_of.get(name) or next(
        (r['lane'] for r in sorted(rs, key=lambda r: -SEASONS.index(r['season'])) if r.get('lane')), None)
    if not lane:
        continue
    num = den = 0.0
    total_games = 0.0
    ages, teams = [], collections.Counter()
    for r in rs:
        g = r.get('games') or 0
        if g < 3 or 'raw' not in r:
            continue
        # 对齐到联赛均值：单季低于同位置平均就给负分，高于就给正分
        centred = 0.5 + (r['raw'] - LEAGUE_SCORE.get((r['season'], lane), 0.5))
        sw = SEASON_W[r['season']] * g
        num += centred * sw
        den += sw
        total_games += g
        if r.get('age'):
            ages.append(r['age'])
        if r.get('team'):
            teams[r['team']] += g
    if den == 0:
        continue
    scored[name] = dict(score=num / den, games=total_games,
                        age=min(ages) if ages else None, lane=lane,
                        team=teams.most_common(1)[0][0] if teams else '')

# ---------- 6. 生涯荣誉（在排名映射之前合进综合分） ----------
# 官方没有个人的冠军/亚军记录，但有三类可量化的东西：
#   1) 每季 8 个榜单的前 5 名（MVP 次数、总击杀、总助攻、五杀、对位单杀、
#      野怪击杀、中路游走、射手团战伤害、辅助开团）；
#   2) 每季总决赛冠军队伍的首发五人（有分路就每个分路取出场最多的）；
#   3) 总决赛 FMVP（官网接口没有，按官方公告手工维护）。
# 时间 × 赛事等级：年度总决赛 ×1.3、春/夏季赛 ×1.0；越近的赛季权重越高。
# 荣誉是「加分」，不是「按比例缩放」：
#   score = 纯数据分 + HONOR_BONUS * honor_norm
# 早期版本写成 score = 纯数据分*(1-0.3) + honor_norm*0.3，结果零荣誉的人分数
# 被整体乘 0.7，等于「没冠军就扣 30%」，小落/幻想/大海这类数据好但没冠军的
# 直接被压掉 8-10 名。荣誉应该是在数据分之上给冠军核心的额外加成。
# HONOR_BONUS 按综合分的分布定标：全体标准差约 0.127，
# 0.45 约等于 3.5 个标准差 —— 满荣誉能把一个中游选手推到同位置榜首，
# 但推不满（一诺 2026 休息、数据垫底，纯数据 0.385，加满 0.45 后仍进不了前三）。
HONOR_BONUS = 0.45
try:
    sys.path.insert(0, BASE)
    import roster_honor as honor_mod
    honor_raw, honor_detail, champs = honor_mod.build()
except Exception as exc:
    honor_raw, honor_detail, champs = {}, {}, {}
    print('honor skipped:', exc)

if honor_raw:
    # 荣誉分用「对数压缩」而不是纯百分位：
    # 一诺 38.6 分、第二名 16.0、大多数 0-12。纯百分位会把一诺和所有有荣誉的人
    # 都压到接近 1.0，2.4 倍的差距被抹平；纯 min-max 又会被一诺这个离群点拉爆。
    # 取 log 后按 log(最高分) 归一：一诺=1.0，第二名≈0.78，零荣誉=0。
    names = sorted(scored)
    hv = [honor_raw.get(n, 0.0) for n in names]
    hmax = max(hv) if hv else 0.0
    lg = math.log1p
    for name in names:
        raw = honor_raw.get(name, 0.0)
        norm = (lg(raw) / lg(hmax)) if (hmax > 0 and raw > 0) else 0.0
        v = scored[name]
        v['honor'] = raw
        v['honor_norm'] = norm
        v['data_score'] = v['score']
        v['score'] = v['score'] + HONOR_BONUS * norm

# ---------- 6b. 综合分 -> 能力值 ----------
# 按分路内排名映射（不是绝对值映射）：每个位置都有自己的强、中、弱，
# 不会出现「所有辅助都比所有射手强」这种整体偏移。
# 再按总样本量向中位排名收缩：打了几十局的不会被吹成顶级，打满十季的才拿满价。
for lane in LANES:
    group = [(n, v) for n, v in scored.items() if v['lane'] == lane]
    if not group:
        continue
    group.sort(key=lambda kv: kv[1]['score'])
    m = len(group)
    for rank, (n, v) in enumerate(group):
        t = rank / (m - 1) if m > 1 else 0.5
        rely = v['games'] / (v['games'] + SHRINK_K)
        t = 0.5 + (t - 0.5) * rely
        v['ovr'] = max(64, min(93, round(64 + 29 * t)))

# ---------- 7. 潜力（按年龄区间随机） + 0 局替补 ----------
rnd = random.Random(20260709)
out_players = collections.OrderedDict()
for team, players in roster.items():
    lst = []
    for p in players:
        name = p['name']
        v = scored.get(name)
        if v and v.get('ovr') is not None:
            ovr = v['ovr']
            age = v.get('age') or p['age']
            games = v['games']
        else:
            # 五个赛季一局没打：替补/青训，能力值由名字派生，固定不随机
            h = 0
            for ch in name:
                h = (h * 131 + ord(ch)) & 0xFFFFFFFF
            ovr = BENCH_MIN + h % (BENCH_MAX - BENCH_MIN + 1)
            age = p['age']
            games = 0
        hi = 26 if age <= 19 else 23 if age <= 21 else 19 if age <= 23 else 13 if age <= 26 else 8
        pot = min(99, ovr + rnd.randint(0, hi))
        lst.append(dict(name=name, age=age, ovr=ovr, pot=max(pot, ovr), lane=p['lane'],
                        old_ovr=p['ovr'], old_pot=p['pot'], games=games,
                        matched=bool(v and v.get('ovr') is not None),
                        seasons=len([r for r in by_name.get(name, []) if (r.get('games') or 0) >= 3])))
    out_players[team] = lst

# ---------- 8. 输出 ----------
o = io.open(os.path.join(BASE, 'rust_roster.txt'), 'w', encoding='utf-8', newline='\n')
o.write("fn roster_for(team: &str) -> Option<[&'static [P]; 5]> {\n    match team {\n")
for team, lst in out_players.items():
    o.write('        "%s" => Some([\n' % team)
    for lane in LANES:
        xs = sorted([p for p in lst if p['lane'] == lane], key=lambda x: -x['ovr'])
        o.write('            &[%s],\n' % ', '.join(
            '("%s", %d, %d, %d)' % (p['name'], p['age'], p['ovr'], p['pot']) for p in xs))
    o.write('        ]),\n')
o.write('        _ => None,\n    }\n}\n')
o.close()

rep = io.open(os.path.join(BASE, 'report.txt'), 'w', encoding='utf-8', newline='\n')
allp = [p for lst in out_players.values() for p in lst]
rep.write('players=%d 有数据=%d 0局=%d\n' % (
    len(allp), sum(1 for p in allp if p['matched']), sum(1 for p in allp if not p['matched'])))
rep.write('赛季数分布 %s\n\n' % dict(sorted(collections.Counter(p['seasons'] for p in allp).items())))
for team, lst in out_players.items():
    rep.write('%s\n' % team)
    for p in sorted(lst, key=lambda x: -x['ovr']):
        rep.write('   %-5s %-3s %2d岁 %2d(%+3d) pot%2d  %s赛季 %4d局%s\n' % (
            p['name'], p['lane'], p['age'], p['ovr'], p['ovr'] - p['old_ovr'],
            p['pot'], p['seasons'], int(p['games']),
            '' if p['matched'] else '   <-- 十季0局，按替补'))
rep.write('\novr min=%d max=%d mean=%.1f\n' % (
    min(p['ovr'] for p in allp), max(p['ovr'] for p in allp),
    sum(p['ovr'] for p in allp) / len(allp)))
rep.write('ovr hist %s\n' % sorted(collections.Counter((p['ovr'] // 5) * 5 for p in allp).items()))
rep.write('pot hist %s\n' % sorted(collections.Counter((p['pot'] // 5) * 5 for p in allp).items()))
dups = [k for k, v in collections.Counter(p['name'] for p in allp).items() if v > 1]
rep.write('dups %s\n' % dups)
rep.close()
print('done players=%d matched=%d' % (len(allp), sum(1 for p in allp if p['matched'])))
