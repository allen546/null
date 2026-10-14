# F4 Ops and telemetry

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.7, §3.8, §8, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** F6 (bus), F1 (node health). **Used by:** operators, K4 (dashboards), K1/K3 (health in the digest).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Logs | Deterministic | Structured logs with standard fields |
| 4.2 | Event log shipping and retention | Deterministic | Append-only logs copied to a VPS |
| 4.3 | Metrics | Deterministic | Fixed counters and gauges per process |
| 4.4 | Dashboards | Deterministic | Views over the metrics |
| 4.5 | Alerts | Measured | Rules are fixed; thresholds come from observing normal operation |
| 4.6 | Anti-cheat setback tracking | Deterministic | A setback means our behaviour diverged from vanilla; detection and attribution are mechanical |
| 4.7 | Known failure classes | Deterministic | Each known class maps to a defined recovery rule |
| 4.8 | Novel failure triage | Interface · runtime decision · either | A failure with no matching class needs diagnosis before anyone can write its recovery rule |

## Architecture decisions

None open. Telemetry has standard answers; the metrics backend is a small engineering choice (§9).


---

## 1. Purpose

Things that fail every hour on an SMP (deaths, disconnects, kicks, stuck bots, looted chests, restarts, ambushes) need to be seen, counted and attributed, or the swarm degrades silently (ARCHITECTURE §1.7). F4 is the logging, metrics, dashboards and alerting that let operators and the commander see that.

## 2. Scope

**Owns:** structured logs; shipping and retention of event logs; metrics; dashboards; alerts; anti-cheat setback tracking; the catalogue of known failure classes; the triage queue for novel failures.

**Does not own:** recovery itself (each component's own rules); the commander digest (K1/K3), though F4 feeds it health numbers; the map view (K4).

## 3. Interfaces

- **In:** logs and metrics from every process over the bus or a sidecar; `Kick`, `Ban`, `BotState` from F1; job outcomes from K5/K6.
- **Out:** dashboards (K4 embeds them); alerts to operators (K4 notifications) and summarised health rows to K1 for the digest; `NovelFailure` tickets.

## 4. Requirements

### 4.1 Logs

Every log line has: time (wall), server game time where known, process, node, operation, squad, bot, job id, level, event type, message. Player-authored text in logs is stored inside quarantine fields (CONVENTIONS §3.7) so log viewers and LLMs reading logs can tell it apart.

### 4.2 Event log shipping and retention

- Operation shard logs (F2), HQ logs and audit logs (K2) are append-only and shipped to a VPS.
- Retention by class: audit logs kept long; per-bot debug logs short and sampled.

### 4.3 Metrics

Per bot (aggregated before leaving the node): deaths by cause, time by lifecycle state, job outcomes, items gathered and delivered. Per node: bots, CPU, RAM, tick lag, bus traffic. Per operation: progress, headcount by role, escalations. Per server: estimated MSPT from client-observed tick timing, player count, restarts.

### 4.4 Dashboards

Swarm overview, per operation, per node, per server, and a death and kick map overlay for K4's map.

### 4.5 Alerts

Rules: deaths per 10 min above baseline in an operation; kicks per 10 min; a node missing heartbeats; claim count growing without bound (leak); escalations per hour above threshold; bus lag. Baselines and thresholds are set after watching normal runs in the harness and on live.

### 4.6 Anti-cheat setback tracking

- A server position correction, a rejected interaction, or an anti-cheat kick means our client did something vanilla wouldn't. F4 records the bot's last few seconds of inputs and physics state, and the skill that was running.
- Setbacks are grouped by skill and movement type so engineers can find the bug in B1/B5/B6. The fix is to behave the way vanilla does; this component has no part in disguising behaviour.

### 4.7 Known failure classes

A catalogue mapping failure classes to recovery rules, owned jointly with K5's recovery rules: `died(cause)`, `disconnected`, `stuck(skill)`, `container_changed`, `claim_lost`, `cargo_lost`, `kicked(class)`, `node_down`, `server_restart`, and so on. Each class names its recovery and whether it escalates.

### 4.8 Novel failure triage

> **Interface input (runtime decision).** Who: either (an LLM drafts a diagnosis from logs and replays; a person approves the new rule).
> Why: a failure that matches no known class (a new server plugin behaviour, a mechanic we got wrong, an enemy tactic) has no recovery rule yet, and finding its cause is open-ended investigation.
> Enters the system as: a new failure class and recovery rule added to the catalogue (4.7), plus a harness scenario that reproduces it.
> Null's side: the affected jobs fall back to the generic recovery (release claims, requeue once, then mark the job `blocked(unknown_failure)` and stop retrying), and F4 bundles logs, the replay window and similar past events into the ticket.

Grouping: novel failures with the same signature (skill, error, region, time window) are merged into one ticket, so 37 bots hitting the same new problem make one ticket, not 37.

## 5. Failure handling

Telemetry must not take down the swarm: if the metrics pipeline is down, processes buffer a bounded amount and then drop metrics, never block.

## 6. Scale and performance

Per-bot metrics are aggregated on the node. At 2,000 bots the metrics backend sees per-node and per-operation series, not per-bot series, except for sampled debug.

## 7. Version-sensitive parts

Kick message formats and setback signals differ by server software and version; parsed through F5.

## 8. Tests

- Kill 30 bots in an operation in the harness: the deaths alert fires once, merged.
- Introduce a harness behaviour no class covers (a block that vanishes every 100 gt): one novel-failure ticket with a replay window.

## 9. Open questions

- Which metrics backend fits a mix of home machines and VPSes with intermittent links? Prometheus with remote write, or push over NATS.
