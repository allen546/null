import java.io.PrintStream;
import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.UUID;

import com.mojang.authlib.GameProfile;
import net.minecraft.SharedConstants;
import net.minecraft.commands.arguments.blocks.BlockStateParser;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.level.ClientInformation;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.CollisionContext;

// The vanilla oracle for one collided move: runs vanilla 1.21.11's own Entity#collide (the
// private step of Entity#move that holds the step-up logic) on a player-sized entity in a real
// ServerLevel, and prints the movement vector it returns. Why collide and not move(SELF): move
// adds sneak edge backing, stuck-in-block factors, fall tracking and block effects, none of
// which the fork's `collide` computes; the vanilla `move` calls `collide` once with the input
// vector, so this is the same collision decision.
// Usage: MoveOracle <out.tsv> [work-dir]
// Spec: specs/B1-movement.md §4.2
public class MoveOracle {
    static final PrintStream OUT = System.out;
    // Scene region: cells x 6..11, z 6..11, one chunk (0,0). Row y=99 holds floor and ledge cells,
    // y=98 is a stone base (top at 99.0), y=100 holds an optional block above a ledge cell.
    static final int X0 = 6, X1 = 11, Z0 = 6, Z1 = 11, BASE_Y = 98, ROW_Y = 99, UPPER_Y = 100;
    static final double R = (double) (0.6f / 2.0f); // half width as vanilla computes it (float)

    // Floor states with their top above the row cell's bottom.
    static final String[][] FLOORS = {
        {"air", "0"}, {"stone", "1"}, {"oak_slab[type=bottom]", "0.5"}, {"white_carpet", "0.0625"},
        {"oak_trapdoor[half=bottom,open=false]", "0.1875"}, {"snow[layers=5]", "0.5"},
        {"soul_sand", "0.875"}, {"farmland[moisture=0]", "0.9375"},
    };

    static final String[] LEDGES = {
        "stone", "oak_slab[type=bottom]", "oak_slab[type=top]", "oak_slab[type=double]",
        "oak_stairs[facing=east,half=bottom,shape=straight]", "oak_stairs[facing=west,half=bottom,shape=straight]",
        "oak_stairs[facing=north,half=bottom,shape=straight]", "oak_stairs[facing=south,half=bottom,shape=straight]",
        "oak_stairs[facing=east,half=top,shape=straight]", "oak_stairs[facing=west,half=top,shape=straight]",
        "oak_stairs[facing=north,half=top,shape=straight]", "oak_stairs[facing=south,half=top,shape=straight]",
        "oak_stairs[facing=east,half=bottom,shape=inner_left]", "oak_stairs[facing=east,half=bottom,shape=inner_right]",
        "oak_stairs[facing=east,half=bottom,shape=outer_left]", "oak_stairs[facing=north,half=bottom,shape=outer_right]",
        "oak_stairs[facing=east,half=top,shape=outer_left]", "oak_stairs[facing=south,half=top,shape=inner_right]",
        "snow[layers=1]", "snow[layers=2]", "snow[layers=3]", "snow[layers=4]", "snow[layers=5]",
        "snow[layers=6]", "snow[layers=7]", "snow[layers=8]",
        "white_carpet", "oak_trapdoor[half=bottom,open=false]", "oak_trapdoor[half=top,open=false]",
        "oak_trapdoor[half=bottom,open=true,facing=east]", "lily_pad", "daylight_detector", "cake[bites=0]",
        "cake[bites=3]", "soul_sand", "farmland[moisture=0]", "dirt_path", "honey_block",
        "end_portal_frame[eye=false]", "enchanting_table", "chest", "composter[level=0]", "cauldron",
        "flower_pot", "repeater", "comparator", "stonecutter", "hopper", "anvil", "candle", "white_bed[part=foot]",
        "oak_fence", "cobblestone_wall[up=true]", "oak_fence_gate[open=false]", "glass_pane", "bell[attachment=floor]",
        "grindstone", "lectern", "turtle_egg", "iron_chain", "lantern", "amethyst_cluster", "brewing_stand",
        "dragon_egg", "conduit", "sculk_sensor", "stone_button", "heavy_core", "end_rod", "cactus", "mud",
    };
    static final String[] UPPERS = {"air", "air", "air", "stone", "oak_slab[type=bottom]", "oak_slab[type=top]"};

    record Scene(int id, String kind, String floor, String l1, String l2, String upper, double floorTop) {}

    static BlockState parse(String s) throws Exception {
        return BlockStateParser.parseForBlock(BuiltInRegistries.BLOCK, s, false).blockState();
    }

    static String d(double v) { return Double.toString(v); }

    public static void main(String[] a) throws Exception {
        try {
            run(a);
        } catch (Throwable t) {
            t.printStackTrace(OUT);
            System.exit(1);
        }
        System.exit(0);
    }

    // Cell contents of a scene: a map from "x,y,z" to the state string.
    static Map<String, String> cells(Scene s) {
        Map<String, String> m = new LinkedHashMap<>();
        for (int x = X0; x <= X1; x++) {
            for (int z = Z0; z <= Z1; z++) {
                m.put(x + "," + BASE_Y + "," + z, "stone");
                boolean ledge = switch (s.kind) {
                    case "wall" -> x >= 9;
                    case "corner" -> x >= 9 && z >= 9;
                    default -> x == 9 && z == 9; // pillar
                };
                if (ledge) {
                    m.put(x + "," + ROW_Y + "," + z, x == 9 ? s.l1 : s.l2);
                    if (!s.upper.equals("air")) m.put(x + "," + UPPER_Y + "," + z, s.upper);
                } else if (!s.floor.equals("air")) {
                    m.put(x + "," + ROW_Y + "," + z, s.floor);
                }
            }
        }
        return m;
    }

    static void run(String[] a) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Path work = a.length > 1 ? Path.of(a[1]) : Files.createTempDirectory("vanilla-move");
        OracleServer server = OracleServer.create(work);
        server.open();
        ServerLevel level = server.overworld();
        ServerPlayer player = new ServerPlayer(server, level, new GameProfile(UUID.nameUUIDFromBytes("oracle".getBytes()), "oracle"), ClientInformation.createDefault());
        Method collide = Entity.class.getDeclaredMethod("collide", Vec3.class);
        collide.setAccessible(true);

        Random rnd = new Random(20261115L);
        List<Scene> scenes = new ArrayList<>();
        // Named scenes first: the six step_up_tests cases, rebuilt from vanilla blocks.
        // S0: ledge 0.6875 over a 0.1875 floor (soul sand over a trapdoor). S1: ledge 0.5 over no floor.
        scenes.add(new Scene(0, "wall", "oak_trapdoor[half=bottom,open=false]", "soul_sand", "soul_sand", "air", 0.1875));
        scenes.add(new Scene(1, "wall", "air", "oak_slab[type=bottom]", "oak_slab[type=bottom]", "air", 0.0));
        String[] kinds = {"wall", "wall", "wall", "corner", "corner", "pillar"};
        for (int i = 2; i < 260; i++) {
            String[] f = FLOORS[rnd.nextInt(FLOORS.length)];
            String l1 = LEDGES[rnd.nextInt(LEDGES.length)];
            String l2 = rnd.nextInt(3) == 0 ? LEDGES[rnd.nextInt(LEDGES.length)] : l1;
            scenes.add(new Scene(i, kinds[rnd.nextInt(kinds.length)], f[0], l1, l2, UPPERS[rnd.nextInt(UPPERS.length)], Double.parseDouble(f[1])));
        }

        StringBuilder out = new StringBuilder();
        out.append("# vanilla-move oracle: P state boxes | S id kind cells | C id scene pos onGround move box result\n");

        // P rows: vanilla's collision boxes of each palette state (separates shape errors from step errors).
        java.util.TreeSet<String> palette = new java.util.TreeSet<>();
        for (String[] f : FLOORS) palette.add(f[0]);
        for (String l : LEDGES) palette.add(l);
        for (String u : UPPERS) palette.add(u);
        BlockPos probe = new BlockPos(8, ROW_Y, 8);
        for (String p : palette) {
            BlockState st = parse(p);
            level.setBlock(probe, st, 18);
            List<AABB> boxes = level.getBlockState(probe).getCollisionShape(level, probe, CollisionContext.empty()).toAabbs();
            List<String> bs = new ArrayList<>();
            for (AABB b : boxes) bs.add(d(b.minX) + "," + d(b.minY) + "," + d(b.minZ) + "," + d(b.maxX) + "," + d(b.maxY) + "," + d(b.maxZ));
            out.append("P\t").append(p).append('\t').append(bs.isEmpty() ? "-" : String.join(";", bs)).append('\n');
        }

        for (Scene s : scenes) {
            Map<String, String> m = cells(s);
            StringBuilder cs = new StringBuilder();
            for (var e : m.entrySet()) cs.append(cs.length() > 0 ? ";" : "").append(e.getKey()).append('=').append(e.getValue());
            out.append("S\t").append(s.id).append('\t').append(s.kind).append('\t').append(cs).append('\n');
        }

        // Cases: (scene, start pos, onGround, move).
        record Case(int scene, double x, double y, double z, boolean ground, double dx, double dy, double dz) {}
        List<Case> cases = new ArrayList<>();
        double touchX = 9.0 - R;
        // The six step_up_tests cases (ids 0..5). Start bottom is the floor top plus the offset.
        double f0 = ROW_Y + 0.1875, f1 = ROW_Y;
        cases.add(new Case(0, touchX, f0 + 0.125, 8.0, false, 0.1, -0.3, 0.0));
        cases.add(new Case(0, touchX, f0 + 0.1, 8.0, false, 0.1, -0.3, 0.0));
        cases.add(new Case(1, touchX, f1 + 0.125, 8.0, false, 0.1, -0.3, 0.0));
        cases.add(new Case(0, touchX, f0, 8.0, true, 0.1, 0.0, 0.0));
        cases.add(new Case(1, touchX, f1, 8.0, true, 0.1, 0.0, 0.0));
        cases.add(new Case(1, touchX, f1 + 0.4, 8.0, false, 0.1, 0.0, 0.0));

        double[] fallDy = {-0.0784, -0.1552, -0.3, -0.08, -0.5, -0.8, 0.42, -0.0980, 0.0, -0.0784000015258789};
        double[] gaps = {0.0, 1e-7, 1e-5, 0.0625, 0.1, 0.25};
        for (int i = 0; i < 3600; i++) {
            Scene s = scenes.get(2 + rnd.nextInt(scenes.size() - 2));
            double off;
            int r = rnd.nextInt(20);
            if (r < 7) off = 0.0;
            else if (r < 14) off = (1 + rnd.nextInt(16)) / 16.0;
            else off = rnd.nextDouble();
            boolean ground = off == 0.0 ? rnd.nextInt(10) < 7 : rnd.nextInt(100) < 15;
            double y = ROW_Y + s.floorTop + off;
            double x = rnd.nextInt(3) == 0 ? touchX - gaps[rnd.nextInt(gaps.length)]
                : 7.0 + rnd.nextDouble() * 1.7;
            double z = s.kind.equals("wall") ? 7.0 + rnd.nextDouble() * 3.0 : 7.5 + rnd.nextDouble() * 2.2;
            if (!s.kind.equals("wall") && rnd.nextInt(2) == 0) z = 8.4 + rnd.nextDouble() * 1.2;
            double mag = rnd.nextInt(5) == 0 ? rnd.nextInt(13) / 16.0 : rnd.nextDouble() * 0.8;
            double ang;
            int ar = rnd.nextInt(10);
            if (s.kind.equals("wall")) ang = ar < 6 ? (rnd.nextDouble() - 0.5) * Math.PI * 2 / 3 : rnd.nextDouble() * 2 * Math.PI;
            else ang = ar < 7 ? rnd.nextDouble() * Math.PI / 2 : rnd.nextDouble() * 2 * Math.PI;
            double dx = mag * Math.cos(ang), dz = mag * Math.sin(ang);
            int axis = rnd.nextInt(8);
            if (axis == 0) dz = 0.0; else if (axis == 1) { dx = mag * Math.signum(Math.cos(ang)); dz = 0.0; }
            double dy;
            int yr = rnd.nextInt(10);
            if (yr < 3) dy = 0.0;
            else if (yr < 5) dy = fallDy[rnd.nextInt(fallDy.length)];
            else dy = -0.8 + rnd.nextDouble() * 1.22;
            cases.add(new Case(s.id, x, y, z, ground, dx, dy, dz));
        }

        Scene loaded = null;
        int n = 0;
        for (Case c : cases) {
            Scene s = scenes.get(c.scene);
            if (loaded != s) {
                // Clear the whole region column, then write the scene's cells.
                for (int x = X0; x <= X1; x++) for (int z = Z0; z <= Z1; z++) for (int y = BASE_Y; y <= UPPER_Y; y++)
                    level.setBlock(new BlockPos(x, y, z), net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(), 18);
                for (var e : cells(s).entrySet()) {
                    String[] p = e.getKey().split(",");
                    level.setBlock(new BlockPos(Integer.parseInt(p[0]), Integer.parseInt(p[1]), Integer.parseInt(p[2])), parse(e.getValue()), 18);
                }
                loaded = s;
            }
            player.setPos(c.x, c.y, c.z);
            player.setOnGround(c.ground);
            AABB bb = player.getBoundingBox();
            Vec3 res = (Vec3) collide.invoke(player, new Vec3(c.dx, c.dy, c.dz));
            out.append("C\t").append(n++).append('\t').append(c.scene).append('\t')
               .append(d(c.x)).append(',').append(d(c.y)).append(',').append(d(c.z)).append('\t')
               .append(c.ground ? 1 : 0).append('\t')
               .append(d(c.dx)).append(',').append(d(c.dy)).append(',').append(d(c.dz)).append('\t')
               .append(d(bb.minX)).append(',').append(d(bb.minY)).append(',').append(d(bb.minZ)).append(',')
               .append(d(bb.maxX)).append(',').append(d(bb.maxY)).append(',').append(d(bb.maxZ)).append('\t')
               .append(d(res.x)).append(',').append(d(res.y)).append(',').append(d(res.z)).append('\n');
        }
        Files.writeString(Path.of(a[0]), out.toString());
        OUT.println("scenes " + scenes.size() + " palette " + palette.size() + " cases " + n + " bytes " + out.length());
    }
}
