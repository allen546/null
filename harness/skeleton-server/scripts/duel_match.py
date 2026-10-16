#!/usr/bin/env python3
"""One clean last-team-standing match: team A (null_a*) against team B (null_b*), no respawn.

Normal difficulty, no mob spawning. Steps: stop the server, wipe the bots' player data (allen546 is kept), start it, start the two
nodes with telemetry, set teams and glow (A red, B blue), kit (chestplate and shield by seeded
coin flips first, sword last; bots wait for the sword), then sample every bot's server-side
position and health twice a second until one team is all dead (or --max-s). Then stop the nodes
and run duel_analyze.py on the run directory.

usage: duel_match.py [--n 8] [--seed 7] [--max-s 600]
Stdlib only. The server keeps running afterwards so the field can be inspected.
"""
import argparse, hashlib, json, os, random, re, signal, subprocess, sys, time, uuid
from pathlib import Path

S = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(S / 'scripts'))
import rcon  # noqa: E402

NODE = os.environ.get('NODE', '/Users/allen/Desktop/null/target/release/null-node')
DEATH = re.compile(r'\]: (null_[ab]\d+) (?!joined|left|lost connection|has made|has reached|has completed)(.*)$')


def offline_uuid(name):
    h = bytearray(hashlib.md5(f'OfflinePlayer:{name}'.encode()).digest())
    h[6] = h[6] & 0x0F | 0x30
    h[8] = h[8] & 0x3F | 0x80
    return str(uuid.UUID(bytes=bytes(h)))


class Rcon:
    """One persistent connection (rcon.run opens a new one per call)."""
    def __init__(self):
        import socket
        self.s = socket.create_connection(('127.0.0.1', 25577)); self.s.settimeout(30)
        self.s.sendall(rcon.pkt(1, 3, 'skeleton')); rcon.rd(self.s)
    def __call__(self, cmd):
        self.s.sendall(rcon.pkt(2, 2, cmd)); return rcon.rd(self.s)[2]


def wait_rcon(timeout=120):
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            return Rcon()
        except OSError:
            time.sleep(1)
    sys.exit('server did not come up')


def nums(s):
    return [float(x) for x in re.findall(r'-?\d+(?:\.\d+)?(?:E-?\d+)?', s.split('data:', 1)[-1])]


CENTER = (416, 352)


def ground_y(r, x, z, top=100, bottom=50):
    """Highest y to stand at (x, z): ground that is not leaves or a trunk, with two free blocks above."""
    for y in range(top, bottom, -1):
        c = (f'execute unless block {x} {y} {z} #minecraft:replaceable unless block {x} {y} {z} #minecraft:leaves '
             f'unless block {x} {y} {z} #minecraft:logs if block {x} {y + 1} {z} #minecraft:replaceable '
             f'if block {x} {y + 2} {z} #minecraft:replaceable')
        if r(c).startswith('Test passed'):
            return y + 1
    return None


def place_formation(r, teams, spec, log):
    """Each team in ROWS x COLS, 2 blocks apart; A north of the centre facing south, B south facing north."""
    rows, cols = map(int, spec.lower().split('x'))
    cx, cz = CENTER
    r(f'forceload add {cx - 2 * cols} {cz - 12 - 2 * rows} {cx + 2 * cols} {cz + 12 + 2 * rows}')
    for t, sign, yaw in (('a', -1, 0), ('b', 1, 180)):
        for i, b in enumerate(teams[t][:rows * cols]):
            row, col = divmod(i, cols)
            x, z = cx + (col - cols // 2) * 2, cz + sign * (12 + row * 2)
            y = ground_y(r, x, z)
            if y is None:
                log('no ground for', b, 'at', x, z)
                continue
            r(f'tp {b} {x + 0.5} {y} {z + 0.5} {yaw} 0')
    r('forceload remove all')
    log('formation', spec, 'placed')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--n', type=int, default=8)
    ap.add_argument('--seed', type=int, default=7)
    ap.add_argument('--max-s', type=int, default=600)
    ap.add_argument('--formation', help='ROWSxCOLS per team, e.g. 4x5: teams face each other across the centre')
    ap.add_argument('--wait-for', help='hold the start (swords) until this player is online')
    ap.add_argument('--delay', type=float, default=0, help='seconds after --wait-for joins before the start')
    a = ap.parse_args()
    teams = {t: [f'null_{t}{i}' for i in range(1, a.n + 1)] for t in 'ab'}
    bots = teams['a'] + teams['b']
    run = S / 'runs' / time.strftime('match-%Y%m%d-%H%M%S'); run.mkdir(parents=True)
    log = lambda *m: print(time.strftime('%H:%M:%S'), *m, flush=True)

    # Clean start: no leftover inventory, effects or position for any bot.
    subprocess.run(['pkill', '-f', 'target/release/null-node'])
    subprocess.run([str(S / 'scripts/stop_server.sh')], capture_output=True)
    wd = S / 'server/world'
    for b in bots:
        u = offline_uuid(b)
        for p in [wd / 'playerdata' / f'{u}.dat', wd / 'playerdata' / f'{u}.dat_old',
                  wd / 'stats' / f'{u}.json', wd / 'advancements' / f'{u}.json']:
            p.unlink(missing_ok=True)
    subprocess.Popen([str(S / 'scripts/start_server.sh')], stdout=subprocess.DEVNULL)
    time.sleep(3)
    r = wait_rcon()
    # Normal difficulty heals at survival speed (peaceful healed ~1.6 HP/s); no mobs join in.
    for c in ['difficulty normal', 'gamerule spawn_monsters false', 'gamerule spawn_phantoms false',
              'gamerule spawn_patrols false', 'kill @e[type=!player]',
              'gamerule keep_inventory false', 'gamerule immediate_respawn false',
              'gamerule natural_health_regeneration true', 'time set day', 'weather clear',
              'kill @e[type=item]', 'team add A', 'team modify A color red',
              'team add B', 'team modify B color blue']:
        r(c)
    base = sum(1 for _ in open(S / 'server/server.log'))
    (run / 'baseline').write_text(str(base))

    procs = []
    for t, o in (('a', 'b'), ('b', 'a')):
        procs.append(subprocess.Popen(
            [NODE, '--server', '127.0.0.1:25567', '--accounts', f'null_{t}1..null_{t}{a.n}',
             '--center', '416,352', '--fight', '--no-respawn', '--enemies', f'null_{o}1..null_{o}{a.n}',
             '--state-log', str(run / f'{t}.state.jsonl'), '--telemetry', str(run / f'{t}.tel.jsonl')],
            stdout=open(run / f'{t}.stdout', 'w'), stderr=open(run / f'{t}.stderr', 'w')))
    log('nodes started, waiting for', len(bots), 'bots')
    t0 = time.time()
    while time.time() - t0 < 120:
        online = r('list')
        if all(b in online for b in bots):
            break
        time.sleep(1)
    else:
        log('not all bots joined:', r('list'))

    rng = random.Random(a.seed)
    kits = {}
    for b in bots:
        kits[b] = {'team': b[5], 'chest': rng.random() < 0.5, 'shield': rng.random() < 0.5}
    for b in bots:
        r(f'team join {"A" if b[5] == "a" else "B"} {b}')
        r(f'effect give {b} glowing infinite 0 true')
        r(f'clear {b}')
        if kits[b]['chest']:
            r(f'item replace entity {b} armor.chest with iron_chestplate')
        if kits[b]['shield']:
            r(f'item replace entity {b} weapon.offhand with shield')
    if a.formation:
        place_formation(r, teams, a.formation, log)
    if a.wait_for:
        log('waiting for', a.wait_for, 'to join')
        while a.wait_for not in r('list'):
            time.sleep(1)
        log(a.wait_for, f'joined; starting in {a.delay:.0f} s')
        time.sleep(a.delay)
    start = time.time()
    for b in bots:  # sword last: it is the bots' signal to start
        r(f'give {b} iron_sword')
    (run / 'kits.json').write_text(json.dumps(kits, indent=1))
    log('kits given, match on')

    alive = set(bots); deaths = []
    logf = open(S / 'server/server.log'); [logf.readline() for _ in range(base)]
    samples = open(run / 'server.samples.jsonl', 'w')
    ended = None
    while True:
        for line in iter(logf.readline, ''):
            m = DEATH.search(line)
            if m and m.group(1) in alive:
                alive.discard(m.group(1))
                killer = re.search(r'by (null_[ab]\d+)', m.group(2))
                deaths.append({'bot': m.group(1), 'msg': m.group(2).strip(), 'killer': killer and killer.group(1),
                               't_s': round(time.time() - start, 2), 'wall_ms': int(time.time() * 1000)})
                log('death', m.group(1), m.group(2).strip(), f'({len(alive)} left)')
        for b in sorted(alive):
            ms = int(time.time() * 1000)  # per bot: one round takes ~100 ms
            pos, hp = nums(r(f'data get entity {b} Pos')), nums(r(f'data get entity {b} Health'))
            if len(pos) == 3 and hp:
                samples.write(json.dumps({'wall_ms': ms, 'bot': b, 'x': pos[0], 'y': pos[1], 'z': pos[2], 'hp': hp[0]}) + '\n')
        samples.flush()
        left = {t: [b for b in teams[t] if b in alive] for t in 'ab'}
        if not left['a'] or not left['b']:
            ended = 'b' if not left['a'] else 'a'
            break
        if time.time() - start > a.max_s:
            ended = 'timeout'
            break
        time.sleep(0.5)

    dur = round(time.time() - start, 1)
    time.sleep(1)
    for p in procs:
        p.send_signal(signal.SIGINT)
    for p in procs:
        try:
            p.wait(10)
        except subprocess.TimeoutExpired:
            p.kill()
    end = sum(1 for _ in open(S / 'server/server.log'))
    (run / 'server.log').write_text(''.join(open(S / 'server/server.log').readlines()[base:end]))
    match = {'winner': ended, 'duration_s': dur, 'start_wall_ms': int(start * 1000), 'n': a.n, 'seed': a.seed,
             'survivors': sorted(alive), 'deaths': deaths}
    (run / 'match.json').write_text(json.dumps(match, indent=1))
    log('match over:', 'team ' + ended.upper() if ended != 'timeout' else 'timeout', f'in {dur}s; survivors', sorted(alive))
    subprocess.run([sys.executable, str(S / 'scripts/duel_analyze.py'), str(run)])
    log('report:', run / 'report.md')


if __name__ == '__main__':
    main()
