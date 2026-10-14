# B3 Inventory and crafting

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.5, §3.10, §10; ONLINE_BUILDER.md §2 (`tune`, container contents). Conventions: `CONVENTIONS.md`.
**Depends on:** F1, F2, F5, B1. **Used by:** B2, B4, B6, C1, C3, C4.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Slot management | Deterministic | 36 slots, a hotbar layout per role and a keep-list |
| 4.2 | Crafting | Deterministic | Recipes are game data |
| 4.3 | Smelting | Deterministic | Fuel values and smelt times are game data |
| 4.4 | Enchanting | Deterministic | Table outcomes are random but the policy (reroll, use books) is fixed; anvil order is a known optimisation |
| 4.5 | Brewing | Deterministic | Brewing recipes are game data |
| 4.6 | Shulker boxes | Deterministic | Pack, label, carry, unpack |
| 4.7 | Container transactions | Deterministic | Open, verify against belief, move, verify, close |
| 4.8 | Exact container contents | Deterministic | Fill to a per-slot spec, verify by reading back |
| 4.9 | Villager trading | Deterministic | Offers are visible; buy against a target list |

## Architecture decisions

None open.


---

## 1. Purpose

Inventory slots and walking distance are the real constraints of the swarm's economy, not CPU (ARCHITECTURE §1.5). B3 is everything one bot does with items: keeping the right ones in the right slots, crafting, smelting, enchanting, brewing, packing shulkers, and moving items in and out of containers with verification, including filling containers to the exact per-slot contents that redstone machines depend on.

## 2. Scope

**Owns:** slot layout; crafting (inventory and table, and the crafter block as a manual station); smelting at furnaces, blast furnaces and smokers; enchanting at tables and anvils; brewing; shulker box handling; container transactions; exact-content fills; villager trades.

**Does not own:** deciding what to craft or where items go (C3, K5); automated production machines (C4); the ledger (C3).

## 3. Interfaces

Skills:
- `craft(item, n, station?)`, `smelt(item, n, fuel?)`, `enchant(item, target_enchants, method)`, `brew(potion, n)`
- `take(container, items[])`, `put(container, items[])` → actual moved counts
- `fill_exact(container, slots{index: item×count})` → verified contents
- `pack_shulker(items[], label)`, `unpack_shulker(box)`
- `trade(villager_uuid, offer_index, times)`

Every container skill returns what the container held when opened, which updates F2.

## 4. Requirements

### 4.1 Slot management

- Per-role hotbar layout (tool, weapon, food, blocks, bucket) and a keep-list B2 depends on.
- When full, drop or store the lowest-value non-keep items, per a role-specific priority, rather than refusing pickup of job items.

### 4.2 Crafting

- Recipes from F5 data. Use the recipe book packet path where it's equivalent to a player clicking, otherwise manual slot clicks.
- Multi-step crafts are expanded (planks → sticks → tools) within the job; missing inputs return `blocked(missing{item: n})`.
- Azalea ignores the recipe, recipe-book and merchant-offer packets and doesn't track enchantment offers or furnace, brewing and anvil progress. Crafting is manual slot clicks driven by F5 data, and trading, enchanting, smelting and brewing need packet watchers of ours. The serverbound packets exist in `azalea-protocol` without client helpers (Azalea reality check, 2026-10-13).

### 4.3 Smelting

- Load input and fuel, wait game ticks, collect output. Long smelts are handed to C4's furnace arrays rather than tying up a bot.
- Collect XP from the furnace output (mending repair needs it).

### 4.4 Enchanting

- Table enchanting: reroll strategy fixed per target (cheap level-1 enchants to reroll offers), lapis supply.
- Anvil: combine books and items in the order that minimises cost and stays under the "too expensive" cap.

### 4.5 Brewing

Brewing stand loading with blaze powder fuel, ingredient order per recipe, collection.

### 4.6 Shulker boxes

Pack by item type, label (an item name is ours, not player text, but still stored as our own field), carry, place, unpack, pick up. A carried shulker counts as 27 slots in the ledger.

### 4.7 Container transactions

1. Open (opening has side effects: barrels change `open`, trapped chests emit power; the job must allow it).
2. Compare contents to belief; differences go to F2 and, if they break a reservation, to C3.
3. Move the requested items.
4. Re-read and report what actually moved.
5. Close.

### 4.8 Exact container contents

- `fill_exact` places specific items in specific slots with exact counts, for machines that read signal strength from contents (ONLINE_BUILDER §2: contents go in with the block's own placement, not after).
- Verify by reading back the open container; any mismatch is fixed before closing or returned as a failure.

### 4.9 Villager trading

Read offers (prices are visible after opening), buy against a target list within a spend policy, report trade locks.

## 5. Failure handling

| failure | response |
|---|---|
| Container contents differ from belief | Update F2; if reserved items are gone, `blocked(container_changed)` |
| Station busy or missing | `blocked(station)` |
| Inventory full mid-transaction | Return partial counts |

## 6. Scale and performance

Slot clicks are rate-limited to what a vanilla client produces. Bulk moves use shift-click where vanilla allows.

## 7. Version-sensitive parts

Recipes, smelt times, enchantment tables, brewing, bundles, the crafter block, shelves (1.21.9), via F5.

## 8. Tests

- Craft a diamond pickaxe from logs and diamonds in inventory.
- `fill_exact` a hopper with a specific item count; verify signal strength through a comparator in the harness.
- Chest emptied by the harness between two transactions: `blocked(container_changed)`.

## 9. Open questions

None at this stage.
