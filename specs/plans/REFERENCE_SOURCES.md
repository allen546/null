# Reference sources for fork patches

**Plan, 2026-10-17 (Allen).** Before more Azalea fork patches, set up local reference sources so each patch cites the vanilla method it reproduces (AZALEA_PATCHES.md). Nothing here goes into this repo or the public fork: Mojang's code and mappings are for reference only, and Paper and Grim are GPLv3 while Azalea is MIT, so patches describe behaviour in our own words and cite the source; they never copy code.

## Where

One folder outside the repo: `~/Desktop/null-reference/`, one subfolder per source, and `MANIFEST.md` at the top listing each source with its version, download URL, SHA-1 or git commit, date fetched and the tool versions used. Disk: about 7–9 GB in total with the Gradle caches; the disk has about 65 GB free (2026-10-17).

## Facts that set the plan

- 1.21.11 is the last obfuscated release. Mojang ships its official mappings (ProGuard format, `client_mappings` and `server_mappings` in the version JSON) for it. Yarn stops at 1.21.11. From 26.1 the jars ship unobfuscated.
- Mojang also published unobfuscated test builds of the 1.21.11 pre-releases and release candidates (for example `1.21.11-rc3_unobfuscated`). They are close to the release, not the release: a cross-check only.
- Azalea and Paper both use Mojang names, so Mojang names come first; Yarn is optional.
- Paper's own build decompiles vanilla: after `applyPatches`, `paper-server/src/minecraft` is a git repo whose base commit is the decompiled vanilla server with Mojang names (PaperMC's mache), with Paper's patches as commits on top. So one Paper checkout gives both the vanilla server source and the Paper diff.

## Sources

| id | source | version pin | how | decompile |
|---|---|---|---|---|
| R1 | Minecraft client and server jars and Mojang mappings | 1.21.11 release | `version_manifest_v2.json` on piston-meta → the 1.21.11 JSON → `client.jar`, `server.jar`, `client_mappings.txt`, `server_mappings.txt`; check each SHA-1 against the JSON | optional, R1d |
| R1d | Decompiled client and server, Mojang names | same | a scratch Gradle project with Fabric Loom, `mappings loom.officialMojangMappings()`, then `./gradlew genSources` (Vineflower). The result is merged client and server source. Optional second run with Yarn 1.21.11 mappings for Yarn names | yes |
| R2 | Paper server source, including the Bukkit API (`paper-api`) | Paper's `1.21.11` branch, pinned to the commit of build 132 (the jar in `harness/grim-server`) | `git clone` PaperMC/Paper, check out the commit, `./gradlew applyPatches` (needs Java 21+; Java 25 is installed) | done by Paper's build |
| R2b | Spigot and CraftBukkit (optional) | 1.21.11 | Spigot's BuildTools `--rev 1.21.11`. Only if a Bukkit behaviour is not clear from Paper's tree, since Paper descends from it | done by BuildTools |
| R3 | mineflayer and its parts | latest release tags, recorded by commit | `git clone` PrismarineJS `mineflayer`, `prismarine-physics` (movement simulation), `node-minecraft-protocol`, `minecraft-data` (check that 1.21.11 data exists), `mineflayer-pathfinder` | no (JavaScript) |
| R4 | GrimAC (recommended addition) | tag 2.3.74, the version in `harness/grim-server` | `git clone` GrimAnticheat/Grim, check out the tag | no |
| R5 | Meteor Client (automation: auto-eat, auto-tool, placement and interaction helpers, inventory handling) | the commit whose `gradle.properties` names Minecraft 1.21.11 | `git clone` MeteorDevelopment/meteor-client, check out that commit | no (Java source, Yarn names) |
| R6 | Baritone (automation: pathfinding, mining, building, block-break timing) | the branch closest to 1.21.11; record the gap if it lags | `git clone` cabaletta/baritone, check out that branch | no (Java source) |

Meteor Client and Baritone stay in (Allen, 2026-10-17): they have useful automation code to refer to. Both are client mods, so their vanilla calls show how a real client does a thing, and Baritone's movement and break logic are a reference for the fork's pathfinder work. Meteor is GPLv3 and Baritone LGPLv3: read, never copy, as above.

## Steps

1. Make the folder and `MANIFEST.md`. Run R1 by script (`fetch_vanilla.py` in the reference folder, `uv run`), with SHA-1 checks.
2. R2: clone Paper, check out build 132's commit, `applyPatches`. Check that `paper-server/src/minecraft` has the base commit and the patch commits. Record the commit.
3. R3–R6: clone and pin.
4. Optional R1d: Loom `genSources` with Mojang names. Spot-check `Player.attack`, `ServerPlayerGameMode.handleBlockBreakAction` and `FoodData.eat` against Paper's base commit. They must agree, because both decompile the same jar.
5. Index each tree for search (`rg` works; optionally a ctags file).
6. Rule for patches from then on: each fork patch row in AZALEA_PATCHES.md names the vanilla method it reproduces, for example "vanilla 1.21.11, Mojang names: `net.minecraft.server.level.ServerPlayerGameMode#handleBlockBreakAction`", and the Grim check it must pass, when there is one. The first use: check patch 6's interaction boxes for hopper, composter and scaffolding against the vanilla `getInteractionShape` methods.

Estimated work: about an hour of wall time, mostly Paper's `applyPatches` and Loom's decompile. It fits one subagent task.
