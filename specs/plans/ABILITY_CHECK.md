# Ability check: what the bots can do, proven live

**Plan, 2026-10-17 (Allen).** A milestone between S4 and S5 in [BUILD_PLAN.md](BUILD_PLAN.md). Before any skill is built on top, confirm that the basic abilities exist and act correctly on a real server. Where Azalea is missing one or gets it wrong, patch the fork ([reference/AZALEA_PATCHES.md](../reference/AZALEA_PATCHES.md)) or add it in `mc-adapter`. The duel already found four Azalea bugs by accident; this looks for the rest on purpose.

The starting picture is the Azalea reality check ([reference/AZALEA_REALITY_CHECK.md](../reference/AZALEA_REALITY_CHECK.md) §3): mining, inventory clicks, respawn are covered; placing is partial; item use (eating, shields, bows) and crafting helpers are missing; attackers are not tracked.

## The bar

An ability passes only when the **server** confirms it, not the bot's own view:
- read back through RCON (`data get entity`, `data get block`, `execute if block`, `clear <bot> <item> 0` to count items);
- no anticheat or movement warnings in the server log ("moved too quickly", "moved wrongly", kicks);
- **zero GrimAC alerts.** Null targets vanilla servers, and the fork and adapter must also run cleanly on Paper with an anticheat (F5 §4.6, Allen, 2026-10-17). So a scenario passes only when it passes on both: vanilla `harness/skeleton-server` (the target) and Paper + Grim `harness/grim-server` (the strict check, timing included, zero alerts). A bot never sends a click a vanilla client could not make: it refuses on the client, with a reason, and sends nothing;
- vanilla timing: eating takes 32 ticks, a break takes the vanilla time for the tool. Timings a server can change are beliefs, not facts (Allen, 2026-10-17): the shield's raise-to-block delay is the held item's `blocks_attacks.block_delay_seconds` (0.25 s = 5 ticks by default), and plugins or older Paper settings can change it, so A6 measures the server's delay and checks the bot's belief against it.

Each ability is a scripted scenario: RCON sets the scene (give items, place blocks, set food), one bot does the thing, RCON checks the result. The runner prints one pass or fail line per scenario with the evidence, so a verifier can rerun it. A failure becomes a fix in the fork (raw capability) or the adapter (translation only, F5 §4.1), then a rerun.

## Abilities

| id | ability | needed by | spec | pass when (server truth) |
|---|---|---|---|---|
| A1 | Select a hotbar slot; swap to the offhand | everything | B2 §4.4, B3 §4.1 | the server's `SelectedItemSlot` and offhand match |
| A2 | Break a block with the right tool, at vanilla speed | S5 (fell) | B6 §4.4 | the block is air; time within a tick of vanilla for wood by hand, axe, and stone by pickaxe; no warnings |
| A3 | Pick up drops | S5 | B6 §4.5 | the item count in the bot's inventory rises by the drop |
| A4 | Place a block on a chosen face; see the acknowledgement | S5 (saplings), builder | B6 §4.2, §4.6 | the block is at the target; a placement refused by the server (no support, entity in the way) is reported as refused, not as placed |
| A5 | Eat: hold use until done; stop early cancels | B2, combat healing | B2 §4.1 | food rises by the item's value after 32 ticks; released early, nothing is eaten; movement slowed while eating, no warnings |
| A6 | Shield: raise, block from the front, lower | combat M3 | B5 §4.5 | a hit from the front during block takes no damage; a hit from behind does; slowed while raised (fork patch 3) |
| A7 | Attack with the cooldown; critical hits | combat M3 | B5 | the damage dealt matches a full-cooldown hit and a critical hit |
| A8 | Know who hit us (`recent_attackers`) | combat M3, AD-K2.2 | B5, K2 | after an RCON-scripted hit by a named player, the bot's list names that player within a tick |
| A9 | Move items in the inventory: pick up, place, split, quick-move, drop | B3 | B3 §4.1 | the server's `Inventory` matches the expected slots |
| A10 | Open every storage block (Allen, 2026-10-18): chest, double chest, trapped chest, barrel, shulker box, ender chest, hopper, dispenser, dropper; put and take; read its exact contents | logistics | B3 §4.7, §4.8 | the block's `Items` (the player's `EnderItems` for an ender chest) match; the bot's reading matches; slot rules match vanilla (no shulker box in a shulker box); blocked opens (a block on a chest's top, in front of a shulker's lid) send the click and report that no screen opened; no slot corrected by the server |
| A10E | Open storage on entities: chest minecart, hopper minecart, chest boat (animals' inventories later, with their skill) | logistics, builder (hopper minecarts as chunk loaders) | B3 §4.7, §4.8 | as A10, read from the entity's `Items`; the click aims at the entity's real hitbox and passes the server's and Grim's entity reach and hitbox checks |
| A11 | Craft by clicks: planks and sticks in the 2x2 grid, a pickaxe at a crafting table | S5 onward | B3 §4.2 | the result is in the inventory and the inputs are gone |
| A12 | Die and respawn; know it happened | everything | B2 §4.7 | the bot is alive at spawn after `/kill`, and its view says it died |
| A2S, A4S | Break and place with every non-cube shape family (Allen, 2026-10-17): wire, carpet, rails, plates, snow layers; fences, walls, panes, bars, chains, rods; slabs, stairs, path, chests; torches, lanterns, buttons, ladders, trapdoors, doors, beds, signs; cauldron, hopper, composter, scaffolding, anvil, bell; grindstone on floor, wall and ceiling, lectern, brewing stand, stonecutter (added 2026-10-18); grass and flowers | builder, S5 | B6 §4.1, §4.2, §4.4 | clicks aim at the real outline shape and are accepted wherever vanilla accepts them (e.g. a block on top of redstone wire); the placed state matches vanilla for the look and hit point (stairs facing and half, slab merge, door halves); breaks take vanilla time, instant breaks included; zero Grim alerts |

Later, not in this milestone: bows and crossbows, smelting, sleeping, boats and minecarts, elytra, enchanting, brewing, trading. Each gets a row here when its skill comes up.

## Tasks (Sonnet implements, 500–800 lines each; Haiku or Sonnet verifies live)

1. **Runner, slots, breaking, pickup, placing (A1–A4).** A `null-abilitycheck` binary: connects one bot through `mc-adapter`, talks RCON to `skeleton-server` (port 25577), runs scenarios by name, prints pass or fail with evidence. Adds `break_block`, `place_block(pos, face)` and the inventory view to `null-bot-api` and `mc-adapter`.
2. **Item use and attackers (A5–A8).** Hold and release with the real duration (`using_item` reports the hand and ticks); eating; shield; attack checks; `recent_attackers` from the damage event and the hurt packet's source (likely a fork patch).
3. **Inventory, containers, crafting (A9–A11).** Click actions, `open_container`, container contents in the view, crafting by clicks without the recipe book. A10E follows A10: aiming at an entity's hitbox and the entity interact packets as a vanilla client sends them (Azalea's port of `getEntityHitResult` exists; nothing aims or checks an entity click yet), then the same container rows on minecarts and a chest boat. Menus that do work (furnaces, anvil, enchanting, brewing, trading and the rest) come with their skills; their result slots are computed by the server, so their rows allow corrections there only.
4. **Respawn (A12)**, then the whole suite run live by a separate verifier; failures go back as fixes.

Every new function carries its `// Spec:` link. Every fork change is one commit on `null-main` with a row in AZALEA_PATCHES.md, and `Cargo.toml`'s pinned `rev` moves to it.

## Status

| id | status |
|---|---|
| A1–A4 | done 2026-10-17: `null-abilitycheck` passes on Paper + Grim with zero alerts (fork patches 5, 10; A2 adds `chain_of_30_breaks`) |
| A2S, A4S | done 2026-10-17: breaking, clicking on and placing blocks of every non-cube shape family (`scenarios_shapes.rs`, 166 rows) pass on Paper + Grim with zero alerts; the `FastBreak` cap seen during the work was a bot bug (one tick early, no destroy delay), fixed by fork patch 10. Since the evening every row runs at full length without Haste, except one instamine row |
| A5–A8 | done 2026-10-17 on Paper + Grim, zero alerts (fork patches 7, 8, 9). The servers are on normal since the evening: A5 checks the food rise exactly (12 -> 16, saturation 0 -> 9.6, 32 ticks); A6 damage is exact with natural regeneration off. Shield delay as a belief done 2026-10-18 (fork patch 12, `null_combat::ShieldDelay`): A6 sweeps hit timings on the default shield and on a 1.0 s shield given by RCON, and checks the server's bracketed delay and the bot's belief |
| A9 | done 2026-10-18: `scenarios_inventory.rs`, 46 rows (15 clicks in the player window: pick up, place, split, place one, swap, merge, quick-move both ways and to armour, number-key and offhand swaps, drop one, drop stack, drop outside, double click; 31 custom items for the component hashes), 46/46 on vanilla and on Paper + Grim, zero Grim alerts, no slot corrected by the server. Fork patches 16 (component hashes from vanilla, offline vectors) and 17 (inventory sync and clicks like vanilla, `OpenScreen`, `ClickResults`) |
| A10 | done 2026-10-18: `scenarios_containers.rs`, 26 rows on vanilla and on Paper + Grim, zero Grim alerts, no slot corrected after any click: open and read a single chest (menu kind, slot count, `Items`), take, put, quick-move out and in, double chest (right half is slots 0-26), barrel (`open` property), trapped chest (lamp lit while open), shulker box, shulker box refuses shulker boxes (pickup and quick-move), ender chest (`EnderItems`), hopper (top face, sideways hopper from south and east), dispenser, dropper, exact counts in exact slots, a change by the server while open, out of reach refused, chest, ender chest and shulker lid blocked (click sent, no screen), crouching with an item refused, crouching with empty hands opens, items with components moved through a chest. Fork patch 18. |
| A11 | done 2026-10-18: `scenarios_crafting.rs`, 14 rows on vanilla and on Paper + Grim, zero Grim alerts: planks from a log and sticks (2x2), a crafting table from planks, a wooden pickaxe at a table (3x3, one item per right-click), shift-click crafts all (client crafts once, the server loops: it updates the result, the grid and the destination slot), one-craft shift-click, shift-click at a table, shift-click from the inventory into the table's grid, a wrong shape and an empty result, a cake (3 empty buckets), close with items in the grid (player window and table: they return, nothing drops), a renamed ingredient, two damaged pickaxes repaired (the result's `damage` hash). The server updates only the result slot after a click (it recomputes it after every grid change), except the shift-click multi-craft above; on Paper a click on the result also resyncs the whole window and the offhand (CraftBukkit `CraftItemEvent`), which the rows allow only on Paper. Fork patch 19 |
| A12, A10E | not started |
| whole suite | verified 2026-10-17 evening: 195/195 on vanilla and on Paper + Grim, independent verifier, patch-10 negative control; 245/245 on each with A9 (2026-10-18); open items in [HANDOFF.md](../../HANDOFF.md) |
