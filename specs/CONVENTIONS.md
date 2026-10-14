# Null component specs: conventions

Spec set v1.2, 2026-10-14. Derived from `ARCHITECTURE.md` v0.7 §10 (31 components) and brought up to date with v0.8, which adopted the specs' proposals and kept v0.7's section numbers, so the `ARCHITECTURE §n` references in every spec are still valid. Design only, no code. The architecture document stays the source of truth; where a spec and it disagree, the architecture wins until it is changed.

Every spec has the same shape, uses the same verdicts and flags, and inherits the shared rules below. Every flagged architecture decision is listed in `INDEX.md`.

---

## 1. Spec layout

| section | what it holds |
|---|---|
| Header | id, name, sources in ARCHITECTURE.md, depends on, used by |
| **Judgment map** | one row per subsection of §4, with a verdict and a one-line reason |
| **Architecture decisions** | the flagged points (⚑): open or debatable architecture choices, each with options, a recommendation and why it needs a person |
| 1. Purpose | why the component exists, in two or three sentences |
| 2. Scope | what it owns and, just as important, what it does not |
| 3. Interfaces | messages in and out, contracts, who calls whom |
| 4. Requirements | numbered subsections, including interface inputs |
| 5. Failure handling | what breaks and what the component does about it |
| 6. Scale and performance | targets at 20 and at ~2,000 bots |
| 7. Version-sensitive parts | what must go through the version layer (F5) |
| 8. Tests | F3 harness scenarios and acceptance checks |
| 9. Open questions | things this spec could not settle |

---

## 2. Verdicts and flags

**The question for every point: is this architecture settled, or is it still open?** A point is settled when it can be built as written and nothing breaks if details change later (the order of stages in the bot runtime, a threshold, a retry count). A point is open when it can't be decided yet, or reasonable people could pick differently and the choice is expensive to undo. Only open points are flagged (Allen, 2026-10-13).

Things a person or an LLM decides **while the swarm runs** (approvals, strategy, diplomacy, answers to a stuck report) are not architecture questions. They are part of a component's interface, and the spec describes them as interface inputs.

### Verdicts on subsections (the judgment map)

| verdict | meaning |
|---|---|
| **Deterministic** | Implementable as written. Details can change later without harm. |
| **Measured** | Implementable as written; constants come from harness or live measurement. |
| **Interface · runtime decision** | The component takes a decision from a person, the commander or an LLM while running. The spec names the channel (ticket, approval, policy) and the default while waiting. |
| **Interface · authored input** | The component loads something a person or LLM writes (a macro, a doctrine file, a pack, a rubric, a rule set). The spec names the format and what happens when it's missing. |

None of these are flags. They are all buildable now.

### Flags: architecture decisions (⚑)

Each spec has an **Architecture decisions** section after its judgment map. A decision is flagged when at least one of these holds:

- it can't be settled yet (it waits on a measurement, a spike, or a fact only Allen knows, such as which servers);
- it's open to other opinions: the options have real trade-offs and none clearly dominates;
- it belongs to someone else (Allen's builder agent), or it would change a decision recorded in ARCHITECTURE.md.

And getting it wrong would be costly to undo. Each one is written as:

```
### ⚑ AD-<component>.<n> Question

- Options: (a) … (b) … (c) …
- Recommendation: which option, and why
- Why it needs Allen: one line
- Settled by: your call | a spike or measurement | Allen's builder agent
- If decided late: what has to change
```

Who decides is Allen unless the entry says otherwise.

### How interface inputs are written

```
### 4.n Subsection name

> **Interface input (runtime decision | authored).** Who: person | LLM | either | commander.
> Why: one line.
> Enters the system as: the channel (ticket, approval, policy, macro library, pack, doctrine file).
> Null's side: what the framework does deterministically around it (detect, report,
>   default, wait), so the swarm never stalls or improvises.
```

Who: **person** (needs human values, accountability or taste), **LLM** (open-ended but bounded and frequent, with a person able to override), **either** (an LLM drafts or decides, a person can review), **commander** (whoever holds the commander seat, a person or the LLM).

## 3. Shared rules every spec inherits

### 3.1 Job contracts (ARCHITECTURE §1.3)

Every job carries: inputs (parameters, items, claims), preconditions, an observable success test, outcomes `succeeded | failed(reason) | blocked(on) | preempted | interrupted(incident)` with a structured report, and a budget. Jobs also carry:

- `reflexes`: the reflex settings the job needs while it runs (§4 below);
- `checkpoint`: `none | resumable` (cargo jobs are resumable, ARCHITECTURE §3.14);
- `preemption_floor`: the minimum crew that can't be preempted (cargo jobs only).

`preempted` means Null stopped the job on purpose, for higher-priority work. `interrupted(incident)` means the world did something the job didn't expect: the job was contained and K5 places it elsewhere (INTERRUPTS §2). `failed(reason)` is for known failure classes (F4 §4.7).

### 3.2 Time units (ARCHITECTURE §3.14)

Every duration field names its unit. **Game ticks (`gt`)** for anything the game does: charge times, cooldowns, farm rates, settle times, budgets for in-game work. **Wall time (`ms`, `s`)** for network timeouts, lease heartbeats and node health. The server's tick rate is read from the server, never assumed to be 20.

### 3.3 Claims (ARCHITECTURE §1.2, §3.7)

Every claim has an owner, an expiry and a priority, and every claim expires on its own. Kinds: region, block set, container, items in a container, entity, bot, plus cross-shard and moving claims. A component that takes a claim names when it releases it.

### 3.3a Permits, claims and territory are different things (Allen, 2026-10-13)

- A **permit** is the operator's approval for the swarm to modify blocks in an area. Permits come only from the operator (a person, through K2 or a map annotation). They say nothing about who owns the land: on pure vanilla nothing allocates land to anyone, and server land-claim plugins are out of scope. ARCHITECTURE v0.7 §4.2 called this "claimed territory"; v0.8 and the specs say "permit" so it isn't confused with a server land claim or with the claims below.
- A **claim** (§3.3) is Null's own internal reservation, between operations and bots, of a region, block set, container, entity or bot. Claims coordinate the swarm; they grant no permission to modify anything.
- **Territory** (C8 §4.1) is what the faction defends and patrols. It usually lies inside permits, but defending an area and being allowed to modify it are separate settings.

### 3.4 Belief and staleness (ARCHITECTURE §1.1)

Every value read from the world model carries who observed it and when. Anything that acts on a count or a state checks its age against a staleness limit first, and re-verifies by visiting when it is too old.

### 3.5 Version boundary (ARCHITECTURE §9.1)

Decision crates (skills, reflexes, arbitration, the cargo filter, squad tactics, and everything above the node) never depend on `bevy*` or `azalea*`; they see the game only through Null's `BotView` and `BotCommands`. Only named mechanism crates (`mc-adapter`, `mc-nav`, `mc-act`) import Azalea and Bevy (F5 §4.1, AD-F5.2). Block, item and entity facts come from per-version generated data. Mechanics that differ by version come from the mechanics table, queried by feature, never by version number.

### 3.6 Vanilla survival only

No op, no creative, no commands, no `/forceload`, no Carpet, no accurate-placement protocol, no helper plugins. The test harness (F3) may use op to set up and observe; bots never do.

### 3.7 Player-authored text is quarantined (COMMANDER_INTERFACE §7.1)

Chat, signs, books, item names and player names never reach a planner or an LLM as instructions. They travel only inside a delimited, truncated, labelled field, and no action is ever triggered directly by such text.

---

## 4. Reflex control (Allen, 2026-10-13: reflexes must be possible to turn off)

Reflexes are per-tick behaviours that can override a bot's active skill (ARCHITECTURE §1.4, §5): eat when low, flee lava, shield, dodge, defend self, MLG bucket, re-path. They are what keeps a bot alive without waiting on the network. They are also what kills cargo, breaks certified builder replays, and starts fights the commander didn't want. So **every reflex can be turned off**, with no exceptions.

### 4.1 The reflex registry

Each reflex is registered by the skill component that owns it (B1, B2, B5, B7) with:

| field | example |
|---|---|
| `id` | `upkeep.eat`, `combat.defend_self`, `move.mlg_bucket` |
| `group` | `movement`, `survival`, `combat`, `mob` |
| `trigger` | hunger ≤ 14 and no food reserve warning |
| `action` | eat the best food in the keep-list |
| `priority` | for arbitration when two reflexes fire in one tick |
| `default` | `on` or `off` |
| `params` | thresholds, with units |

The registry is versioned with the skill library and published to the commander API so selectors and policies can name reflexes and groups.

### 4.2 Switches and scope

A reflex is `on` or `off`, optionally with parameter overrides. Settings exist at five scopes:

```
swarm policy  <  operation policy  <  squad  <  job  <  bot (person, debug only)
```

- The most specific explicit setting wins. Unset scopes inherit.
- Each scope can set one reflex, a group (`combat: off`), or everything (`all: off`).
- **Job-scope settings last only while the job runs.** Builder replay sets `all: off` for its duration, and the bot returns to its inherited settings when the step ends.
- A setting may carry an expiry (`for: 12000 gt`, or `until: job_end`).
- Who can set what: the commander through `set_policy` (K2) at swarm and operation scope; the operation and squad leader through job contracts and tactics (a squad in formation may switch off `combat.defend_self` so bots don't break ranks); a person at bot scope for debugging.

### 4.3 What "off" means

- **Off means the bot takes no autonomous action for that reflex.** It does not mean the bot stops noticing. The trigger is still evaluated every tick, and a `reflex_suppressed(id, bot, cause)` event goes to the running skill and to the squad leader, merged and rate-limited. The skill or squad decides what to do: the builder halts its replay when lava appears, a squad pulls a starving bot off the line.
- A change takes effect on the next tick after the bot receives it, and the bot acknowledges it.
- Every change is audited with who set it, at what scope, and why.
- The effective reflex state of any bot, squad or operation is visible through `inspect`. When any scope has a **survival** reflex off, the digest says so in one line, so nobody forgets that bots are running without their safety net.
- Turning survival reflexes off swarm-wide is a costly change under the commander interface's echo-before-effect rule (COMMANDER_INTERFACE §7.6): the API states what it will do and waits for confirmation. It is never refused.

### 4.4 Jobs and reflexes

A job contract declares what it needs:

- `reflexes_off: [...]`: reflexes that must be off while it runs (builder replay: `all`; cargo loading: `combat.defend_self`; an errand trigger step: `all`).
- `reflexes_assumed_on: [...]`: reflexes the job's budget and safety assume (long mining assumes `upkeep.eat`).

The squad leader applies `reflexes_off` when the job starts and restores the bot's settings when it ends. If a policy has switched off a reflex the job assumes, the scheduler doesn't silently run it: the job comes back `blocked(reflex_policy: upkeep.eat off)`, so the conflict is visible.

**Skills never rely on reflexes to stay correct.** A skill that needs a safety behaviour (the builder's halt on unexpected damage) implements it itself, so it still works with every reflex off.

### 4.5 Not reflexes

These are not switched by reflex control:

- **Cargo exclusion.** Never targeting entities tagged as cargo (ARCHITECTURE §3.10) is a targeting filter applied to reflexes and skills alike. It is turned off by un-tagging the cargo, not by reflex settings.
- **Client physics and movement.** Builder packet replay turns these off through the replay mode in B1 and B6 (ARCHITECTURE §3.9), which is a separate switch.
- **Halt and recall.** These are commands, never gated (ARCHITECTURE §4.1).
- **Azalea's built-in behaviours.** Anything autonomous Azalea does by default (`auto_respawn`, its fixed-delay reconnect, the pathfinder mining or placing on its own) is either disabled, wrapped and registered as a reflex, or replaced by Null's own (F1 §4.3 reconnect, B1 `allow_mining`). None runs unregistered (spec audit, Azalea reality check).
- **Local fallback.** "Finish the current safe step, then hold" when the link drops (ARCHITECTURE §5) is part of the bot runtime (F1), not a reflex. A person can still configure what "hold" means.

---

## 5. File list

| file | component |
|---|---|
| `F1-swarm-runtime.md` | F1 Swarm runtime |
| `F2-world-model.md` | F2 World model |
| `F3-test-harness.md` | F3 Test harness |
| `F4-ops-telemetry.md` | F4 Ops and telemetry |
| `F5-version-layer.md` | F5 Version layer |
| `F6-bus-deployment.md` | F6 Bus and deployment |
| `B1-movement.md` … `B7-mob-handling.md` | bot skills |
| `C1-builder.md` … `C11-content-packs.md` | swarm capabilities |
| `K1-hq.md` … `K7-program-service.md` | control |
| `INDEX.md` | every architecture decision, plus the interface inputs by component |
| `DECISIONS.md` | both sides of each open non-builder architecture decision |
| `debates/` | write-ups of debates run on open decisions |
