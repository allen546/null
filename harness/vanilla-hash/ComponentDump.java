import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.stream.Stream;

import com.google.common.hash.HashCode;
import com.mojang.brigadier.StringReader;
import io.netty.buffer.ByteBuf;
import io.netty.buffer.Unpooled;
import net.minecraft.SharedConstants;
import net.minecraft.commands.arguments.item.ItemParser;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.Registry;
import net.minecraft.core.RegistryAccess;
import net.minecraft.core.component.DataComponentType;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.component.TypedDataComponent;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.nbt.NbtOps;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.nbt.NbtIo;
import net.minecraft.resources.RegistryDataLoader;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.RegistryLayer;
import net.minecraft.server.packs.PackType;
import net.minecraft.server.packs.repository.ServerPacksSource;
import net.minecraft.server.packs.resources.MultiPackResourceManager;
import net.minecraft.tags.TagLoader;
import net.minecraft.util.HashOps;

// Dumps, for each item in an /give-syntax list, every added component's network bytes and vanilla hash,
// plus the order of the synchronized data-driven registries. Registries load as WorldLoader#load does.
public class ComponentDump {
    // Bootstrap swaps System.out for a logger, so keep the real stream.
    static final java.io.PrintStream OUT = System.out;

    public static void main(String[] a) throws Exception {
        try {
            run(a);
        } catch (Throwable t) {
            t.printStackTrace(OUT);
            System.exit(1);
        }
    }

    static void run(String[] a) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        var rm = new MultiPackResourceManager(PackType.SERVER_DATA, List.of(ServerPacksSource.createVanillaPackSource()));
        var layered = RegistryLayer.createRegistryAccess();
        var pending = TagLoader.loadTagsForExistingRegistries(rm, layered.getLayer(RegistryLayer.STATIC));
        var forLoading = layered.getAccessForLoading(RegistryLayer.WORLDGEN);
        var lookups = TagLoader.buildUpdatedLookups(forLoading, pending);
        var worldgen = RegistryDataLoader.load(rm, lookups, RegistryDataLoader.WORLDGEN_REGISTRIES);
        var all = Stream.concat(lookups.stream(), worldgen.listRegistries()).toList();
        var dims = RegistryDataLoader.load(rm, all, RegistryDataLoader.DIMENSION_REGISTRIES);
        RegistryAccess access = layered.replaceFrom(RegistryLayer.WORLDGEN, worldgen, dims).compositeAccess();
        pending.forEach(Registry.PendingTags::apply);

        var nbtOps = access.createSerializationContext(NbtOps.INSTANCE);
        for (var data : RegistryDataLoader.SYNCHRONIZED_REGISTRIES) {
            dumpRegistry(access, data, nbtOps);
        }

        var ops = access.createSerializationContext(HashOps.CRC32C_INSTANCE);
        var parser = new ItemParser(access);
        Set<DataComponentType<?>> covered = new HashSet<>();
        int n = 0;
        for (String line : Files.readAllLines(Path.of(a[0]))) {
            line = line.strip();
            if (line.isEmpty() || line.startsWith("#")) continue;
            int idx = n++;
            ItemParser.ItemResult result;
            try {
                result = parser.parse(new StringReader(line));
            } catch (Exception ex) {
                OUT.println("ERROR\t" + idx + "\t" + line + "\t" + ex.getMessage());
                continue;
            }
            for (Map.Entry<DataComponentType<?>, Optional<?>> e : result.components().entrySet()) {
                if (e.getValue().isEmpty()) continue;
                covered.add(e.getKey());
                dumpComponent(idx, line, access, ops, e.getKey(), e.getValue().get());
            }
        }
        // These two have no persistent codec, so ItemParser cannot set them; build them directly.
        dumpComponent(n, "(java) creative_slot_lock", access, ops, DataComponents.CREATIVE_SLOT_LOCK, net.minecraft.util.Unit.INSTANCE);
        covered.add(DataComponents.CREATIVE_SLOT_LOCK);
        dumpComponent(n + 1, "(java) map_post_processing=lock", access, ops, DataComponents.MAP_POST_PROCESSING, net.minecraft.world.item.component.MapPostProcessing.LOCK);
        dumpComponent(n + 2, "(java) map_post_processing=scale", access, ops, DataComponents.MAP_POST_PROCESSING, net.minecraft.world.item.component.MapPostProcessing.SCALE);
        covered.add(DataComponents.MAP_POST_PROCESSING);
        for (DataComponentType<?> t : BuiltInRegistries.DATA_COMPONENT_TYPE) {
            if (!covered.contains(t)) {
                OUT.println("UNCOVERED\t" + BuiltInRegistries.DATA_COMPONENT_TYPE.getKey(t));
            }
        }
    }

    @SuppressWarnings("unchecked")
    static <T> void dumpRegistry(RegistryAccess access, RegistryDataLoader.RegistryData<T> data, com.mojang.serialization.DynamicOps<net.minecraft.nbt.Tag> ops) throws Exception {
        Registry<T> reg = access.lookupOrThrow(data.key());
        for (T value : reg) {
            var id = reg.getKey(value);
            // Azalea parses only these two into structs; for the rest, the order is enough.
            String nbt = "-";
            if (data.key().equals(Registries.DIMENSION_TYPE) || data.key().equals(Registries.ENCHANTMENT)) {
                var tag = data.elementCodec().encodeStart(ops, value).getOrThrow();
                var bytes = new ByteArrayOutputStream();
                NbtIo.writeUnnamedTag(tag, new DataOutputStream(bytes));
                nbt = hex(bytes.toByteArray());
            }
            OUT.println("REG\t" + data.key().identifier() + "\t" + reg.getId(value) + "\t" + id + "\t" + nbt);
        }
    }

    @SuppressWarnings("unchecked")
    static <T> void dumpComponent(int idx, String item, RegistryAccess access, com.mojang.serialization.DynamicOps<HashCode> ops, DataComponentType<T> type, Object value) {
        T v = (T) value;
        ByteBuf raw = Unpooled.buffer();
        var buf = new RegistryFriendlyByteBuf(raw, access);
        ((net.minecraft.network.codec.StreamCodec<RegistryFriendlyByteBuf, T>) type.streamCodec()).encode(buf, v);
        byte[] bytes = new byte[raw.readableBytes()];
        raw.readBytes(bytes);
        // A type with no persistent codec cannot be hashed (vanilla refuses too); print "-".
        String hash = "-";
        if (!type.isTransient()) {
            HashCode h = new TypedDataComponent<>(type, v).encodeValue((com.mojang.serialization.DynamicOps<HashCode>) ops).getOrThrow();
            hash = Integer.toUnsignedString(h.asInt());
        }
        OUT.println("CASE\t" + idx + "\t" + BuiltInRegistries.DATA_COMPONENT_TYPE.getKey(type) + "\t" + hex(bytes) + "\t" + hash + "\t" + item);
    }

    static String hex(byte[] b) {
        StringBuilder s = new StringBuilder();
        for (byte x : b) s.append(String.format("%02x", x));
        return s.toString();
    }
}
