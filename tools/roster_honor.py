# -*- coding: utf-8 -*-
"""把官方的个人榜单和赛季冠军折算成「个人荣誉分」。

数据来源（都在 kpl.qq.com 的接口里）：
  getPlayerRank 的 8 个榜单：MVP 次数、总击杀、总助攻、五杀、对位单杀、
                              野怪击杀、中路游走、射手团战伤害、辅助开团
  getScheduleList：每个赛季的总决赛结果 → 哪支队夺冠

量化规则（按赛事等级 × 榜单权重 × 赛季权重）：
  赛事等级系数：年度总决赛 1.3 / 春季赛·夏季赛 1.0 / 挑战者杯 0.7
                （2025S3、2026S3 是年度总决赛）
  榜单权重：MVP 3.0、总击杀 1.6、总助攻 1.0、五杀 1.5、
            对位单杀 1.0、野怪击杀 0.4、中路游走 0.4、
            射手团战伤害 1.0、辅助开团 1.0
  名次分：第 1 名 1.0、第 2 名 0.75、第 3 名 0.55、第 4 名 0.4、第 5 名 0.3
  冠军成员：该季夺冠队的首发/轮换选手，每季 +2.0（按 1.3/1.0 的赛事等级再乘）
  荣誉分最后做 min-max 归一，再以很小权重（HONOR_W）加进综合分。
"""
import json, io, os, re, collections

BASE = os.environ.get('KPL_DATA', os.path.dirname(os.path.abspath(__file__)))
SEASONS = ['KPL2025S1', 'KPL2025S2', 'KPL2025S3', 'KPL2026S1', 'KPL2026S2']

EVENT_TIER = {                     # 赛事等级
    'KPL2025S1': 1.0, 'KPL2025S2': 1.0, 'KPL2025S3': 1.3,
    'KPL2026S1': 1.0, 'KPL2026S2': 1.0,
}

# 总决赛 FMVP：官网接口不提供（getPlayerRank 只有常规榜单，没有 FMVP 字段），
# 这里按官方公告手工记录。键是赛季 id。
FINALS_MVP = {
    'KPL2025S3': '一诺',      # 2025KPL 年度总决赛 FMVP（成都AG超玩会 4:2 重庆狼队）
}
FINALS_MVP_PTS = 6.0           # 一次总决赛 FMVP ≈ 三个榜单独占鳌头
SEASON_W = {'KPL2026S2': 1.00, 'KPL2026S1': 0.85, 'KPL2025S3': 0.60,
            'KPL2025S2': 0.45, 'KPL2025S1': 0.30}
BOARD_W = {
    'mvp_list': 3.0, 'total_kills_list': 1.6, 'total_assists_list': 1.0,
    'five_kill_list': 1.5, 'top_solo_kills_list': 1.0, 'jug_count_list': 0.4,
    'mid_lane_roam_count_list': 0.4, 'adc_teamfight_damage_list': 1.0,
    'sup_initiation_count_list': 1.0,
}
RANK_PTS = [1.0, 0.75, 0.55, 0.4, 0.3]
CHAMPION_PTS = 2.0
LANE_OF_POS = {1: 'top', 2: 'mid', 3: 'bot', 4: 'jg', 5: 'sup'}


def _load_id2name():
    """2025 三季的 perf 没有选手名，只有稳定的 player_id；
    用 2026 两季的 id→名字表回填。"""
    m = {}
    for season in ('KPL2026S1', 'KPL2026S2'):
        p = os.path.join(BASE, 'perf_%s.json' % season)
        if not os.path.exists(p):
            continue
        for r in json.load(io.open(p, encoding='utf-8')).get('data') or []:
            if r.get('player_id') and r.get('player_short_name'):
                m[r['player_id']] = r['player_short_name']
    return m


ID2NAME = _load_id2name()


def clean_name(s):
    """'成都AG超玩会.一诺(徐必成)' -> '一诺'；'长沙TES.A书源' -> '书源'。

    分隔符有的用 '.'，有的直接用队名后缀，所以从右边找「队名前缀」剥掉：
    先按 '.' 切，再按已知队名（含 . 的）前缀剥，最后去括号。
    """
    if not s:
        return None
    s = s.strip()
    if '.' in s:
        head, tail = s.split('.', 1)
        # 'TES.A书源' 这种：'.' 后面还剩一个字 + 名字
        if tail and not re.match(r'^[A-Za-z]', tail) and len(tail) > 1:
            s = tail
        elif re.match(r'^[A-Za-z]{1,3}[^\x00-\x7f]', tail):
            s = tail
        else:
            s = tail
    s = re.sub(r'^[A-Za-z]{1,4}(?=[^\x00-\x7f])', '', s)   # 剥掉 TES.A / NBW. 残留
    s = re.split(r'[（(]', s)[0]
    return s.strip() or None


def final_match(season):
    """返回该赛季的总决赛（时间最晚的那场）。列表是按日期排的，不能用阶段内的最后一场。"""
    p = os.path.join(BASE, 'sched_%s.json' % season)
    if not os.path.exists(p):
        return None
    lst = (json.load(io.open(p, encoding='utf-8')).get('data') or {}).get('list') or []
    cands = [x for x in lst
             if x.get('stageid') in ('jhs', 'zjs')
             or '决赛' in (x.get('stage_name') or '')]
    if not cands:
        return None
    return max(cands, key=lambda x: int(x.get('start_timestamp') or 0))


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
    """返回 name -> dict(honor=加权荣誉分, detail=[...])"""
    honor = collections.defaultdict(float)
    detail = collections.defaultdict(list)
    champs = {}
    for season in SEASONS:
        p = os.path.join(BASE, 'mvp_%s.json' % season)
        if not os.path.exists(p):
            continue
        d = json.load(io.open(p, encoding='utf-8')).get('data') or {}
        tier = EVENT_TIER.get(season, 1.0)
        for board, w in BOARD_W.items():
            for pos, item in enumerate(d.get(board) or []):
                nm = clean_name(item.get('player_name'))
                if not nm or pos >= len(RANK_PTS):
                    continue
                v = RANK_PTS[pos] * w * tier
                honor[nm] += v
                detail[nm].append('%s#%d' % (board.replace('_list', ''), pos + 1))
        champs[season] = season_champion(season)

    # ---------- 历史冠军（队伍荣誉里，早于 2025 的那些） ----------
    # 官方 team_honor 只给到「哪支队在哪个赛事拿了冠军」，不含具体选手。
    # 所以只能给「该选手在那些冠军阵容里呆过」的人加分；这里用一条保守规则：
    # 只在选手当前所在队的历史冠军 ≤ 该选手登场年份时才计入，避免把
    # 2024 年才出道的选手算成 2019 年冠军成员。用登场时间（boarding 不在
    # perf 里）无法判断，于是干脆不计 —— 宁可少算，也不虚抬。
    # （五季之内的冠军首发是精确的，已经算进去了。）
    pass

    # 总决赛 FMVP
    for season, nm in FINALS_MVP.items():
        if nm:
            honor[nm] += FINALS_MVP_PTS * EVENT_TIER.get(season, 1.0)
            detail[nm].append('%s FMVP' % season)

    # 冠军成员：该季该队所有出场过的选手
    for season in SEASONS:
        champ = champs.get(season)
        if not champ:
            continue
        tier = EVENT_TIER.get(season, 1.0)
        p = os.path.join(BASE, 'sched_%s.json' % season)
        lst = (json.load(io.open(p, encoding='utf-8')).get('data') or {}).get('list') or []
        team_ids = set()
        for x in lst:
            if x.get('team_a_name') == champ:
                team_ids.add(x.get('team_a_id'))
            if x.get('team_b_name') == champ:
                team_ids.add(x.get('team_b_id'))
        # 冠军成员 = 该队当季出场最多的五人组（按分路各取一人），
        # 不是「所有上过场的」——替补蹭冠军荣誉会把板凳球员的评分虚抬上去。
        pp = os.path.join(BASE, 'perf_%s.json' % season)
        if not os.path.exists(pp):
            continue
        # 2025 三季的 perf 没有 player_short_name，用 2026 的 player_id -> 名字表回填
        rows = []
        for r in json.load(io.open(pp, encoding='utf-8')).get('data') or []:
            if r.get('team_id') not in team_ids:
                continue
            nm = r.get('player_short_name') or ID2NAME.get(r.get('player_id'))
            if nm:
                r = dict(r, player_short_name=nm)
                rows.append(r)
        if not rows:
            continue
        by_lane = collections.defaultdict(list)
        for r in rows:
            lane = LANE_OF_POS.get(r.get('position'))
            if lane:
                by_lane[lane].append(r)
        starters = []
        if len(by_lane) >= 4:            # 官方标了分路：每个分路取出场最多的
            for lane, rs in by_lane.items():
                starters.append(max(rs, key=lambda r: float(r.get('Matchcount') or 0)))
        else:                            # 没标分路：直接取出场最多的 5 人
            starters = sorted(rows, key=lambda r: -float(r.get('Matchcount') or 0))[:5]
        for r in starters:
            nm = r['player_short_name']
            honor[nm] += CHAMPION_PTS * tier * SEASON_W.get(season, 1.0)
            detail[nm].append('%s冠军(首发)' % season)
    return honor, detail, champs


if __name__ == '__main__':
    honor, detail, champs = build()
    o = io.open(os.path.join(BASE, 'honor_report.txt'), 'w', encoding='utf-8')
    o.write('champions: %s\n\n' % champs)
    for nm, v in sorted(honor.items(), key=lambda kv: -kv[1]):
        o.write('%-8s %6.2f  %s\n' % (nm, v, ','.join(detail[nm][:8])))
    o.close()
    print('honor players', len(honor))
