# Vanilla move oracle

Fork patch 74 ported vanilla's step-up logic (`Entity#collide`, `collectCandidateStepUpHeights`). Its unit tests held values worked out by reading the source. This harness gets the values from vanilla itself: it runs vanilla 1.21.11's own `Entity#collide` on a player-sized entity in a real `ServerLevel` and writes the movement vector vanilla returns.

The fork test `azalea-physics/src/collision/vanilla_move_cases.rs` (branch `stack-fix-stepup`, patch 75) reads the TSV, runs `collide_with` on every case with the same blocks, and compares the vector exactly.

## How it works

- It reuses `../vanilla-place/OracleServer.java` (a `MinecraftServer` on a throwaway flat world: no port, no tick loop). Compile both folders' files together.
- `MoveOracle.java` calls `Entity#collide` (private, by reflection) with a `ServerPlayer` that is not in the level. It does not call `Entity#move(MoverType.SELF)`: `move` adds sneak edge backing, stuck-in-block factors, fall tracking and block effects that the fork's `collide` does not compute. `move` calls `collide` once with the input vector, so the collision decision is the same.
- The entity has the player's size (0.6 x 1.8, set by `setPos`, which makes the box the way vanilla does) and step height 0.6. The harness sets `onGround` per case. The TSV also holds vanilla's box, so the fork test checks its own box construction.
- Scenes live in one chunk, cells x 6..11, z 6..11: a stone base at y 98, a floor row at y 99, a ledge on the east side (x 9 and up), and an optional block above the ledge at y 100. Kinds: `wall` (all z), `corner` (x >= 9 and z >= 9), `pillar` (one cell). Coordinates are absolute, so the doubles match the fork exactly.
- Block order matters: vanilla ends an axis once the move is under 1.0E-7, checked before each shape, so which shape comes first changes the result. The fork test walks cells in the same `Cursor3d` order as the fork's own query.

## Cases

- Ids 0..5: the 6 cases of `step_up_tests`, rebuilt from vanilla blocks (ledge tops and floors are block shapes: 0.6875 is soul sand over a closed trapdoor, 0.5 is a bottom slab over no floor). Case 1 is the 0.7 case with start bottom 0.1 above the floor.
- Then 3600 cases from a fixed seed (`20261115`): 258 random scenes (floors: air, stone, slab, carpet, trapdoor, snow, soul sand, farmland; ledges: about 70 states including stairs in all facings and shapes, snow layers 1..8, slabs, carpet, trapdoors, cake, fences and others; an optional upper block), start bottoms 0 to 1 above the floor (sixteenths and random), grounded and airborne, horizontal move up to 0.8, y from -0.8 to 0.42, and gaps from 0 to 0.25 to the ledge.
- `P` rows hold vanilla's collision boxes for each palette state; the fork test checks the fork's shape for each, so a shape error is told apart from a step error.

## Run

Setup is the same as `harness/vanilla-hash` (see `../vanilla-place/README.md`):

```
W=$(mktemp -d)
(cd $W && unzip -q ~/Desktop/null-reference/vanilla-1.21.11/server.jar 'META-INF/libraries/*')
MC=$(ls ~/.gradle/caches/fabric-loom/minecraftMaven/net/minecraft/minecraft-merged/1.21.11-loom.mappings.1_21_11.layered+hash.2198-v2/*-v2.jar)
CP="$MC:$(find $W -name '*.jar' | tr '\n' ':')"
javac -nowarn -cp "$CP" -d $W/out harness/vanilla-move/MoveOracle.java harness/vanilla-place/OracleServer.java
mkdir $W/run && (cd $W/run && java -cp "$CP:$W/out" MoveOracle $W/cases.tsv $W/run/world 2>&1 | grep -v 'WARNING\|INFO')
cp $W/cases.tsv <fork>/azalea-physics/tests/data/vanilla_move_cases.tsv
```

It takes a few seconds and prints `scenes N palette N cases N bytes N`. Run it from a folder you can throw away.

Then in the fork: `cargo test -p azalea-physics --lib vanilla_move -- --nocapture`.

## TSV columns

- `P  state  boxes` (`minX,minY,minZ,maxX,maxY,maxZ;...`)
- `S  id  kind  cells` (`x,y,z=state;...`)
- `C  id  scene  pos  onGround  move  box  result` (`pos`, `move`, `result` are `x,y,z`; `box` is the entity box)

## Limits

- No entities and no world border in the colliders.
- Blocks with a position offset (bamboo, dripstone) are not in the palette.
- Few cases are true step-ups (about 90 of 3600); many more are falls cut both down and sideways (about 165). The rest are free moves, walls and low-ledge non-steps.
