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
