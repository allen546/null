#!/usr/bin/env -S uv run --no-project python
# Runs the ability check on the Pi against one server it starts and stops itself, with a 10 s monitor
# (tick query, load, memory, server CPU) beside the run. Stdlib only; full_suite.sh runs it with uv (/home/allen/.local/bin/uv, not on the SSH PATH).
# Usage: run_server_suite.py vanilla|grim <prefix> [--repeat N] [null-abilitycheck args, default --scenario all]
# --repeat N runs the check N times on one server start (repro runs of a flaky module); the run log holds all N.
# Logs: logs/<prefix>_<kind>_run.log and _monitor.log. Spec: specs/plans/ABILITY_CHECK.md §How to run
import os, re, resource, subprocess, sys, threading, time
H = '/mnt/pi-data/null-harness'
sys.path.insert(0, H + '/skeleton-server/scripts')
import rcon

kind, prefix, extra = sys.argv[1], sys.argv[2], sys.argv[3:]
repeat = 1
if extra[:1] == ['--repeat']: repeat, extra = int(extra[1]), extra[2:]
extra = extra or ['--scenario', 'all']
cfg = {'vanilla': dict(dir=H + '/skeleton-server/server', jar='server.jar', port=25567, rc=25577),
       'grim': dict(dir=H + '/grim-server/server', jar='paper.jar', port=25568, rc=25578)}[kind]
L = H + '/logs/'
slog = cfg['dir'] + '/server.log'
def ts(): return time.strftime('%H:%M:%S')
mon = open(L + f'{prefix}_{kind}_monitor.log', 'w', buffering=1)

t0 = time.time()
srv = subprocess.Popen(['java', '-Xms1G', '-Xmx2G', '-XX:+UseG1GC', '-XX:+ParallelRefProcEnabled', '-XX:MaxGCPauseMillis=100',
                        '-jar', cfg['jar'], 'nogui'],
                       cwd=cfg['dir'], stdout=open(slog, 'w'), stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL)
open(cfg['dir'] + '/server.pid', 'w').write(str(srv.pid))
mon.write(f'{ts()} server pid {srv.pid}\n')
ready = None
while time.time() - t0 < 600:
    if srv.poll() is not None:
        mon.write(f'{ts()} SERVER DIED before RCON\n'); sys.exit(1)
    if 'RCON running' in open(slog, errors='replace').read():
        ready = time.time() - t0; break
    time.sleep(1)
mon.write(f'{ts()} RCON running after {ready} s\n')
if ready is None:
    srv.kill(); sys.exit(1)
time.sleep(5)

stop = threading.Event()
def cpu(pid):
    try:
        f = open(f'/proc/{pid}/stat').read().rsplit(')', 1)[1].split()
        return (int(f[11]) + int(f[12])) / os.sysconf('SC_CLK_TCK')
    except Exception:
        return -1
def monitor():
    while not stop.is_set():
        try: tq = rcon.run(['tick query'], port=cfg['rc'])[0].replace('\n', ' | ')
        except Exception as e: tq = f'ERR {e}'
        load = open('/proc/loadavg').read().split()[:3]
        mem = {l.split(':')[0]: int(l.split()[1]) // 1024 for l in open('/proc/meminfo')
               if l.split(':')[0] in ('MemFree', 'MemAvailable')}
        mon.write(f'{ts()} TICK {tq} | LOAD {" ".join(load)} | MEM {mem} | srvcpu {cpu(srv.pid)}\n')
        stop.wait(10)
th = threading.Thread(target=monitor); th.start()

def check(account, log, args, mode='w'):
    return subprocess.Popen([H + '/bin/null-abilitycheck', '--server', f'127.0.0.1:{cfg["port"]}', '--rcon-port', str(cfg['rc']),
                             '--server-log', slog, '--account', account, *args],
                            cwd=H, stdout=open(L + f'{prefix}_{kind}_{log}.log', mode), stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL)
# Warm-up: a fresh server lags when the first players join (cold JIT, chunk loads), and every account
# online then draws Grim TimerLimit alerts (pi_base, 2026-10-21: 37 in 18 s, all before the first row).
# Three short modules on their own account take that lag; the suite's alerts are counted after it.
W = time.time()
wrc = check('null_warm', 'warmup', ['--scenario', 'A1,A3,A8']).wait()
time.sleep(10)
mon.write(f'{ts()} WARMUP exit {wrc} wall {time.time() - W:.1f}s\n')
mark = os.path.getsize(slog)
r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
T = time.time()
rc = 0
for i in range(repeat):
    if repeat > 1: mon.write(f'{ts()} REPEAT {i + 1}/{repeat}\n')
    rc = check('null_pi', 'run', extra, 'w' if i == 0 else 'a').wait() or rc
r = resource.getrusage(resource.RUSAGE_CHILDREN)  # minus r0: the warm-up run is reaped too
mon.write(f'{ts()} BOT exit {rc} wall {time.time() - T:.1f}s botcpu user {r.ru_utime - r0.ru_utime:.1f} '
          f'sys {r.ru_stime - r0.ru_stime:.1f} maxrss_MB {r.ru_maxrss / 1024:.0f} (max over both runs)\n')
stop.set(); th.join()
def alerts(text):
    return sum(1 for l in text.splitlines() if re.search(r'Grim.* failed ', re.sub(r'\x1b\[[0-9;]*m', '', l)))
def allowed_alerts():
    # Rows of Ctx::run_allowing print "expected Grim alerts from <why> (...): Check xN, ..." (N = log lines).
    n = 0
    for l in open(L + f'{prefix}_{kind}_run.log', errors='replace'):
        m = re.search(r'expected Grim alerts from .*?\): ([^;]*)', re.sub(r'\x1b\[[0-9;]*m', '', l))
        if m: n += sum(int(x) for x in re.findall(r' x(\d+)', m.group(1)))
    return n
whole = open(slog, 'rb').read()
total = alerts(whole[mark:].decode(errors="replace"))
allowed = min(allowed_alerts(), total)
# The first number is the alerts no row allowed (full_suite.sh reads it); the allowed ones follow.
mon.write(f'{ts()} GRIM alerts in the suite: {total - allowed} (plus {allowed} allowed by rows, vanilla would draw them too); '
          f'in the warm-up: {alerts(whole[:mark].decode(errors="replace"))}\n')
try: rcon.run(['stop'], port=cfg['rc'])
except Exception as e: mon.write(f'stop err {e}\n')
try: srv.wait(180)
except Exception:
    mon.write('server did not stop in 180 s, killed\n'); srv.kill()
mon.write(f'{ts()} server exit {srv.returncode}\n')
