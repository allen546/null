# Minecraft Java technical builds: inventory

Research notes, 2026-10-13. Target: vanilla Java 1.21.11 survival servers, no op, no commands, no Carpet, no mods. Three groups: **A** common survival tech, **B** digital (computational) redstone tech, **C** SMP-specialized tech. Each entry gives a name and how it works, plus version notes and a note on what it means for a bot builder or swarm.

## How this was made, and how far to trust it

Three Sonnet agents browsed the web, one per group, and the findings were merged here with one edit (the Orbital Strike Cannon entry in C.1, rewritten to record Allen's description of the latest design). Sources are cited per entry or per section.

- **Tags.** `unverified` means the agent could not confirm it. `(background)` and `[gk]` mean general knowledge, not fetched this session. `[W]`, `[M]`, `[GH]`, `[N]` in section B mark wiki, Mojira, GitHub and news sources.
- **Source gaps.** YouTube (HTTP 429), Fandom / Technical Minecraft Wiki, and Reddit were blocked or rate-limited. Pages were read through a summarising fetcher, so wording is paraphrased and some pages were truncated. Mechanics are mostly from minecraft.wiki, Paper docs, Mojira and GitHub READMEs. Later sections of A (transport, doors, base) are thinner than the mob and farm sections.
- **Open version questions for 1.21.11.** (1) Redstone Experiments: opt-in data pack, assume OFF on default servers; whether a server can enable it after world creation is unconfirmed. (2) TNT duper and gravity-block duper status: the piston TNT duper is listed as working since 1.14 and unpatched in vanilla, Paper/Spigot disable it by default; nothing 1.21.11-specific was found. (3) Pearl chunk loading: since 1.21.2, unchanged through 26.x per sources; pearls vanish when the thrower logs out or dies.
- **Computers in B.** Verified: MASIC, CHUNGUS 2, Deep Thought, The Redgame 1, SethBling's Atari emulator, LordBaryhobal's 16-bit computer, Ryan Boulds' 32-bit build. Other names that were searched for (Ironbark, MCCPU, BigBoy, Sinec8, Nephalem) have no source and are marked unverified. MCHPRS lists only 1.20.4 and Redstone Tweaks only 1.16.x, so neither is a ready 1.21.11 tool.
- **Orbital Strike Cannon.** Allen's account of the latest generation (256 breezes in a 1x2 area, their wind charges deliver a TNT-duper-built payload) is recorded in C.1 as the working description. No web source the agents could read covers it; the documented designs are cubicmetre's Mk.x line, newest Mk.6.1 (1.21.11 schematic).

## Contents

**A** (65 entries)

- **A. Common survival tech**

- **A.1 Hostile mob farms (generic mechanics)**
  - Generic natural-spawn mob farm (spawning platforms/tower + water flush + fall/lava killer)
  - Mob grinder / killing methods
  - Monster spawner (dungeon) farms: zombie / skeleton / spider / cave spider / blaze
  - Skeleton farm (natural or spawner)
  - Creeper farm (gunpowder)
  - Spider / cave spider (string) farm
  - Enderman farm (XP/pearls)
  - Witch farm
  - Drowned farm (tridents, nautilus shells, copper, zombie loot)
  - Guardian farm / elder guardian farm
  - Blaze farm (spawner)
  - Wither skeleton farm
  - Zombified piglin gold farm
  - Piglin bartering (barter farm)
  - Slime farm
  - Hoglin / ghast / magma farms (secondary Nether)
  - Raid farm / pillager outpost farm
  - Iron golem farm
  - Breeze / trial chamber farm
  - Creaking heart / resin farm (pale garden)
  - Phantom suppression (non-farm tech)

- **A.2 Villager tech**
  - Villager breeder
  - Villager trading hall
  - Zombie villager curing (discounts)

- **A.3 Boss/XP farms**
  - Wither and ender-dragon "farms"
  - XP farm overview (which methods matter)
  - Sculk (catalyst) XP farm

- **A.4 Crop, tree and animal farms**
  - Wheat / carrot / potato / beetroot farm
  - Sugar cane farm
  - Bamboo farm
  - Kelp farm
  - Melon / pumpkin farm
  - Cactus farm
  - Tree farm (bone meal/TNT, birch/oak, mega spruce/jungle, mangrove, huge fungus)
  - Honey / bee farm
  - Sheep / wool farm
  - Chicken / egg farm
  - Cow / pig / animal farms
  - Fish farm / AFK fishing
  - Turtle scute farm / armadillo scute

- **A.5 Mining and block farms**
  - Cobblestone / stone generator
  - Basalt generator
  - Obsidian farm
  - Lava farm (dripstone + cauldron)
  - Dripstone farm
  - Amethyst farm
  - Ore generation
  - Gravity-block (sand/gravel) farming and dupers
  - TNT duper
  - Concrete powder / sand-to-concrete, ice, mud, clay

- **A.6 Item infrastructure**
  - Item sorter (hopper-comparator) and overflow-proof sorter
  - Bulk storage / shulker box loader & unloader
  - Copper golem sorting (new in 1.21.9)
  - Item elevators (bubble column, dropper, hopper, piston, allay)
  - Item transport via water streams, ice, minecarts
  - Hopper clocks / redstone clocks / copper bulb
  - Auto-crafter (Crafter block)
  - Automatic smelter / furnace array

- **A.7 Transport**
  - Nether highway / portal linking & stacking
  - Ice boat road
  - Minecart / rail networks
  - Ender pearl stasis chamber
  - Flying machines (slime/honey)

- **A.8 Doors, bases, defense**
  - Piston doors
  - Hidden base and mob-proofing

- **A.9 What changed in 1.21.x (selected, wiki-sourced)**

- **A.10 Version-dependent flags**

- **Sources used**

**B** (57 entries)

- **B. Digital (computational) redstone tech**

- **B.1 Version-sensitive core: update order, QC, Redstone Experiments, 1.21.x**
  - Redstone Experiments (24w33a/24w34a, "1.21.2" experimental data pack)
  - Classic (vanilla default) redstone-dust update order
  - Quasi-connectivity (QC) - Java only, unchanged by default through 1.21.11
  - 1.21.x component changes affecting computational redstone (summary)

- **B.2 Logic gates and basic building blocks**
  - NOT / inverter (torch, comparator-subtract, piston instant)
  - OR / NOR
  - AND / NAND
  - XOR / XNOR / IMPLY
  - Repeater diode, locking, isolation
  - Comparator subtract / compare logic
  - Observer-based and piston-based "instant" gates

- **B.3 Pulse, edge, timing, 0-tick**
  - Edge detectors (rising / falling / dual)
  - Pulse limiters / pulse extenders / monostables
  - Block Update Detectors (BUD) and observers
  - 0-tick pulses and 0-tick tech (Java only)
  - Tick delay, torch burnout, repeater locking as timing primitives

- **B.4 Latches and memory**
  - RS NOR latch (and variants)
  - T flip-flop (toggle)
  - Copper bulb T flip-flop / 1-bit toggle memory (1.21)
  - D latch / D flip-flop (repeater-locking, "modern")
  - JK latch/flip-flop
  - Comparator memory cells (analog latch and analog counter)
  - Item-based memory: hopper/dropper/barrel/chest storage as state
  - Chiseled bookshelf as memory cell (1.20+)
  - Lectern / book page as 4-bit register and pulse source
  - Decorated pot and barrel as comparator stores; shelf as 3-bit register (1.21.9)

- **B.5 Clocks**
  - Torch clocks / rapid pulsar / 20gt torch loops
  - Repeater clocks (4-gt loop, 2-gt Java loop, torch-repeater hybrids)
  - Comparator clocks
  - Observer clocks
  - Hopper clocks (Etho clock EHC, multi-item, MHDC)
  - Piston clocks (Java only)
  - Long-period clocks: despawn, minecart, geyser, clock multiplier, LFSR, daylight

- **B.6 Signal strength, ADC/DAC, decoders**
  - Signal-strength decoders (analog to one-hot / binary)
  - Analog-to-digital and digital-to-analog (container ADC/DAC)
  - Sculk sensor / calibrated sculk sensor as wireless decoder (1.19/1.20)
  - Item sorters as logic (hopper filter, overflow gates)

- **B.7 Counters, arithmetic, registers**
  - Binary counters
  - Adders: half, full, ripple-carry, carry-lookahead
  - ALU (arithmetic logic unit)
  - Registers, accumulators, program counters
  - Instruction decoders

- **B.8 RAM, ROM, storage-as-memory**
  - Redstone-cell RAM (locking-repeater / D-flip-flop arrays)
  - Container-based RAM/ROM (barrel, hopper, chest, dropper-lined) - scheme level
  - Redstone ROM (diode/repeater/torch matrices, piston/dropper-lined)
  - Crafter circuits (1.21)

- **B.9 CPUs and famous computers (verify list)**
  - Verified named computers

- **B.10 Displays**
  - 7-segment, lamp/pixel and piston displays

- **B.11 Flying machines, tileable and compact designs**
  - Flying machines (slime/honey engines, observer engines) and pulse transport
  - Tileable / stackable / compact designs

- **B.12 Signal transmission and multiplexing**
  - Signal transmission: dust, repeater chains, observer chains, rails, sculk/amethyst, ladders
  - Redstone "compression", multiplexing, buses

- **B.13 1.21 trigger/sensor blocks used in logic**
  - Wind charge, breeze, trial vault/spawner as triggers (1.20.3+/1.21)
  - Creaking heart, lightning rod, copper golem statue, target block, daylight detector (analog sensors)

- **B.14 Simulation and design tooling**
  - MCHPRS (Minecraft High Performance Redstone Server) + Redpiler
  - Redstone Tweaks and Alternate Current (mods; not for the vanilla server)
  - Logisim / Yosys / PERSHING / dewey (netlist -> redstone place-and-route)

- **B.15 Not-found / gap list (for the planner)**

**C** (38 entries)

- **C. SMP-specialized tech**

- **C.1 Orbital Strike Cannon (Mk.x documented, latest breeze-driven generation)**
  - Orbital Strike Cannon (OSC), cubicmetre designs, Mk.1 to Mk.6.1
  - Latest OSC design (Orbital Skybender): breeze-driven delivery, as described by Allen

- **C.2 TNT: cannons, dupers, wall breaking**
  - TNT cannon fundamentals (single, double, multi, sand, "tunneler", etc.)
  - Sand / falling-block cannons
  - Ender pearl cannons / player cannons (360 FTL, wind-charge launchers)
  - TNT duplicators ("TNT dupers"): piston-based, observer/slime variants
  - Wall-breaking: obsidian breaching, "tunneler" cannons, wither wall breakers

- **C.3 Wind charge, breeze, mace, spear tech (1.21+)**
  - Wind charge basics and block triggering
  - Wind-charge elytra/launch mobility, wind-charge stasis
  - Mace smash combat, Wind Burst enchantment
  - Spears (new in 1.21.11) and lunge
  - Breeze mob facts and breeze rods
  - Trial chamber / trial spawner farms

- **C.4 Ender pearl tech**
  - Ender pearl stasis chamber (trapdoor + bubble column)
  - Ender pearl chunk loader (personal)
  - Pearl-trapping / pearl-kill defence

- **C.5 Base defence, traps, alarms**
  - Beds and respawn anchors as bombs / traps
  - End crystal PvP and base breaching
  - Totems of Undying and auto-armour
  - Obsidian, crying obsidian, bedrock-level and hidden bases
  - Water/lava moats, trapdoors on liquid, cobweb, mob-based defence
  - Redstone / sculk / tripwire alarms
  - Claim plugins: GriefPrevention, WorldGuard, FTB Chunks
  - Anti-grief and chunkban awareness

- **C.6 Chunk loaders**
  - Nether portal chunk loader
  - Minecart-based portal chunk loader (1.21.2+)
  - End portal and end gateway loaders; /forceload

- **C.7 Travel**
  - Elytra + firework travel and nether-roof highways
  - Boat / ice highways
  - Nether portal networks / highways

- **C.8 Storage, economy, automation**
  - Mass storage / stash and auto sorting
  - Auto-farms and trial/breeze economy
  - Item duplication glitches (summary, per version)

- **C.9 Lag, chunkbans, anti-cheat**
  - Lag machines and server-lag tech (defensive awareness)
  - Anti-cheat constraints (Grim, Vulcan, etc.)

- **C.10 Tunnelling and access**
  - Tunnelling: cave-borer, TBM, liquid ceilings
  - Nether roof / bedrock-floor access
  - Wither skeleton / creeper / guardian mob defences

---

## A. Common survival tech

Scope: technical-survival builds in Java Edition, target = vanilla Java 1.21.11 ("Mounts of Mayhem", released 2025-12-09), no ops/commands/Carpet/mods.

Research caveats (read first):
- Primary source = Minecraft Wiki (minecraft.wiki) tutorial/mechanic pages, fetched in this session (URLs listed per section; base `https://minecraft.wiki/w/`). The wiki currently already contains 26.x ("Java 26.1/26.2/26.3/26.4") content; anything the wiki marks 26.x is NOT in 1.21.11 and is flagged as such.
- Fetch tool summarised pages with a small model; where a page was truncated or the summary did not cover a point, I say so. Items labelled **(background)** are from my own prior knowledge, not verified in this session. Items labelled **unverified** are uncertain/conflicting.
- Could not read: YouTube (HTTP 429 on https://www.youtube.com/watch?v=E9U8bMNVl6k, a "Minecraft TNT Duper 1.21.11" video found by search, content unread), r/technicalminecraft (only a mirror listing titles was readable), Technical Minecraft Wiki/Fandom (search proxy rejected the query), several wiki Tutorials pages (Villager_breeding, Mob_farming, Skeleton_farming, Spawner_farming, Resin_farming, Mob-proofing, Piglin_Bartering returned fetch errors).
- Wiki-sourced rule of thumb: most entries are "player must stay within simulation/spawn range" farms. Java natural spawning happens only within 128 blocks of a player (wiki Mob_spawning); hostile mobs >128 blocks from the nearest player are removed immediately (wiki Mob_spawning); the extra "random despawn when >32 blocks away" rule is **(background)**. Mob caps in Java: 70 monster / 10 creature base values, per-player/global, passive-mob spawn cycle only every 400 ticks (wiki Mob_spawning).
- Light rules (wiki Light): Overworld hostile mobs need internal sky light <= 7 and block light 0; passive mobs in unexposed areas need block light >= 9. So any lit block (torch) stops hostile spawns in its radius; only fully dark (block light 0) areas spawn mobs.

---

## A.1 Hostile mob farms (generic mechanics)

### Generic natural-spawn mob farm (spawning platforms/tower + water flush + fall/lava killer)
**How it works:** A large dark area (all other surfaces within ~128 blocks lit/slabbed/covered) is converted into spawn pads (slabs/solid blocks) so the monster mob cap fills up on the pads. Mobs are moved (water flush, fall trap, trapdoors/sinkhole, piston shifting floors) to a central shaft and killed by fall damage (most hostiles die from ~24 blocks, witches ~30, endermen ~43), lava blade, suffocation, campfire, magma etc. The player AFKs ~>24 blocks away at a height/position that maximises spawnable area (typically in the sky or at a calculated height). Designs: spawning tower (stacked pads flushed periodically), sinkhole (cheap/low yield), canal/water-flush, sunlight farm, villager/turtle-egg lure, slime-pushed. Items collected via hoppers/hopper minecart. No XP from automated kills (XP only if player or tamed wolf kills). Output order of magnitude: hundreds to a few thousand items/hour for big sky/ocean designs; underground/surface far less and time-dependent.
**Survival requirements:** Overworld. Cobblestone/slabs/glass, water buckets, hoppers, droppers/observers or redstone clock, lots of lighting work. Sky farms need ~y=200+ and huge material counts.
**Version notes:** 1.18+ light rules (see header). Since 1.19.3 mobs no longer spawn on scaffolding (use top slabs) (wiki Mob_farm). Mobs in boats/minecarts don't despawn and are removed from mob cap since 1.16 (wiki Creeper_farming). 1.21.5: random ticks now run in chunks loaded by ender pearls etc., but mob spawning/fire/lightning still only within 8 chunks of players (wiki Java_Edition_1.21.5).
**Bot-relevance:** Needs a bot parked AFK in range (>~24 blocks, <128 blocks) in loaded chunks for hours; wide-area lighting/slab work and exact flush timing; hard to build precisely.
Sources: https://minecraft.wiki/w/Mob_farm, https://minecraft.wiki/w/Tutorials/Mob_grinder, https://minecraft.wiki/w/Mob_spawning, https://minecraft.wiki/w/Light

### Mob grinder / killing methods
**How it works:** Killers: fall damage; magma blocks (2 HP/s, ineffective vs Nether mobs, witches, spiders); wither roses (1 HP/s + Wither, items sink through soul sand); lava blade (kills fast but lava destroys item drops, baby zombies slip through); campfire (cheap, not witches); piston suffocation; entity cramming (Java; >24 mobs/block); cactus choke (spiders); sweet berries, silverfish grinder, anvil smasher. Trident killer is Bedrock-only. Item collection by hoppers at end of water flows or hopper minecarts. Key rule: XP only drops when a player or tamed wolf lands the kill; Looting only applies to player/wolf kills, so a "player-kill chamber" (drop to 1 HP, hit once) is the standard design when XP/Looting wanted.
**Survival requirements:** Overworld/Nether materials; Looting III sword for gunpowder/etc. rates.
**Version notes:** Java entity cramming default 24 (maxEntityCramming gamerule since 1.11). Pre-1.21.5 aggroed zombified piglins dropped XP when killed by automation (patched 1.21.5).
**Bot-relevance:** Player-kill chamber requires bot to attack mobs repeatedly (combat loop) while AFK-chunk loaded; automatic grinders are drop-only.
Sources: https://minecraft.wiki/w/Tutorials/Mob_grinder, https://minecraft.wiki/w/Mob_farm

### Monster spawner (dungeon) farms: zombie / skeleton / spider / cave spider / blaze
**How it works:** A found spawner activates when a non-spectator player is within 16 blocks of its centre; each cycle makes 4 spawn attempts in a 9x3x9 volume; delay between successful cycles 200-799 ticks; if >= 6 mobs of its type are within a 9x9x9 area it "poofs" (no spawn). Light needs follow the mob (zombie/skeleton/spider need dark; a torch next to the spawner stops dark-mob spawns across the volume; blaze/silverfish blocked by light 12; magma cubes ignore light). Because it ignores the natural mob cap, farms use a room/trench funnelling mobs to a kill chamber or drop. Skeleton-spawner XP farms run AFK because tamed wolves attack skeletons without being provoked (about 100 XP levels over 8 hours AFK, plus bows/armor). Zombie spawner can be flooded to convert zombies to drowned. Spawners cannot be obtained in survival (even Silk Touch); mining one with a pickaxe drops 15-43 XP.
**Survival requirements:** Locate a dungeon/mineshaft/fortress spawner; torches to disable until built; pickaxe; wolves + bones for the skeleton AFK design; Fire Resistance helpful for blazes.
**Version notes:** 1.21.2 only added unique sounds to spawners (wiki Monster_Spawner). Spawn-egg and custom spawners are command-only (n/a).
**Bot-relevance:** Needs AFK within 16 blocks (so very close to mobs) and a safe room; wolf management; exact spawn-volume geometry matters.
Sources: https://minecraft.wiki/w/Monster_Spawner, https://minecraft.wiki/w/Tutorials/Experience_farming

### Skeleton farm (natural or spawner)
**How it works:** See generic farm and spawner entries. Skeletons need dark platforms; the classic variant uses a monster-room spawner with wolves for AFK XP and loot (bows, armor, bones, arrows). Music discs drop only when a skeleton/stray kills a creeper (wiki Mob_farm). Wolves cause skeletons/wither skeletons to flee (cat/wolf redirection used in creeper/skeleton farms).
**Survival requirements:** Overworld; spawner dungeon or dark platforms; tamed wolves + food.
**Version notes:** 1.21.11 adds "parched" (desert skeleton variant, light level 0, shoots Weakness arrows) replacing some skeletons in deserts; and bogged (swamp/trial) from 1.21. 
**Bot-relevance:** Needs player nearby; ranged enemies make manual combat risky, so prefer drop-only.
Sources: https://minecraft.wiki/w/Tutorials/Experience_farming, https://minecraft.wiki/w/Java_Edition_1.21.11

### Creeper farm (gunpowder)
**How it works:** Dark platforms with a ceiling lowered to 1 13/16 blocks (trapdoors on 2-high ceiling) are tall enough for creepers but too short for zombies/skeletons (spiders need 3x3 clearing). Creepers are funnelled and killed by campfires or by a Looting III sword; cats (in boats) steer creepers toward the kill chamber. Mobs stored in minecarts/boats don't despawn and don't count towards the mob cap (Java 1.16+). Cited yield: Chapman hourglass design ~700 gunpowder/hour. Charged creepers (lightning/channeling) give mob heads: wither skeleton skull 100% from a charged creeper kill.
**Survival requirements:** Overworld; slabs/trapdoors; cats; Looting III sword for rates.
**Version notes:** Trapdoor-height trick not on Bedrock. Wiki says charged-creeper methods are "mostly obsolete" - unverified for 1.21.11.
**Bot-relevance:** Gunpowder is easy to get from normal combat/night; farm is a luxury; needs AFK range and cat handling.
Sources: https://minecraft.wiki/w/Tutorials/Creeper_farming

### Spider / cave spider (string) farm
**How it works:** Spiders climb walls, so farms need overhangs/flat funnels; cave spiders fit through 0.5-block gaps; mineshaft cave spider spawners are the usual source, ideal for string. Pistons/lava/cactus/water kill them (cactus choke works on spiders; magma doesn't). Armadillos scare spiders within 6 blocks (relevant to savanna/badlands farms).
**Survival requirements:** Mineshaft spawner (cave spider) or dark platform; string is also needed for fishing rods, bows, wool.
**Version notes:** **String dupers (tripwire) were removed in 1.21.2**, so natural farms/fishing/cats are the only automatic string sources (wiki Wool_farming).
**Bot-relevance:** String can be mined from cobwebs / killed from spiders; farm only useful for large-scale (wool, bundles).
Sources: https://minecraft.wiki/w/Mob_farm, https://minecraft.wiki/w/Tutorials/Wool_farming, https://minecraft.wiki/w/Armadillo

### Enderman farm (XP/pearls)
**How it works:** Endermen spawn at much higher rate in the End (and warped forests) than the Overworld. Farm is built on a lit/slabbed main island or at an outer island at least 128 blocks from other spawnable area, with an endermite-lure or bridge; water herds endermen onto a single bridge block where they are damaged (endermen teleport away from water) and cramming/sweep attacks kill them. A fully built farm reaches level 30 in under a minute; the wiki says it must be built at y=1 in the End (high void risk) for the fastest design. Ender pearls from the farm supply Eyes of Ender and stasis chambers. Endermen can pick up/break farm blocks (mobGriefing). Endermen are aggroed by eye contact (background).
**Survival requirements:** Dragon beaten or at least an End portal + gateways reached; lots of obsidian/slabs, water, hoppers.
**Version notes:** DashPum4 design is Java 1.19+ only; gnembon's design has an updated 1.14+ video. No 1.21.11-specific change found.
**Bot-relevance:** High-risk build (void); needs player/bot AFK in the End (loaded chunks); endermen break the build; look-at aggro precautions.
Sources: https://minecraft.wiki/w/Tutorials/Enderman_farming, https://minecraft.wiki/w/Tutorials/Experience_farming, https://minecraft.wiki/w/Mob_farm

### Witch farm
**How it works:** Witches spawn naturally only in swamp huts (sky light <= 7 required on platforms), during raids, or when lightning hits a villager. A farm builds up to 3 spawn platforms per hut (up to 4 huts active within 128 blocks) and flushes or drops witches into a killer (witches have 26 HP and need ~30-block fall). Drops: redstone (guaranteed from 1.21), glowstone, sugar, gunpowder, glass bottles, spider eyes, sticks (sticks ~2x as often).
**Survival requirements:** Swamp hut; slabs, water, hoppers; high AFK platform; Looting III helps.
**Version notes:** Redstone drop guaranteed from 1.21 (wiki Witch_farming). Spawn rate very limited by hut count.
**Bot-relevance:** Low priority; only for redstone/glowstone luxury; needs player nearby.
Sources: https://minecraft.wiki/w/Tutorials/Witch_farming

### Drowned farm (tridents, nautilus shells, copper, zombie loot)
**How it works:** Three approaches: (1) flood a monster-room zombie spawner so zombies convert to drowned in a water trap channel (loot: rotten flesh, iron/gold/chainmail gear, copper ingots; no tridents); (2) aerial platform >= 24 blocks above kill area with a thin water layer for natural drowned spawning (tridents, nautilus shells, copper; slow because of mob cap, usually baited with turtle egg or villager); (3) underwater ocean farm with villager bait and bubble-column funnels (complex/risky). Drowned also are used for trident (Channeling for charged creeper/witch) strategies.
**Survival requirements:** Ocean/river or monster room; water buckets, glass/cobble, hoppers; Looting for trident chance.
**Version notes:** Wiki says no tridents/no nautilus shells from monster-room drowned on Java. **1.21.11 adds nautilus and zombie nautilus** (zombie nautilus spawns with a drowned rider holding a trident), so new drowned+trident sources exist in oceans (wiki Java_Edition_1.21.11). Treat nautilus-shell dropping rules as unverified.
**Bot-relevance:** Trident farming is hard to get right; for bots, hunting drowned in oceans is simpler than farm.
Sources: https://minecraft.wiki/w/Tutorials/Drowned_farming, https://minecraft.wiki/w/Java_Edition_1.21.11

### Guardian farm / elder guardian farm
**How it works:** Guardians spawn only in ocean monuments (deep ocean). Farms either drain the monument (sponges/TNT/lava/Frost Walker) or use no-drain designs; flooded spawn area, a transport method (guardians fit through 1-block gaps), killer (fall of ~34 blocks because of 30 HP and thorns, lava drop, conduit, suffocation, iron golems that wear out) and item collection. Output: prismarine shards/crystals, cod, and the fastest XP of any farm (up to ~180,000 XP/hour in optimised builds; level 30 in ~30 s). Elder guardians (3, fixed) drop wet sponges; they cannot be farmed in unlimited numbers without special designs (unverified details).
**Survival requirements:** Deep ocean with monument; Water Breathing/Conduit/Respiration, large amounts of cobble, hoppers; Mining Fatigue cure (milk) for elder guardians.
**Version notes:** None specific to 1.21.x found.
**Bot-relevance:** Very complex underwater building; needs AFK loaded chunk; guardian lasers/thorns hazardous.
Sources: https://minecraft.wiki/w/Tutorials/Guardian_farming, https://minecraft.wiki/w/Tutorials/Experience_farming

### Blaze farm (spawner)
**How it works:** Nether fortress blaze spawner: enclosed spawn cage, funnel (pistons/pressure plates or lava flow in 1.16+ Java) to a small kill chamber; blazes spawn at light level <= 11 (so keep it dark); suffocation brings them to 1 HP for a one-hit kill. Blaze rods are the only blaze powder source (needed for brewing and Eyes of Ender) and are strong fuel; blazes drop double XP. Open-fortress farms are harder because other mobs spawn too. Design variants include wolf-based automatic killing (ImpulseSV's broke in 1.8), Cheesedud6 1.16+ lava funnel (Java only).
**Survival requirements:** Nether; fire resistance potion/effect; blocks (cobble), hoppers/pistons, water bucket won't work in Nether (use lava/ice or piston designs).
**Version notes:** Wiki page truncated; unverified for 1.21.11 beyond general 1.16+ Java notes.
**Bot-relevance:** Needed to beat the game (blaze rods); manual spawner fighting by bots is feasible; farm makes it AFK but needs player within 16 blocks.
Sources: https://minecraft.wiki/w/Tutorials/Blaze_farming, https://minecraft.wiki/w/Monster_Spawner

### Wither skeleton farm
**How it works:** A spawning pad built inside a Nether fortress (wither skeletons spawn only in fortresses, light 0-7, in groups; within a single fortress-piece bounding box on any block, in the whole fortress bounding box only on nether bricks; spawn weight 8/28). All other nether-brick surfaces within the farm's spawn range (~250x250, unverified) are removed/covered so the cap fills with wither skeletons; they're funnelled by water/pistons to a kill chamber. Drops: coal, bones, skull 2.5% (+1% per Looting level, 5.5% at Looting III) only for player/tamed-wolf kills; a charged creeper kill drops exactly 1 skull. Wither roses can be placed as kill blocks (wither skeletons are immune). 1.16+: piglins attack wither skeletons.
**Survival requirements:** Nether fortress; Looting III sword; many blocks; Fire resistance.
**Version notes:** Wiki tutorial describes 1.16 only; Java spawn rules above per wiki Wither_Skeleton. No 1.21.11-specific change found - **version-dependent**, treat bounding-box rules as 1.16+ Java.
**Bot-relevance:** Wither skulls needed for the Wither (optional boss); manual hunting in fortress is simpler than farm; farm needs AFK near.
Sources: https://minecraft.wiki/w/Tutorials/Wither_skeleton_farming, https://minecraft.wiki/w/Wither_Skeleton, https://minecraft.wiki/w/Tutorials/Wither_farming

### Zombified piglin gold farm
**How it works:** Zombified piglins spawn naturally in Nether wastes, crimson forests and fortresses, and from Nether-portal blocks (Overworld portals spawn them too, twice as often on Normal and three times on Hard vs Easy, on the frame's bottom blocks). Java natural designs (ianxofour, Ilmango's magma platform (patched), gnembon, Rays Works, DashPum4 ~1,600 gold blocks/hour with Looting III) use magma/slabs/cramming and a kill chamber. Overworld portal farms use large/interlocking obsidian portals with trapdoors; Java Edition's portals can also be activated by entities in 1.21 per the wiki (unverified). Piglin kills yield gold nuggets/swords (smelt to nuggets) + XP; level 100 in ~1 hour for good designs. Killing aggro'd zombified piglins no longer drops XP always (1.21.5).
**Survival requirements:** Nether (or obsidian portal in Overworld) 10+ obsidian; diamond pickaxe; Looting III sword; lots of blocks.
**Version notes:** **Version-dependent**: 1.21.5 patched "zombified piglins always drop XP when angered" (wiki says some designs' XP changed). Ilmango design patched earlier. Wiki says Java zombified piglin portal spawning continues in 1.21.x.
**Bot-relevance:** Gold is needed for bartering/piglin diplomacy; farm needs AFK/player proximity; hard to build.
Sources: https://minecraft.wiki/w/Tutorials/Gold_farming, https://minecraft.wiki/w/Nether_portal

### Piglin bartering (barter farm)
**How it works:** Adult piglins accept gold ingots, examine them for ~6 s (Java; 8 s Bedrock), then drop a random item; they won't barter while aggravated; baby piglins take gold and give nothing. Loot table (background, unverified here): ender pearls (2-4), obsidian, crying obsidian, fire resistance potions, soul speed books, iron nuggets, gravel, string, quartz, leather, etc. Barter farms feed gold ingots into piglin cells with hoppers/droppers and collect drops; linking a gold farm to a bartering farm yields obsidian/pearls. Piglin cells must avoid zombification (Overworld piglins turn zombified after 15 s).
**Survival requirements:** Nether, gold, golden armor to avoid aggro (or bribe-only), cell walls.
**Version notes:** In Java the barter pool can be changed by data pack (n/a vanilla). 1.21.11 piglins can hold golden spears (attack preference) per wiki.
**Bot-relevance:** Practical way for a bot to get ender pearls (needed for eyes of ender) and obsidian; stochastic - plan ~ many ingots.
Sources: https://minecraft.wiki/w/Piglin, https://minecraft.wiki/w/Tutorials/Gold_farming

### Slime farm
**How it works:** Slime chunks (~10% of chunks, seed-dependent in Java) spawn slimes below y=40; swamps spawn slimes y=51-69 at light <= 7 (lower rate, no digging). Spawn pads of slabs with 2.5-block gap; iron golems or other methods lure/hold slimes; magma blocks, wither roses, or soul campfires kill without losing drops (cactus loses ~20% slimeballs). Slimeballs for sticky pistons/slime blocks/magma cream (Fire Resistance). Splash Potion of Oozing (1.21+) on mobs: each oozing mob dying releases two medium slimes.
**Survival requirements:** Overworld; slabs; slime-chunk finder (external) or swamp; AFK within 128 blocks.
**Version notes:** 1.21.11 adds `surface_slime_spawn_chance` environment attribute (wiki Java_Edition_1.21.11) - unverified for rates; moon-phase dependency is **(background)**, not in fetched page.
**Bot-relevance:** Needed for pistons/leads; farm or hunting slimes in swamps at night; AFK required.
Sources: https://minecraft.wiki/w/Tutorials/Slime_farming, https://minecraft.wiki/w/Java_Edition_1.21.11

### Hoglin / ghast / magma farms (secondary Nether)
**How it works:** Hoglins breed in crimson forest; warped fungus repels them; kill with lava/campfire; drops pork/leather. Ghast tears: Ghasts need open Nether space; "Tears" disc only from player-deflected fireball kills (1.21.6). Magma cubes are immune to fall/fire, spawn in basalt deltas/bastion spawners.
**Survival requirements:** Nether.
**Version notes:** Little to report; all unverified for farm rates.
**Bot-relevance:** Low priority.
Sources: https://minecraft.wiki/w/Tutorials/Animal_farming, https://minecraft.wiki/w/Mob_farm, https://minecraft.wiki/w/Java_Edition_1.21.6

### Raid farm / pillager outpost farm
**How it works:** Pillager farm: spawn platform with villager + composter bait at an outpost (outposts spawn only pillagers; spawn box centered at highest floor), slab "guard rails" keep pillagers from shooting villagers, two tall bubble elevators lift them, fall leaves 1 HP, they go to a kill chamber. Ominous bottle used at the chamber starts a raid that pulls pillagers in, freeing mob cap. Output: XP (up to ~150% of an enderman farm), ominous banners (raid captains drop them), crossbows; real raid farms use captain's Ominous Bottle (1.21: raids start by Raid Omen, Bad Omen from bottles) for totems/emeralds in Java: totem of undying from evokers, emeralds from villager gifts after victory (wiki Raid). Raids: 3/5/7 waves on Easy/Normal/Hard (+1 at Omen II+), expire after 48,000 ticks (~40 min).
**Survival requirements:** Pillager outpost destroyed/cleared; villagers; slabs/bubble columns; Looting/axe; Bad Omen from captain.
**Version notes:** **Version-dependent**: raids triggered by Raid Omen since 1.21 (Bad Omen -> Raid Omen on entering village); 1.21.2: raid doesn't start if raiders can't find a spawn point, must spawn within 96 blocks vertically; `can_start_raid` attribute (1.21.11).
**Bot-relevance:** Complex; skip unless totems desired; "totem" is optional for beating game.
Sources: https://minecraft.wiki/w/Tutorials/Pillager_farming, https://minecraft.wiki/w/Raid

### Iron golem farm
**How it works:** Villagers summon golems when gossiping (>= 5 villagers within 10 blocks, at least one gossiping) or every 5 s while panicking (>= 3 panicking, often from zombies or pillagers within scare range 6/15). Required: villager slept within last 20 minutes, no golem seen within 16 blocks in last 30 s, no recent summon within 10 blocks; spawn spot found in 17x13x17 box (<= 10 attempts) needing solid block with non-solid blocks above (water above prevents? wiki says air or water allowed; buttons/pressure plates block). Golems are removed instantly (lava blade/ chute) to keep conditions met; farm runs as long as chunks loaded (no player needed). Theoretical max ~404 ingots/h (Bedrock) typical 240-400; Java rates not documented (unverified). Drops 3-5 iron ingots and 0-2 poppies. Nether portals / chutes reset a villager's golem-seen cap.
**Survival requirements:** 3-5+ villagers in beds/pods (breeder), zombie or other panic source (zombie 6-block scare), job/bed, lava or water transport, hoppers.
**Version notes:** **Version-dependent / Java vs Bedrock differ**: Java uses villager-summon only (no village-wide natural spawn); Bedrock has village spawn with 20 beds + 10 villagers. Golem spawn rules 17x13x17 in Java; wiki notes Java golems spawn even if spawn_mobs rule false. Wiki lacks 1.21 changes for iron farms.
**Bot-relevance:** Iron source independent of mining; needs villagers + zombie handling; AFK loaded chunk; precise pod layout.
Sources: https://minecraft.wiki/w/Iron_Golem, https://minecraft.wiki/w/Tutorials/Iron_farming, https://minecraft.wiki/w/Tutorials/Iron_golem_farming

### Breeze / trial chamber farm
**How it works:** Breezes (30 HP, wind charge attacks within 16 blocks, 1.6 s cooldown, leaps up to 15 blocks, fall-damage immune, deflect arrows/tridents but not wind charges) spawn only from trial spawners in trial chamber combat rooms. Trial spawners activate when a survival/adventure player is within 14 blocks with line of sight; then no player needed; spawn attempt every 2 s, 4-block radius with line of sight, 6 mobs total, 2 alive at once (1 player), +1/+2 per extra player; mobs don't despawn; loot ejects only when all mobs dead; cooldown 30 min. Farms manipulate spawn locations/LOS and kill with magma or campfire; Rays Works design ~32 breeze rods/hour. Ominous trial spawners via Trial Omen/Bad Omen. Breeze rods -> wind charges, mace (with heavy core from ominous vault 7.5%).
**Survival requirements:** Trial chamber (y negative, Overworld); Armor, shield, slabs; Bad Omen for ominous.
**Version notes:** Added 1.21; wiki states 1.21 behaviours; nothing found about changes in 1.21.2-1.21.11 beyond Mace (unverified).
**Bot-relevance:** Optional; trial chambers are combat-heavy; breeze rods only needed for wind charge/mace utility.
Sources: https://minecraft.wiki/w/Tutorials/Breeze_farming, https://minecraft.wiki/w/Trial_Spawner, https://minecraft.wiki/w/Breeze, https://minecraft.wiki/w/Mace

### Creaking heart / resin farm (pale garden)
**How it works:** Creaking heart generates in pale oak trees (natural) and activates when placed between two pale oak logs aligned; creakings spawn at night near an active heart, despawn with the day cycle, attack within 32 blocks, freeze when a player looks at them, can't be hurt by cactus etc. Hitting a creaking makes its heart release 2-3 resin clumps; breaking the heart drops 1-3 clumps (Fortune applies) plus 20-24 XP for natural hearts. Resin farm = repeatedly damage creakings while limited by night-only spawning and daytime despawn. Resin -> bricks, block of resin; trim.
**Survival requirements:** Pale garden (Overworld); pale oak logs; Silk touch not needed.
**Version notes:** Pale garden/creaking introduced 1.21.4 (previously experimental 1.21.2). Wiki 1.21.4 page: creaking hearts, spawn near; 1.21.5 allows naming creakings. No 1.21.11 changes found. `creaking_active` environment attribute added by 1.21.11 per wiki.
**Bot-relevance:** Resin is optional (decorative); creaking AI (freeze-on-look) is combat-tricky.
Sources: https://minecraft.wiki/w/Java_Edition_1.21.4, https://minecraft.wiki/w/Java_Edition_1.21.2, https://minecraft.wiki/w/Java_Edition_1.21.11

### Phantom suppression (non-farm tech)
**How it works:** Phantoms spawn if a player hasn't slept in 3 days (wiki Mob_farm); AFK farms commonly just sleep periodically or accept them. **(background)** gamerule `doInsomnia` can't be changed without ops.
**Survival requirements:** Bed.
**Version notes:** Unchanged to 1.21.11 as far as found.
**Bot-relevance:** Bots that stay awake >3 days attract phantoms; schedule sleeping or use roofs.
Sources: https://minecraft.wiki/w/Mob_farm

---

## A.2 Villager tech

### Villager breeder
**How it works:** Adult villagers breed when "willing" (>= 12 nutrition: bread = 4 points, carrots/potatoes/beetroot = 1 each; after breeding unwilling for 5 minutes) and there are beds with >= 2 free blocks above, reachable by pathfinding; in Java breeding fails with no unclaimed bed in a 48-block radius. Job sites are not required for breeding. Designs: bed pods or pen with farmer-fed villagers; babies are moved by water/boats/minecart/ piston to a trading hall or iron farm. Throughput order ~ one baby per couple per several minutes.
**Survival requirements:** Villagers (village or boat-transported), beds, food (farm), glass/walls, zombie and lightning protection.
**Version notes:** Java requires beds in village bounds (wiki villager page); no 1.21.11-specific breeder changes found. 1.21.11 introduced `gameplay/villager_activity` attributes.
**Bot-relevance:** Needs villager transport and bed placement precision; relies on loaded chunks.
Sources: https://minecraft.wiki/w/Villager (partially read), https://minecraft.wiki/w/Java_Edition_1.21.11

### Villager trading hall
**How it works:** Cells hold villagers each near a job-site block; villagers lose profession if the job block is removed before first trade; reroll offers by breaking/replacing job block until wanted trade shows; once traded they lock the trade list. Restock up to twice/day when at job site (Java, needs working at job site); price = base * multiplier + demand + reputation, with Hero of the Village -30% (I) up to -55% (V), and gossip. Curing a zombie villager gives large permanent discounts. Discard mechanism removes unwanted villagers without credit. Key trades: Mending books (librarian), diamond gear, bottles o' enchanting, bell, fletcher stick/emerald, farmer crop emerald, etc.
**Survival requirements:** Villagers, job blocks (lectern, grindstone...), emerald stock, cells with trapdoor/piston, zombie/lightning protection.
**Version notes:** **Version-dependent**: Villager Trade Rebalance is an experimental data pack (not default in 1.21.11 per wiki - the wiki does not say it is default). Cartographer/wandering trader rework fully implemented in Java 1.21.5 (wiki Villager_Trade_Rebalance). Java 26.4 additions mentioned (not in 1.21.11).
**Bot-relevance:** Highly valuable (Mending/Fortune/Efficiency books); needs precise block-interaction and entity handling; also tradeable emeralds from crops/rotten flesh.
Sources: https://minecraft.wiki/w/Tutorials/Villager_trading_hall, https://minecraft.wiki/w/Trading, https://minecraft.wiki/w/Villager_Trade_Rebalance

### Zombie villager curing (discounts)
**How it works:** Apply Weakness effect to a zombie villager and feed a golden apple; after ~5 minutes (3-5 per background) it converts to a villager, giving major permanent positive reputation (discounts on most trades for employed cured villagers). Zombie villager conversion from killed villagers: 0% Easy, 50% Normal, 100% Hard. Zombify-cure loops are used in trading-hall designs (zombie + weakness splash + gapple dispensers).
**Survival requirements:** Zombie (villager), splash potion of Weakness (brewing), golden apple.
**Version notes:** Discount mechanics Java vs Bedrock differ: negative reputation raises prices only in Java.
**Bot-relevance:** Gains big cost reductions in trading; requires brewing + stable zombie handling.
Sources: https://minecraft.wiki/w/Zombie_Villager, https://minecraft.wiki/w/Trading, https://minecraft.wiki/w/Villager

---

## A.3 Boss/XP farms

### Wither and ender-dragon "farms"
**How it works:** Wither: build soul sand/soil T + 3 skulls; farms are mostly obsidian generation (wither caged and its explosion destroying blocks - not item farms). Passive Nether-star production via bedrock-suffocation exists on Bedrock only (wiki claims; Java options: bedrock ceiling in the Nether, underside of End portal, cobble grid). Dragon respawn via 4 end crystals gives only 500 XP (first kill 12,000 XP) so dragon XP farm is "almost worthless" per wiki; dragon head/egg are one-time. End-spike wither cage yields "up to 600,000 obsidian/hour" per wiki (relies on bugs).
**Survival requirements:** Wither fight: Smite/Protection IV armor/healing; skulls from farm.
**Version notes:** The End-platform obsidian farm relies on a bug that may be patched; may be disallowed on servers (unverified for 1.21.11).
**Bot-relevance:** Skip; bots only need to beat the dragon once.
Sources: https://minecraft.wiki/w/Tutorials/Wither_farming, https://minecraft.wiki/w/Tutorials/Obsidian_farming, https://minecraft.wiki/w/Tutorials/Experience_farming

### XP farm overview (which methods matter)
**How it works:** Fastest: guardian farm (~180k XP/h), enderman farm (level 30 < 1 min), gold/zombified piglin farm (level 100 ~1 h), skeleton spawner AFK (~100 levels in 8 h), fishing (~1,200 XP/h with Mending, 1,680 without; rain +25%), auto-smelting cactus (~360 XP/h in one furnace on Java), potatoes ~252/h, kelp ~72/h; sculk farm (1 XP per sculk block, 5 for sensor/shrieker); villager Bottles o' Enchanting (~7 XP each). XP only from player/tamed-wolf kills (sculk catalyst bypasses). Amethyst/spawner XP drops 15-43 on breaking.
**Survival requirements:** varies.
**Version notes:** 1.21.5 piglin XP patch; others unchanged.
**Bot-relevance:** Bots mostly need XP for enchanting; passive furnace XP (cactus/kelp) is easiest.
Sources: https://minecraft.wiki/w/Tutorials/Experience_farming, https://minecraft.wiki/w/Tutorials/Sculk_farming

### Sculk (catalyst) XP farm
**How it works:** A sculk catalyst turns nearby mob deaths into sculk, which can be mined for XP; unlike normal XP, it works for automated kills. Amount of sculk depends on the mob's XP value (zombie/skeleton ~ 3x3 area). Components: mob farm -> kill chamber -> catalyst platform -> sculk storage -> AFK mining area; sensors/shriekers give 5 XP.
**Survival requirements:** Sculk catalysts (Silk Touch from ancient city, warden-free), mob farm.
**Version notes:** Wiki page marked work-in-progress; no rates (unverified).
**Bot-relevance:** Niche.
Sources: https://minecraft.wiki/w/Tutorials/Sculk_farming

---

## A.4 Crop, tree and animal farms

### Wheat / carrot / potato / beetroot farm
**How it works:** Farmland within 4 blocks of water is hydrated; crops need light >= 9; bone meal advances 2-5 stages; harvesting by flood (water dispensers), piston/observer farms, villager farms (farmer + composter + hopper), flying machines. Harvest at ~80% maturity since stragglers take long. Semi-auto 9x9 plot with centre water is the standard.
**Survival requirements:** Hoe, seeds/crops, water, light, hoppers; villagers for auto-harvest.
**Version notes:** Dead bushes can be placed on farmland (1.21.5). Direct piston pushes turn farmland into dirt.
**Bot-relevance:** Easy with careful tilling; bots need to replant and keep light; loaded chunks for growth (random tick range).
Sources: https://minecraft.wiki/w/Tutorials/Crop_farming

### Sugar cane farm
**How it works:** Needs adjacent water on dirt/sand/mud; grows 1 block per 16 random ticks (~18 minutes per block average); max natural 3. Observer detects growth and a piston pushes the cane into a water stream/hoppers; flying-machine designs scale best (risk if unloaded while running). Zero-tick designs do not work in Java (as of 1.16 per wiki); bone meal duping is Bedrock-only.
**Survival requirements:** Observers, pistons, hoppers, water, (slime for flying machines).
**Version notes:** In Java cane drops later after losing water than Bedrock.
**Bot-relevance:** Paper/book/rocket supply; needs precise observer-piston placement.
Sources: https://minecraft.wiki/w/Tutorials/Sugar_cane_farming

### Bamboo farm
**How it works:** No water needed, grows to 12-16 blocks, one growth step about every 68 s per plant on Java default random tick speed; piston-break chain with observers above seed bamboo; two-way flying machine for big farms. Fuel for furnaces (smelts 0.25 items per bamboo) so needs rate; scaffolding supply.
**Survival requirements:** Bamboo, mud/dirt/sand, observers, pistons, hoppers.
**Version notes:** Unchanged in 1.21.
**Bot-relevance:** Good AFK fuel/scaffolding; requires large collections.
Sources: https://minecraft.wiki/w/Tutorials/Bamboo_farming

### Kelp farm
**How it works:** Grows to a random max height (age 0-24 → 2-26 blocks) with 14% growth chance per random tick; pistons break second-lowest block triggered by observers or clock; dried kelp block is a renewable fuel (one block smelts 20 items). Composting kelp gives bone meal (~1 per 23 kelp).
**Survival requirements:** Underwater setup, pistons/observers, hoppers.
**Version notes:** Java observer behaviour noted separately on the wiki.
**Bot-relevance:** Cheap renewable fuel with smelter; needs underwater building.
Sources: https://minecraft.wiki/w/Tutorials/Kelp_farming

### Melon / pumpkin farm
**How it works:** Stem growth needs light 9; fruit appears on adjacent block; ~4 m 32 s average per fruit; observers on stems trigger sticky pistons to break fruit; collect via water/hoppers. A horizontal design yields ~12-13 melons/min.
**Survival requirements:** Seeds, farmland, observers, pistons, hoppers.
**Version notes:** BUD designs not on Bedrock.
**Bot-relevance:** Melon slices -> emerald trades / food; pumpkins for golems/carved pumpkins; observer-piston build.
Sources: https://minecraft.wiki/w/Tutorials/Pumpkin_and_melon_farming

### Cactus farm
**How it works:** Cactus grows and breaks when any adjacent block touches it (any block, not only full); drops go to hoppers; smelting cactus yields green dye + XP (Java ~360 XP/h in one furnace; Bedrock much less). Efficiency tips: use thin blocks (fence posts, iron bars); end rods don't break cactus.
**Survival requirements:** Sand, cactus, hoppers, furnace.
**Version notes:** 1.21.5 added cactus flower (cactus can drop flower; bees attracted) - mechanism unverified.
**Bot-relevance:** Passive XP source; simple to place.
Sources: https://minecraft.wiki/w/Tutorials/Cactus_farming, https://minecraft.wiki/w/Java_Edition_1.21.5

### Tree farm (bone meal/TNT, birch/oak, mega spruce/jungle, mangrove, huge fungus)
**How it works:** Plant saplings, bone meal or wait, harvest (manual, flying-machine, or TNT-based with explosive collectors). Oak: apples; birch: fast/uniform for automation; dark oak/pale oak: fast but few saplings; jungle: high yield but hard; spruce giant: needs spiral technique; huge fungi (Nether) use bone meal and nylium; mangrove growable underwater. Auto designs: grow with bone meal dispensers + observer detection, TNT harvest, stripped log farm.
**Survival requirements:** Saplings, bone meal (skeleton/composter), axes, TNT for TNT designs (TNT is craftable, dupers separate).
**Version notes:** Pale oak sapling (1.21.4) grows 2x2; 1.21.x tree sizes unchanged. 
**Bot-relevance:** Wood renewable; bots can do manual cutting; automation needs TNT/flying machines.
Sources: https://minecraft.wiki/w/Tutorials/Tree_farming

### Honey / bee farm
**How it works:** Bees pollinate crops/flowers; hive honey level 5 allows harvest: shears -> 3 honeycomb; bottle -> honey bottle. Dispensers with shears/bottles and comparators detect full hives (campfire below, carpet, keeps bees calm). Bees can be killed by water/lava/fire/cactus; 1.21.4 wither roses harm bees.
**Survival requirements:** Bee nests (Silk Touch), flowers, hives, dispensers, shears.
**Version notes:** Bee breeding by flowers; unchanged.
**Bot-relevance:** Honey/honeycomb; low priority.
Sources: https://minecraft.wiki/w/Tutorials/Honey_farming

### Sheep / wool farm
**How it works:** Sheep regrow wool by eating grass (1/1000 per tick chance); automation: observer watches grass block eaten, dispenser with shears shears any sheep in front; hoppers collect. Dye sheep for colours. String duplication removed 1.21.2, so non-white wool needs dye.
**Survival requirements:** Shears, dispenser, observer, sheep.
**Version notes:** 1.21.5 sheep colors depend on biome.
**Bot-relevance:** Wool for beds (spawn point) - needed early; manual shearing is simple.
Sources: https://minecraft.wiki/w/Tutorials/Wool_farming, https://minecraft.wiki/w/Java_Edition_1.21.5

### Chicken / egg farm
**How it works:** Adult chicken lays ~8 eggs/hour (5-10 min each); eggs hatch 1 in 8; chicks take ~20 min to mature; eggs despawn after 5 min; cooking chickens with lava; 11x11 hopper farm 2.6 chickens/min start. 
**Survival requirements:** Chickens, seeds, hoppers, dispensers.
**Version notes:** Cooker designs broke in 1.21.6-1.21.8 (bug); cauldron-enclosure change is 26.1 (not applicable to 1.21.11).
**Bot-relevance:** Feather/food/arrow source; low.
Sources: https://minecraft.wiki/w/Tutorials/Egg_farming

### Cow / pig / animal farms
**How it works:** Pen >= 2 animals, breed (wheat for cow/sheep, carrots for pigs, seeds for chickens), 5-minute cooldown, babies 20 min; cull half; cramming crushers or lava/dispenser designs give cooked meat; lighting to deter hostiles. Entity cramming kills at >24 in a block (Java).
**Survival requirements:** Fences, food, hoppers.
**Version notes:** 1.21.5 biome-specific variants (cold/temperate/warm pigs/cows/chickens).
**Bot-relevance:** Primary food; feasible for bots (breed/cull).
Sources: https://minecraft.wiki/w/Tutorials/Animal_farming, https://minecraft.wiki/w/Java_Edition_1.21.5

### Fish farm / AFK fishing
**How it works:** Fishing: wait 5-30 s, Lure cuts 5 s per level; treasure needs open water (at least 5x5, 2 deep, open sky). AFK designs: tripwire + iron door/note-block or sculk sensor detecting bite (20w49a) → hopper → chest; ~8 catches/min outdoors. "God rod" Luck of the Sea III, Lure III, Mending, Unbreaking III. Cod/salmon smelt.
**Survival requirements:** Fishing rod, water, redstone parts.
**Version notes:** 20w12a broke old AFK designs; sculk-sensor designs after; wiki mentions 1.21 changes but they weren't in the fetched text (unverified).
**Bot-relevance:** Reliable enchanted-book/saddle source; AFK-friendly but needs loaded chunk and player.
Sources: https://minecraft.wiki/w/Tutorials/Fish_farming, https://minecraft.wiki/w/Tutorials/Experience_farming

### Turtle scute farm / armadillo scute
**How it works:** Turtles bred with seagrass; eggs on sand hatch at night after days; each baby drops a scute when growing; hoppers collect. Armadillos drop 1 scute every 5-10 min naturally; brush gives 1 scute each (brush has 4 uses on Java); don't kill; breed with spider eyes; spiders flee armadillos within 6 blocks. Needed for turtle helmet/Potion of Turtle Master/ wolf armor.
**Survival requirements:** Beach/sand, seagrass; savanna/badlands for armadillos; brush.
**Version notes:** Armadillo 1.20.5+; wolf armor via scutes; Golden dandelion on babies is 26.1 (not 1.21.11).
**Bot-relevance:** Minor.
Sources: https://minecraft.wiki/w/Tutorials/Turtle_farming, https://minecraft.wiki/w/Armadillo

---

## A.5 Mining and block farms

### Cobblestone / stone generator
**How it works:** Lava flowing into water becomes cobblestone (flowing lava over water from above gives stone); piston generators push new blocks out of the way (single piston core up to 13 blocks since pistons push max 12), quad piston: 4 blocks per 4th cycle; AFK stone miner with waterlogged protection and dispenser replacing pickaxes; semi-auto dispenser+button 24 stone per press. Doesn't work in Nether (water evaporates).
**Survival requirements:** Water, lava, pistons, observers/clock, pickaxe; protect water source from lava (stair trick) to prevent obsidian.
**Version notes:** Unchanged to 1.21.
**Bot-relevance:** Basic infinite cobble for bots; manual-mining version is easy and robust; AFK loaded.
Sources: https://minecraft.wiki/w/Tutorials/Cobblestone_farming

### Basalt generator
**How it works:** Lava flowing over soul soil and adjacent to blue ice makes basalt; piston + clock automates; build in Nether (lava flows faster).
**Survival requirements:** Blue ice, soul soil (not soul sand), lava, piston, clock, pickaxe.
**Version notes:** 1.16+.
**Bot-relevance:** Niche; blackstone/basalt build blocks only.
Sources: https://minecraft.wiki/w/Tutorials/Basalt_farming

### Obsidian farm
**How it works:** Pour water on lava; renew lava by dripstone/cauldron; manual diamond pickaxe 6.4 blocks/min; Efficiency V+Haste II ~32-39 blocks/min. Automatic: wither-caged portal loop (Java 1.21 portals can be created by entities), End platform/spike wither cage (up to 600,000/hr, bug-based), piglin barter. Nether portal re-creation yields ~14 obsidian/portal.
**Survival requirements:** Lava source, water bucket, diamond pickaxe.
**Version notes:** Bug-based End designs may be patched/disallowed.
**Bot-relevance:** Obsidian needed for Nether portal; bots can do lava-bucket + water casting.
Sources: https://minecraft.wiki/w/Tutorials/Obsidian_farming

### Lava farm (dripstone + cauldron)
**How it works:** Pointed dripstone under a lava source fills a cauldron: 15/256 chance per random tick (~19 minutes average refill) with stalactite < 11 tall and cauldron within 10 blocks below with only air between; containment non-flammable.
**Survival requirements:** Pointed dripstone, cauldron, lava bucket.
**Version notes:** No 1.21 changes found.
**Bot-relevance:** Renewable lava for fuel; needs random tick (player within simulation distance).
Sources: https://minecraft.wiki/w/Tutorials/Lava_farming

### Dripstone farm
**How it works:** Pointed dripstone grows on dripstone block with water above, ~1 block per 100 min; automatic via observers+pistons or flying machines; stonemason sells dripstone (1/4 Bedrock, 2/7 Java chance).
**Survival requirements:** Dripstone blocks, water, observers.
**Version notes:** None found.
**Bot-relevance:** Prerequisite for lava farm.
Sources: https://minecraft.wiki/w/Tutorials/Dripstone_farming

### Amethyst farm
**How it works:** Budding amethyst (geodes only; drops nothing) grows buds: each random tick 20% to create small bud if side free; ~7 in-game days to mature; harvest timer ~167 min (200,400 ticks); piston harvest gives 2 shards/cluster (Fortune III by hand avg ~10). **Java 1.21.5+: growth continues without nearby player if chunk is loaded (e.g. by ender pearl).**
**Survival requirements:** Geode; pistons/flying machine/water.
**Version notes:** See 1.21.5 random-tick change.
**Bot-relevance:** Shards for spyglass/tinted glass, not core.
Sources: https://minecraft.wiki/w/Tutorials/Amethyst_farming, https://minecraft.wiki/w/Java_Edition_1.21.5

### Ore generation
**How it works:** There is no renewable ore generator in vanilla survival (stone generators produce stone only); renewables: iron (golems), gold (piglin), copper (drowned, 1.21), coal (charcoal, wither skeletons), redstone (witches, trades), lapis (trades), diamond (none - villager trades), emerald (trades), netherite scrap (none), quartz (piglin trades? background). Mining via branch/strip, TNT, Fortune.
**Survival requirements:** n/a
**Version notes:** Copper from drowned in wiki Drowned_farming (1.21.x unverified).
**Bot-relevance:** Diamonds must be mined; bots plan around caves/Y levels.
Sources: https://minecraft.wiki/w/Tutorials/Drowned_farming, https://minecraft.wiki/w/Tutorials/Block_and_item_duplication

### Gravity-block (sand/gravel) farming and dupers
**How it works:** Natural sources: desert/beach mining. Wiki's duplication page lists a gravity-block duper (sand, gravel, anvils, dragon eggs fall through End portal, caught by pistons); patched in Paper 1.15.2 #358; vanilla status not stated. **Status in 1.21.11: unverified** (the page lists no confirmed working Java dupers in 1.21.x).
**Survival requirements:** End portal + pistons, etc.
**Version notes:** Servers commonly ban dupers (rule-based, not tech).
**Bot-relevance:** Avoid; plan on mining.
Sources: https://minecraft.wiki/w/Tutorials/Block_and_item_duplication

### TNT duper
**How it works:** Piston-driven duper that spawns primed TNT without consuming the block (1.14+); wiki says Mojang "left it in vanilla on purpose" while Paper (1.15.2 b358) can disable. Search shows a "Minecraft TNT Duper 1.21.11" video exists (unread), suggesting designs still exist; r/technicalminecraft mirror shows a thread "Why is the duper not working?" on 1.21.11 failing on a friend's server (title only) - consistent with servers (Paper) disabling it. **Status unverified.**
**Survival requirements:** Slime/observers/pistons, TNT; chunk loaded.
**Version notes:** Version-dependent; server software dependent.
**Bot-relevance:** Do not rely on; used for tree farm & mining ammo.
Sources: https://minecraft.wiki/w/Tutorials/Block_and_item_duplication, https://red.applefritter.com/r/technicalminecraft/controversial, https://www.youtube.com/watch?v=E9U8bMNVl6k (not readable)

### Concrete powder / sand-to-concrete, ice, mud, clay
**How it works:** Concrete powder hardens upon water contact (common automatic dispenser designs); ice farm freezes water adjacent to non-water block; blue ice via packed ice (nine) craft; mud/clay farm via mud + rooted dirt (dripstone mud?) **(background)**.
**Survival requirements:** Sand, gravel, dyes; Silk Touch for ice.
**Version notes:** None found.
**Bot-relevance:** Cosmetic except ice/blue ice for highways.
Sources: https://minecraft.wiki/w/Tutorials/Ice_farming

---

## A.6 Item infrastructure

### Item sorter (hopper-comparator) and overflow-proof sorter
**How it works:** Two hoppers; top filter hopper holds item to sort plus junk fillers; bottom hopper is locked by a signal so it only unlocks when enough sorted items accumulate; a comparator reads the filter hopper. Basic overflow-protected design 2.5 items/s per sorter (~10 redstone dust); double-speed 5 items/s; hybrid designs (oh-A/oh-B) fully isolated; stack-size-16 items need fewer fillers. Unstackable filter, shulker box filter using brewing stands. Hopper timing: 8-tick cooldown, 0.4 s transfer.
**Survival requirements:** Hoppers (5 iron ingots each), comparators, redstone, fillers.
**Version notes:** Torch burnout in some hybrid designs on Java 1.18+; oh5 works on 1.18+.
**Bot-relevance:** Bots can use chest-by-chest sorting instead; sorter requires precise redstone placement.
Sources: https://minecraft.wiki/w/Tutorials/Item_sorting, https://minecraft.wiki/w/Hopper

### Bulk storage / shulker box loader & unloader
**How it works:** Loader: filtered items go into a shulker box; when full, input hopper locked, piston breaks the box, dispenser places new one; unloader: hopper below box empties it, comparator detects empty, piston/dispenser swap. Minecart hopper unloaders (~20 items/s) laggy. **Hopper hashing** (Java; MC-96709 Won't Fix) can break loader designs after chunk reload.
**Survival requirements:** Shulker shells (End cities), pistons, dispensers, comparators.
**Version notes:** Java-specific hashing.
**Bot-relevance:** Optional; chest rooms enough.
Sources: https://minecraft.wiki/w/Tutorials/Shulker_box_storage

### Copper golem sorting (new in 1.21.9)
**How it works:** A copper golem (copper block + carved pumpkin/jack-o-lantern on top, pumpkin last) takes up to 16 items from copper chests and delivers them to wooden/trapped chests that are empty or already contain that item type; it examines each chest ~3 s, remembers 9 chests, searches 65x17x65 area; ignores barrels/ender chests/shulker contents; Java can't distinguish potion types. Unwaxed oxidizes (~7 h) and can become a statue (drops held item); wax stops. Gives simple, redstone-free sorting.
**Survival requirements:** Copper ingots, chests, pumpkin.
**Version notes:** 1.21.9 (2025-09-30); unchanged in 1.21.10/11 per hotfix notes.
**Bot-relevance:** Greatly simplifies sorting; bot only builds golem + chests; slow throughput but robust.
Sources: https://minecraft.wiki/w/Copper_Golem, https://minecraft.wiki/w/Java_Edition_1.21.9

### Item elevators (bubble column, dropper, hopper, piston, allay)
**How it works:** Bubble elevator: enclosed water column with soul sand at bottom (fast) or without (slow); dropper elevator: chain of upward droppers powered in sequence; block elevator is outdated; pistons move items diagonally; allays with note blocks for short distances. Hopper pipes 2.5 items/s.
**Survival requirements:** Soul sand, water, droppers, hoppers.
**Version notes:** 1.21.10 fixed entities flying high on bubble columns (1.21.9 regression); bubble column fewer particles 1.21.5.
**Bot-relevance:** Use for farm vertical transport; bot simply builds standard column.
Sources: https://minecraft.wiki/w/Tutorials/Item_transportation, https://minecraft.wiki/w/Java_Edition_1.21.10

### Item transport via water streams, ice, minecarts
**How it works:** Water streams (7 block flow, bridged with ice/signs/trapdoors), packed/blue ice speeds, slowers (cobweb, sea pickles). Hopper minecart collects items; minecarts with hoppers/chests reliable but slow. 
**Survival requirements:** water, ice, rails.
**Version notes:** Minecart Improvements is an experiment (1.21.2); not default in 1.21.11 per wiki.
**Bot-relevance:** Prefer chest routes.
Sources: https://minecraft.wiki/w/Tutorials/Item_transportation, https://minecraft.wiki/w/Minecart

### Hopper clocks / redstone clocks / copper bulb
**How it works:** Hopper clock: two hoppers pass items; comparator reads; 8 tick cooldown. Copper bulb toggles each redstone pulse and gives comparator 15 when lit, forming a compact T flip-flop (1.21). Observer clocks, daylight sensor.
**Survival requirements:** Hoppers/redstone/copper ingots.
**Version notes:** Copper bulb 1.21.
**Bot-relevance:** Useful for automating farms; wiring precision.
Sources: https://minecraft.wiki/w/Copper_Bulb, https://minecraft.wiki/w/Hopper

### Auto-crafter (Crafter block)
**How it works:** 3x3 block with disable slots; hoppers/droppers fill enabled slots; redstone pulse crafts after 4 ticks and ejects result (into container or world); comparator output counts occupied/disabled slots. Pistons can't move crafters (Java). Recipe: iron ingots, crafting table, redstone, dropper.
**Survival requirements:** Redstone, iron, dropper.
**Version notes:** Added 1.21 (24w18a).
**Bot-relevance:** Used to auto-craft (e.g., bone meal blocks); setup precise.
Sources: https://minecraft.wiki/w/Crafter

### Automatic smelter / furnace array
**How it works:** Hopper-fed furnaces; semi-automatic 3-chest, sorter-integrated, Mumbo design 0.5-1.2 items/s, ilmango 23 furnaces ~2.3 items/s accepts lava buckets; smokers/blast furnaces for double speed. Fuel: bamboo/kelp/charcoal from farm; copper chests work with copper golems.
**Survival requirements:** Furnaces, hoppers, chests, fuel.
**Version notes:** None.
**Bot-relevance:** Excellent for bot ore smelting; single-furnace chest loop is easy.
Sources: https://minecraft.wiki/w/Tutorials/Automatic_smelting

---

## A.7 Transport

### Nether highway / portal linking & stacking
**How it works:** 1:8 horizontal scale; Java portal search: 128 block radius in Overworld, 16 in Nether (since 1.16.2); closest portal by straight-line; creation in 3x4 spot within 16, else forced platform. Portal stacking shares obsidian; connected portals share 4 s countdown. Entities: cooldown 15 s; boats/minecarts 0.5 s (1.21.2). Chunk loading on teleport for 15 s. Highways: ice boat on blue ice 72.73 b/s overworld; via Nether 581.84 b/s equivalent; elytra via Nether 268 b/s.
**Survival requirements:** Obsidian, ice, boats, pickaxe.
**Version notes:** 1.21.2 boat portal cooldown 0.5 s; non-linked portals create return portal again 1.21.
**Bot-relevance:** Efficient travel; precise portal alignment; boats complicated.
Sources: https://minecraft.wiki/w/Nether_portal, https://minecraft.wiki/w/Tutorials/Transportation_methods

### Ice boat road
**How it works:** Boats on ice reach 40 b/s (ice/packed), 72.73 b/s on blue ice; need thousands of blocks to accelerate; 1.21.2 boat rework (boats' behavior changed? unverified).
**Survival requirements:** Blue ice, boats.
**Version notes:** Wiki lists boat speeds without version caveat; Bedrock differs.
**Bot-relevance:** For long trips; bot boat control questionable.
Sources: https://minecraft.wiki/w/Tutorials/Transportation_methods

### Minecart / rail networks
**How it works:** Max speed 8 b/s default (11.3 diagonal), powered rail boosting; Minecart Improvements experiment (1.21.2) rework unlikely default.
**Survival requirements:** Rails, minecarts.
**Version notes:** Experimental.
**Bot-relevance:** Rails are slower than ice boats.
Sources: https://minecraft.wiki/w/Minecart, https://minecraft.wiki/w/Tutorials/Transportation_methods

### Ender pearl stasis chamber
**How it works:** Pearl kept alive in bubble column/trapdoor; owner teleports by triggering; a pearl loads chunks while thrower online. Java 1.21.2: pearls load chunks on throw/landing (24w37a), thrown pearls load and tick a 3x3 area; **pearls unload when owner logs out and reappear when owner logs back in** (wiki 1.21.2 summary). Pearls vanish on owner's death (`enderPearlsVanishOnDeath`, 1.20.2) default true? background unverified.
**Survival requirements:** Ender pearls, trapdoors, bubble column.
**Version notes:** 24w37a changes; r/technicalminecraft mirror shows "Update suppression 1.21.10?" asking about stasis chambers.
**Bot-relevance:** Use as bot teleport; fragile.
Sources: https://minecraft.wiki/w/Ender_Pearl, https://minecraft.wiki/w/Java_Edition_1.21.2

### Flying machines (slime/honey)
**How it works:** Observers + pistons + slime/honey blocks; 12-block push limit; used for sugar cane, kelp, bamboo, amethyst farms.
**Survival requirements:** Slime blocks (slimeballs), pistons, observers.
**Version notes:** 1.21.10 wind charge fix; 1.21.2 piston changes.
**Bot-relevance:** Needs precision.
Sources: https://minecraft.wiki/w/Tutorials/Flying_machines

---

## A.8 Doors, bases, defense

### Piston doors
**How it works:** Flush 2x2 Jeb door, 1x2 simplest, scalable observers-based door 15x7. Java quasi-connectivity needed for many.
**Survival requirements:** Pistons/sticky pistons, redstone.
**Version notes:** Bedrock lacks QC.
**Bot-relevance:** Hidden access for bot bases; precise.
Sources: https://minecraft.wiki/w/Tutorials/Piston_doors

### Hidden base and mob-proofing
**How it works:** Light every spawnable spot with block light >= 1 (torches), slabs; hostile mobs spawn only where block light is 0 and sky light <= 7; passive mobs need block light >= 9 in unexposed areas.
**Survival requirements:** torches.
**Version notes:** 1.18+.
**Bot-relevance:** Bots should light base thoroughly.
Sources: https://minecraft.wiki/w/Light

---

## A.9 What changed in 1.21.x (selected, wiki-sourced)

- 1.21 (Tricky Trials): crafter, trial chambers/trial spawners, vaults, breeze (breeze rods, wind charge), mace, copper bulbs/grates, bogged, armadillo (1.20.5), wolf armor (1.20.5), Raid Omen replaces Bad Omen trigger for raids (wiki Raid); witch redstone drop guaranteed; Java portals with passengers.
- 1.21.2 (Bundles of Bravery): string duper removed; boats/minecarts portal cooldown 0.5 s; raids need valid spawn within 96 vertical blocks; ender pearls chunk-loading rework; pale garden/creaking as experiment; minecart improvements experiment.
- 1.21.4 (Garden Awakens): pale garden, creaking, resin, eyeblossom, pale oak.
- 1.21.5 (Spring to Life): random ticks in chunks loaded by pearls etc.; farm animal variants; zombified piglin XP patch; cactus flower; bush; leaf litter.
- 1.21.6: happy ghast, dried ghast, harness, locator bar, Tears disc.
- 1.21.9 (2025-09-30): copper golem, copper chests, copper tools/armor, shelves, copper bars/chains/lanterns/torches, lightning rod oxidation.
- 1.21.10 (2025-10-07): hotfix for piston/cobweb/wind charge/bubble-column bugs.
- 1.21.11 (2025-12-09): spears (jab/charge/lunge), camel husk, parched, nautilus/zombie nautilus, fire_spread_radius_around_player gamerule (doFireTick removed), environment attributes (villager_activity, bed_rule, can_start_raid, creaking_active, piglins_zombify...), pillager patrol skylight rule, stopwatch.

## A.10 Version-dependent flags

- Wither skeleton spawn rules: Java fortress bounding-box vs nether brick rule; 1.16+ piglin/wither skeleton aggression.
- Zombified piglin XP: patched 1.21.5.
- Villager trading: Trade Rebalance experimental; cartographer/wandering trader fully implemented 1.21.5.
- Golem spawn: Java summon-only vs Bedrock village spawn.
- TNT/gravity dupers: unverified in 1.21.11; server software may block.
- Ice-boat speeds: figures from wiki; not version-tagged.
- String dupers removed 1.21.2.
- Ender pearls: 1.21.2 behavior.
- 26.x features (cauldron chicken cookers, golden dandelion) not in 1.21.11.

## Sources used
minecraft.wiki pages listed above; r/technicalminecraft mirror; YouTube (unreadable).


---

## B. Digital (computational) redstone tech

**Scope/legend.** Target: vanilla Java 1.21.11 (released 2025-12-09), survival, no op/commands/Carpet. `gt` = game tick (0.05 s), `rt` = redstone tick (2 gt). Tags: **[W]** = fetched from minecraft.wiki this session (dated 2025-26 live wiki, content read through a summarising fetcher so wording is paraphrased); **[M]** = Mojira / Mojang page fetched; **[GH]** = GitHub README/paper fetched; **[N]** = news article fetched; **[gk]** = general knowledge not re-verified this session (treat as "likely, check before relying"); **unverified** = explicitly could not confirm. Several wiki pages (Tutorial:Memory, Tutorials/Clocks, Tutorials/Redstone_circuits, Tutorial:Flying_machine) failed to fetch; the canonical pages `Memory_circuit`, `Clock_circuit`, `Logic_circuit`, `Pulse_circuit`, `Block_update_detector` worked and are the main design sources.

**Global build rules for a placement bot (apply to every entry unless it says otherwise).**
1. Java redstone is *locational and directional*: result can depend on build orientation, coordinates (chunk/tick phase), and the **order in which blocks were placed**. Re-placing the same schematic in a different rotation/location can behave differently for 0-tick, piston-BUD, and QC designs. Prefer designs whose schematic says "orientation-independent".
2. **Place power last.** Levers, buttons, redstone blocks, torches (which power as soon as placed), repeaters/comparators feeding a line, and observers' target blocks are the "ignition". Build the whole circuit with the ignition piece missing, then add it. Torch loops (clocks, latches) start running the instant the loop closes; leave one dust/repeater out and place it last.
3. **Pistons, dispensers, droppers are quasi-connected (Java only)** - they react to power at the block *above* their position and fire on the next block update, so a bot that merely places a neighbour can fire them. Piston placement also evaluates power on placement (gk). Crafters, hoppers, copper bulbs, lamps are *not* QC.
4. **Orientation-sensitive blocks** (observer, piston, sticky piston, hopper, dropper/dispenser, comparator, repeater, torch wall/floor) take facing from player look direction and clicked face. Sources disagree/I could not confirm the exact observer convention (the wiki fetch said "observing face points away from the player"); the bot should place, then read back the block-state and correct/replace if wrong rather than assume.
5. Repeater delay and comparator mode are set by right-clicking after placement; each click is a block update and can re-trigger QC pistons or locked repeaters. Set them before the line is powered.
6. Containers (hoppers, barrels, droppers) used as logic must be filled *after* the logic is final; hoppers start moving items the moment they are placed next to a source.
7. All timing tech needs the area in a **ticking (entity-processing) chunk**; a bot standing >simulation distance away freezes clocks/flying machines mid-cycle. Flying machines and long item pipes should not straddle a loaded-chunk border.
8. Lag/low TPS shifts nothing in game-tick logic, but item-entity, despawn, and minecart-based timing does drift (see clocks).

---

## B.1 Version-sensitive core: update order, QC, Redstone Experiments, 1.21.x

### Redstone Experiments (24w33a/24w34a, "1.21.2" experimental data pack)
- **How it works:** An *opt-in experimental data pack* ("Redstone Experiments") that replaces the redstone-wire evaluator. All connected wire gets its new signal strength computed first, then updates go out; wire only sends block updates to blocks that can actually be powered by it; update order is breadth-first (nearer wires first), neighbour updates ordered by the direction the wire was reached from (back, front, left, right, down, up), left-first on ties (24w34a made this non-random); if power direction is ambiguous (powered from above/below) the order was random in 24w33a and left-first from 24w34a. Intent: make wire behave the same regardless of location/orientation. A SpaceWalkerRS analysis gist notes it also stops wire updating blocks that are only quasi-connected (e.g. dust on a slab no longer updates a piston beside/below), so QC-fed pistons and 0-tick repeater designs that relied on old timing break.
- **Typical size/speed:** n/a (engine change).
- **Version notes:** Added 24w33a (1.21.2 cycle), still described on the wiki as a *current* (not retired, not merged) Java experiment. Mojira MC-276479 (affects 24w36a-1.21.4, still Unresolved when fetched) shows even with it enabled, a fast 0-tick piston clock stays orientation-dependent. By 1.21.11: **no source I found says it was released into the default game or removed**; the 1.21.11 release article text I read (all but last ~4 KB) lists only Zombie Nautilus variants and "Environment Attributes" as experimental, and the wiki 1.21.11 page names only Timelines as experimental - neither mentions redstone. Sibling pack Minecart Improvements got a rename in 1.21.11 (`minecartMaxSpeed` -> `max_minecart_speed`, 25w44a), which shows opt-in experimental packs still existed in 1.21.11. **Conclusion: default 1.21.11 survival servers run classic wire; assume the experiment is OFF unless the server owner created the world with it** (unverified how/if a dedicated server can enable it later; it is not retroactively toggleable in a normal way - treat as world-creation-time).
- **Build-hazard:** Detect, don't assume: a bot cannot read the datapack list without commands. Prefer designs that work under both evaluators (no wire-on-slab QC feeds, no 0-tick repeater circuits, no reliance on wire update order). If a circuit is wire-order-sensitive (rare), flag it to the planner.
- **Sources:** https://minecraft.wiki/w/Redstone_Experiments ; https://minecraft.wiki/w/Java_Edition_1.21.2 ; https://gist.github.com/SpaceWalkerRS/25052d03ff956b988c50a75a08619545 ; https://bugs-legacy.mojang.com/browse/MC-276479 ; https://minecraft.wiki/w/Minecart_Improvements ; https://www.minecraft.net/en-us/article/minecraft-java-edition-1-21-11 ; https://www.minecraft.net/en-us/article/minecraft-snapshot-24w34a

### Classic (vanilla default) redstone-dust update order
- **How it works:** In vanilla dust each wire recomputes power in isolation and recursively emits block/shape updates, so update order depends on location and orientation (the hidden state that the Experiments pack tried to remove). Alternate Current (mod; not vanilla) re-implements this as network-wide propagation and emits updates only at the final power level, and states it fixes MC-11193.
- **Version notes:** Classic behaviour is the shipped behaviour through 1.21.11 (no wiki/changelog source I read reports a default-game change). In 1.19 neighbour updates moved to a queued "collecting" updater [gk, unverified here] which changed some instant-update chains vs <=1.18 - old (<=1.18) schematics may need re-testing. A Mojang-side regression in 25w02a (1.21.5 cycle) changed *sloped-rail* neighbour-update order, fixed in 25w03a (MC-279339) - proof that update-order bugs do slip into snapshots; releases >=1.21.5 are fine for that.
- **Build-hazard:** Never rely on wire update order across schematics; prefer repeaters/torches/comparators as barriers. Rails/powered-rail chains are sensitive too.
- **Sources:** https://github.com/SpaceWalkerRS/alternate-current ; https://bugs-legacy.mojang.com/browse/MC-279339

### Quasi-connectivity (QC) - Java only, unchanged by default through 1.21.11
- **How it works:** Pistons (incl. sticky), dispensers and droppers activate if *the space above them* would be powered (even if that space is air or a non-conductor). *Immediate QC* occurs when the powering component is 1 block from the space above the piston (comparators, dust, repeaters, torches, buttons/levers etc. on blocks below, detector rails, trapped chests...), so the piston is updated in the same step. *Update-based QC*: the piston sees the QC power but does not fire until it receives a block update (placing/breaking a neighbour, changing a repeater's delay, changing a torch state within 2 blocks). Neighbour updates reach at most 2 blocks (taxicab), but QC may need an update at distance 3.
- **Typical size:** n/a. Used for 1-wide T flip-flops, piston BUDs, compact dropper/piston control.
- **Version notes:** Java + Legacy Console only; Bedrock has none. Powered rails stopped being QC in 1.2.1. **Crafters (1.21) are NOT QC** - MC-265943 "Works As Intended". April-fools vote rule `fix_qc` (23w13a/b) is not in releases. The Experiments pack removes QC-driven wire updates (above).
- **Build-hazard:** The #1 reason a bot-built piston/dropper fires mid-build. Place QC components first, power later; avoid dust, torches, levers, repeaters and comparators in the 3x3x? shell above a piston until final. Standard insulation: a slab with dust over it, a repeater/comparator (+2 gt), or a cauldron/composter on a moving piston. Never "fix" by re-clicking a repeater near a piston.
- **Sources:** https://minecraft.wiki/w/Tutorial:Quasi-connectivity ; https://minecraft.wiki/w/Piston ; https://minecraft.wiki/w/Crafter ; https://bugs-legacy.mojang.com/browse/MC-265943

### 1.21.x component changes affecting computational redstone (summary)
- **Version notes:** 1.21: Crafter and Copper Bulb no longer need the experimental pack (crafter: 23w42a pack, 4-gt delay since 23w46a; bulb same cycle). 1.21 trial chambers (trial spawner, vault, breeze, wind charge from 1.20.3). 1.21.2: Winter Drop pack gave creaking heart (comparator output = creaking distance); it shipped in 1.21.4 and the `winter_drop` flag was removed. 1.21.4: pressure plates/doors now drop with any tool; creaking-heart resin triggers sculk sensors (frequency 13). 1.21.9: copper chest, copper golem (+statue block with comparator pose output), shelf, new lightning-rod oxidation variants. 1.21.11: no redstone change found in the sources read; game rules registry-renamed (`doFireTick` removed); copper chest texture change. Later versions exist (26.x), irrelevant for the target.
- **Build-hazard:** 1.21.9+ blocks exist for the schematic but nothing changed core mechanics: do not assume Copper Chest/Shelf behave like chests for hopper timing.
- **Sources:** https://minecraft.wiki/w/Java_Edition_1.21.4 ; https://minecraft.wiki/w/Java_Edition_1.21.9 ; https://minecraft.wiki/w/Java_Edition_1.21.11 ; https://minecraft.wiki/w/Copper_Chest

---

## B.2 Logic gates and basic building blocks

### NOT / inverter (torch, comparator-subtract, piston instant)
- **How it works:** A redstone torch attached to a powered block turns off (1 rt delay) and vice versa. A comparator in subtract mode with a full-strength rear input from a constant source inverts with a 1 rt (2 gt) delay and no burnout up to a 2-clock. The "instant inverter" uses pistons for 0-delay and is the base of 0-tick logic.
- **Size/speed:** Torch: 1 wide, 1 rt, silent. Comparator: flat, 1 rt effectively (comparator itself 2 gt). Piston inverter: ground ~bulkier, 1-wide version smallest and 2-tileable.
- **Version notes:** Torch burnout (several toggles in a short window) unchanged; Java burnt-out torch re-lights on next update. Bedrock soft inversion (torch on piston) is Bedrock-only - **do not import Bedrock schematics**.
- **Build-hazard:** Torch inverters flip as soon as placed; build with input absent. Piston inverter is "hardest to build correctly" (wiki) - needs exact facing and QC-free surroundings. Comparator inverters need the reference input to exceed the signal strength (a repeater feed).
- **Sources:** https://minecraft.wiki/w/Logic_circuit

### OR / NOR
- **How it works:** OR = join inputs on one dust/block (isolated versions use a torch (NOR+NOT), repeaters ("diodes", expandable to 15 inputs), or upside-down stairs/slabs under glowstone to pass signal upward only). NOR = any number of inputs onto a block with one torch on it; extra inputs by OR'ing first.
- **Size/speed:** OR-by-wire 0 delay; repeater OR +1 rt; torch NOR 1 rt.
- **Version notes:** Stable through 1.21.11.
- **Build-hazard:** Dust joining makes inputs *physically coupled* (backfeed). Use repeaters/diodes when inputs are driven by different circuits. Dust auto-connects to adjacent dust/components - verify no unintended side connection when dust is next to repeaters/torches of other circuits.
- **Sources:** https://minecraft.wiki/w/Logic_circuit ; https://minecraft.wiki/w/Redstone_circuit

### AND / NAND
- **How it works:** AND is built as NOR on inverted inputs (two input torches, a wire OR, an output torch) - 2 rt. NAND = AND without the final inverter, or stacked. Piston AND (an input moves a block to gate the other) is tri-state-like but cannot drive a low current (wiki caveat).
- **Size/speed:** Torch AND ~2 rt, 3 torches. NAND also universal (any gate can be made from NAND).
- **Version notes:** Stable; Java-only quirks only matter for piston versions.
- **Build-hazard:** Torch-only gates are the safest to bot-build (no QC, no facing subtleties except torch placement face). Place the input torches before wiring them to the OR dust to avoid transient pulses.
- **Sources:** https://minecraft.wiki/w/Logic_circuit

### XOR / XNOR / IMPLY
- **How it works:** Torch XOR (most common, larger, slower); piston XOR (faster, more compact); diode XOR; comparator-subtract XOR (two comparators, each subtracting the other: flat, fast, silent but both inputs must be within ~1 signal strength or one overwhelms the other, needs Nether quartz); XNOR = inverted XOR; IMPLY (1-2 rt depending on output state).
- **Size/speed:** Comparator XOR ~3x3 flat; torch XOR ~5x5+.
- **Version notes:** Comparator logic is Java/Bedrock compatible but Java comparators have edge cases with 1-tick pulses. Stable.
- **Build-hazard:** Comparator XOR requires specific side/rear layout - facing must be exact; ensure inputs are at equal strengths (repeater-regenerate to 15). Piston XOR needs QC-clean surroundings.
- **Sources:** https://minecraft.wiki/w/Logic_circuit

### Repeater diode, locking, isolation
- **How it works:** A repeater is a one-way, signal-regenerating, 1-4 rt delay buffer; powering a repeater from the side with another repeater/comparator **locks** it in its current state (the basis for compact latches, D latches, T flip-flops and RAM cells). Dust, repeaters and comparators are directional so buses are inherently one-way.
- **Size/speed:** 1 block per delay stage; locking needs side-input of strong power from a repeater/comparator/observer, not from dust.
- **Version notes:** Java 2-gt-precise; Bedrock repeaters only respond on odd game ticks and have minimum 1 rt output - circuits differ.
- **Build-hazard:** Delay is set by right-click AFTER placement (default 1 rt); count clicks (cycles 1->2->3->4->1) and read back block state. A repeater placed next to a powered dust/side source can lock immediately in an unintended state.
- **Sources:** https://minecraft.wiki/w/Memory_circuit ; https://minecraft.wiki/w/Tutorials/Redstone_computers

### Comparator subtract / compare logic
- **How it works:** Rear input vs. strongest side input: *compare* mode outputs the rear value only if no side input exceeds it; *subtract* outputs max(rear - max(side), 0). Side inputs accept only strong power (dust, repeater, comparator, redstone block placed beside counts in Java). A comparator also *reads* a container/cauldron/etc. at its rear (see "Signal strength" section). 2 gt per pass; in Java very short pulses (e.g. 2 gt) can be missed unless they come from simultaneous observer pulses.
- **Size/speed:** 2 gt per stage; 1 block.
- **Version notes:** Stable. New readable blocks over time: chiseled bookshelf (1.20), copper bulb (1.21), creaking heart (1.21.2 pack -> 1.21.4), shelf and copper golem statue (1.21.9).
- **Build-hazard:** Facing and mode toggle (front torch up/down) are right-click state; set mode before powering. A comparator behind a chest with a *conductive block above* reads 0 (Java), and a sitting cat blocks it - use barrels or shulkers for deterministic reading.
- **Sources:** https://minecraft.wiki/w/Redstone_Comparator

### Observer-based and piston-based "instant" gates
- **How it works:** Observers (1.11, Java: detect shape/state updates from the observed face) emit a 2-gt pulse; moved by pistons they also pulse, enabling flying-machine and pulse-transport logic. Piston/sticky-piston gates move a conductive block in/out of a circuit (0-1 gt start delay in Java) and are the basis of "instant" inverters and tri-state buffers.
- **Size/speed:** Observer 1x1x1; pulse 2 gt; piston stroke 2 gt.
- **Version notes:** Java observers detect *state changes* (PP updates), Bedrock detects *any block update* - not interchangeable. Piston start delay is 0 or 1 gt in Java depending on tick phase vs. fixed 2 gt in Bedrock [W].
- **Build-hazard:** Observers fire when the observed block changes - including when the *bot* places the neighbour. Place observers last; watch for output pulse hitting a QC piston. Sticky-piston gates require the moving block to be placed last.
- **Sources:** https://minecraft.wiki/w/Observer ; https://minecraft.wiki/w/Piston ; https://minecraft.wiki/w/Block_update_detector

---

## B.3 Pulse, edge, timing, 0-tick

### Edge detectors (rising / falling / dual)
- **How it works:** Rising-edge detector (RED): input split into a direct path and a delayed-inverted path ANDed (circuit breaker, 1 rt; NOR-gate 2 rt; locked-repeater 3 rt; dust-cut 0-1 rt where a piston cuts the dust line). Falling-edge (FED): moved-block (sticky piston), dust-cut. Dual-edge: observer-based. Pulse doubler: an observer on the input gives two 1-rt pulses per input pulse (needs >=3 rt input).
- **Size/speed:** Circuit breaker 1 rt delay, output 2 gt (Java).
- **Version notes:** Moved-observer RED/FED: Java delay 2 rt vs Bedrock 4 rt (Java-only by the wiki). The wiki flags the page as outdated/Java-centric; trust timings only approximately.
- **Build-hazard:** Pulse widths depend on repeater delay settings (right-click). Dust-cut variants rely on piston timing and QC cleanliness.
- **Sources:** https://minecraft.wiki/w/Pulse_circuit

### Pulse limiters / pulse extenders / monostables
- **How it works:** Limiters shorten long pulses (circuit breaker, dust-cut, moved-block, locked-repeater, dropper-hopper). Extenders lengthen short pulses (repeater 1-4 rt; parallel repeaters; repeater line; dropper-latch up to ~256 s with each item adding 8 gt; hopper-clock extender; fader of comparators (decaying signal); RS-latch extender; MHC up to ~22 h and MHDC up to ~81 h; "cooldown" needs command blocks so not survival).
- **Size/speed:** Repeater extender = delay of the repeater; hopper extender 4 rt up to 256 s.
- **Version notes:** Java: piston can "abort" and drop a sticky block if the pulse ends before extension; lamps need >=2-tick off-pulse to switch off; comparators/torches unreliable at 1 tick.
- **Build-hazard:** Dropper/hopper timers need exactly the right item count (put items in last). A long input can keep a hopper timer locked - block it. Hopper/dropper timers keep running across chunk unload only if unloaded state persists (item despawn clocks differ).
- **Sources:** https://minecraft.wiki/w/Pulse_circuit

### Block Update Detectors (BUD) and observers
- **How it works:** A BUD is a circuit held in a state it "should" have updated out of; a neighbouring block change gives the update and fires it. Types: piston/QC-based (feedback loop), stuck-piston, dropper/dispenser/door, torch (burnt-out torch resets on update, ~56-tick cooldown), redirection, powered-rail (transmits updates cheaply), fluid, trapdoor-attached component. Observer (16w39a, 1.11) is the block-form BUD: Java detects state changes (PP updates) rather than NC updates; Bedrock detects everything except state changes like stem growth.
- **Size/speed:** Observer 1 block; torch BUD smallest; piston BUDs need resetting.
- **Version notes:** Many BUDs died over time (13w01a stopped data-change detection; 1.9 removed boat BUDs; 14w25a enabled torch BUD). Most Java BUDs do **not** work in Bedrock.
- **Build-hazard:** BUDs are *the* mechanism that fires when a bot places or removes the block next to them. If the schematic has a BUD, build the BUD's reset/power element last and avoid placing/breaking neighbours after arming. T-BUD toggles on every update.
- **Sources:** https://minecraft.wiki/w/Block_update_detector

### 0-tick pulses and 0-tick tech (Java only)
- **How it works:** A piston fed a pulse shorter than its start delay (a 0-tick on or off pulse produced by moved-block/dust-cut/observer-piston arrangements) still extends/retracts in Java; sticky pistons then toggle without dropping the block. Used for ultra-fast clocks, piston doors, instant dropper lines (via wire-order), duplication machines. The Clock_circuit page lists 1gt/2gt/3gt pulse clocks and a true "0-tick clock" (Myren Eario) relying on block-event exploits that stall the tick.
- **Size/speed:** 0-tick outputs have 0 gt duration; 1gt piston clock is 20 Hz.
- **Version notes:** **Highly version-dependent.** Java-only. Mojira shows renderer/signal weirdness (MC-151705, 1.14+), observer-clock pulse oddities (MC-153356, 18w43a-1.16), and MC-137638 (observer repeatedly firing piston) - all low priority/unresolved. Under Redstone Experiments they are partly broken (QC stripped) or still orientation-dependent (MC-276479). Dust "doesn't light" visually on 0-tick lines.
- **Build-hazard:** Do not put a 0-tick design in a bot's autonomous plan unless the schematic comes with a verified version tag and orientation. Needs exact facing of every piston; transplanting rotated/shifted can break it (tick-phase dependent).
- **Sources:** https://minecraft.wiki/w/Clock_circuit ; https://minecraft.wiki/w/Pulse_circuit ; https://bugs-legacy.mojang.com/browse/MC-151705 ; https://bugs-legacy.mojang.com/browse/MC-153356 ; https://bugs-legacy.mojang.com/browse/MC-137638 ; https://bugs-legacy.mojang.com/browse/MC-276479

### Tick delay, torch burnout, repeater locking as timing primitives
- **How it works:** Timing is built from repeater delays (2-8 gt), torch delays (2 gt, burnout if a torch toggles too often), comparators (2 gt), observers (2 gt delay then 2 gt pulse), pistons (2 gt stroke), hoppers (8 gt cooldown; 7 gt when a pushed item enters an empty hopper in Java), dispensers/droppers (4 gt), crafter (4 gt).
- **Version notes:** Crafter 4-gt (23w46a), hopper 8/7 gt from wiki; unchanged to 1.21.11.
- **Build-hazard:** Tick budgets are the contract - schematics for compute must not be "optimised" by a bot.
- **Sources:** https://minecraft.wiki/w/Hopper ; https://minecraft.wiki/w/Crafter

---

## B.4 Latches and memory

### RS NOR latch (and variants)
- **How it works:** Two cross-coupled NOR gates (torches); set/reset pulses hold the state. Wiki lists variants A-J (duplex I/O; isolated I/O; 3x1x4 vertical; same-side inputs), RS-NAND (larger), input-stabilising (ISR) latches, piston/dropper latches (M, N, O, P: two redstone blocks give a dual-output 1x1 latch; droppers + comparator give a 3x1x2 tileable latch), enable/disable latch, and analog RS latch (holds highest strength, fades by R).
- **Size/speed:** Variant F 3x1x4; P 3x1x2.
- **Version notes:** Stable. Dropper latch design P: power droppers through an adjacent block - powering a dropper directly also activates the other, in unpredictable order.
- **Build-hazard:** Setting S and R together is undefined; during build a torch latch comes up in a random/stable state depending on placement order - always pulse reset after build (the bot can place a reset button/lever last and activate it, or accept uninitialised memory). Torch loops start oscillating transiently.
- **Sources:** https://minecraft.wiki/w/Memory_circuit

### T flip-flop (toggle)
- **How it works:** One input toggles output. Designs: repeater-locking (L3 high-level latch; L5 rising-edge; L6 1-high D-flip-flop based; L4 falling-edge piston-based handling 2-8 tick off pulses; L7 for long on-pulses such as stone buttons); piston QC design O (1 wide, Java-only, toggles on falling edge, uses a redstone block; R adds reset); observer designs O1-O3 (one observer, one redstone block, two sticky pistons; O1 2-delay, 6x1x1 horizontal, rising; O3 falling); copper bulb TFF (see below). A wooden button's long pulse makes L3 oscillate; stone button OK.
- **Size/speed:** Observer TFF 4x2x1 to 6x1x1; QC TFF 1-wide.
- **Version notes:** Observer/QC designs Java-only. Pistons rely on a sticky-piston quirk where short pulses toggle a block; under Redstone Experiments QC-fed pistons break.
- **Build-hazard:** Redstone block and piston placement order matters (redstone block placed last; piston orientation exact). Wiki notes several designs "undocumented/untested" - prefer the "best in class" ones (L3-L7, O1-O3).
- **Sources:** https://minecraft.wiki/w/Memory_circuit

### Copper bulb T flip-flop / 1-bit toggle memory (1.21)
- **How it works:** The copper bulb toggles lit/unlit on each redstone pulse (on the power rising edge) and holds state without continuous power. It is not conductive and shows a red dot while receiving power. A comparator reads it: 15 when lit, 0 when unlit, regardless of oxidation. Comparator + bulb = the simplest known TFF (wiki lists it first among TFFs). Light level drops with oxidation (15/12/8/4) but waxing is irrelevant to logic.
- **Size/speed:** 1 bulb + 1 comparator; instant toggle; output readable 2 gt later.
- **Version notes:** Added 1.21 (announced MC Live 2023, 23w46a experiments); stable 1.21-1.21.11; Bedrock toggles only on C-tick. Mojira MC-312228 is only a wax-off sound bug.
- **Build-hazard:** In Java I believe the bulb also evaluates power on placement and neighbour changes (gk/unverified): placing a bulb next to an already-live source may flip it, and leaving it powered/unpowered at final state sets its start state. Place the bulb before the source; read-back its `lit` state with a comparator-free check if needed. Oxidation does not change logic.
- **Sources:** https://minecraft.wiki/w/Copper_Bulb ; https://minecraft.wiki/w/Memory_circuit

### D latch / D flip-flop (repeater-locking, "modern")
- **How it works:** A gated D latch built from two repeaters (very compact) using side-locking; two latches back-to-back (master-slave) give a rising-edge D flip-flop (4 repeaters + 1 torch; reversing block/torch makes it falling edge). Analog D latch (J): low-level triggered, 3-tick delay, output strength follows D. Older torch D latches are larger and fragile; a BUD-based D flip-flop (I) is small but forbids using neighbouring blocks.
- **Size/speed:** Modern gated D latch ~1x1x3 range; flip-flop ~2-3 wide.
- **Version notes:** Locking repeaters work in Java and Bedrock, with different tick phases; use Java-tagged schematics.
- **Build-hazard:** Side lock must come from a repeater/comparator, not dust. Repeater delays must be set via right-click. The BUD version will fire on placement of neighbours. This is the primary bit-cell for registers; the bot should treat each bit cell as an indivisible schematic with fixed facing.
- **Sources:** https://minecraft.wiki/w/Memory_circuit ; https://minecraft.wiki/w/Tutorials/Redstone_computers

### JK latch/flip-flop
- **How it works:** J sets, K resets, both high toggles. Wiki designs A (9x2x11, edge-triggered, no Q-bar), C (7x4x5 level-triggered), D (5x2x7 edge-triggered, repeaters), E (14x10x1 vertical). Level-triggered C can race if the clock is held high; usually a D or T cell is smaller.
- **Version notes:** Stable; rarely used.
- **Build-hazard:** Large, many facing-sensitive parts; avoid unless required by a schematic.
- **Sources:** https://minecraft.wiki/w/Memory_circuit

### Comparator memory cells (analog latch and analog counter)
- **How it works:** A comparator looped to itself in compare mode with side-lock holds an analog value (strength 0-15) - one cell stores 4 bits. The wiki's analog RS latch holds the highest strength it was set to and decays by the reset strength every two ticks; an analog counter adds increment/decrement inputs. Faders (comparator loops) also double as delay lines/timers.
- **Size/speed:** Compact; 15-level storage per cell; decays unless refreshed.
- **Version notes:** Works in Java and Bedrock (with tick-phase differences).
- **Build-hazard:** Comparator mode and facing must be exact; self-loop designs must be built with the feedback line broken and the loop closed last. Dust strength loss on any long wire corrupts the value - use repeaters.
- **Sources:** https://minecraft.wiki/w/Memory_circuit ; https://minecraft.wiki/w/Redstone_Comparator

### Item-based memory: hopper/dropper/barrel/chest storage as state
- **How it works:** State is the presence/quantity of items in a container, read by a comparator (strength ~ 1 + 14 * average slot fullness, floored; empty = 0). Hopper has 5 slots (with unstackables, 1 item -> strength 3, 2 -> 6, 3 -> 9, 4 -> 12, 5 -> 15 by the wiki formula); barrel/chest 27/54 slots; dropper 9. Dropper-latch and hopper-lock designs hold a bit as "item here or not". Unstackable or 16-stack items give controlled strengths. Hoppers keep items when the world saves (non-volatile).
- **Size/speed:** Hopper cooldown 8 gt (7 gt after pushing into an empty hopper, Java). Reading takes 2 gt via comparator; comparators can read items passing in a horizontal hopper pipe in Java but vertical pipes may read 0.
- **Version notes:** Chest read blocked by a conductive block above or a cat (Java); barrels/shulkers always readable. Stable 1.21.11. Copper chest (1.21.9) not confirmed for comparator behaviour - **unverified**.
- **Build-hazard:** Items must be inserted by the bot *after* the plumbing is built; locked/unlocked hopper state follows the power the instant it exists. Items can fall out if a hopper outputs into nothing. Item ordering in the first slot matters for filter-style designs.
- **Sources:** https://minecraft.wiki/w/Hopper ; https://minecraft.wiki/w/Redstone_Comparator ; https://minecraft.wiki/w/Barrel ; https://minecraft.wiki/w/Crafter

### Chiseled bookshelf as memory cell (1.20+)
- **How it works:** A comparator reads the **last interacted slot** (1-6; 0 if none), so six slots hold a stateful pointer; hoppers also set it, even when they fail to remove a book (set to the last non-empty slot). Observers can detect books added/removed. `last_interacted_slot` is stored in block data (0-5, -1 none).
- **Size/speed:** 1 block, 6 states.
- **Version notes:** Introduced 1.19.3/1.20 experimental then 1.20; stable. The "memory" use is an inference by community, not stated by the wiki.
- **Build-hazard:** A bot interaction (placing/removing books) sets the pointer, so the bot's own filling work leaves a defined initial state; but a hopper pointed at it changes the state silently.
- **Sources:** https://minecraft.wiki/w/Chiseled_Bookshelf

### Lectern / book page as 4-bit register and pulse source
- **How it works:** A comparator reads the page: Java strength S = floor(1 + 14(P-1)/(M-1)) (P current page, M page count; single page -> 15). Turning a page emits a 2-gt full-strength pulse (since 19w02a). Bedrock reads differently (even pages, right-hand page). Gives a hand-settable analog value or a clock-like tick per page turn.
- **Version notes:** Java vs Bedrock formulas differ - do not trust Bedrock schematics.
- **Build-hazard:** Requires a written/signed book with correct page count and a player/bot to turn pages; a bot builder would need to place the book (item, not block) after building.
- **Sources:** https://minecraft.wiki/w/Lectern

### Decorated pot and barrel as comparator stores; shelf as 3-bit register (1.21.9)
- **How it works:** A decorated pot holds one stack (one item type) and is readable by a comparator (strength values not given on the page); hoppers/droppers/crafters insert, hoppers extract. Shelves (1.21.9): three slots, comparator reads left=1, middle=2, right=4 (max 7, comparator must face the shelf's back); powered shelves can link up to three adjacent shelves and swap with the player's hotbar - effectively a 3-bit register/ROM word addressable by item presence.
- **Version notes:** Shelf added with 1.21.9 (images 25w31a); pot hit by projectiles/wind charges shatters.
- **Build-hazard:** Pots shatter from wind charge/arrows/pistons moving them drop them; shelf swapping is player-interaction driven, avoid powered shelves near active bots.
- **Sources:** https://minecraft.wiki/w/Decorated_Pot ; https://minecraft.wiki/w/Shelf

---

## B.5 Clocks

### Torch clocks / rapid pulsar / 20gt torch loops
- **How it works:** Torches in a loop invert in sequence; the "rapid pulsar" (two torches short-circuited through one dust) runs at 4 gt (5 Hz); the classic 20-gt loops extend with more torch pairs; a 16-gt torch clock (D) has each torch feeding two others (like RS-NOR latches).
- **Version notes:** Torch burnout threshold unchanged; clocks faster than a 3-clock burn out.
- **Build-hazard:** Starts the instant the loop is closed; leave a gap. A burnt-out torch re-lights on next update, restarting the clock.
- **Sources:** https://minecraft.wiki/w/Clock_circuit

### Repeater clocks (4-gt loop, 2-gt Java loop, torch-repeater hybrids)
- **How it works:** Two repeaters in a loop = 4 gt period (5 Hz); more repeaters lengthen; a switchable version has a sticky piston close/open the loop. Torch-repeater loops (12-gt basic; vertical 72-gt with 32-gt delay). 2-gt Java-only (10 Hz) loop works but pistons/non-update-instant parts see it as constantly powered. 9-16 repeater loops are cheaper as T flip-flop/clock multiplier.
- **Version notes:** 2-gt loop is Java-only.
- **Build-hazard:** An input pulse longer than 4 gt locks repeaters permanently (needs rebuilding). Close loop last.
- **Sources:** https://minecraft.wiki/w/Clock_circuit

### Comparator clocks
- **How it works:** A comparator in subtract mode feeds its own side input (4-gt, 5 Hz; works down to input strength 2); inline subtraction clock (12 blocks); 8-20 gt versions via repeater setting; 2x3 super-compact 4-gt clock (6 blocks); fader pulsers (18 gt: 2 gt on/16 off; 58 gt with two comparators) rely on decay; alternating clock swaps strengths every rt.
- **Version notes:** Stable.
- **Build-hazard:** Faders are "hard to tune precisely"; comparator mode and rear/side connection must be exact.
- **Sources:** https://minecraft.wiki/w/Clock_circuit

### Observer clocks
- **How it works:** A single observer watching its own output via dust = 4 gt (add repeaters to lengthen); two observers facing each other = 6 gt (on 2, off 4), which depends on how they were placed; a piston pushing one observer into place gives 4 gt; N-observer ring: period N rt; odd counts need opaque blocks/dust/powered rail bridges.
- **Size:** 2 blocks for the fastest clocks.
- **Version notes:** Java-only behaviour relative to Bedrock observers; Bedrock waterlogged-observer clock stopped working in Bedrock 1.14.1. Java 6-gt double observer depends on placement order.
- **Build-hazard:** Starts when the second observer is placed facing the first (placement itself pulses) - build with a blocking block removed last. Long chains can carry multiple pulses.
- **Sources:** https://minecraft.wiki/w/Clock_circuit ; https://minecraft.wiki/w/Observer

### Hopper clocks (Etho clock EHC, multi-item, MHDC)
- **How it works:** Items shuttle between two hoppers; comparators or lock logic sense item presence. Single-item EHC period ~14 gt in Java (16 in Bedrock; first cycle differs); multi-item EHC extends with item count; variants include loop, N-hopper, tileable, piston-less, self-locking with trigger input. MHDC (multiplicative hopper-dropper clock) multiplies period by stage (a 3-stage build reaches ~10.7 years). Dropper-dropper clock: single-item 32 gt period (6 on, 26 off), multi-item (n*8+6)*2 gt.
- **Version notes:** Hopper cooldown 8 gt (7 gt after receive-push into empty hopper in Java).
- **Build-hazard:** Items inserted last; do not let hoppers chain into unintended containers. Dropper clocks may need re-priming after world reload.
- **Sources:** https://minecraft.wiki/w/Clock_circuit ; https://minecraft.wiki/w/Hopper

### Piston clocks (Java only)
- **How it works:** Extendible basic (sticky piston + dust): 6 gt (5 on, 1 off); dual-block 40 gt (44 Bedrock); inverted basic 6 gt (moved block placed last); simple 4-gt (redstone block on sticky piston); self-powered QC+BUD 6-gt vertical. Some break after exit/reload (MC-89146).
- **Version notes:** Java-only; Redstone Experiments strips QC so QC-based clocks break.
- **Build-hazard:** Highly placement-order-sensitive ("moved block placed last"). Not safe to build autonomously.
- **Sources:** https://minecraft.wiki/w/Clock_circuit

### Long-period clocks: despawn, minecart, geyser, clock multiplier, LFSR, daylight
- **How it works:** Item despawn clock (~5 min; players/pressure-plate/roll-off pitfalls); minecart loop (36/55 gt); clock multiplier via latching chain (about 12 stages max); repeater-feedback LFSR (cycle 0.4*(2n-1) s: 10 elements ~409 s); daylight detector (one period per in-game day); T flip-flop doubles any period. Geyser clocks are from a modded/other-game context in the cited page ("armor stands on geysers over sulphur" - appears to be a newer-version feature; **unverified for 1.21.11**).
- **Build-hazard:** Entity-based timers (items, minecarts) are chunk-load- and player-dependent; avoid in schematics intended to run unattended.
- **Sources:** https://minecraft.wiki/w/Clock_circuit

---

## B.6 Signal strength, ADC/DAC, decoders

### Signal-strength decoders (analog to one-hot / binary)
- **How it works:** A signal of strength 0-15 is converted to n lines by comparators in compare/subtract mode with reference lines at 1..15 (each comparator passes if strength >= k), or by dust-length decay trees; classic "signal strength to binary" ladders. Used to read a barrel/hopper/chest fullness, a lectern page, a chiseled-bookshelf slot, or a shelf (3 bits) into digital buses.
- **Size/speed:** One comparator per level; 2 gt/stage.
- **Version notes:** Stable. Container strengths per the comparator formula; chest read can be blocked (see above).
- **Build-hazard:** Reference rails must be exact strengths (dust strength drops 1/block); use redstone blocks/repeaters (15) and a decoy dust length.
- **Sources:** https://minecraft.wiki/w/Redstone_Comparator ; https://minecraft.wiki/w/Hopper

### Analog-to-digital and digital-to-analog (container ADC/DAC)
- **How it works:** Fill-level of a container is the ADC (e.g. a hopper with unstackable items gives 3/6/9/12/15); a binary-weighted set of dust lengths or repeater-chain attenuators is a DAC. Hopper-sorter circuits depend on strength 3 unlocking only one hopper while 4 would unlock neighbours.
- **Version notes:** Unchanged through 1.21.11; shelf (1.21.9), copper golem statue and creaking heart add new analog sources.
- **Build-hazard:** Items in the container are the data; a bot cannot "preload" correctly unless the schematic includes item data.
- **Sources:** https://minecraft.fandom.com/wiki/Tutorials/Hopper ; https://minecraft.wiki/w/Redstone_Comparator

### Sculk sensor / calibrated sculk sensor as wireless decoder (1.19/1.20)
- **How it works:** Sculk sensor: 8-block sphere, output strength = max(1, 15 - floor(15/range * distance)), active 30 gt, 10-gt cooldown; vibrations carry frequency 1-15 (piston/lever/button/pressure plate/door activate = 10, deactivate = 9; container open/close 10/9; block place 13; block destroy 12; lectern/composter/bell block change 11). Calibrated sculk sensor (range 16, active 10 gt then 10 gt cooldown) filters by the signal strength N on its amethyst side (no input = highest frequency) and relays through amethyst blocks. Wool/carpet between source and sensor occludes. Not triggered by dispensers, droppers, crafters, jukeboxes.
- **Typical size:** 1 block; transmission 1 block per gt.
- **Version notes:** Frequency table on the wiki is Bedrock-labelled; Java differs slightly (sculk sources/warden don't trigger). Creaking-heart resin triggers sensors at frequency 13 from 1.21.4.
- **Build-hazard:** Sensors are fired by the *bot itself* walking, placing, or breaking within range - mid-build false triggers are likely. Place sensors and their outputs last, or keep the bot outside range (8/16 blocks).
- **Sources:** https://minecraft.wiki/w/Sculk_Sensor ; https://minecraft.wiki/w/Calibrated_Sculk_Sensor

### Item sorters as logic (hopper filter, overflow gates)
- **How it works:** The classic sorter: top hopper holds filter item in slot 1 plus distinct junk items in slots 2-5; a comparator reading it unlocks the bottom hopper only when more filter items arrive (strength 3 unlocks one hopper, 4 would unlock neighbours). It is effectively an equality comparator on item type and a conditional (AND). Throughput ~2.5 items/s (5 with double-speed variant). Hybrids tile alternate circuits so the comparators stay isolated; torch-based ones burn out in 1.18+ ("prone to burnout").
- **Version notes:** Stable; unstackable-filter sorters use items with stack size 1.
- **Build-hazard:** Filter items are put in last and in exact slot order; junk items must be unique and non-conflicting. Chests/hoppers adjacent can leak items. Copper golem (1.21.9) can sort into wooden chests: another item-logic source with 32x8-block search - may steal from copper chests near a build.
- **Sources:** https://minecraft.fandom.com/wiki/Tutorials/Hopper ; https://minecraft.wiki/w/Copper_Golem

---

## B.7 Counters, arithmetic, registers

### Binary counters
- **How it works:** A chain of T flip-flops (each stage divides by two), or a ripple counter with pulse limiter; 1-tick binary counter exists as a Java-only design. Holding the input low can count repeatedly and burn out a torch, so add a limiter. Analog counter uses comparator strength steps (0-15). Dropper-hopper dividers have reset periods during which pulses are not counted.
- **Size/speed:** One TFF per bit.
- **Version notes:** TFF designs: Java piston/observer ones are Java-only.
- **Build-hazard:** Reset to zero is a bot-side operation (pulse the reset last). Counter state after build is arbitrary until reset.
- **Sources:** https://minecraft.wiki/w/Pulse_circuit ; https://minecraft.wiki/w/Memory_circuit ; https://minecraft.wiki/w/Redstone_circuit

### Adders: half, full, ripple-carry, carry-lookahead
- **How it works:** Half adder = XOR + AND; full adder = two half adders + OR on carries; N-bit ripple adder chains the carry; subtraction by two's complement (invert B, carry-in 1). Carry-lookahead (precompute generate/propagate in parallel) is a speed optimisation used in faster CPUs. A modular adder/subtractor on the wiki (Tutorial:Calculator) uses full adders chained by carries and half adders at the end, with multiplexers built from XOR/IMPLY for subtract. The wiki does not name ripple-carry or CLA; they are standard digital design applied to Minecraft - **designs are community-specific (unverified here)**.
- **Size/speed:** Ripple delay ~ N * 2-4 rt per bit depending on gates; compact full adders use comparator XORs.
- **Version notes:** Gate-level stable; 0-tick/QC-based compact adders break across versions.
- **Build-hazard:** Wire order of inputs matters (A1,B1,A2,...). Fixed pitch layouts (3-wide bit-slices) must be pasted whole; repeaters inside a slice must have set delays.
- **Sources:** https://minecraft.wiki/w/Tutorial%3ACalculator ; https://minecraft.wiki/w/Tutorials/Redstone_computers

### ALU (arithmetic logic unit)
- **How it works:** Selects between add/sub, AND/OR/XOR/NOT, shifts via function-select lines and multiplexers; MASIC 8-bit computer's ALU performs add, left/right shift, bitwise NOT, with overflow flagged from the final carry; multiplication by repeated addition or shift-and-add. CHUNGUS 2 reports 40+ ALU functions including barrel shifter, multiplier, divider and square-rooter.
- **Size/speed:** From ~10 blocks tall 4-bit to city-scale for 16/32-bit.
- **Version notes:** Gate-level stable.
- **Build-hazard:** Big schematics use hundreds of repeaters with set delays; the bot must set each repeater delay and comparator mode after placement - schematics store block states, not click counts, so confirm that the builder preserves `delay`/`mode`/`locked` states.
- **Sources:** https://minecraft.wiki/w/Tutorials/Redstone_computers ; https://pcgamer.com/minecraftception-redstone-pc-chungus

### Registers, accumulators, program counters
- **How it works:** A register is N D-latch/flip-flop cells (or N analog cells) sharing a write-enable; the program counter is a counter + load logic; MAR (memory address register) and ACC (accumulator) are registers. Data buses are one-way because repeaters are directional, so read/write ports need duplicated buses and multiplexers.
- **Size/speed:** CHUNGUS 2 reports 7 general-purpose registers (8-bit data).
- **Version notes:** Same as D latch.
- **Build-hazard:** Each bit's reset state is arbitrary after build; reset sequence required. Bus segments must be built with their drivers last.
- **Sources:** https://minecraft.wiki/w/Tutorials/Redstone_computers ; https://pcgamer.com/minecraftception-redstone-pc-chungus

### Instruction decoders
- **How it works:** A decoder maps opcode bits to control lines using AND/NOR trees (or a ROM lookup); microcoded designs use a ROM; MASIC uses 4-bit opcode + 4-bit operand with fetch-decode-execute control timing and finite-state control (Deep Thought uses a finite-state machine to save space).
- **Version notes:** Gate-level stable.
- **Build-hazard:** Timing slack between control lines; if repeater delays are wrong a CPU hangs. Verify by simulation (see tooling).
- **Sources:** https://minecraft.wiki/w/Tutorials/Redstone_computers ; https://www.minecraft.net/en-us/article/deep-thought

---

## B.8 RAM, ROM, storage-as-memory

### Redstone-cell RAM (locking-repeater / D-flip-flop arrays)
- **How it works:** Each bit is a locking-repeater latch (compact, timing-critical) or a D flip-flop (simpler, larger), addressed by an address decoder with a control bus for read/write; Minecraft memory is effectively non-volatile (state persists across logout). Example: MASIC 8-bit computer has 16 bytes of RAM and a 7-segment display. Large builds use "a few thousand blocks of space".
- **Version notes:** Java repeater locking stable; Bedrock timing differs.
- **Build-hazard:** Memory state after build is undefined; initial values are written by running the machine (outside bot scope). Piston/dropper-based cells are placement-order-sensitive.
- **Sources:** https://minecraft.wiki/w/Tutorials/Redstone_computers ; https://minecraft.wiki/w/Memory_circuit

### Container-based RAM/ROM (barrel, hopper, chest, dropper-lined) - scheme level
- **How it works:** Store words as items in barrels/hoppers; address selects a barrel; a comparator reads the fill level (0-15), an ADC to a 4-bit word per container. Another scheme stores code in dropper/dispenser lines or in the order of items in hoppers. I did **not** find a verified named design or source for barrel-based RAM; treat as a **conceptual scheme (unverified)**. Hoppers can lock based on comparator output so item flow itself is the control signal.
- **Size/speed:** 27-slot barrel gives up to 15 levels from 27 slots; reading latency ~2 gt + item transport 8 gt per hopper hop.
- **Version notes:** Stable; chests may be blocked by conductive blocks above (use barrels).
- **Build-hazard:** Needs bot-supplied item data; dropper/hopper pipes cause item entity spill if leaks. Chunk-boundary: hopper chains crossing unloaded chunks freeze.
- **Sources:** https://minecraft.wiki/w/Redstone_Comparator ; https://minecraft.wiki/w/Hopper (design itself unverified)

### Redstone ROM (diode/repeater/torch matrices, piston/dropper-lined)
- **How it works:** A diode ROM places repeaters/diodes at word/bit crossings: present = 1, absent = 0; torch ROMs invert. Address decoded onto a word line. Quartz/glowstone-free ROM possible with torches; item ROMs use drop lines or barrels.
- **Size/speed:** CHUNGUS 2 reports 4 KiB program storage in 32 pages of 128 bytes; Deep Thought is 40x40x35.
- **Version notes:** Diode ROMs are gate-level stable.
- **Build-hazard:** ROM content = which blocks are placed; schematic fidelity is the program. Missing a single repeater changes code - verify block-by-block.
- **Sources:** https://pcgamer.com/minecraftception-redstone-pc-chungus ; https://www.minecraft.net/en-us/article/deep-thought

### Crafter circuits (1.21)
- **How it works:** 3x3 crafter crafts the recipe formed by its contents on a redstone signal, after a **4 gt** delay; output goes out the front into a container (or drops). Slots can be disabled by clicking (a mask), enabling fixed-shape recipes. Comparator output = number of slots that are disabled or hold an item (0-9). Hoppers/droppers insert from any side, hopper below pulls ingredients not results. Not affected by QC; pistons can't move it. Uses: item-converters, decoders (output item type depends on input pattern), edge-timed gates, ROM-like lookup tables.
- **Version notes:** Stable since 1.21 (pack removed); Bedrock differs on timing.
- **Build-hazard:** Slot toggling is a GUI state - the bot must disable slots in the GUI (not placeable via schematic block data); signal arrives 4 gt after power so a QC/BUD nearby may mis-fire.
- **Sources:** https://minecraft.wiki/w/Crafter ; https://bugs-legacy.mojang.com/browse/MC-265943

---

## B.9 CPUs and famous computers (verify list)

### Verified named computers
- **MASIC (wiki example):** 8-bit, 16 bytes RAM, 7-seg display, 4-bit opcode + 4-bit operand; fetch-decode-execute. https://minecraft.wiki/w/Tutorials/Redstone_computers
- **CHUNGUS 2 (Sammyuri, with Uwerta and StackDoubleFlow):** per PC Gamer: 1 Hz clock, 4-stage pipeline, 8-bit data/16-bit instructions, 256 B RAM, 64 B 8-way data cache, 4 KiB program storage, 6 KB graphics memory, up to 256 I/O ports, 7 GPRs, 40+ ALU functions, GPU "AMOGUS" 96x64 2-colour; ran a Minecraft-in-Minecraft demo; accelerated by StackDoubleFlow's high-performance server (the Minecraft.net piece says it speeds redstone by thousands of times; it doesn't name MCHPRS - however MCHPRS is by StackDoubleFlow). **Not buildable in vanilla survival at normal speed** (1 Hz at what speed unclear: unverified).
- **Deep Thought (n00b-asaurus, 2017):** 4-bit (planned 8), 40x40x35 blocks, AND/XOR/OR/ADD+C, finite-state control, Intel 4004-like.
- **The Redgame 1 (dudearent006, 2011):** 10-bit with GPU; command-block + redstone.
- **SethBling's Atari 2600 emulator (2016):** vanilla Minecraft, extremely slow, 128 B emulated RAM; built mostly with command blocks (command-block aspect unverified in the source fetched).
- **LordBaryhobal "minecraft-computer":** 16-bit, MC 1.15.2+, no mods; instruction set taken from the wiki tutorial; command blocks/datapack for debugging/I-O.
- **Ryan Boulds (2025 news):** 32-bit, 16 registers, ~1.1 M blocks, heavily modded client (Fabric performance mods) - not vanilla-playable.
- **Unverified names:** Ironbark, "MCCPU", BigBoy, Sinec8, Nephalem - **I found no source confirming these as redstone computers**; do not use these names without a source.
- **Version notes:** Pre-1.14 builds often break (update order/BUD changes); large builds validated on 1.16-1.20 may behave differently under 1.21.x only if they rely on 0-tick/QC.
- **Build-hazard:** Any CPU schematic of this class is 10^5-10^6 blocks and often depends on a high-speed simulator or command blocks for debug/IO; survival bots need a block budget and an initial-state reset procedure.
- **Sources:** as listed above plus https://www.minecraft.net/en-us/article/-minecraftception ; https://www.fanaticalfuturist.com/2025/03/geeky-student-makes-a-huge-ai-and-functioning-redstone-computer-in-minecraft/ ; https://github.com/LordBaryhobal/minecraft-computer ; https://www.windowscentral.com/crazy-person-built-full-atari-2600-emulator-using-vanilla-minecraft ; https://www.minecraftforum.net/forums/minecraft-java-edition/redstone-discussion-and/332550-list-of-minecraft-computers

---

## B.10 Displays

### 7-segment, lamp/pixel and piston displays
- **How it works:** A 7-segment display is 7 lamps driven by a decoder (BCD-to-7-seg). Pixel displays use redstone lamps (or copper bulbs) in a grid addressed by row/column; "piston displays" move blocks (e.g. sticky piston arrays of wool/glass) for flip-dot style pixels; scrolling displays move rows with pistons or shift registers. CHUNGUS 2's GPU is a 96x64 two-colour screen.
- **Version notes:** Redstone lamps in Java need >=2-tick off pulse to switch off; copper bulbs toggle (state, not level) so need explicit reset; light level changes with oxidation (15/12/8/4) but not logic.
- **Build-hazard:** Lamp arrays are inert until driven; place drivers last. Piston displays are QC-hazard prone if the piston array sits near powered lines.
- **Sources:** https://minecraft.wiki/w/Tutorials/Redstone_computers ; https://minecraft.wiki/w/Copper_Bulb ; https://minecraft.wiki/w/Pulse_circuit (lamp note)

---

## B.11 Flying machines, tileable and compact designs

### Flying machines (slime/honey engines, observer engines) and pulse transport
- **How it works:** Pistons push a stack of blocks (limit 12 per piston) with slime/honey blocks pulling adjacent non-slime blocks; observers placed on a piston-pushed block fire as it moves, driving the next cycle (observer engine = 2 pistons + slime/honey + observers). Used as long-range signal transport or "carry a pulse," or to reposition blocks. Slime and honey do not stick to each other; glazed terracotta does not stick.
- **Size/speed:** Tiny (~2 pistons + 2 observers + slime).
- **Version notes:** Java and Bedrock differ in observer-on-piston timing (Java: inactive moved observer pulses after its usual delay; Bedrock: always pulses immediately). Behaviour tied to piston push rules; unchanged for 1.21.11 in sources read.
- **Build-hazard:** Flying machines leave chunks (stall at unloaded chunk border) and can wreck the build; they are entity-less but move block-entities (they cannot push vaults/calibrated sculk sensor; Java crafters can't be moved by pistons). A bot must build with the machine's last observer/piston placed after the stopper block, or else it launches. Consider only as an explicit goal.
- **Sources:** https://minecraft.wiki/w/Observer ; https://minecraft.wiki/w/Piston (fetch of Tutorial:Flying_machine failed)

### Tileable / stackable / compact designs
- **How it works:** Wiki flags designs as tileable (e.g. 1-wide QC T flip-flop O, 3x3 piston T flip-flop N, dropper latch P (3x1x2) and "2-tileable" piston inverter; hybrid sorter tiles alternating circuits) which allows dense vertical stacking in storage/RAM. "World-eater"/mega-compact designs were not verified.
- **Version notes:** Tileable QC designs are Java-only and the ones using QC are dead under Redstone Experiments.
- **Build-hazard:** Tile neighbours inherit QC/BUD hazard; spacing exactly as in schematic; a missing isolation block makes adjacent tiles interact.
- **Sources:** https://minecraft.wiki/w/Memory_circuit ; https://minecraft.wiki/w/Logic_circuit

---

## B.12 Signal transmission and multiplexing

### Signal transmission: dust, repeater chains, observer chains, rails, sculk/amethyst, ladders
- **How it works:** Dust carries 15 blocks (strength drops 1/block); repeaters regenerate every <=15 blocks (needed every 15 on staircases and ladders); observer chains transport pulses; powered rails (cheap NC updates) transmit; daylight detector sends downward in 1 tick; sculk sensor vibrations travel 1 block/gt up to 8 (16 calibrated) and calibrated sensors relay via amethyst; bundles-of-wire do not exist in vanilla.
- **Version notes:** Bedrock glass/piston two-way ladders are Bedrock-only.
- **Build-hazard:** Dust connects sideways to anything redstone-compatible - a bot must keep isolation gaps; sculk-based transmission is triggered by the bot's own movement.
- **Sources:** https://minecraft.wiki/w/Redstone_circuit ; https://minecraft.wiki/w/Sculk_Sensor ; https://minecraft.wiki/w/Calibrated_Sculk_Sensor

### Redstone "compression", multiplexing, buses
- **How it works:** No bundled wires in vanilla. Multiplexing in practice: (a) analog multiplexing - one wire carries a strength 0-15 (4 bits) decoded by comparators; (b) time-division multiplexing on a shared bus gated by AND gates / tri-state-like piston gates; (c) comparator-compressed memory (container fill level = word); (d) sculk frequency multiplexing (16 channels on the calibrated sensor, wireless). The wiki's "bundle" talk is irrelevant (bundles are an item).
- **Version notes:** All stable; analog sources grew in 1.21.x.
- **Build-hazard:** Analog buses lose strength with distance; repeaters reset to 15, so analog values need comparator-only paths.
- **Sources:** https://minecraft.wiki/w/Redstone_Comparator ; https://minecraft.wiki/w/Calibrated_Sculk_Sensor

---

## B.13 1.21 trigger/sensor blocks used in logic

### Wind charge, breeze, trial vault/spawner as triggers (1.20.3+/1.21)
- **How it works:** A wind burst presses buttons, flips levers, non-iron doors/trapdoors, fence gates; rings bells; extinguishes candles; shatters decorated pots; breaks chorus flowers. Dispensers fire wind charges (2.4-block burst), breezes fire `breeze_wind_charge` (6-block). Vault and trial spawner have no direct redstone input in the sources read; a vault cannot be pushed by pistons.
- **Version notes:** Java changes listed 1.20.3-1.21.2; nothing for 1.21.11 found.
- **Build-hazard:** Breeze/wind-charge triggers can fire logic remotely and shatter decorated-pot memory; keep mob spawn lit and doors iron.
- **Sources:** https://minecraft.wiki/w/Wind_Charge ; https://minecraft.wiki/w/Vault

### Creaking heart, lightning rod, copper golem statue, target block, daylight detector (analog sensors)
- **How it works:** Creaking heart gives comparator strength by creaking distance; copper golem statue emits a signal based on pose; lightning rods (incl. oxidation variants since 1.21.9) conduct strikes; daylight detector yields analog light level; target block outputs by hit position (gk). All feed analog decoders.
- **Version notes:** Creaking heart (Winter Drop pack in 1.21.2 -> 1.21.4); copper golem statue 1.21.9.
- **Build-hazard:** Environmental triggers (weather, mobs) are uncontrollable; avoid as mission-critical inputs.
- **Sources:** https://minecraft.wiki/w/Java_Edition_1.21.2 ; https://minecraft.wiki/w/Java_Edition_1.21.9

---

## B.14 Simulation and design tooling

### MCHPRS (Minecraft High Performance Redstone Server) + Redpiler
- **How it works:** A multithreaded creative server for redstone (512x512 plot per thread) with a redstone compiler ("Redpiler") that precomputes connections for large speed-up; ships a reimplemented WorldEdit subset; adjustable redstone tick rate (`/rtps`), `/radvance`. README: Minecraft **1.20.4** only; many vanilla features absent; `--optimize` can break builds; block edits reset Redpiler.
- **Version notes:** README lists 1.20.4 only - not 1.21.11; its redstone semantics = 1.20.4 classic (no Experiments pack). Pistons/sculk etc. may be incomplete.
- **Build-hazard:** Designs validated in MCHPRS may still differ in vanilla (update order, tick phase, QC/0-tick edge cases); treat as a pre-check, not proof. Use it to test schematics before the swarm builds them; re-test unusual designs on a vanilla world.
- **Sources:** https://github.com/MCHPR/MCHPRS

### Redstone Tweaks and Alternate Current (mods; not for the vanilla server)
- **How it works:** Redstone Tweaks (Fabric, MC 1.16.3-1.16.5 only, last updated ~5 years ago) lets creators change delays, tick priority, piston behaviour, power limits, update order and QC direction (debugging/slow-tick/neighbour-update visualisers). Alternate Current replaces dust with a network-based algorithm (claims up to ~20x lower MSPT contribution with high vanilla parity; fixes MC-11193).
- **Version notes:** Redstone Tweaks is old - do not apply to 1.21.11. Alternate Current supported versions unconfirmed in README fetched.
- **Build-hazard:** Servers running Alternate Current change update order for dust; designs built on vanilla ordering may break; surface the server's mods before building.
- **Sources:** https://modrinth.com/mod/redstonetweaks ; https://github.com/SpaceWalkerRS/alternate-current

### Logisim / Yosys / PERSHING / dewey (netlist -> redstone place-and-route)
- **How it works:** Verilog -> Yosys -> BLIF netlist -> PERSHING places and routes it as a redstone layout (placement by simulated annealing and Lee-style routing in dewey) -> `inserter.py` writes into a Minecraft world; dewey (SIGTBD'18 paper by Quan Nguyen) is a faster C rewrite (2-bit adder 8.8 s vs 117 s; 4-bit counter 31.9 s vs 1084 s). PERSHING is Python 2.7 and no MC version stated; the paper does not discuss ticks.
- **Version notes:** Research-grade, old toolchain (2018); output not guaranteed for 1.21.11 (repeater delay assumptions).
- **Build-hazard:** Generated layouts rely on repeater buffering and cell library timing; re-simulate in MCHPRS before the bot builds. Logisim-style designs must be mapped to Java-timed gates.
- **Sources:** https://github.com/89Mods/pershing ; https://www.ocf.berkeley.edu/~qmn/assets/nguyen_dewey_2018.pdf

---

## B.15 Not-found / gap list (for the planner)
- No source located for: Ironbark, MCCPU, BigBoy, Sinec8, Nephalem (unverified names); hopper/barrel-RAM named designs; verified carry-lookahead adder schematics; verified 2025-26 r/redstone, r/technicalminecraft, r/mccpu pages (Reddit was blocked from the fetcher; searches returned mostly SEO pages); Technical Minecraft Wiki/Fandom pages (not reached).
- Cannot confirm from the sources read whether Redstone Experiments is toggleable on a dedicated server after world creation.
- Confirm unchanged status of piston/observer/comparator timings in 1.21.11: no change reported in the 1.21.11 article/wiki text I read, but the last ~4 KB of the release article was not read.


---

## C. SMP-specialized tech

**Research notes / confidence legend.** Compiled 2026-10-13. Fandom (Technical Minecraft Wiki, Minecraft-PvP wiki, Unstable Universe wiki), Reddit, and YouTube pages were blocked or rate-limited from this environment (YouTube returned HTTP 429, so video descriptions could not be read). Most mechanics below are therefore sourced from minecraft.wiki, Paper docs, GitHub, Planet Minecraft/mineschematic/buildpaste/Modrinth pages. Items marked **(unverified)** come from general community knowledge or low-reliability sources and were not confirmed by a page I could read. Version naming: Mojang moved to year-style names after 1.21.11 (sources mention 26.1, 26.2, 26.3, 26.4); the target server is 1.21.11 (wiki lists its snapshots as 25w41a–25w42a era, adds spears and the `fire_spread_radius_around_player` gamerule). "No op / no commands" means you cannot change gamerules, so everything the wiki says is gated by a gamerule (e.g. `ender_pearls_vanish_on_death`, `tntExplodes`, `mobGriefing`, entity cramming) is at server default.

Key sources used repeatedly: minecraft.wiki pages (Wind_Charge, Breeze, Ender_Pearl, Mace, Spear, Elytra, TNT, Explosion, Trial_Spawner, Respawn_Anchor, End_Crystal, Bed, Nether, Bedrock, Ice, Java_Edition_1.21.11, Tutorial:TNT_cannons, Tutorial:Chunk_loader, Tutorial:Block_and_item_duplication, Tutorial:Griefing_prevention, Tutorial:Defense, Tutorial:Breeze_farming); Paper global-config docs https://docs.papermc.io/paper/reference/global-configuration/ ; https://syntaxmine.com/articles/ender-pearl-chunk-loader-26-3 ; https://wiki.legendaryhc.org/Ender_Pearl_Stasis_Chamber ; https://github.com/6b6t/feedback/issues/7 .

---

## C.1 Orbital Strike Cannon (Mk.x documented, latest breeze-driven generation)

### Orbital Strike Cannon (OSC), cubicmetre designs, Mk.1 to Mk.6.1
- **How it works.** A huge redstone/entity TNT artillery system (not a conventional ballistic cannon). Per the Planet Minecraft listing (https://www.planetminecraft.com/project/orbital-strike-cannon-4-3-by-cubicmetre/): targets and payload type are set via a linked Google Sheet from origin and target coordinates, the charge cycle takes 2 to 10 minutes, then the operator holds a fishing rod on a pressure plate, travels through a portal to the target area, and clicks the rod to release the payload ("lag indicates the payload has been released"). Payloads: **Stab** ("small but deep hole") and **Nuke** (clears roughly a 50-block radius). Grokipedia (low reliability, https://grokipedia.com/page/Orbital_Strike_Cannon_Minecraft) adds that it needs chunk loaders linked through the Nether, ammunition comes from TNT dupers, and a "nuke builder" lets the operator program warhead shape. The detailed internal delivery mechanism (honey-block/entity alignment, portal-based payload placement above the target) is **unverified** here because the YouTube descriptions could not be read.
- **Typical numbers.** Mk.6.1 schematic: 58 x 68 x 47 footprint, 13,501 blocks, 55 block types, mostly white concrete, activator rail and observers (https://mineschematic.com/s/the-orbital-strike-cannon-mk-6-1-40cb8504). Buildpaste copy of "orbital strike cannon" by cubicmetre: ~242k blocks, posted 2026-03-04, version not listed (https://buildpaste.net/build/orbital-strike-cannon-tgCb6y6hjmkMdC7Fc920). Grokipedia: shots use "hundreds to thousands of primed entities"; a minimum-range test at 100 blocks used 171 primed TNT and finished in 13 s; Mk.6 nuke builder uses about half the TNT of earlier designs.
- **Version notes (per sources).** Mk.1 ~March 2023 (proof of concept); Mk.2 "original nuclear strike cannon"; Mk.4 Dec 2023 replaced small TNT dupers with larger "10-TNT dupers" and moved control inputs to the Nether; Mk.5 scrapped because "1.21.4" changes broke entity/honey-block alignment (Grokipedia, undated, unverified); Mk.6 full rebuild for 1.21.1, "compatible with 1.21 or higher"; Mk.6.1 replaced the Mk.6 schematic with "exactly the same features but with a fix" (cubicmetre YouTube community post, text truncated, https://www.youtube.com/post/UgkxEipS9kSjq2LT1_qXiI8bN4vnh1rR9Luj). The mineschematic page lists Mk.6.1 as made for **Java 1.21.11** ("works in 1.21+"). Planet Minecraft "4.3" is the older design, "made for 1.19.4 and above".
- **Server/SMP note.** It was built for and used on Lifesteal-style SMPs (Wemmbu, OnlyASquid, Minute Tech per Grokipedia) with admin cooperation or lax rules. On a Paper server TNT dupers are disabled by default (`allow-piston-duplication: false`), so the ammo supply would not work; Paper-style servers also often add TNT/explosion griefing limits. A vanilla no-op server allows it mechanically, but it needs ~13.5k blocks placed with Litematica/WorldEdit-style tools (not available to survival bots without a printer), portals and always-loaded chunks.
- **Bot-relevance.** Not buildable from scratch by a survival bot swarm in practice (13.5k precise blocks + dupers + Nether chunk-loaders); worth knowing mainly as a thing enemy factions may fire at you (huge crater, lag spike on release).

### Latest OSC design (Orbital Skybender): breeze-driven delivery, as described by Allen
- **How it works (per Allen, 2026-10-13; project owner's account, not corroborated by any web source read).** The latest OSC generation, also called the Orbital Skybender, packs **256 breezes into a 1x2 area** and uses the **wind charges the breezes throw** to deploy the payload to its destination. The **payload is built with TNT dupers**. Allen has also said the Skybender is proven and has dual chunk loaders, so its mechanics should be trusted; the constraint list below says what an operator-side builder has to satisfy, not whether the design works. Treat this as the project's working description of the current design. It replaces the "rod / pressure plate / portal-handoff" delivery described for the Mk.x designs in the entry above, which the web sources document.
- **What the web search found (for context, not a contradiction).** No page the research agent could read mentions breezes or wind charges together with the OSC. Checked: mineschematic Mk.6.1, buildpaste, Planet Minecraft 4.3, Grokipedia, the Modrinth datapack page and the Spigot plugin page. "Orbital Skybender" returned no relevant results in three searches. YouTube and Fandom were unreadable (HTTP 429 / blocked), and the newest design is likely documented only in videos, so the gap is more likely a source-coverage problem than a sign the design is wrong. The newest documented version is cubicmetre's Mk.6.1 (1.21.11 schematic).
- **Mechanics the design must satisfy (from sourced vanilla behaviour; these are constraints for a builder, not objections).**
  - Breezes shoot only at players or iron golems within **16 blocks**, once about every **32 ticks** (https://minecraft.wiki/w/Breeze). The design therefore needs a target stand-in near the breezes and a way to aim them, or a different trigger.
  - 256 mobs in a 1x2 footprint exceeds the default **entity-cramming limit of 24** (general knowledge, not found in a fetched source). A no-op server cannot change the gamerule, so a survival build would have to keep them from taking cramming damage some other way, or the design relies on a gamerule setting. Unresolved here.
  - Wind-charge explosions push entities (explosion knockback; the wiki's Explosion and Wind Charge pages give different multipliers, so flag the numbers; https://minecraft.wiki/w/Wind_Charge). Pushing primed TNT with wind charges is plausible from standard knockback, but no fetched page states it.
  - Breezes spawn from trial spawners in trial chambers (vault/spawner blocks are immovable), so 256 breezes means either an in-place spawner farm or a way to move captured mobs. Breezes drop breeze rods only when killed.
  - No source gives TNT count, range or exact version for the breeze design. Ask Allen for the schematic or video link if any of these numbers are needed.
- **Bot-relevance.** Not buildable by a survival bot swarm without a schematic-grade universal builder plus a breeze farm and TNT dupers. For the framework it is an operator-built, operator-marked structure (see the design principle: the framework has no built-in cannon call). Its Nether chunk-loaders and the lag spike on release matter for fleet placement and for what enemy factions might fire at us.

---

## C.2 TNT: cannons, dupers, wall breaking

### TNT cannon fundamentals (single, double, multi, sand, "tunneler", etc.)
- **How it works.** Primed TNT is accelerated by explosions of "booster" TNT detonating at a launch point (platform on blocks, or mid-air ejector). Because TNT explosion knockback has no cap per tick beyond ~1 block/tick per exposure, stacking boosters yields very long ranges. The Minecraft Wiki catalogue (https://minecraft.wiki/w/Tutorial:TNT_cannons) lists: loading styles (underfoot, overhead, crossbow, semi-automatic, double cannon, blockstream, preloaded, machine gun, duper, dispenser); damage patterns (shotgun = all TNT at one spot, ripple, mortar, scatter, nuke, multi-impact, **tunneler** "often most effective"); trajectories (ascend, descend, artillery, chunk phasing); special-purpose (piercing, anti-cannon, playerkill). "Lazy acceleration": projectiles in lazy chunks (tick blocks but not entities) let cannons charge for longer than 4 s with fewer TNT loaded.
- **Typical numbers.** Wiki gives no numeric ranges except player/pearl cannons "hundreds to tens of thousands of blocks". Primed TNT fuse 80 ticks; TNT power 4 (https://minecraft.wiki/w/TNT).
- **Version notes.** Wiki page lists no 1.21+ changes. Relevant 1.21.x TNT changes: 1.21 primed TNT can travel through Nether portals but cannot destroy portal blocks after pre4; 1.21.2 TNT stopped destroying item frames/paintings/armor stands in water and `explosion_power` field added; 1.21.5 `tntExplodes` gamerule added and default `fuse` for commanded TNT 0 to 80; 1.21.6 TNT again destroys items/armor stands in water (https://minecraft.wiki/w/TNT).
- **Server/SMP note.** The wiki's own griefing page says "restricting TNT makes cannons useless" (plugin region protection). Most claim plugins block explosion damage inside claims; the `tntExplodes`/`mobGriefing` gamerules can disable it entirely. On a no-op vanilla anarchy-style server everything works.
- **Bot-relevance.** Medium: bots can place dispenser-fired TNT or flint-and-steel TNT; an actual aimed cannon needs precise layout and is heavy for a swarm to construct on demand.

### Sand / falling-block cannons
- **How it works.** Falling sand/gravel/concrete powder/anvils are launched by TNT like TNT entities; they fly "for usually less than four seconds" (wiki: https://minecraft.wiki/w/Tutorial:TNT_cannons), so range is shorter than TNT, but the projectile does not self-detonate and can damage players (anvils, dripstone), or be used to fill/overwrite.
- **Typical numbers.** <4 s flight; anvil/dripstone damage per wiki.
- **Version notes.** Concept unchanged. Paper's `allow-unsafe-end-portal-teleportation` (default false) specifically blocks the end-portal sand duplication used to feed sand cannons on Paper.
- **Server/SMP note.** Used for covered-wall bombing and for sand-walling; sand duplication is bannable on most SMPs.
- **Bot-relevance.** Low-medium; cheap payload if sand is plentiful but accuracy needs engineering.

### Ender pearl cannons / player cannons (360 FTL, wind-charge launchers)
- **How it works.** TNT-boosted player or pearl cannons "launch players up to tens of thousands of blocks", with TNT minecarts or pigs for lazy acceleration (wiki TNT cannons). The classic "360 degree ender pearl cannon / FTL" (designed March 2019) aims by rotating with a pearl as payload and uses chunk loading to land a pearl remotely (https://minecraft.wiki/w/Tutorial:360_degree_ender_pearl_cannon). A wind-charge variant exists: four inward-facing dispensers loaded with wind charges around a slab launch the player ~68 blocks with no TNT (https://beebom.com/minecraft-player-cannon-wind-charge-design/; version not stated, 1.21+).
- **Typical numbers.** Wind-charge launcher: 68 blocks (single source). Pearl: 5 HP fall/teleport damage, 1 s cooldown (https://minecraft.wiki/w/Ender_Pearl).
- **Version notes.** 1.21.2 pearls load chunks on throw/landing, pearls no longer pass through world border (24w34a). Teleport formula experiment reverted in 24w37a.
- **Server/SMP note.** Anarchy servers allow; claim servers often block pearl-in-claims or teleport commands (plugin level, unverified). Fall damage rule: only 5 HP pearl damage if you throw before hitting ground.
- **Bot-relevance.** Medium: pearls are the standard bot escape (cooldown 1 s) and long-range travel; building cannons is not.

### TNT duplicators ("TNT dupers"): piston-based, observer/slime variants
- **How it works.** Wiki (https://minecraft.wiki/w/Tutorial:Block_and_item_duplication): "Piston-based TNT duplication, Java 1.14+; patched on Paper (1.15.2 build #358, can be disabled); vanilla not patched, left in on purpose." A piston pushes a TNT block so that a primed TNT entity is created without removing the TNT block (one design: minecart on detector rail with a coral fan). Variants use observers, slime/honey blocks as flying machines (YouTube design titles: "TNT duper 1.21.5", "Fastest TNT duper 1.21+", "best TNT duping flying machine 1.21.4"; descriptions not readable, so mechanism details **unverified**). Sportskeeda notes a compact, slime-block-free design attributed to Mercury or Ianxofour that works only in Java, due to its dependence on dead coral fans (no version stated, https://sportskeeda.com/minecraft/minecraft-player-shares-simple-tnt-duper-design).
- **Typical numbers.** Not documented in my sources. Mk.4 OSC used "10-TNT dupers" (Grokipedia). Rates are limited by fuse time (80 ticks) and lag.
- **Version notes.** Working status in vanilla 1.14 through at least 1.21.x (wiki says unpatched/tolerated; no 1.21.11-specific statement found, so state as "no evidence of a patch in 1.21.x; test"). Grokipedia: "TNT duplication: patched several times, then tolerated as vanilla behavior." Rail and carpet piston dupes also still work in 1.21+ (Grokipedia, low reliability). Bedrock differs.
- **Server/SMP note.** Paper/Spigot/Folia forks: `allow-piston-duplication` = false by default (covers TNT, carpets, rails; not sand) (https://docs.papermc.io/paper/reference/global-configuration/). So most plugin servers (and any host that uses Paper) disable it; pure vanilla jar or anarchy servers allow. Many SMPs ban duping even where mechanically possible.
- **Bot-relevance.** High if the server is vanilla and builds are allowed: an automatic TNT supply multiplies firepower; but building the machine requires precise placement; check Paper vs vanilla via server brand first (F3 / `Server brand` in the handshake).

### Wall-breaking: obsidian breaching, "tunneler" cannons, wither wall breakers
- **How it works.** Obsidian/crying obsidian/ancient debris/reinforced deepslate/anvil/enchant table/respawn anchor all have blast resistance 1200, so single TNT (power 4) can't reliably break them; bedrock is 3,600,000 (https://minecraft.wiki/w/Explosion). Breaching approaches: (1) stacked simultaneous TNT ("shotgun"/"tunneler" cannons that detonate in a column to dig a shaft, https://minecraft.wiki/w/Tutorial:TNT_cannons); (2) "Netherite buster" missiles; (3) water/lava around tricks: water neutralises block damage, while ender chests and enchanting tables are only protected from weak explosions; (4) mining instead with netherite pickaxe/efficiency V haste; (5) wither explosions (power 7 on spawn, skulls power 1) for specific walls (unverified numeric).
- **Typical numbers.** Craters ~4/3 of power in radius; obsidian needs many overlapping explosions, a "tunneler" is the efficient route.
- **Version notes.** No 1.21.x change to blast resistances found in sources. Bed (power 5) and charged creeper/end crystal (6) are the strongest cheap sources.
- **Server/SMP note.** Claim plugins stop block damage inside claims; anarchy servers allow. Water-logged blocks are explosion-immune (wiki griefing page).
- **Bot-relevance.** Medium: for raiding a base, bots can simply mine obsidian with diamond pickaxes in parallel (swarm advantage); TNT is faster only with the cannon.

---

## C.3 Wind charge, breeze, mace, spear tech (1.21+)

### Wind charge basics and block triggering
- **How it works.** Wind charge: 10-tick (0.5 s) use cooldown; direct hit 1 HP; burst knocks back nearby entities without damage; player/dispenser charge burst 2.4-block sphere, breeze burst 6 blocks; flips non-iron doors, trapdoors, fence gates, levers, presses buttons, rings bells, extinguishes candles, breaks chorus flowers, shatters decorated pots (https://minecraft.wiki/w/Wind_Charge). Dispensers fire them with a cone of random offset.
- **Typical numbers.** Aiming straight down launches ~6 blocks (pre1 1.21; was ~2.5); fall damage reduced only below burst Y.
- **Version notes.** Java 1.21 (24w18a) out of experiment; 1.21-pre1 crouch no longer matters, straight-down launch ~6 blocks; 1.21.2 charges stop at world border and have `use_cooldown` component; wiki has no 1.21.11-specific entry.
- **Server/SMP note.** Door activation lets you open non-iron doors from range (trapdoor/door bypass of "closed door" defences; lever/button flips mean redstone triggers remotely). Claim plugins that protect doors via interaction events may or may not cover projectile-triggered flips (**unverified**).
- **Bot-relevance.** High: cheap mobility burst + breach of wooden doors/trapdoors/buttons from range.

### Wind-charge elytra/launch mobility, wind-charge stasis
- **How it works.** A wind charge thrown at your feet launches ~6+ blocks upward; chained with the 10-tick cooldown, players climb quickly. Combined with elytra: gain height with wind charge then deploy elytra and use fireworks (Elytra: firework boost ~33.5 b/s; https://minecraft.wiki/w/Elytra). "Wind-charge stasis" (storing a wind-charge dispenser as a reusable trigger) is community terminology; I could not find a sourced definition, so **unverified**.
- **Typical numbers.** 68-block flight with dispenser quad launcher (single source above). Elytra 432 durability, 1 per second; glide ratio ~10.06:1.
- **Version notes.** 1.21.5: climbable blocks cancel gliding; 1.21.11: gliding through vines allowed (`#can_glide_through`).
- **Server/SMP note.** Fall damage negation reworked; anti-cheat plugins may flag large velocity; Grim claims movement simulation including knockback (https://github.com/GrimAnticheat/Grim), though nothing is said about wind charges.
- **Bot-relevance.** High: vertical movement with no blocks; also escape from ground ambushes.

### Mace smash combat, Wind Burst enchantment
- **How it works.** Smash attack triggers after falling 1.5+ blocks while not grounded; bonus damage 4 HP per block for the first 3, 2 HP for the next 5, 1 HP after; crit +50%; Density adds per-block; Breach adds armor-piercing; Wind Burst launches the attacker; smash resets fall distance, nullifying fall damage; entities within 2.5 blocks are knocked back; Slow Falling disables; in Java, elytra flight also blocks smash (must unequip) (https://minecraft.wiki/w/Mace). Standard combo: wind charge up, then fall and smash.
- **Typical numbers.** Java weapon attack speed 0.6. Damage scales with fall height; Breach IV is highest vs diamond/netherite armour in Java (wiki).
- **Version notes.** Wiki gives no version history in the text I got. Mace is trial-chamber loot (vault/ominous vault, https://minecraft.wiki/w/Trial_Spawner); not craftable.
- **Server/SMP note.** Many servers cap or ban mace; anti-cheat can flag "fall damage" patterns. Vanilla allows.
- **Bot-relevance.** Very high for a faction: mace is the strongest burst melee; the swarm can ambush from above, but needs wind charges and reach logic.

### Spears (new in 1.21.11) and lunge
- **How it works.** Seven tiers; jab (low damage, knockback, multi-hit) and charge (damage = tier multiplier x relative speed; thresholds 4.6 b/s for damage, 5.1 b/s attacker speed for knockback; stage 1 dismounts). Range 2 to 4.5 blocks (min reach 2). Java spears can knock back primed TNT horizontally. Lunge enchant propels forward on jab, costs hunger and durability (https://minecraft.wiki/w/Spear, https://minecraft.wiki/w/Java_Edition_1.21.11).
- **Typical numbers.** Jab damage Java: wood 1, stone/copper 2, iron 3, diamond 4, netherite 5.
- **Version notes.** Added in 1.21.11, i.e. present on the target. Charge damage scales with relative speed, so elytra/horse/boat speed attacks matter; elytra interaction is not stated.
- **Server/SMP note.** Newest weapon; fewer plugin countermeasures; unknown if anti-cheat reach rules treat 4.5 range as cheat (**unverified**).
- **Bot-relevance.** High: mounted/charging swarm attacks; also pushing TNT away.

### Breeze mob facts and breeze rods
- **How it works.** Breeze: fires a wind charge every 32 ticks (30 on Bedrock) at players/iron golems within 16 blocks; hitbox 0.6 x 1.77; spawns only from trial spawners in combat chambers with line of sight; drops 1 to 2 breeze rods (Looting III 4 to 8) only to player/tamed wolf kills (https://minecraft.wiki/w/Breeze). Breeze rods craft wind charges and mace-adjacent items.
- **Typical numbers.** Breeze farming "around 32 breeze rods per hour" (Rays Works design on https://minecraft.wiki/w/Tutorial:Breeze_farming).
- **Version notes.** In 1.21.x breeze burst has multiplier 0.6 (Explosion page).
- **Server/SMP note.** Trial chambers are unprotected natural structures, but servers with claim plugins often exclude them; mobs spawning in them is not changed.
- **Bot-relevance.** Medium: needed to craft wind charges, and trial chamber farming is a contested resource zone.

### Trial chamber / trial spawner farms
- **How it works.** Trial spawner activates for survival players within 14 blocks with line of sight; default limits 6 total, 2 simultaneous mobs, +2/+1 per extra player; 30-minute cooldown that keeps counting while unloaded; mobs in unloaded chunks aren't counted toward the simultaneous limit, so mobs accumulate; loot per player; ominous spawners via Trial Omen (https://minecraft.wiki/w/Trial_Spawner). Farm designs manipulate line of sight and kill via magma blocks or campfires (Tutorial:Breeze_farming).
- **Typical numbers.** 1 spawn attempt / 2 s, 50% key chance per player.
- **Version notes.** Java 1.21 (24w18a) onward; no 1.21.11 change in the text I read.
- **Server/SMP note.** Per-player loot means swarms of bot accounts multiply loot; servers with alt limits may see this as abuse.
- **Bot-relevance.** High for a swarm: each extra bot raises spawn limits and loot per run (keys, wind charges, mace via vaults).

---

## C.4 Ender pearl tech

### Ender pearl stasis chamber (trapdoor + bubble column)
- **How it works.** A pearl is held in flight by an upward bubble column (soul sand at the bottom of water; Legendary HC wiki: 11-deep hole, soul sand on the 11th block, 10 water blocks, kelp+bone meal as filler). A pearl teleports its thrower only when it contacts a solid block, so the bubble column keeps it suspended; a trapdoor (or water-removal circuit) is triggered, the pearl falls, and the thrower is teleported. Remote designs trigger when the owner logs in (https://wiki.legendaryhc.org/Ender_Pearl_Stasis_Chamber).
- **Typical numbers.** Pearl does 5 HP damage on teleport; 1 s cooldown.
- **Version notes.** Java only; Bedrock behaves differently. 1.21.2 added chunk loading on throw/landing. Chambers must be inside the loaded chunk (5x5 loading area, only the centre chunk is fully ticking; the other 8 are lazy) (https://syntaxmine.com/articles/ender-pearl-chunk-loader-26-3; https://minecraft.wiki/w/Ender_Pearl).
- **Server/SMP note.** Pearls vanish when the thrower logs out and return at login; pearls vanish on thrower death by default (`ender_pearls_vanish_on_death`; Paper calls it `enderPearlsVanishOnDeath`). On Folia/Paper forks a bug removes a stasis pearl about 2 s after owner teleport (Folia #421 open since 2025-12-27 on 1.21.11, #478 on 26.1.2; https://github.com/6b6t/feedback/issues/7). Legendary HC notes its server moved to Folia in Dec 2025 which broke frost-walker designs.
- **Bot-relevance.** Very high: standard instant-teleport home for a faction; each bot can carry its own stasis pearl. Constraint: pearls only exist while the owner is online, so a bot that disconnects loses the chunk-loader function.

### Ender pearl chunk loader (personal)
- **How it works.** A pearl creates chunk tickets when entering a new chunk and about every 39 ticks, loading 5x5 chunks (centre fully, 8 surrounding lazy). Continues while the thrower is online, even in another dimension; pauses at logout (https://minecraft.wiki/w/Ender_Pearl; https://minecraft.wiki/w/Tutorial:Chunk_loader).
- **Typical numbers.** 5x5 chunks; centre ticking only; mob spawning still needs a nearby player (128 blocks natural, 16 for spawners).
- **Version notes.** Added in 1.21.2 per SyntaxMine (the wiki history also lists 24w36a/37a: "pearls began loading chunks when thrown and when landing"). SyntaxMine states behaviour unchanged through 26.3. I found no evidence that pearl loading was removed in a snapshot up to 1.21.11 (the user's question about removal: not supported by any source; flagged **unverified/negative result**).
- **Server/SMP note.** Plugin servers can set pearl limits; Paper's own handling of owner removal is buggy on Folia. Chunk loading is per-owner, so it doesn't help offline farms.
- **Bot-relevance.** High: lets a bot keep a base chunk simulated (furnaces, hoppers, redstone) while online.

### Pearl-trapping / pearl-kill defence
- **How it works.** Because pearls belong to the owner and die with them (by default), killing a thrower removes all their in-flight pearls (wiki Ender Pearl); an enemy who knows your stasis location can destroy the chamber (break the trapdoor/column) to waste the pearl. Conversely, pearl-landing in a base is checked by solid collision.
- **Typical numbers.** n/a.
- **Version notes.** `ender_pearls_vanish_on_death` gamerule exists in 1.21.x; 1.21.2 pearls no longer pass world border.
- **Server/SMP note.** Gamerule is op-controlled; with no op it stays default (vanish on death).
- **Bot-relevance.** Medium: do not rely on a pearl when the bot dies; keep two-step safe retreat.

---

## C.5 Base defence, traps, alarms

### Beds and respawn anchors as bombs / traps
- **How it works.** A bed used outside the Overworld explodes with power 5 (stronger than TNT, weaker than crystal) centred on the bed head and ignites the player; respawn anchors set as spawn in the Overworld/End while charged explode at power 5 and set fire (https://minecraft.wiki/w/Bed; https://minecraft.wiki/w/Respawn_Anchor). Anchor holds up to 4 glowstone charges; blast resistance 1200; dispensers can charge anchors; shields block the explosion since 1.19.3. Trap use: a player lures enemies to interact with a pre-placed bed/anchor or detonates them as "bed bombs" for AoE vs. obsidian/stasis.
- **Typical numbers.** Power 5 each (bed), anchor 5 (up to multiple stacked for ~obsidian-breaking when many are used; **unverified**). End crystal 6 for comparison.
- **Version notes.** 1.21: charges consumed on exit; 1.21.2: no longer consumed on exit portal.
- **Server/SMP note.** Mojang added `respawnBlocksExplode` gamerule on Bedrock only (wiki); Java has no such gamerule in the text I read. Many SMPs ban bed bombing; anarchy servers use them heavily.
- **Bot-relevance.** Medium-high: cheap AoE for swarm raids; also used to dig down through low-blast-resistance blocks.

### End crystal PvP and base breaching
- **How it works.** End crystal explosion power 6, instantly on any damage; placed only on obsidian/bedrock with two air blocks above and no entity intersection; shields block it (1.19.3+); water reduces the blast effect but not damage/knockback; an explosion that damages another crystal makes it disappear without exploding in Java (https://minecraft.wiki/w/End_Crystal).
- **Typical numbers.** Power 6; a crystal needs 1 obsidian; cost ~ 7 glass + eye of ender + ghast tear.
- **Version notes.** 1.21 pre1 crystals could explode on placement when fire-damaged (fixed pre2); 1.21.9 particle change; rarity back to Common in 1.21.2.
- **Server/SMP note.** Most competitive servers fight with crystals; anti-crystal-spam plugins exist in plugin servers; the wiki notes `mobGriefing` false stops block damage.
- **Bot-relevance.** High but hard: precise timing; bots can scale it by volume (many crystals around a base).

### Totems of Undying and auto-armour
- **How it works.** Standard crystal/anchor PvP uses totems to survive one lethal hit. Auto-armour (swapping armour automatically when it breaks) is a hacked-client module; it is client-side automation of inventory and is **not vanilla** (module names are from 2b2t glossary, https://2b2t.miraheze.org/wiki/SMIB, which lists hacked client modules generally).
- **Typical numbers.** n/a.
- **Version notes.** Not covered by sources.
- **Server/SMP note.** Anticheat (Grim, Vulcan etc.) flags rapid inventory actions. On no-mod servers, an automated bot is functionally a modified client.
- **Bot-relevance.** Very high: bots can implement equip logic natively via packets (inventory actions), but must respect click delays to avoid anti-cheat.

### Obsidian, crying obsidian, bedrock-level and hidden bases
- **How it works.** Blast resistance 1200 for obsidian/crying obsidian/ancient debris/reinforced deepslate; bedrock 3.6 million; waterlogged blocks are explosion-immune; wiki's player tips include building far from spawn and roads, hidden/underground bases, ender chests and shulker boxes, decoy bases, building in the Nether (https://minecraft.wiki/w/Tutorial:Griefing_prevention). Nether bedrock: 5 layers at bottom (y0 to 4) and top (y123 to 127); y127 is solid; Java build limit 256 in the Nether lets players build 128 above the roof (https://minecraft.wiki/w/Nether). Java 1.18+ Overworld bedrock at y -64 to -60.
- **Typical numbers.** Nether roof access gives +128 blocks of build space above the roof, unloaded by most players.
- **Version notes.** Bedrock breaking exploits ("headless pistons") are patched on Paper (`allow-headless-pistons`, `allow-permanent-block-break-exploits`, both default false).
- **Server/SMP note.** Anarchy servers: roof and floor bases common; plugin servers: forced via `allow-permanent-block-break-exploits=false`, and usually Nether roof access is blocked by plugin ("nether roof prevention").
- **Bot-relevance.** High: bots can mine obsidian and dig very deep without fatigue; hiding bases far from spawn reduces discovery.

### Water/lava moats, trapdoors on liquid, cobweb, mob-based defence
- **How it works.** Walls at least 3 high with overhang, clear top; water or lava on the outer face; moats 3 deep+wide; cobweb ditches; magma and burning netherrack; carpet over string to confuse mob pathing; dispenser arrow traps on pressure plates; kill mine = upward dispenser with harming potions on pressure plates; fireworks launchers vs. elytra attackers; iron golems in enclosed trench; snow golems (https://minecraft.wiki/w/Tutorial:Defense). "Trapdoors on liquid" is not covered by that page, so **unverified**.
- **Typical numbers.** n/a.
- **Version notes.** Wiki flags "cheap doorway trap" invalid since 1.13; otherwise unchanged. Wind charges (1.21+) can flip trapdoors/doors from range, defeating door-only defences.
- **Server/SMP note.** Player-trap bans: most plugin servers disallow "trap kill" or stunts that don't give a chance to escape.
- **Bot-relevance.** Medium: a bot swarm can quickly dig moats/place cobweb as a rally defence.

### Redstone / sculk / tripwire alarms
- **How it works.** Pressure plates connected to note blocks, observers/tripwire hooks, sculk sensors listening for vibration, plus alarms into hoppers/lights; "chunk-border and render-distance tricks" are about hiding where you can't be seen: enemies at the render distance see your lit chunks; the wiki's chunk page isn't in my fetched set, so details **unverified**. Self-destruct TNT in walls is described as a last resort (Defense page).
- **Typical numbers.** n/a.
- **Version notes.** Sculk sensors from 1.17; calibrated sensors 1.19; unchanged to 1.21.11 in sources.
- **Server/SMP note.** Only works while chunk loaded (see chunk loaders); plugin servers do not block.
- **Bot-relevance.** Medium: bots can place sculk sensors/tripwires; an alarm ties into a faction chat or chunk loader.

### Claim plugins: GriefPrevention, WorldGuard, FTB Chunks
- **How it works.** Plugin-level protection by claim (golden shovel), region flags, trust levels; the wiki lists logging/rollback (CoreProtect) and block/chunk protection; Spigot OSC plugin shows WorldGuard flag `osc-enable` controlling cannons per region (https://www.spigotmc.org/resources/orbital-strike-cannon-tnt-cannon-1-21-26-1-2.131685/). GriefPrevention docs page could not be fetched (fandom blocked), so rule specifics (siege, explosion rules) are from general knowledge, **unverified**.
- **Typical numbers.** n/a.
- **Version notes.** Plugins track Paper/Spigot (1.21.x supported).
- **Server/SMP note.** On a claim server, you cannot damage another player's blocks or containers without trust; explosions normally disabled inside claims; some servers enable "siege" mode. The target scenario is "no op, no commands" so it may be vanilla (no protection) or plugin-protected (unknown). Detect it by trying a harmless block break on another's structure.
- **Bot-relevance.** High: check server brand/rules before raiding; a faction bot swarm should claim its own base if a claim plugin exists.

### Anti-grief and chunkban awareness
- **How it works.** Chunkban exploit fills a chunk with furnaces/signs/items so anyone who loads it is disconnected; mitigation: lower render distance to ~2, travel a couple hundred blocks away, restore render distance (wiki griefing page); 2b2t glossary lists chunkban as an exploit (https://2b2t.miraheze.org/wiki/SMIB). Other lag griefing: item drops, redstone clocks, armor stands, overloaded books.
- **Typical numbers.** n/a.
- **Version notes.** Oversized-item sanitiser in Paper (`oversized-item-component-sanitizer`) targets container/bundle/charged crossbow exploits.
- **Server/SMP note.** Paper config mitigates some; vanilla does not.
- **Bot-relevance.** Medium: bots must avoid loading chunkbanned chunks (watch kicks, fail gracefully, set low render distance on reconnect).

---

## C.6 Chunk loaders

### Nether portal chunk loader
- **How it works.** Any entity going through a portal loads the destination chunk fully for 15 s plus 8 surrounding chunks; passing an entity at least every 15 s keeps both sides loaded (https://minecraft.wiki/w/Tutorial:Chunk_loader).
- **Typical numbers.** 15 s window, 9 chunks.
- **Version notes.** Before 1.21.2: dropper/item entity loops were common; 1.21.2: vehicles have 0.5 s portal cooldown (was 15 s); minecart loaders need at least 3 rail pieces; before 1.21.5 loaders stopped when the world was exited, 1.21.5 reloads them on world reopen; random tick farms could need a player before 1.21.5.
- **Server/SMP note.** Works on vanilla; Paper/other servers may change portal entity transfer or limit entity per chunk; unloaded redstone stops.
- **Bot-relevance.** Medium: a bot can keep a farm running without standing there; fragile (piglin spawning, creepers, ghast).

### Minecart-based portal chunk loader (1.21.2+)
- **How it works.** A minecart runs on powered rails into a portal; with the new 0.5 s cooldown it cycles between portal sides continuously. Failures: mobs blocking the cart, piglins spawning in the portal, creeper explosions, ghast fireballs, lightning; mitigations include raising rails, 3+ rails, lighting, lightning rod (wiki Chunk_loader).
- **Typical numbers.** 3+ rails.
- **Version notes.** Design works from 1.21.2.
- **Server/SMP note.** Same as above; also contributes lag, so some servers cap entities/tick.
- **Bot-relevance.** Medium.

### End portal and end gateway loaders; /forceload
- **How it works.** End portal loads a 9-chunk area around the End platform only; end gateway loads 3x3 around the gateway for 15 s per item; `/forceload` needs cheats (wiki Chunk_loader).
- **Typical numbers.** 9 chunks / 15 s.
- **Version notes.** None found.
- **Server/SMP note.** `/forceload` unavailable without ops. Wither/dragon "eater" loaders from the user's list could not be confirmed in the sources (**unverified**).
- **Bot-relevance.** Low-medium.

---

## C.7 Travel

### Elytra + firework travel and nether-roof highways
- **How it works.** Elytra: min speed ~7.2 m/s at 30 deg up pitch; firework rocket boost 33.5 b/s; Unbreaking III ≈ 28 min flying; wall collision damage = 10 x delta horizontal velocity − 3; ceiling collision is harmless (https://minecraft.wiki/w/Elytra). The Nether 8:1 ratio plus highways on the Nether roof (above bedrock at y>127, reachable only with exploit) are common on anarchy servers (2b2t "Highway" glossary entry, https://2b2t.miraheze.org/wiki/SMIB).
- **Typical numbers.** 33.5 b/s with rockets; 432 durability Java.
- **Version notes.** 1.21.5: gliding cancelled by climbables; 1.21.11: gliding through vines allowed. Mace smash disabled during elytra flight in Java.
- **Server/SMP note.** Elytra speed rules are watched by anti-cheat; 2b2t "ElytraFly" hacks are cheats. Vanilla rocket flight is legal.
- **Bot-relevance.** Very high for logistics, but bots must handle durability, rocket stock and chunk loading as they move.

### Boat / ice highways
- **How it works.** Boats on ice reach 40 m/s; blue ice ~1.8 x (about 72 m/s, derived, wiki gives only the multiplier); in the Nether, 8:1 yields "up to 320 m/s" equivalent between Overworld locations (https://minecraft.wiki/w/Ice).
- **Typical numbers.** 40 / ~72 m/s.
- **Version notes.** None.
- **Server/SMP note.** Fully vanilla; unaffected by plugins except claim blocks.
- **Bot-relevance.** High: simple, deterministic path-following for cargo bots.

### Nether portal networks / highways
- **How it works.** Portal linking 8:1; obsidian frame 4x5 to 23x23; portal destroyed by frame break, nearby explosion, water; survival needs standing 4 s (https://minecraft.wiki/w/Nether). Networks of portals link bases via hub.
- **Typical numbers.** 8:1.
- **Version notes.** Primed TNT can pass through portals since 1.21 but cannot destroy portal blocks after pre4 (TNT page).
- **Server/SMP note.** Some servers restrict portal lighting near spawn.
- **Bot-relevance.** High.

---

## C.8 Storage, economy, automation

### Mass storage / stash and auto sorting
- **How it works.** Shulker box stashes (2b2t counts "dubs" of double chests of shulkers); ender chest and shulkers for protection (wiki griefing tips; 2b2t glossary). Auto-sorting systems are generic redstone (item filters) and not specifically sourced.
- **Typical numbers.** Not sourced.
- **Version notes.** Shulker duplication through update suppression is listed in Grokipedia (low reliability).
- **Server/SMP note.** Plugin servers log container access (CoreProtect).
- **Bot-relevance.** High: bots can handle sorted deposits with chest-API; inventory management needs click delays.

### Auto-farms and trial/breeze economy
- **How it works.** Standard farms (iron, gold, bone, crop) as economy; breeze rods for wind charges (see C.3). Rays Works breeze farm ≈ 32 rods/h.
- **Typical numbers.** 32 rods/h.
- **Version notes.** Random-tick farms (cane, lava) required a nearby player before 1.21.5 with portal loaders; lazy chunks produce incorrect spawns (pearl loader).
- **Server/SMP note.** Many SMPs have anti-AFK and farm-entity limits.
- **Bot-relevance.** High.

### Item duplication glitches (summary, per version)
- **How it works.** Wiki table (https://minecraft.wiki/w/Tutorial:Block_and_item_duplication): Java piston TNT dupe (1.14+); piston rail/carpet dupe (useful in 1.19); explosive fireworks + elytra (1.16.5, patched on Spigot/Paper); dragon's breath; shulker box breaking glitch (to 1.19, patched on Spigot/Paper); gravity block via end portal loop (1.14+, Paper patched); book player-profile rollback (to 1.18?); lead dupe patched 1.16.4. Grokipedia (low reliability): player/chunk save desync via force crash (since Infdev), LAN profile dupe (12w24a), update suppression, bonus chest dupe (22w03a); patched list includes bundle overstack (20w48a), donkey dupe (20w16a), pearl disconnect dupe (12w24a). A Medium post claiming "all working 1.21.11 dupes" lists no actual dupe (https://medium.com/@ultimate-minecraft/new-java-and-bedrock-dupe-glitch-minecraft-1-21-11-is-breaking-c06402e6015d): **not credible**. Carpet/chest-boat dupes: I found no source; **unverified**.
- **Typical numbers.** n/a.
- **Version notes.** Unclear for 1.21.11; the wiki page has no 1.21.11 Java entries beyond the persistent ones.
- **Server/SMP note.** Grokipedia: most community servers ban duping; anti-dupe plugins (ExploitFixer, DupeDetect etc.); Paper gates piston dupe/portal dupe behind config flags default false. Anarchy servers (2b2t) allow.
- **Bot-relevance.** Low: ban risk and no reliable current dupe; recommend not using.

---

## C.9 Lag, chunkbans, anti-cheat

### Lag machines and server-lag tech (defensive awareness)
- **How it works.** Mass item drops, redstone clocks, armour stands, abusive mob spawning, chunk-loaded TNT dupers and OSC fire all spike MSPT (OSC release described as producing lag; Planet Minecraft). Wiki recommends killing entities / mob limits via plugins.
- **Typical numbers.** n/a.
- **Version notes.** Lazy chunk interactions: cannons can break when chunk loaded (wiki).
- **Server/SMP note.** Paper entity limits and plugin lag detectors; vanilla has none.
- **Bot-relevance.** High (defensive): a swarm itself causes entity lag; throttle pathfinding and packet rates.

### Anti-cheat constraints (Grim, Vulcan, etc.)
- **How it works.** Grim: movement simulation (walking, swimming, knockback, cobwebs, bubble columns, boats, pigs, striders), claims "3.01 reach", "99.99% antikb", versions 1.8 to 26.2, requires Spigot/Paper/Folia/Fabric; Geyser players exempt (https://github.com/GrimAnticheat/Grim). It does not mention elytra/wind charge/mace in the README, so I cannot confirm they are handled; rely on vanilla-legal movement.
- **Typical numbers.** Reach ≤3.01 (tag).
- **Version notes.** Newer weapons (spear 4.5 reach, mace smash, wind-charge launch) are new targets for false positives; **unverified**.
- **Server/SMP note.** Plugin servers run it; vanilla jars don't. Bots should: legit speeds, CPS limits, human-like rotation, no impossible placement.
- **Bot-relevance.** Very high: the main practical constraint on automated PvP.

---

## C.10 Tunnelling and access

### Tunnelling: cave-borer, TBM, liquid ceilings
- **How it works.** TNT "tunneler" cannons dig a shaft (above); mining with Efficiency V pickaxes plus Haste; lava/water ceilings keep cave-ins off; Tutorial:Tunneling could not be fetched, so TBM details are **unverified**.
- **Typical numbers.** n/a.
- **Version notes.** Not sourced.
- **Server/SMP note.** Mining speed hacks (nuker) are flagged by anti-cheat.
- **Bot-relevance.** High: swarms can tunnel in parallel at legal speed.

### Nether roof / bedrock-floor access
- **How it works.** Nether roof is bedrock at y123 to 127; the y127 layer is solid and blocks upward movement outside Creative/Spectator (https://minecraft.wiki/w/Bedrock). Getting through survival requires "headless piston"/bedrock-break exploits (patched by Paper default).
- **Typical numbers.** +128 blocks of build space above roof (Java).
- **Version notes.** Java 1.18 added Overworld bedrock at y -64 to -60; bedrock generation depends on seed since 21w40a.
- **Server/SMP note.** `allow-headless-pistons`/`allow-permanent-block-break-exploits` false on Paper; anarchy servers keep them.
- **Bot-relevance.** Medium: only on vanilla/anarchy-style servers.

### Wither skeleton / creeper / guardian mob defences
- **How it works.** Iron golems, wolves, snow golems per Defense page; breezes attack iron golems actively (Breeze page); creeper four-door trap (Defense page). Wither skeleton and guardian defences are not in my sources, **unverified**.
- **Typical numbers.** n/a.
- **Version notes.** n/a.
- **Server/SMP note.** Mob caps and despawn rules apply.
- **Bot-relevance.** Low-medium.
