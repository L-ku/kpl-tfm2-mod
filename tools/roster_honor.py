# -*- coding: utf-8 -*-
"""生涯荣誉量化。

官网接口能查到的（全部自动算，不靠手填）：
  getPlayerRank     -> 每季 9 个榜单前 5（MVP 次数、总击杀、总助攻、五杀、
                       对位单杀、野怪击杀、中路游走、射手团战伤害、辅助开团）
  getScheduleList   -> 每季总决赛结果 -> 冠军队伍
  getSeasonPlayerPerf -> 冠军队伍当季出场最多的五人组（分路各取一人）= 冠军首发

官网查不到、只能按官方公告手工维护的：
  FMVP      每季总决赛 MVP
  SEASON_MVP 每季常规赛/赛季 MVP
（getRetireCert 只覆盖已退役选手，现役一律「retire cert not found」，
 而且 honors 基本都是空数组 —— 所以现役的个人 MVP 只能手工记录。）

量化口径：
  赛事等级 年度总决赛 1.3 / 春季赛·夏季赛 1.0 / 世界冠军杯 1.2 / 挑战者杯 0.9
  年度总决赛不评选赛季 MVP（只有 FMVP）
  榜单权重 MVP 3.0、总击杀 1.6、五杀 1.5、总助攻 1.0、对位单杀 1.0、
           射手团战伤害 1.0、辅助开团 1.0、野怪击杀 0.4、中路游走 0.4
  名次分 1.0 / 0.75 / 0.55 / 0.4 / 0.3
  冠军首发 每次 2.0 × 赛事等级 × 该季时间权重
  FMVP     每次 6.0 × 赛事等级
  赛季MVP  每次 4.0 × 赛事等级 × 该季时间权重
  榜单分、FMVP、赛季MVP、所有冠军统一乘时间权重（近重远轻，每往前一季约 ×0.85）
"""
import json, io, os, re, collections

BASE = os.environ.get('KPL_DATA', os.path.dirname(os.path.abspath(__file__)))

SEASONS = ['KPL2023S1', 'KPL2023S2', 'KPL2024S1', 'KPL2024S2', 'KPL2024S3',
           'KPL2025S1', 'KPL2025S2', 'KPL2025S3', 'KPL2026S1', 'KPL2026S2']

# 赛事等级：年度总决赛（S3）1.3，其余 1.0
EVENT_TIER = {s: (1.3 if s.endswith('S3') else 1.0) for s in SEASONS}

# 时间权重，和 roster_score.py 保持一致
SEASON_W = {'KPL2026S2': 1.00, 'KPL2026S1': 0.90, 'KPL2025S3': 0.78,
            'KPL2025S2': 0.70, 'KPL2025S1': 0.62, 'KPL2024S3': 0.55,
            'KPL2024S2': 0.50, 'KPL2024S1': 0.45, 'KPL2023S2': 0.38,
            'KPL2023S1': 0.32}

BOARD_W = {
    'mvp_list': 3.0, 'total_kills_list': 1.6, 'total_assists_list': 1.0,
    'five_kill_list': 1.5, 'top_solo_kills_list': 1.0, 'jug_count_list': 0.4,
    'mid_lane_roam_count_list': 0.4, 'adc_teamfight_damage_list': 1.0,
    'sup_initiation_count_list': 1.0,
}
RANK_PTS = [1.0, 0.75, 0.55, 0.4, 0.3]
CHAMPION_PTS = 2.0
FINALS_MVP_PTS = 6.0
SEASON_MVP_PTS = 4.0
LANE_OF_POS = {1: 'top', 2: 'mid', 3: 'bot', 4: 'jg', 5: 'sup'}

# 非季赛冠军（挑战者杯 / 世界冠军杯 / 冬冠）：KPL 的赛季接口不覆盖这些赛事，
# 但它们是官方认可的头衔（一诺的 7 冠里就有 2023 世界冠军杯和 2024 挑战者杯）。
# 只有「夺冠年份该选手在该队且有出场」才计入，见 build() 的 extra_titles。
EXTRA_TITLES = {
    # 赛事名: (年份, 冠军队伍, 赛事等级, {FMVP: 选手})
    '2023世界冠军杯KIC': ('2023', '成都AG超玩会', 1.2, {'FMVP': '一诺'}),
    '2024挑战者杯': ('2024', '成都AG超玩会', 0.9, {}),
}

# 总决赛 FMVP（官方公告，接口不提供，按知情人口述录入）
FINALS_MVP = {
    'KPL2023S1': 'Fly',       # 2023 春，重庆狼队
    'KPL2023S2': '不然',      # 2023 夏，广州TTG（已不在现名单）
    'KPL2024S1': '小胖',      # 2024 春，重庆狼队
    'KPL2024S2': '钟意',      # 2024 夏，成都AG超玩会
    'KPL2024S3': '长生',      # 2024 年度总决赛，成都AG超玩会
    'KPL2025S1': '钟意',      # 2025 春，成都AG超玩会
    'KPL2025S2': '大帅',      # 2025 夏，成都AG超玩会
    'KPL2025S3': '一诺',      # 2025 年度总决赛，AG 4:2 狼队
    'KPL2026S1': '流浪',      # 2026 春，KSG
    'KPL2026S2': '小胖',      # 2026 夏，广州TTG
}

# 赛季 MVP（官方公告，接口不提供）。年度总决赛（S3）不评选赛季 MVP —— 
# 2024S3 / 2025S3 一律留空，不要因为一诺是那两届 FMVP 就顺手填上。
SEASON_MVP = {
    'KPL2023S1': '暖阳',
    'KPL2023S2': '一笙',
    'KPL2024S1': '轩染',
    'KPL2024S2': '花云',      # 已不在现名单
    'KPL2025S1': '一诺',
    'KPL2025S2': '小胖',
    'KPL2026S1': '无畏',
    'KPL2026S2': '小胖',
}


def _load(path):
    p = os.path.join(BASE, path)
    if not os.path.exists(p):
        return None
    return json.load(io.open(p, encoding='utf-8'))


def _load_id2name():
    m = {}
    for s in ('KPL2026S1', 'KPL2026S2', 'KPL2026S3'):
        d = _load('perf_%s.json' % s) or {}
        for r in d.get('data') or []:
            if r.get('player_id') and r.get('player_short_name'):
                m[r['player_id']] = r['player_short_name']
    return m


ID2NAME = _load_id2name()


def clean_name(s):
    """'成都AG超玩会.一诺(徐必成)' -> '一诺'；'长沙TES.A书源' -> '书源'。"""
    if not s:
        return None
    s = s.strip()
    if '.' in s:
        s = s.split('.', 1)[1]
    s = re.sub(r'^[A-Za-z]{1,4}(?=[^\x00-\x7f])', '', s)
    s = re.split(r'[（(]', s)[0]
    return s.strip() or None


def final_match(season):
    """该赛季总决赛 = 时间最晚的那场（不是阶段内最后一场）。"""
    d = _load('sched_%s.json' % season)
    if not d:
        return None
    lst = (d.get('data') or {}).get('list') or []
    cand = [x for x in lst
            if x.get('stageid') in ('jhs', 'zjs')
            or '决赛' in (x.get('stage_name') or '')]
    if not cand:
        return None
    return max(cand, key=lambda x: int(x.get('start_timestamp') or 0))


def season_champion(season):
    f = final_match(season)
    if not f:
        return None
    try:
        a, b = int(f.get('team_a_score')), int(f.get('team_b_score'))
    except Exception:
        return None
    return f.get('team_a_name') if a > b else f.get('team_b_name')


def build():
    """返回 (honor_raw, detail, champs)。"""
    honor = collections.defaultdict(float)
    detail = collections.defaultdict(list)
    champs = {}

    for season in SEASONS:
        tier = EVENT_TIER.get(season, 1.0)

        # 1) 官方榜单
        d = _load('mvp_%s.json' % season) or {}
        data = d.get('data') or {}
        for board, w in BOARD_W.items():
            for pos, item in enumerate(data.get(board) or []):
                nm = clean_name(item.get('player_name'))
                if not nm or pos >= len(RANK_PTS):
                    continue
                honor[nm] += RANK_PTS[pos] * w * tier * SEASON_W.get(season, 0.5)
                detail[nm].append('%s %s#%d' % (season, board.replace('_list', ''), pos + 1))

        # 2) FMVP / 赛季 MVP
        nm = FINALS_MVP.get(season)
        if nm:
            honor[nm] += FINALS_MVP_PTS * tier * SEASON_W.get(season, 0.5)
            detail[nm].append('%s FMVP' % season)
        nm = SEASON_MVP.get(season)
        if nm:
            honor[nm] += SEASON_MVP_PTS * tier * SEASON_W.get(season, 0.5)
            detail[nm].append('%s 赛季MVP' % season)

        # 3) 冠军首发五人
        champ = season_champion(season)
        champs[season] = champ
        if not champ:
            continue
        d = _load('sched_%s.json' % season) or {}
        lst = (d.get('data') or {}).get('list') or []
        team_ids = set()
        for x in lst:
            if x.get('team_a_name') == champ:
                team_ids.add(x.get('team_a_id'))
            if x.get('team_b_name') == champ:
                team_ids.add(x.get('team_b_id'))
        perf = _load('perf_%s.json' % season) or {}
        rows = []
        for r in perf.get('data') or []:
            if r.get('team_id') not in team_ids:
                continue
            nm = r.get('player_short_name') or ID2NAME.get(r.get('player_id'))
            if nm:
                rows.append(dict(r, player_short_name=nm))
        if not rows:
            continue
        # 出场最多的五人组：官方标了分路就每个分路取出场最多的；
        # 老赛季（2023-2025）position 全是 0，退回「按出场局数取前五」。
        by_lane = collections.defaultdict(list)
        for r in rows:
            lane = LANE_OF_POS.get(r.get('position'))
            if lane:
                by_lane[lane].append(r)
        if len(by_lane) >= 4:
            starters = [max(rs, key=lambda r: float(r.get('Matchcount') or 0))
                        for rs in by_lane.values()]
        else:
            starters = sorted(rows, key=lambda r: -float(r.get('Matchcount') or 0))[:5]
        for r in starters:
            nm = r['player_short_name']
            honor[nm] += CHAMPION_PTS * tier * SEASON_W.get(season, 0.5)
            detail[nm].append('%s 冠军首发' % season)

    # 4) 非季赛冠军（世界冠军杯 / 挑战者杯）：赛季接口不覆盖，
    #    但队伍荣誉里有记录。判定保守 —— 夺冠当年该选手在该队出场 >= 20 局才算，
    #    避免把「那年刚转会来」的人也算成冠军成员。
    for title, (year, team, tier, awards) in EXTRA_TITLES.items():
        played = collections.Counter()
        for season in SEASONS:
            if not season.startswith('KPL' + year):
                continue
            perf = _load('perf_%s.json' % season) or {}
            for r in perf.get('data') or []:
                tname = r.get('team_name') or ''
                if tname not in (team, team.replace('成都AG超玩会', 'AG')):
                    if not tname.startswith(team[:2]):
                        continue
                nm = r.get('player_short_name') or ID2NAME.get(r.get('player_id'))
                if nm:
                    played[nm] += float(r.get('Matchcount') or 0)
        year_w = max(SEASON_W.get(s, 0.5) for s in SEASONS if s.startswith('KPL' + year))
        for nm, g in played.items():
            if g >= 20:
                honor[nm] += CHAMPION_PTS * tier * year_w
                detail[nm].append('%s 冠军' % title)
        mvp = awards.get('FMVP')
        if mvp:
            honor[mvp] += FINALS_MVP_PTS * tier * year_w
            detail[mvp].append('%s FMVP' % title)

    return honor, detail, champs


if __name__ == '__main__':
    honor, detail, champs = build()
    o = io.open(os.path.join(BASE, 'honor_report.txt'), 'w', encoding='utf-8')
    o.write('champions:\n')
    for s, c in sorted(champs.items()):
        o.write('  %-12s %s\n' % (s, c))
    o.write('\nranking:\n')
    for nm, v in sorted(honor.items(), key=lambda kv: -kv[1]):
        o.write('%-8s %6.2f  %s\n' % (nm, v, ', '.join(detail[nm])))
    o.close()
    print('honor players', len(honor))
