#!/usr/bin/env python3
"""Analytics for one duel_match.py run: per-bot and per-team numbers, plus checks that compare
what each bot believed (its telemetry) with what the server says (RCON samples, server log).
Writes report.md and analysis.json into the run directory. usage: duel_analyze.py RUN_DIR

Checks (each anomaly is listed at the top of the report):
- self position: bot's own x,z against the server's, at the nearest sample
- target distance: bot's believed distance to its target against the server's positions
- target health: bot's belief against the server's health of the target
- hits: an attack counts as a hit when the victim's telemetry shows damage within 350 ms
- shield speed: moving faster than vanilla allows (about 1.5 b/s) while the shield is up
- stuck: chasing (seek with an enemy, or fight) someone over 4 blocks away, but moved under 0.5 blocks in 3 s
- relock: time from a fight ending (kill or lost target, not a retreat) to the next fight
- idle: time seeking with no enemy known while the enemy team still had bots alive
- dead targets: attacks or fights aimed at a bot that the server log already shows dead
- server log: kicks, movement warnings, WARN and ERROR lines
"""
import bisect, json, math, re, statistics as st, sys
from collections import defaultdict
from pathlib import Path

run = Path(sys.argv[1])
match = json.loads((run / 'match.json').read_text())
kits = json.loads((run / 'kits.json').read_text())
tel = [json.loads(l) for t in 'ab' for l in open(run / f'{t}.tel.jsonl') if l.strip()]
tel.sort(key=lambda e: e['wall_ms'])
srv = [json.loads(l) for l in open(run / 'server.samples.jsonl') if l.strip()]
slog = (run / 'server.log').read_text().splitlines()
t0 = match['start_wall_ms']
dead_at = {d['bot']: d['wall_ms'] for d in match['deaths']}
bots = sorted(kits)

by_bot = defaultdict(list)
for e in tel:
    by_bot[e['bot']].append(e)
srv_by = defaultdict(list)
for s in srv:
    srv_by[s['bot']].append(s)
srv_t = {b: [s['wall_ms'] for s in v] for b, v in srv_by.items()}



def chasing(x):
    """A sample where the bot should be moving toward someone more than 4 blocks away."""
    if x['phase'] == 'fight':
        return bool(x.get('target')) and x.get('target_dist', 0) > 4
    return x['phase'] == 'seek' and bool(x.get('enemy')) and (x.get('enemy_dist') or 0) > 4

def srv_around(bot, ms, tol=700):
    """The raw server samples just before and after `ms` (health changes in steps, so it is never interpolated)."""
    ts = srv_t.get(bot)
    if not ts:
        return []
    i = bisect.bisect_left(ts, ms)
    return [srv_by[bot][j] for j in (i - 1, i) if 0 <= j < len(ts) and abs(ts[j] - ms) <= tol]


def srv_at(bot, ms, tol=700, edge=True):
    """Server state at `ms`, linearly interpolated between the two samples around it."""
    ts = srv_t.get(bot)
    if not ts:
        return None
    i = bisect.bisect_left(ts, ms)
    if 0 < i < len(ts) and ts[i] - ts[i - 1] <= 2 * tol:
        a, b = srv_by[bot][i - 1], srv_by[bot][i]
        f = (ms - ts[i - 1]) / max(1, ts[i] - ts[i - 1])
        return {k: a[k] + (b[k] - a[k]) * f for k in ('x', 'y', 'z', 'hp')}
    if not edge:
        # Past the first or last sample a sprinting bot is up to 0.7 s from where we compare it.
        return None
    j = min((j for j in (i - 1, i) if 0 <= j < len(ts)), key=lambda j: abs(ts[j] - ms), default=None)
    return srv_by[bot][j] if j is not None and abs(ts[j] - ms) <= tol else None


def q(xs):
    if not xs:
        return {'n': 0}
    xs = sorted(xs)
    return {'n': len(xs), 'median': round(st.median(xs), 2), 'p90': round(xs[int(0.9 * (len(xs) - 1))], 2), 'max': round(xs[-1], 2)}


anoms = []
per = {}
hurts = defaultdict(list)  # victim -> wall_ms of hurt events
for e in tel:
    if e['ev'] == 'hurt':
        hurts[e['bot']].append(e['wall_ms'])

for b in bots:
    ev = by_bot[b]
    # From the start signal (swords) only: a formation hold is not a stall.
    samples = [e for e in ev if e['ev'] == 'sample' and e['wall_ms'] >= t0]
    attacks = [e for e in ev if e['ev'] == 'attack']
    hurt = [e for e in ev if e['ev'] == 'hurt']
    phases = [e for e in ev if e['ev'] == 'phase']
    p = {'team': kits[b]['team'], 'chest': kits[b]['chest'], 'shield': kits[b]['shield']}
    d = next((x for x in match['deaths'] if x['bot'] == b), None)
    p['died_s'] = d and d['t_s']
    p['death'] = d and d['msg']
    p['kills'] = sum(1 for x in match['deaths'] if x['killer'] == b)
    p['attacks'] = len(attacks)
    p['crit_tries'] = sum(1 for a in attacks if a.get('crit_try'))
    hits = 0
    for a in attacks:
        v = a.get('target')
        hs = hurts.get(v, [])
        i = bisect.bisect_left(hs, a['wall_ms'])
        if i < len(hs) and hs[i] - a['wall_ms'] <= 350:
            hits += 1
    p['hits'] = hits
    p['hit_rate'] = round(hits / len(attacks), 2) if attacks else None
    p['damage_taken'] = round(sum(h['dmg'] for h in hurt), 1)
    p['hurt_while_blocking'] = sum(1 for h in hurt if h.get('blocking'))
    p['shield_ups'] = sum(1 for e in ev if e['ev'] == 'shield' and e.get('up'))
    share = defaultdict(int)
    for s in samples:
        share[s['phase']] += 1
    p['phase_share'] = {k: round(v / len(samples), 2) for k, v in share.items()} if samples else {}
    p['blocking_share'] = round(sum(1 for s in samples if s['blocking']) / len(samples), 2) if samples else None
    p['fight_ends'] = dict(sorted(((w, sum(1 for x in phases if x['from'] == 'fight' and x.get('why') == w))
                                   for w in {x.get('why') for x in phases if x['from'] == 'fight'}), key=str))

    # Belief checks against the server.
    pos_err, dist_err, hp_err = [], [], []
    for s in samples:
        me = srv_at(b, s['wall_ms'])
        between = srv_at(b, s['wall_ms'], edge=False)
        if between:
            pos_err.append(math.hypot(between['x'] - s['x'], between['z'] - s['z']))
        tgt = s.get('target')
        if tgt and me:
            ts = srv_at(tgt, s['wall_ms'])
            if ts:
                real = math.dist((me['x'], me['y'], me['z']), (ts['x'], ts['y'], ts['z']))
                dist_err.append(abs(real - s['target_dist']))
                around = srv_around(tgt, s['wall_ms'])
                if s.get('target_hp') is not None and around:
                    # Right if it equals the server's value on either side of a hit.
                    hp_err.append(min(abs(n['hp'] - s['target_hp']) for n in around))
    p['self_pos_err'] = q(pos_err)
    p['target_dist_err'] = q(dist_err)
    p['target_hp_err'] = q(hp_err)

    # Shield speed and stuck spells, from consecutive own samples (0.5 s apart).
    fast_block, stuck = [], 0
    for s1, s2 in zip(samples, samples[1:]):
        dt = (s2['wall_ms'] - s1['wall_ms']) / 1000
        if not 0.2 < dt < 1.5:
            continue
        v = math.hypot(s2['x'] - s1['x'], s2['z'] - s1['z']) / dt
        if s1['blocking'] and s2['blocking'] and v > 1.5:
            fast_block.append(round(v, 2))
    for i in range(len(samples) - 6):
        w = samples[i:i + 7]
        if all(chasing(x) for x in w):
            if math.hypot(w[-1]['x'] - w[0]['x'], w[-1]['z'] - w[0]['z']) < 0.5:
                stuck += 1
    enemies = [x for x in bots if kits[x]['team'] != kits[b]['team']]
    idle = 0
    for s1 in samples:
        alive_enemies = [x for x in enemies if dead_at.get(x, 1e18) > s1['wall_ms']]
        # Newer telemetry names seek's enemy; older runs only had fight targets.
        if s1['phase'] == 'seek' and not s1.get('target') and not s1.get('enemy') and alive_enemies:
            idle += 1
    p['idle_s'] = round(idle * 0.5, 1)
    p['fast_while_blocking'] = q(fast_block)
    p['stuck_windows'] = stuck

    # Relock: fight end -> next fight start.
    relock = []
    last_end = None
    for x in phases:
        if x['from'] == 'fight' and x.get('why') in ('target_dead', 'target_lost', 'no_target'):
            last_end = x['wall_ms']
        elif x['to'] == 'fight' and last_end is not None:
            relock.append((x['wall_ms'] - last_end) / 1000)
            last_end = None
    p['relock_s'] = q(relock)

    # Aimed at the dead: attacks or fight starts on a target the server log has as dead.
    p['attacks_on_dead'] = sum(1 for a in attacks if a.get('target') in dead_at and a['wall_ms'] > dead_at[a['target']] + 500)
    p['fights_on_dead'] = sum(1 for x in phases if x['to'] == 'fight' and x.get('target') in dead_at
                              and x['wall_ms'] > dead_at[x['target']] + 500)
    per[b] = p

    if p['self_pos_err'].get('max', 0) > 2:
        anoms.append(f'{b}: own position off from the server by up to {p["self_pos_err"]["max"]} blocks')
    if p['target_dist_err'].get('p90', 0) > 1.5:
        anoms.append(f'{b}: believed target distance off by {p["target_dist_err"]["p90"]} blocks (p90), max {p["target_dist_err"]["max"]}')
    if p['target_hp_err'].get('p90', 0) > 4:
        anoms.append(f'{b}: believed target health off by {p["target_hp_err"]["p90"]} (p90)')
    if p['fast_while_blocking']['n']:
        anoms.append(f'{b}: moved faster than 1.5 b/s with the shield up {p["fast_while_blocking"]["n"]} times (max {p["fast_while_blocking"]["max"]} b/s)')
    if p['idle_s'] >= 3:
        anoms.append(f'{b}: {p["idle_s"]} s with no target while enemies were alive')
    if stuck:
        anoms.append(f'{b}: stuck {stuck} three-second windows with a target over 4 blocks away')
    if p['relock_s'].get('max', 0) > 1.5:
        anoms.append(f'{b}: slow relock, median {p["relock_s"]["median"]} s, max {p["relock_s"]["max"]} s')
    if p['attacks_on_dead'] or p['fights_on_dead']:
        anoms.append(f'{b}: aimed at dead bots: {p["attacks_on_dead"]} attacks, {p["fights_on_dead"]} fight starts')
    if p['attacks'] >= 10 and p['hit_rate'] is not None and p['hit_rate'] < 0.4:
        anoms.append(f'{b}: low hit rate {p["hit_rate"]} over {p["attacks"]} attacks')
    if p['hurt_while_blocking']:
        anoms.append(f'{b}: took damage {p["hurt_while_blocking"]} times with the shield up (hit from the side or behind, or the block failed)')
    if not p['attacks'] and p['died_s'] is not None:
        anoms.append(f'{b}: died without attacking')

warn = [l for l in slog if re.search(r'WARN|ERROR|kicked|moved too quickly|moved wrongly', l)]
for l in warn[:10]:
    anoms.append('server log: ' + l.split(']: ', 1)[-1][:160])
nonpvp = [d for d in match['deaths'] if not d['killer']]
for d in nonpvp:
    anoms.append(f'{d["bot"]}: not killed by a bot: "{d["msg"]}"')

team = {}
for t in 'ab':
    mem = [b for b in bots if per[b]['team'] == t]
    team[t] = {k: sum(per[b][k] for b in mem) for k in ('kills', 'attacks', 'hits', 'crit_tries')}
    team[t]['damage_taken'] = round(sum(per[b]['damage_taken'] for b in mem), 1)
    team[t]['survivors'] = [b for b in mem if per[b]['died_s'] is None]

kit = defaultdict(lambda: [0, 0, 0])  # kit -> [bots, survived, kills]
for b in bots:
    k = ('chest' if per[b]['chest'] else '-') + '+' + ('shield' if per[b]['shield'] else '-')
    kit[k][0] += 1
    kit[k][1] += per[b]['died_s'] is None
    kit[k][2] += per[b]['kills']

(run / 'analysis.json').write_text(json.dumps({'match': match, 'team': team, 'bots': per, 'anomalies': anoms}, indent=1))

L = [f'# Duel report: {run.name}', '',
     f'Winner: **{"team " + match["winner"].upper() if match["winner"] != "timeout" else "none (timeout)"}** in {match["duration_s"]} s. Survivors: {", ".join(match["survivors"]) or "none"}.', '',
     f'## Anomalies ({len(anoms)})', '']
L += [f'- {a}' for a in anoms] or ['- none found']
L += ['', '## Teams', '', '| team | kills | attacks | hits | crit tries | damage taken | survivors |', '|---|---|---|---|---|---|---|']
for t in 'ab':
    x = team[t]
    L.append(f'| {t.upper()} | {x["kills"]} | {x["attacks"]} | {x["hits"]} | {x["crit_tries"]} | {x["damage_taken"]} | {len(x["survivors"])} |')
ends = defaultdict(int)
for b in bots:
    for k, v in per[b]['fight_ends'].items():
        ends[k] += v
L += ['', 'Fight ends: ' + ', '.join(f'{k} {v}' for k, v in sorted(ends.items(), key=lambda kv: -kv[1]))]
L += ['', '## Kits', '', '| kit | bots | survived | kills |', '|---|---|---|---|']
L += [f'| {k} | {v[0]} | {v[1]} | {v[2]} |' for k, v in sorted(kit.items())]
L += ['', '## Bots', '',
      '| bot | kit | died at s | death | kills | attacks | hit rate | crits tried | dmg taken | hurt w/ shield | blocking share | relock median/max s | idle s | pos err max | target dist err p90 | stuck |',
      '|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|']
for b in bots:
    p = per[b]
    kt = ('C' if p['chest'] else '-') + ('S' if p['shield'] else '-')
    rl = p['relock_s']
    L.append(f'| {b} | {kt} | {p["died_s"] if p["died_s"] is not None else "alive"} | {p["death"] or ""} | {p["kills"]} | {p["attacks"]} | {p["hit_rate"]} | {p["crit_tries"]} | {p["damage_taken"]} | {p["hurt_while_blocking"]} | {p["blocking_share"]} | '
             f'{rl.get("median", "-")}/{rl.get("max", "-")} | {p["idle_s"]} | {p["self_pos_err"].get("max", "-")} | {p["target_dist_err"].get("p90", "-")} | {p["stuck_windows"]} |')
L += ['', '## Deaths in order', '']
L += [f'- {d["t_s"]} s: {d["bot"]} {d["msg"]}' for d in match['deaths']]
L += ['', 'Raw data in this folder: `a.tel.jsonl`, `b.tel.jsonl` (bot telemetry: samples every 10 ticks and events), `server.samples.jsonl` (RCON, 2 Hz), `server.log`, `kits.json`, `match.json`, `analysis.json`.']
(run / 'report.md').write_text('\n'.join(L) + '\n')
print('\n'.join(L[:8 + len(anoms)]))
