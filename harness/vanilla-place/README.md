# Vanilla place oracle

Fork patches 33, 70 and 71 predict the block state a vanilla client writes when a bot places a block (`BlockItem#place`: `BlockPlaceContext#canPlace`, `Block#getStateForPlacement`, `canSurvive`, `Level#isUnobstructed`). Their expected values came from reading the source. This harness gets them from vanilla itself: it runs vanilla 1.21.11's own code on a real `ServerLevel` and writes one TSV line per case, the state vanilla would place or `none`.

The fork test `azalea-client/tests/vanilla_place_cases.rs` (branch `place-oracle`) reads the TSV, runs `predict_place` on every case and lists each difference.

## How it works

- `OracleServer.java`: a `MinecraftServer` subclass built the way vanilla's game-test server is built. It opens a flat overworld in a throwaway folder. It opens no network port, reads no `server.properties`, and runs no tick loop: the harness calls vanilla code on its own thread, which is the server thread the object is built with.
- `PlaceOracle.java`: for each case it builds the scene (flags 18, so no neighbour updates), makes a `ServerPlayer` with the case's yaw, pitch and sneak state (the player is not added to the level, so it never obstructs), and runs the steps of `BlockItem#place` up to the write: `canPlace`, `updatePlacementContext`, `getPlacementState` (the protected method, called by reflection; it holds `getStateForPlacement`, `canSurvive` and `isUnobstructed`), and the "same state" check of `setBlock`. It then cross-checks against the real `BlockItem#place` and prints `REAL-PLACE DIFFERS` for each case where the server's own follow-up (the placed block's `onPlace` and neighbour updates, redstone, unsupported blocks popping off) leaves another state. Those are server-side effects the client does not compute in the same call; they are not errors.
- `Cases.java`: the input list. Neighbour-heavy families (fences, panes, walls, gates, leaves, chests, torches, ladders) use a fixed-seed random sample over a palette of neighbour states, so the file is the same every run. The other families are enumerated from each rule's branches (faces, hit heights at 0, 0.25, 0.5, 0.51, 0.75 and 1, yaws on and across the 45 degree borders, pitch, sneak, water source and flowing water, the replace rule). The first block of a family and a few named blocks get the full set; the other blocks of the family get a short set.
- `families.tsv`: which blocks the fork predicts, by family. It comes from `azalea-block/src/place_families.rs` with `families_from_fork.py` (`uv run --no-project harness/vanilla-place/families_from_fork.py <fork>/azalea-block/src/place_families.rs > harness/vanilla-place/families.tsv`). Blocks in no family are `Unknown` in the fork and are not asked.

## Run

Setup is the same as `harness/vanilla-hash` (the Mojang-named jar and the libraries inside the vanilla server jar):

```
W=$(mktemp -d)   # work folder: libraries and classes
(cd $W && unzip -q ~/Desktop/null-reference/vanilla-1.21.11/server.jar 'META-INF/libraries/*')
MC=$(ls ~/.gradle/caches/fabric-loom/minecraftMaven/net/minecraft/minecraft-merged/1.21.11-loom.mappings.1_21_11.layered+hash.2198-v2/*-v2.jar)
CP="$MC:$(find $W -name '*.jar' | tr '\n' ':')"
javac -nowarn -cp "$CP" -d $W/out harness/vanilla-place/*.java
mkdir $W/run && (cd $W/run && java -cp "$CP:$W/out" PlaceOracle $OLDPWD/harness/vanilla-place/families.tsv $W/cases.tsv $W/run/world 2>&1 | grep -v 'WARNING\|INFO')
cp $W/cases.tsv <fork>/azalea-client/tests/data/vanilla_place_cases.tsv
```

It takes about 20 seconds. Run it from a folder you can throw away (the server writes `logs/` and the world where it runs). The output ends with `cases N written` and `real-place differences: M`.

Then in the fork: `cargo test -p azalea-client --test vanilla_place_cases -- --nocapture`. Set `PLACE_ORACLE_REPORT=<path>` to also write the report (counts per family, skipped families with reason, every mismatch) to a file.

## TSV columns

`id, item, family, clicked, face, hit, yaw, pitch, sneak, setup, result, why`. Names have no `minecraft:` prefix. Positions are offsets from the origin cell (the cell a block goes into when the click lands on a neighbour). `setup` is `x,y,z=state;...` (properties not named keep their defaults). `result` is the placed state with every property, or `none`. `why` is `at=x,y,z` when vanilla's target cell is not the origin (the clicked block was replaced), else `-`.

## Limits

- No entities: the entity-obstruction branch (`Level#isUnobstructed` with an entity in the cell) is not covered.
- Redstone power cases are in the input (gates and trapdoors beside a power source). The fork answers `Unknown(Signal)` for most of them; the test counts them as skipped.
- Doors, beds, redstone components, candles, snow layers, coral, carpets and other families the fork does not predict are not asked.
- Scenes may hold floating blocks (a lever with nothing behind it). The oracle reads the state the scene holds, as the client would, so that is correct for `getStateForPlacement`.
