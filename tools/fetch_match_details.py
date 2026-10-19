# -*- coding: utf-8 -*-
"""抓每一小局的出场记录（选手 + 英雄 + 胜负），补 2025 三季缺失的胜率和英雄池。

官网 perf 接口只有 2026 两季带参团率；2025 三季连名字都没有、也没有绝对量
（分均伤害/承伤、胜率、英雄池）。getScheduleDetail 能给出每小局两边各 5 人
的英雄和获胜方，用它重建「每人每季：胜场 / 总局 / 用过哪些英雄 / 每个英雄打了几局」。
结果写到 gamedata_<season>.json。
"""
import json, io, os, subprocess, collections, time

BASE = os.environ.get('KPL_DATA', os.path.dirname(os.path.abspath(__file__)))
SEASONS = ['KPL2025S1', 'KPL2025S2', 'KPL2025S3', 'KPL2026S1', 'KPL2026S2']
URL = 'https://kplshop-op.timi-esports.qq.com/kplow/getScheduleDetail'


def post(url, body):
    for _ in range(3):
        p = subprocess.run(['curl', '-s', '--max-time', '25', '-X', 'POST', url,
                            '-H', 'Content-Type: application/json',
                            '-H', 'Origin: https://kpl.qq.com',
                            '-H', 'Referer: https://kpl.qq.com/',
                            '-d', json.dumps(body)], capture_output=True)
        try:
            return json.loads(p.stdout)
        except Exception:
            time.sleep(0.4)
    return {}


for season in SEASONS:
    sp = os.path.join(BASE, 'sched_%s.json' % season)
    if not os.path.exists(sp):
        print('skip', season); continue
    lst = (json.load(io.open(sp, encoding='utf-8')).get('data') or {}).get('list') or []
    # playerid -> 统计
    stat = collections.defaultdict(lambda: dict(games=0, wins=0, heroes=collections.Counter(),
                                                wins_by_hero=collections.Counter(), team=''))
    n_ok = n_fail = 0
    for sch in lst:
        sid = sch.get('scheduleid')
        d = post(URL, {'seasonid': season, 'scheduleid': sid})
        rounds = ((d.get('data') or {}).get('round_details') or []) if d.get('result') == 0 else []
        if not rounds:
            n_fail += 1
            continue
        n_ok += 1
        for rd in rounds:
            win_team = rd.get('win_team') or ''
            for pl in rd.get('players') or []:
                pid = pl.get('playerid')
                if not pid:
                    continue
                s = stat[pid]
                s['games'] += 1
                hero = pl.get('hero_name') or ''
                if hero:
                    s['heroes'][hero] += 1
                # win_team 是队 id；选手所属队通过 schedule 的 a/b 队判断
                # 这里无法直接知道 pid 属于哪边，先记录两队的 win_team，
                # 由调用方用 team_id 归属判断；为简化，把 win_team 记在每局里
                s.setdefault('rounds', []).append((sid, win_team, hero))
        # 记住这场两边的队
        for pid in {p.get('playerid') for rd in rounds for p in (rd.get('players') or [])}:
            stat[pid].setdefault('schedules', set()).add((sid, sch.get('team_a_id'), sch.get('team_b_id')))
    print(season, 'ok', n_ok, 'fail', n_fail, 'players', len(stat))
    json.dump({k: dict(games=v['games'],
                       heroes=dict(v['heroes']),
                       rounds=v.get('rounds', []),
                       schedules=sorted(list(x) for x in v.get('schedules', set())))
               for k, v in stat.items()},
              io.open(os.path.join(BASE, 'gamedata_%s.json' % season), 'w', encoding='utf-8'))
