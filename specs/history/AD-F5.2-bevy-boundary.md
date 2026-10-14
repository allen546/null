# Debate: should skills see Bevy? (AD-F5.2)

> **Status (2026-10-14):** the "cheap experiment" below has been run; results in `reference/SPIKE_F5_BOUNDARY.md`. Steps 1–4 passed in Azalea's simulator; a live server wasn't available.

2026-10-13. Allen leans toward "no skills see Bevy directly" and asked for more opinions. A small debate ran: three advocates (one per option), a round where each attacked the other two, and an independent judge who checked the technical claims. Inputs: F5, F1 §4.1 and §4.5, AZALEA_REALITY_CHECK §8, SPEC_AUDIT #1, DECISIONS, ARCHITECTURE §9.

## Result

**All three advocates converged on the same hybrid, and the judge endorsed it.** Allen's lean holds for decision code, which is most of the code people write, but not for the code that extends Azalea itself.

1. **Decision crates see no Bevy and no Azalea.** Skill state machines, reflex triggers and actions, F1 §4.5's arbitration and switch resolution, the cargo filter, and squad tactics are plain Rust against a Null-owned API (`BotView` to read, `BotCommands` to act, `Skill` and `Reflex` traits). One adapter host system inside Azalea's GameTick runs them, borrowing Azalea's components with no copy and no mirror.
2. **Mechanism crates import Azalea and Bevy directly**, and they are named on an allowlist: the host, view construction and command application (`mc-adapter`), pathfinder move families through `successors_fn` (`mc-nav`), and raw packet sequences, ack tracking, container-menu mechanics and the replay patch (`mc-act`). These extend Azalea rather than use it, so no view can hide them.
3. **Enforcement:** a cargo-deny allowlist so only mechanism crates may depend on `bevy*` or `azalea*`, plus a generated list of the Azalea types each mechanism crate uses and an upgrade-impact diff before every fork bump.
4. **Restate the rule** in F5 §4.1 and ARCHITECTURE §9.1 as "porting touches the named mechanism crates, never the decision crates", not "one thin adapter".

## Why it went this way

- **A missing option.** The earlier write-ups assumed skills either are Bevy systems or run outside the ECS through Azalea's slow `Client` handle. There is a third way: skills run inside the ECS tick, called by one host system, without being systems themselves. The judge confirmed this works with no copying, no extra lock per call and no added tick of latency.
- **Several versions at once.** One HQ per deployment spanning servers (AD-K1.1) means servers on different Minecraft versions may run at the same time, with one node binary per version. Two Azalea forks will likely pin different Bevy majors. Only a Bevy-free decision layer compiles unchanged against both. This was the argument that moved the direct-imports advocate.
- **CPU is a wash.** The advocates for B and C withdrew their performance arguments: dynamic dispatch costs microseconds per tick at 1,000 bots, and native reflex systems would serialise on a shared per-bot candidate anyway. A per-bot parallel loop in the host scales at least as well. A benchmark should still confirm it.
- **Testing and authoring.** A reflex trigger becomes a plain function over a fake `BotView`, testable with property tests and no Bevy app. Skill authors, including coding agents, learn one small stable API instead of whichever Bevy the fork pins.

## What it costs (the judge's list)

- "Only the adapter changes" isn't achievable. Mechanism crates may be 30–50% of bot-layer code, and they still churn on every upgrade.
- `BotView` and `BotCommands` must be designed before skills know what they need, and whoever owns them can become a bottleneck. Let skill authors propose additions under a light review rule.
- The view can drift into a one-to-one mirror of Azalea's fields, which keeps the upkeep and loses the protection. Track how many members are plain renames.
- Entity queries (nearest hostile, squadmates, dropped items) need a real per-tick spatial index in the adapter, or `EntitiesRead` becomes a reinvented `Query`.
- Bevy change detection has to be rebuilt as per-bot change flags in the view. It helps only a few triggers anyway.
- Multi-tick reflexes (an MLG bucket: look, swap, place, pick up) need per-bot reflex state; the sketched `Reflex` trait needs a state slot.
- Fake-view tests stay green through semantic drift in Azalea, so simulator tests on every `BotCommands` variant are a hard requirement for the mechanism crates.

**A caution that applies to every option:** Azalea handles packets and some input messages (walk, sprint, attack, use item) in its 60 Hz Update schedule, not the 20 Hz GameTick. If commands are written as messages from GameTick, reflex responses could reach the server a tick late. Commands should write Azalea's input components directly, or the fork should reorder those handlers. Check on the fork.

## Cheap experiment before it's final

A short spike on the fork:
1. Build the API plus a host with two reflexes (eat; fall or lava escape), one movement skill (goto plus a short bridge) and one inventory skill (chest to hotbar). Count `BotView`/`BotCommands` members and how many are plain renames, and how much logic ended up in mechanism crates. Fewer than about 40 members, mostly intent-shaped, points to the hybrid.
2. A same-tick test in Azalea's simulator: inject a fall at tick N and check the reflex's packet leaves in tick N.
3. The decisive one: build the same unmodified skills crate against two adapters, the 1.21.11 fork and upstream main with its Bevy 0.19 bump. If only mechanism crates change, the multi-version claim is proven.
4. A 500–1,000 bot GameTick benchmark, host against native systems, with pathfinding active. More than about 2% slower means revisit.

## Still disagreed

- Whether mechanism crates should name Azalea through an alias module or import it directly with the tooling above. The judge leans direct, aliases optional.
- Whether squad leaders (K6) are decision code through a `SquadView` or mechanism code. The judge leans decision code, if `SquadView` can stay small.
- How big the mechanism layer really is. Only the spike settles it.
- **Whether servers on different versions will actually run at the same time, or the whole deployment ports together on each upgrade.** **Answered (Allen, 2026-10-14): the whole deployment ports together; two versions at once only when a server forces it.** The multi-version argument weighs less, but each port is still a Bevy and Azalea upgrade that the split keeps out of decision code.

---

## Appendix: each side, in its own words

### Option A: No Bevy in skills (Allen's lean)

**Opening thesis.** Skills and reflexes should be plain Rust: per-bot state machines written against Null-owned BotView and BotCommands types. A single adapter-owned Bevy host calls them inside Azalea's GameTick, so they still run in the ECS tick with no added latency. Of the four positions on the table, this is the only one that keeps both Bevy and Azalea out of skill code. It also matches the shape F1 §4.5 already requires (a sequential per-bot pipeline with arbitration), makes skills testable without an App, and confines every Bevy or Azalea bump to one crate. The price is adapter upkeep, not CPU or latency.

**Costs it admitted.**
- The adapter is thick, not 'thin'. Every new piece of game state a skill needs means adding a BotView accessor or a BotCommands variant in mc-adapter first. That is a two-crate change, and the adapter owner becomes a review bottleneck early on, while skills are still finding out what they need.
- Some code that feels like a skill has to live on the adapter side. Pathfinder move families and cost layers use Azalea's MovesCtx, RelBlockPos and Goal types (successors_fn), and menu-specific inventory mechanics do too. B1's new moves (ladders, doors, bridging) end up in mc-nav, an adapter-class crate, so B1 is split across the boundary.
- Designing the view takes judgment up front. If BotView exposes Azalea's semantics one-to-one (raw movement-state fields, Azalea's inventory slot layout), it becomes a mirror in disguise and loses the semantic-change protection. Getting it right needs the movement and inventory skill prototype that DECISIONS calls for.
- Skills lose Bevy-native tools: change detection (Changed<Health>), observers and triggers, run conditions, and bevy-inspector visibility into skill state. Each one used has to be rebuilt as a view field (events this tick, a health delta) or come from Null's own inspect, which F1 requires anyway.
- Lifetime and handle discipline: a view can't be stored across ticks, so skills keep BotId, EntityRef and BlockPos handles that the adapter resolves each tick, plus copies of whatever they remember. Running par_iter_mut means skills must be Send and reach shared stores (claims, belief) only through read snapshots, with writes going out as commands.
- Read now, apply later: a skill can't see its own command take effect within the same tick (select a slot, then read the held item). Bevy's deferred Commands has the same limit, but some multi-step interactions take one more tick or need a combined command.
- Upkeep at upgrade time is real, just concentrated in one place. A semantic change can force a judgment call about Null's intent types (does walk() mean what it meant?). One crate absorbs every churn item, and it needs real adapter tests against Azalea's simulator to keep that honest.
- Ecosystem: third-party Azalea plugins (azalea-hax, azalea-viaversion) can only be wired in at the adapter, and community skill code can't be dropped in directly.

**Conceded after reading the others.**
- Change filtering (B, C) is a real loss in my original sketch. The host has to rebuild it. BotView should carry per-bot change flags from Bevy's Ref::is_changed/last_changed (health, hunger, inventory, position, held item), and a Reflex should be able to declare a watch mask. The host then caches each trigger's last result per bot and re-evaluates only when a watched input or tick-dependent condition changed. That keeps the 'every trigger evaluated every tick' semantics at the cost of a bit test. Without this, B and C would be right that A throws away Bevy's cheapest optimisation.
- C is right that a large share of B1 (move families via successors_fn, replay), B3's packet watchers and B6's raw use-item packet path are adapter-class code, not skill code. The 'adapter' is not thin. In practice it is several crates (mc-adapter, mc-nav, mc-packets) that legitimately import Azalea and Bevy, and B1's spec will be split across the boundary.
- B is right that cross-entity reads (nearest hostile, squadmates, dropped items, cargo-tagged entities) are routine. EntitiesRead has to be a real spatial query API designed up front, not a few ad-hoc methods. K6 squad leaders in the same ECS need a SquadView over member views. If the view grows into a reinvented Query, A loses.
- C's tooling is good under any option. The cargo-deny `wrappers` allowlist is the right enforcement mechanism, and `cargo xtask azalea-surface` plus an upgrade-impact diff should be applied to the adapter-class crates so each upgrade's cost is visible before merging.
- B's 'the surface file measures itself' insight carries over. Under A, the size and one-to-one-ness of BotView/BotCommands is the metric, and a view that mirrors Azalea field for field is the signal that A is failing.
- My dyn-dispatch and par_iter estimates are unmeasured, the same as B's. The F6 GameTick benchmark settles performance, not either side's arithmetic.
- The fork really does control when churn arrives (C). The boundary's value is the size of each rewrite when we take an upgrade, and for multiple versions at once, not avoiding upgrades.

**Final position.** I still hold option A, refined by the debate into a two-tier rule. Decision code (skills, reflexes, squad tactics) is plain Rust against Null-owned BotView, SquadView and BotCommands in null-bot-api, with no bevy* or azalea* dependency. Mechanism code that extends Azalea itself (the reflex host, view construction, command application, pathfinder move families via successors_fn, the replay fork patch, packet watchers for recipe/merchant/enchant, raw use-item packets) lives in an explicit allowlist of adapter-class crates: mc-adapter, mc-nav and mc-packets. Those crates import Azalea and Bevy directly, the way C proposes, and B-style aliases are not worth having there.

Enforcement borrows C's mechanism: a cargo-deny wrappers allowlist, plus an azalea-surface/upgrade-impact xtask over the adapter-class crates. The host must keep what B and C correctly say A would otherwise lose:
- per-bot change flags in the view, and watch masks on reflexes with cached trigger results;
- par_iter over bots, with F1 §4.5 arbitration as one loop and switches as a cached bitset;
- a real spatial EntitiesRead.

Decide/Apply run in a chained set after packet handling and before PhysicsSystems, so reflexes act in the same tick.

The deciding new fact is AD-K1.1. One HQ spanning servers that Null does not control makes several Minecraft versions in production at once the expected case. Only A lets one skill crate compile unchanged against two adapter builds. C's own stated threshold concedes that, and B cannot meet it because Bevy and aliased fields reach every skill.

What would still move me to B: the DECISIONS prototype (one movement skill and one inventory skill, written both ways) showing either of these:
- the views turn into one-to-one renames of Azalea components;
- most skill logic ends up in adapter-class crates anyway.

What would also move me: the F6 benchmark at 500-1,000 bots showing the host, change flags included, more than a few percent of the tick slower than native systems.

C is no longer viable if multi-version deployment is real. If Allen instead commits to one version per deployment forever, C's case returns, but A still wins on arbitration and testability.

### Option B: Curated re-exports (previous spec lean)

**Opening thesis.** Skills should be ordinary Bevy systems that see Azalea only through one curated module of Null-named aliases, with full translation used only where behaviour actually churns (movement, placement, item use, block and world lookups). This is option A applied type by type, at the moment a type starts to churn. It costs nothing at runtime, keeps reflexes in the same tick for free, and lets the measured churn decide which types get the expensive treatment.

**Costs it admitted.**
- Bevy churn still reaches every skill crate. Event becoming Message, set-naming conventions and the observer and trigger changes are edits in every skill at each Bevy major. They are mechanical and the compiler finds them, and the fork means we choose when they happen. But 'porting only touches the adapter' stops being literally true, and F5 4.1 and ARCH 9.1 must be restated as 'bounded skill churn on Bevy majors and Azalea semantic changes'.
- Semantic changes to aliased types leak. If Azalea changes a re-exported component's fields or methods, or turns a message skills read into a trigger, the skills that use those fields break until the type is promoted to a translated one. Aliases only absorb renames and moves.
- The abstraction is visible: compiler errors, rustdoc and go-to-definition show `azalea_entity::Position`, not `BotPosition`, which confuses new skill authors.
- It needs curation discipline. If every requested type gets a `pub use` without review, the surface becomes option C with an extra hop. The file needs an owner and a promotion rule, such as 'any type touched by a semantic change in the last two releases is translated'.
- Leaks through field types: a re-exported component whose public fields are other Azalea types (for example an Azalea Vec3 or an inventory slot type) gives skills those types too, and the Cargo dependency check can't see that. The surface must also re-export or wrap those field types, or skills must use only accessor methods.
- Skill authors work with two styles: plain aliases for data, translated intents and views for movement, placement, item use and world lookups. There is a learning curve, and someone will reach for Azalea's raw movement input instead of WalkIntent unless code review catches it.
- Tests with real components need some Azalea types to be constructible outside a live client. A few are not (menus, World). The adapter must ship and maintain test builders and fakes for those, which is real upkeep, though smaller than a full mirror.
- Compared with Allen's no-Bevy option, skills are less portable to a future non-Azalea or non-Bevy client. If Null ever leaves Bevy entirely, B's skills need rewriting and A's trait-based skills might not.

**Conceded after reading the others.**
- My per-tick CPU argument for B was overstated. A predicted indirect call costs a few nanoseconds, not the 50-100 ns I used. A's figure is right: 30k dyn calls is microseconds. The real cost is the trigger work itself, and that's the same in every design.
- Changed<> filtering is weaker than I claimed. F1 4.5 and CONVENTIONS 4.3 require every trigger to be evaluated every tick, on or off, and any trigger that depends on time or position (lava escape, fall height, mob approach) can't skip unchanged rows. Only triggers that are pure functions of one component (eat on hunger) benefit, and a plain-Rust host can get the same effect from a cheap per-bot dirty bitset.
- A is right that F1 4.5 describes a host, not a fan-out. The pipeline runs per bot in order: switch resolution, ReflexSuppressed, arbitration, cargo filter, skill step only if no reflex won, then the overridden skill is told. As one plain loop that's simpler and harder to get wrong than my Reflex/Arbitrate/Skill/Act sets spread across systems.
- A's testability claim holds. A trigger written as fn(&BotView, &Params) -> Option<Urgency> can be property-tested over thousands of states with no World, and that's better than run_system_once on spawned entities.
- Bevy churn reaches every skill under B. Only A actually shields skill crates from Bevy majors.
- C is right that B1 moves, B1/B6 replay and B3 packet work extend Azalea rather than consume it. No alias or view covers them, so all three positions need an 'engine-extension' tier that uses Azalea and Bevy directly.
- C's tooling is worth adopting no matter which option wins: a generated per-crate azalea-surface.txt checked in CI, and `xtask upgrade-impact` that intersects cargo-public-api diffs with each crate's surface before a merge.

**Final position.** A three-tier hybrid, now closer to A than to my opening B.

Tier 1, the engine-extension crates: mc-adapter, mc-nav (B1 move families and the Replaying marker), B3 packet watchers, the reflex host and K6 squad leaders. These are the only crates allowed to depend on azalea or bevy, enforced with a cargo-deny allowlist. Each one carries C's generated azalea-surface.txt, and every fork bump runs upgrade-impact first. Inside these crates, Azalea types are named through one Null-owned surface module of aliases (B), so a rename is still a one-line change even here.

Tier 2, reflexes and per-bot skills (eat, armour, flee, MLG, inventory upkeep, gather and place drivers): plain Rust, as A proposes. They implement Reflex and Skill traits against BotView and BotCommands. One adapter host system in GameTick runs F1 4.5 as a single per-bot loop: it borrows the components with no mirror, sits after packet-derived state and before physics and send_position, and switches are a cached bitset. This is the tier where arbitration, the switch contract, testability and Bevy-freedom pay off most, and the code here touches the least Azalea state.

Tier 3, what B contributes inside A: BotView accessors start as zero-copy reads over the aliased Azalea components. They get real translation lazily, when churn or a second Minecraft version calls for it. Movement, placement, attack and item use are translated intents from day one. This keeps A's view small and stops it hardening into a mirror designed up front.

The decision should rest on the movement-plus-inventory prototype, with two gates. First, if the inventory skill's BotView needs dozens of one-to-one accessors (container menus, slot layouts), that family moves to Tier 1 as a Bevy system using the alias surface. Second, a 500-1,000 bot GameTick benchmark of the host against native systems has to land within a few percent of the 50 ms budget. Allen's 'no skill sees Bevy' lean holds for Tier 2. It shouldn't be applied to Tier 1, where all three positions, A's included, already admit the code has to touch Azalea directly. Allen also confirmed one HQ per deployment spanning servers, which could mean several Minecraft versions in production at once. Of the three, C is the option I'd now rule out.

### Option C: Direct imports

**Opening thesis.** Skills and reflexes in Null are not consumers of a game interface. They extend Azalea itself: B1 adds pathfinder moves through `successors_fn`, B1/B6 replay filters Azalea's own systems with `Without<...>`, B3 adds packet watchers, and every reflex has to be ordered against Azalea's system sets. A layer in front of those extension points either re-exports them one for one, which costs upkeep for nothing, or grows into a second Azalea. Put the version boundary where it can actually hold: above the bot layer, at the job and event types. Below that line, let skill crates use Azalea and Bevy directly. The fork controls when churn arrives, and a generated per-crate list of the Azalea types each crate uses makes each upgrade's cost visible before we take it.

**Costs it admitted.**
- Every Bevy major and every Azalea ECS-level change touches every allowlisted crate, about ten crates and growing. A port to a new Minecraft version means taking several upstream releases at once, which can be a multi-day migration across all bot-layer crates. F5's 'port without a rewrite' has to be restated as 'port with a mechanical, compiler-guided migration of the bot layer', and ARCH §9.1's 'only one crate touches Azalea's types' is withdrawn.
- Allen's no-Bevy-in-skills variant genuinely shields skill crates from Bevy majors, and C gives that up completely. A Bevy change like Event to Message in 0.17 touches every skill.
- Nothing bounds coupling. A skill can reach into any Azalea internal, and the surface file only makes that visible. Without review discipline, the Azalea surface the project depends on grows quietly.
- Semantic changes in Azalea compile cleanly and change skill behaviour silently. C relies on simulation and harness tests to catch them, so a skill without good tests is exposed.
- Fakes are heavier than trait mocks. Some Azalea state (the shared world, chunk storage, inventory menus) is awkward to build in a unit test, and the test fixtures depend on Azalea test utilities that churn too and may need a fork patch to make public.
- Skill code is bound to Azalea's design choices. If Null ever moved off Azalea, or if upstream stopped (its bus factor is one), skills would be rewritten, not ported. A trait boundary would make a client swap at least conceivable.
- The deprecated-alias trick and the facade crate are fork patches. They are small and additive, but they are still upkeep on every merge.

**Conceded after reading the others.**
- My own first trigger has fired, and it is in the specs, not hypothetical. K1 line 31 now says 'a deployment spanning servers on different versions runs a node binary per version (F5 §4.1)', and Allen confirmed one HQ per deployment spanning servers. So the same skill source may need to compile against two Azalea forks in production at the same time. Under pure C, every skill crate would need cfg-gating or a branch per version. That is the strongest single argument in this debate, and it cuts against C for the decision logic.
- A is right that Bevy is the larger and less controllable source of churn, and that only A shields skill code from it. B and C both leave every skill exposed to Event->Message-class changes.
- My CPU argument was weak. Changed filtering helps very few reflexes, because Azalea's physics touches Position and Physics for every bot every tick and most triggers depend on world or entity state. Parallel reflex systems serialise on the shared candidate component. A's per-bot par_iter host is at least as parallel, and dispatch costs microseconds, not milliseconds (B's 50-100 ns per call is wrong). CPU does not decide this choice. The F6 GameTick benchmark should confirm that, and none of the three options should lean on CPU claims until it has run.
- A's sync-ordering risk is smaller than B and I claimed. A keeps no mirror. It borrows Azalea components in the same system that runs reflexes and applies commands in a chained set before PhysicsSystems. There is exactly one ordering point (the Decide/Apply sets), the same count as B's and C's.
- A's testability and preemption contract are real day-to-day gains: a skill as `tick(&mut self, &BotView, &mut BotCommands) -> Step`, with on_preempted as a trait method.
- B's single Bevy version pin in one crate is good hygiene whichever option wins.
- Not a concession, but a correction: my third trigger (outside content-pack code) does not fire. C11 §4.1 is a typed descriptor schema with no scripting, so packs never ship skill code.

**Final position.** Hybrid: split the bot layer by kind of code, not by a crate count. I now support Allen's lean (option A) for decision code, and C's direct imports for mechanism code, stated honestly. B's alias tier should be dropped.

1. Decision crates have no Bevy and no Azalea. Skill state machines, reflex triggers and actions, arbitration and switch resolution (F1 §4.5), the cargo filter, the skill-step and preemption contract, and squad-leader tactics (K6, through a SquadView) are plain Rust against null-bot-api (BotView, BotCommands, Skill, Reflex, EntityRef, BlockPos, Face). This is the most numerous and most frequently written code, much of it by coding agents. Since K1 allows servers on different versions in one deployment, it must compile unchanged against two forks at once. This is where A earns its cost.

2. Mechanism crates are named and import Azalea and Bevy directly. mc-adapter is the host (Decide/Apply sets in GameTick after packet-derived state, before PhysicsSystems), view construction and command application. mc-nav holds the B1 move families through successors_fn, cost layers and goal mapping. mc-act holds placement, use-item and attack packet sequences, ack tracking, container menu mechanics, and the B6 replay marker with its Without<Replaying> fork patch. Allen should restate F5 §4.1 and ARCH §9.1 to say 'porting touches the named mechanism crates, never the decision crates', not 'one thin crate'. A thin adapter was never achievable, because B1, B3 and B6 are Azalea extensions, not Azalea queries.

3. Enforcement: a cargo-deny ban on azalea* and bevy* with `wrappers` set to those mechanism crates only. K, C and F2 crates are banned as before. My C tooling moves to the mechanism crates: a generated azalea-surface.txt per crate, and `xtask upgrade-impact <ref>` intersecting cargo-public-api diffs with those files, so each upgrade's cost is known before the merge. Deprecated aliases in the fork facade stay optional.

4. Tests: decision crates get fake-view unit and property tests, which are fast. Mechanism crates must have simulator tests (scripted server packets in, packets out) covering every BotCommands variant and every BotView field with version-sensitive meaning. That is where semantic drift such as the strafing fix gets caught, so it is a requirement, not a nice-to-have.

5. Keep the view narrow and shaped as intent. BotCommands is walk/goto(goal), look_at, select_slot, attack(EntityRef), use_item_on(pos, face, hit), click(menu, slot, mode). BotView exposes Null semantics, not Azalea's field layout. Null-owned inputs to nav cost layers, such as the operator-approved region map, pass from the decision side to mc-nav as Null types. That map is one more reason the concept Allen calls 'claimed' should get a name that doesn't suggest server-side land allocation, for example 'authorized area'.

6. Kill criteria, from the prototype DECISIONS already asks for (one movement skill and one inventory skill on the fork). Revisit toward direct imports in decision crates if any of these show up: BotView and BotCommands for those two skills need more than about 40 members that are one-to-one Azalea renames; more than about half of each skill's logic ends up in mc-nav or mc-act; the F6 GameTick benchmark at 300-1,000 bots shows the host design more than about 2% of the tick slower than native systems; or Allen rules out ever running two versions at once. If none of these show up, this hybrid should replace the current lean in DECISIONS.
