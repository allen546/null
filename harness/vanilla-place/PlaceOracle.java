import java.io.PrintStream;
import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

import com.mojang.authlib.GameProfile;
import net.minecraft.SharedConstants;
import net.minecraft.commands.arguments.blocks.BlockStateParser;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.level.ClientInformation;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.Property;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

// The vanilla oracle for block placement states: runs vanilla 1.21.11's own placement code on a
// real ServerLevel and prints, per case, the state vanilla would place.
// Usage: PlaceOracle <families.tsv> <out.tsv> [work-dir]
// Spec: specs/B6-placement-breaking.md §4.10 (the vanilla oracle)
public class PlaceOracle {
    // Bootstrap swaps System.out for a logger, so keep the real stream.
    static final PrintStream OUT = System.out;
    // The case origin: case coordinates are offsets from here. Chunk (0, 0), well above the flat world.
    static final int OX = 8, OY = 100, OZ = 8;

    public static void main(String[] a) throws Exception {
        try {
            run(a);
        } catch (Throwable t) {
            t.printStackTrace(OUT);
            System.exit(1);
        }
        System.exit(0);
    }

    static void run(String[] a) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Path work = a.length > 2 ? Path.of(a[2]) : Files.createTempDirectory("vanilla-place");
        OracleServer server = OracleServer.create(work);
        server.open();
        ServerLevel level = server.overworld();
        ServerPlayer player = new ServerPlayer(server, level, new GameProfile(UUID.nameUUIDFromBytes("oracle".getBytes()), "oracle"), ClientInformation.createDefault());

        List<Case> cases = Cases.generate(Path.of(a[0]));
        Method getPlacementState = BlockItem.class.getDeclaredMethod("getPlacementState", BlockPlaceContext.class);
        getPlacementState.setAccessible(true);

        StringBuilder sb = new StringBuilder();
        sb.append("# id\titem\tfamily\tclicked\tface\thit\tyaw\tpitch\tsneak\tsetup\tresult\twhy\n");
        List<BlockPos> dirty = new ArrayList<>();
        int realDiffs = 0, n = 0;
        for (Case c : cases) {
            // clear what the last case set, then build this scene (flags 18: no neighbour updates)
            for (BlockPos p : dirty) level.setBlock(p, net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(), 18);
            dirty.clear();
            for (String s : c.setup) {
                String[] kv = s.split("=", 2);
                String[] xyz = kv[0].split(",");
                BlockPos p = new BlockPos(OX + Integer.parseInt(xyz[0]), OY + Integer.parseInt(xyz[1]), OZ + Integer.parseInt(xyz[2]));
                level.setBlock(p, parse(kv[1]), 18);
                dirty.add(p);
            }
            Block block = BuiltInRegistries.BLOCK.getValue(net.minecraft.resources.Identifier.parse(c.item));
            ItemStack stack = new ItemStack(block.asItem());
            if (!(stack.getItem() instanceof BlockItem bi)) throw new IllegalStateException("not a block item: " + c.item);
            player.setYRot(c.yaw);
            // Player#tick copies the yaw to the head yaw; the look-direction code reads the head yaw
            player.setYHeadRot(c.yaw);
            player.setXRot(c.pitch);
            player.setShiftKeyDown(c.sneak);
            BlockPos clicked = new BlockPos(OX + c.cx, OY + c.cy, OZ + c.cz);
            BlockHitResult hit = new BlockHitResult(new Vec3(OX + c.hx, OY + c.hy, OZ + c.hz), c.face, clicked, false);

            // BlockItem#place up to the write: replace rule, placement state, survive and obstruction, unchanged state
            String result, why = "";
            BlockPlaceContext ctx = new BlockPlaceContext(player, InteractionHand.MAIN_HAND, stack, hit);
            BlockState placed = null;
            if (!block.isEnabled(level.enabledFeatures())) {
                result = "none"; why = "disabled";
            } else if (!ctx.canPlace()) {
                result = "none"; why = "not_replaceable";
            } else {
                BlockPlaceContext ctx2 = bi.updatePlacementContext(ctx);
                if (ctx2 == null) {
                    result = "none"; why = "no_context";
                } else {
                    placed = (BlockState) getPlacementState.invoke(bi, ctx2);
                    if (placed == null) {
                        result = "none";
                        why = bi.getBlock().getStateForPlacement(ctx2) == null ? "no_state" : "cannot_place";
                    } else if (level.getBlockState(ctx2.getClickedPos()) == placed) {
                        result = "none"; why = "same";
                        placed = null;
                    } else {
                        result = str(placed);
                        // the target cell when it is not the origin, for the fork to compare
                        String at = (ctx2.getClickedPos().getX() - OX) + "," + (ctx2.getClickedPos().getY() - OY) + "," + (ctx2.getClickedPos().getZ() - OZ);
                        why = at.equals("0,0,0") ? "" : "at=" + at;
                    }
                }
            }
            // cross-check: the real BlockItem#place on the server level must leave the state the steps above computed
            if (placed != null) {
                BlockPlaceContext real = new BlockPlaceContext(player, InteractionHand.MAIN_HAND, new ItemStack(block.asItem()), hit);
                BlockPos target = real.getClickedPos();
                bi.place(real);
                dirty.add(target);
                BlockState after = level.getBlockState(target);
                if (after != placed) {
                    realDiffs++;
                    OUT.println("REAL-PLACE DIFFERS " + c.id + ": steps " + str(placed) + " real " + str(after));
                }
            }
            sb.append(c.line(result, why)).append('\n');
            n++;
        }
        Files.writeString(Path.of(a[1]), sb.toString());
        OUT.println("real-place differences: " + realDiffs);
        OUT.println("cases " + n + " written to " + a[1]);
    }

    static BlockState parse(String s) throws Exception {
        return BlockStateParser.parseForBlock(BuiltInRegistries.BLOCK, s, false).blockState();
    }

    /** `minecraft:name[a=b,c=d]` with the properties sorted by name. */
    static String str(BlockState st) {
        String name = BuiltInRegistries.BLOCK.getKey(st.getBlock()).toString();
        List<String> props = new ArrayList<>();
        for (Property<?> p : st.getBlock().getStateDefinition().getProperties()) {
            props.add(p.getName() + "=" + valueName(st, p));
        }
        java.util.Collections.sort(props);
        return props.isEmpty() ? name : name + "[" + String.join(",", props) + "]";
    }

    static <T extends Comparable<T>> String valueName(BlockState st, Property<T> p) {
        return p.getName(st.getValue(p));
    }

    /** One input: where the click lands and what the look and the scene are. All positions are offsets from the origin. */
    static class Case {
        String id, item, family;
        int cx, cy, cz;
        Direction face;
        double hx, hy, hz; // the hit point, offsets from the origin
        float yaw, pitch;
        boolean sneak;
        List<String> setup = new ArrayList<>();

        String line(String result, String why) {
            // the namespace is dropped to keep the file small; every name here is in minecraft:
            return String.join("\t", id, item, family, cx + "," + cy + "," + cz, face.getName(),
                fmt(hx) + "," + fmt(hy) + "," + fmt(hz), fmt(yaw), fmt(pitch), sneak ? "1" : "0",
                setup.isEmpty() ? "-" : String.join(";", setup), result, why.isEmpty() ? "-" : why).replace("minecraft:", "");
        }
    }

    static String fmt(double d) {
        return d == Math.rint(d) ? Long.toString((long) d) : Double.toString(d);
    }
}
