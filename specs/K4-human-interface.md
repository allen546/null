# K4 Human interface

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §4.1, §4.4, §10; COMMANDER_INTERFACE.md §5, §6. Conventions: `CONVENTIONS.md`.
**Depends on:** K2, F4 (dashboards), K1 (map data). **Used by:** operators and a human commander.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | CLI | Deterministic | K2's commands one to one |
| 4.2 | Web console | Deterministic | Views of the digest, programs, tickets and audit |
| 4.3 | Notifications | Deterministic | Tickets and alerts pushed with their options |
| 4.4 | Map view | Deterministic | Layers rendered from the coarse map and annotations |
| 4.5 | In-game whisper commands | Deterministic | An authenticated subset of K2 for operator accounts |
| 4.6 | Per-bot debugging | Deterministic | A person-only view and control of single bots |
| 4.7 | Reflex controls | Deterministic | Show and set reflex state through K2 |

## Architecture decisions

None open. Its views follow AD-K2.1.


---

## 1. Purpose

K4 is how a person uses Null: a CLI, a web console, phone notifications, a map, in-game whisper commands, and a debugging view. It's a client of the same K2 API the LLM uses, so a person and the LLM see and do the same things (ARCHITECTURE §4.1).

## 2. Scope

**Owns:** the CLI; the web console; notifications; the map view; whisper commands; per-bot debugging; reflex controls in the UI.

**Does not own:** command semantics (K2); dashboards' data (F4); the map's data (K1).

## 3. Interfaces

All writes go through K2 with the person as principal. Reads use K2's read commands plus F4's dashboards.

## 4. Requirements

### 4.1 CLI

Every K2 command, with the same validation and audit. Scriptable for operator tooling (errands).

### 4.2 Web console

Digest, programs with quotes and critical paths, operations, the ticket inbox with one-tap answers, the audit log, policies, annotations. Readable on a phone (COMMANDER_INTERFACE §5: a notification with three buttons is what a person wants).

### 4.3 Notifications

Tickets needing a person, alerts from F4, milestones, with the options and default shown, answerable from the notification.

### 4.4 Map view

Layers: territory, permit areas, operations and their regions, threats and sightings, annotations, convoys with their moving claims, facilities, deaths. Grid labels match region names in text views (COMMANDER_INTERFACE §6). The same rendering can serve the LLM's optional map tool.

### 4.5 In-game whisper commands

- A small subset (status, halt, recall, come here, follow me) for operator accounts only, authenticated by account identity and a shared secret, never by name alone.
- Whispers from anyone else are quarantined player text (CONVENTIONS §3.7) and never commands.

### 4.6 Per-bot debugging

A person can inspect one bot (state, job, inventory, effective reflexes, recent inputs) and take direct control for debugging. Never exposed to the LLM (§4.1).

### 4.7 Reflex controls

Show the effective reflex state for any scope, and set reflex policy through K2 (`set_policy`), with the echo-before-effect confirmation for swarm-wide survival changes. A visible banner when any scope has survival reflexes off.

## 5. Failure handling

If the console can't reach HQ, it says so and still offers halt and recall through their own path.

## 6. Scale and performance

Views show operations and programs, not bots, except in debugging.

## 7. Version-sensitive parts

None.

## 8. Tests

- Answer a ticket from a phone notification; the audit log records the person.
- A whisper "halt" from a non-operator account is ignored and logged as quarantined text.

## 9. Open questions

None at this stage.
