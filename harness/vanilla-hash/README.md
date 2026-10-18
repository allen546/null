# Vanilla item component hashes

`VanillaHash.java` prints the hash vanilla 1.21.11 gives an item component, made the way the client makes it for a container click (`ClientPacketListener#decoratedHashOpsGenerator`: the component's codec through `HashOps.CRC32C_INSTANCE` with registry access). Fork patch 15's tests in `azalea-inventory/tests/components.rs` use its values. Add a line to `main` for each new component to check.

It runs against the Mojang-named jar that Loom made for the reference sources (`~/Desktop/null-reference`, R1d) and the libraries inside the vanilla server jar. It is our code calling vanilla's API; no vanilla code is copied.

```
W=$(mktemp -d)   # work folder: libraries and classes
(cd $W && unzip -q ~/Desktop/null-reference/vanilla-1.21.11/server.jar 'META-INF/libraries/*')
MC=$(ls ~/.gradle/caches/fabric-loom/minecraftMaven/net/minecraft/minecraft-merged/1.21.11-loom.mappings.1_21_11.layered+hash.2198-v2/*-v2.jar)
CP="$MC:$(find $W -name '*.jar' | tr '\n' ':')"
javac -nowarn -cp "$CP" -d $W/out harness/vanilla-hash/VanillaHash.java
(cd $W && java -cp "$CP:$W/out" VanillaHash 2>&1 | grep STDOUT)   # it writes logs/ where it runs
```

A named tag needs the ops' own registry owner (`ops.owner(Registries.X)`); a lookup made apart from the ops is refused as "not valid in current registry set".

## ComponentDump: every component, vs Azalea

`ComponentDump.java` loads the registries the way the server does (`WorldLoader#load`: static tags, then the worldgen and dimension registries from the vanilla data pack), so the ids of data-driven registries match a real server. It reads `items.txt` (`/give` syntax, one item per line, parsed with `ItemParser`) and prints tab-separated lines:

- `REG registry index entry nbt`: the order of each synchronized data-driven registry (`nbt` is the entry as network NBT in hex for `dimension_type` and `enchantment`, `-` for the others)
- `CASE n component bytes hash item`: for each added component, its network bytes (`streamCodec()` into a `RegistryFriendlyByteBuf`) and vanilla's hash (`-` for a type vanilla cannot hash)
- `UNCOVERED component`: a component type no case sets (should be none)
- `ERROR n item message`: an item the parser refused (fix the line)

Run it from a work folder set up as above, then copy the output into the fork as the test data:

```
javac -nowarn -cp "$CP" -d $W/out harness/vanilla-hash/ComponentDump.java
(cd $W && java -cp "$CP:$W/out" ComponentDump $OLDPWD/harness/vanilla-hash/items.txt 2>/dev/null > out.tsv)
cp $W/out.tsv <fork>/azalea-inventory/tests/data/vanilla_component_hashes.tsv
```

The Rust test is `azalea-inventory/tests/vanilla_component_hashes.rs` in the fork (`cargo test -p azalea-inventory --test vanilla_component_hashes`). It lists all mismatches and decode failures at once. Add an item line to `items.txt`, rerun the dump, and copy the file when a component or form needs a check.
