import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;

import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.world.level.block.Block;

// The input list: which scenes, clicks and looks to ask vanilla about, per family of the fork's
// place prediction. Neighbour-heavy families use a fixed-seed random sample over a palette of
// neighbour states (same TSV every run); the rest are enumerated by hand from each rule's branches.
// All positions are offsets from the case origin; the target cell (where a block goes when the click
// lands on a neighbour's face) is the origin.
// Spec: specs/B6-placement-breaking.md §4.10 (the vanilla oracle)
public class Cases {
    static final Direction N = Direction.NORTH, E = Direction.EAST, S = Direction.SOUTH, W = Direction.WEST,
        U = Direction.UP, D = Direction.DOWN;
    static final Direction[] H = {N, E, S, W};
    static final Direction[] ALL = {D, U, N, S, W, E};
    static final String STONE = "minecraft:stone";
    // yaws that sit on, near and across the 45 degree borders of Direction#fromYRot
    static final float[] YAW_FULL = {0, 30, 44, 45, 46, 90, 140, 180, 225, 250, 270, -30, -45, -100, -180, 400};
    static final float[] YAW_4 = {0, 90, 180, -90};
    static final float[] YAW_6 = {0, 45, 90, 180, 225, -100};

    static List<PlaceOracle.Case> out;
    static Map<String, List<String>> fam;
    static Map<String, String> idOf = new LinkedHashMap<>();
    static Random rnd;
    static int seq;

    static List<PlaceOracle.Case> generate(Path families) throws Exception {
        out = new ArrayList<>();
        fam = new LinkedHashMap<>();
        for (Block b : BuiltInRegistries.BLOCK) {
            String path = BuiltInRegistries.BLOCK.getKey(b).getPath();
            idOf.put(camel(path), "minecraft:" + path);
        }
        for (String line : Files.readAllLines(families)) {
            if (line.startsWith("#") || line.isBlank()) continue;
            String[] f = line.split("\t");
            String id = idOf.get(f[1]);
            if (id == null) throw new IllegalStateException("no vanilla block for " + f[1]);
            fam.computeIfAbsent(f[0], k -> new ArrayList<>()).add(id);
        }
        rnd = new Random(20261115L);
        plain(); axis(); axisWaterlogged(); waterloggable(); slab(); stairs(); horizontalFacing(); clockwise();
        looking(); vegetation(); clickedFace(); snowy(); hugeMushroom(); torch(); ladder(); fence(); pane(); wall();
        fenceGate(); trapdoor(); leaves(); chest(); rail();
        return out;
    }

    static String camel(String snake) {
        StringBuilder sb = new StringBuilder();
        for (String part : snake.split("_")) sb.append(Character.toUpperCase(part.charAt(0))).append(part.substring(1));
        return sb.toString();
    }

    static List<String> items(String family) {
        List<String> l = fam.get(family);
        return l == null ? List.of() : l;
    }

    /** The first block of the family plus the named ones that exist: these get the full case set. */
    static boolean rep(String family, String item) {
        if (family.equals("Stairs")) return item.equals("minecraft:oak_stairs");
        List<String> l = items(family);
        return l.get(0).equals(item) || List.of("minecraft:oak_stairs", "minecraft:stone_stairs", "minecraft:oak_slab",
            "minecraft:oak_fence", "minecraft:oak_trapdoor", "minecraft:oak_fence_gate", "minecraft:oak_leaves",
            "minecraft:glass_pane", "minecraft:cobblestone_wall", "minecraft:barrel", "minecraft:piston").contains(item);
    }

    // ---- building blocks -------------------------------------------------------------------------

    static String cell(int x, int y, int z, String st) { return x + "," + y + "," + z + "=" + st; }
    static String at(Direction d, String st) { return cell(d.getStepX(), d.getStepY(), d.getStepZ(), st); }
    static String at2(Direction d, int k, String st) { return cell(d.getStepX() * k, d.getStepY() * k, d.getStepZ() * k, st); }

    /**
     * A click on the face `face` of the block next to the origin, so the block goes into the origin.
     * `a` and `b` place the hit on that face (x,z for up/down; for a side face a is the height and b the position along the face).
     * `clicked` is the clicked block. `setup` adds more cells (a later entry for the same cell wins).
     */
    static PlaceOracle.Case click(String family, String item, Direction face, double a, double b, float yaw, float pitch,
                                  boolean sneak, String clicked, String... setup) {
        int cx = -face.getStepX(), cy = -face.getStepY(), cz = -face.getStepZ();
        double fx, fy, fz;
        switch (face.getAxis()) {
            case Y -> { fx = a; fz = b; fy = face.getStepY() > 0 ? 1 : 0; }
            case X -> { fy = a; fz = b; fx = face.getStepX() > 0 ? 1 : 0; }
            default -> { fy = a; fx = b; fz = face.getStepZ() > 0 ? 1 : 0; }
        }
        List<String> all = new ArrayList<>();
        all.add(cell(cx, cy, cz, clicked));
        for (String s : setup) all.add(s);
        return add(family, item, cx, cy, cz, face, cx + fx, cy + fy, cz + fz, yaw, pitch, sneak, all);
    }

    /** A click with the clicked cell the origin itself (the placed block replaces the clicked block). */
    static PlaceOracle.Case clickOrigin(String family, String item, Direction face, double fx, double fy, double fz,
                                        float yaw, float pitch, boolean sneak, String... setup) {
        return add(family, item, 0, 0, 0, face, fx, fy, fz, yaw, pitch, sneak, new ArrayList<>(List.of(setup)));
    }

    static PlaceOracle.Case add(String family, String item, int cx, int cy, int cz, Direction face, double hx, double hy,
                                double hz, float yaw, float pitch, boolean sneak, List<String> setup) {
        PlaceOracle.Case k = new PlaceOracle.Case();
        k.id = family + "-" + (seq++);
        k.item = item; k.family = family;
        k.cx = cx; k.cy = cy; k.cz = cz; k.face = face;
        k.hx = hx; k.hy = hy; k.hz = hz;
        k.yaw = yaw; k.pitch = pitch; k.sneak = sneak;
        k.setup = setup;
        out.add(k);
        return k;
    }

    static final String WATER = "minecraft:water[level=0]", FLOWING = "minecraft:water[level=3]";

    /** A random click: a random face of a stone block next to the origin, a random look, the given neighbour cells. */
    static PlaceOracle.Case randomClick(String family, String item, List<String> cells, boolean sneakOk) {
        Direction face = ALL[rnd.nextInt(6)];
        double a = rnd.nextBoolean() ? 0.25 : 0.75, b = rnd.nextBoolean() ? 0.25 : 0.75;
        float yaw = YAW_FULL[rnd.nextInt(YAW_FULL.length)];
        float pitch = new float[]{0, 0, 20, -20, 60, -60, 85}[rnd.nextInt(7)];
        String clickedKey = cell(-face.getStepX(), -face.getStepY(), -face.getStepZ(), "").replace("=", "");
        List<String> keep = new ArrayList<>();
        for (String c : cells) if (!c.startsWith(clickedKey + "=")) keep.add(c);
        return click(family, item, face, a, b, yaw, pitch, sneakOk && rnd.nextBoolean(), STONE, keep.toArray(new String[0]));
    }

    static String pick(String... palette) { return palette[rnd.nextInt(palette.length)]; }

    // ---- families ---------------------------------------------------------------------------------

    // Block#getStateForPlacement returns the default state; what varies is the replace rule and the cell.
    static void plain() {
        for (String it : items("Plain")) {
            click("Plain", it, U, .5, .5, 0, 0, false, STONE);
            click("Plain", it, E, .25, .75, 90, 0, false, STONE, at(N, STONE));
            click("Plain", it, W, .5, .5, 0, 0, false, STONE, at(D, STONE), "0,0,0=minecraft:short_grass");
            clickOrigin("Plain", it, U, .5, 1, .5, 0, 0, false, "0,0,0=minecraft:short_grass");
            click("Plain", it, N, .5, .5, 0, 0, false, STONE, "0,0,0=" + STONE);
            click("Plain", it, S, .5, .5, 0, 0, false, STONE, "0,0,0=" + WATER);
        }
    }

    // RotatedPillarBlock: the axis of the clicked face.
    static void axis() {
        for (String it : items("Axis")) for (Direction f : ALL) click("Axis", it, f, .5, .5, 0, 0, false, STONE);
    }

    // ChainBlock: the pillar axis plus water.
    static void axisWaterlogged() {
        for (String it : items("AxisWaterlogged")) for (Direction f : ALL) {
            click("AxisWaterlogged", it, f, .5, .5, 0, 0, false, STONE);
            click("AxisWaterlogged", it, f, .5, .5, 0, 0, false, STONE, "0,0,0=" + WATER);
            click("AxisWaterlogged", it, f, .5, .5, 0, 0, false, STONE, "0,0,0=" + FLOWING);
        }
    }

    // The default state, waterlogged by a water source in the cell (not by flowing water).
    static void waterloggable() {
        for (String it : items("Waterloggable")) {
            click("Waterloggable", it, U, .5, .5, 0, 0, false, STONE);
            click("Waterloggable", it, U, .5, .5, 0, 0, false, STONE, "0,0,0=" + WATER);
            click("Waterloggable", it, E, .5, .5, 0, 0, false, STONE, "0,0,0=" + FLOWING);
            click("Waterloggable", it, E, .5, .5, 0, 0, false, STONE, "0,0,0=minecraft:water[level=8]");
            click("Waterloggable", it, E, .5, .5, 0, 0, false, STONE, "0,0,0=minecraft:short_grass");
        }
    }

    // Lightning rods: facing the clicked face plus water.
    static void clickedFace() {
        for (String it : items("ClickedFaceWaterlogged")) for (Direction f : ALL) {
            click("ClickedFaceWaterlogged", it, f, .5, .5, 0, 0, false, STONE);
            click("ClickedFaceWaterlogged", it, f, .5, .5, 0, 0, false, STONE, "0,0,0=" + WATER);
        }
    }

    // SlabBlock: type from the face and hit height, double when the slab joins, water.
    static void slab() {
        for (String it : items("Slab")) {
            String bottom = it + "[type=bottom]", top = it + "[type=top]", dbl = it + "[type=double]";
            click("Slab", it, U, .5, .5, 0, 0, false, STONE);
            click("Slab", it, D, .5, .5, 0, 0, false, STONE);
            click("Slab", it, N, .75, .5, 0, 0, false, STONE);
            click("Slab", it, N, .25, .5, 0, 0, false, STONE);
            click("Slab", it, E, .5, .5, 0, 0, false, STONE, "0,0,0=" + WATER);
            clickOrigin("Slab", it, U, .5, 1, .5, 0, 0, false, "0,0,0=" + bottom);
            if (!rep("Slab", it)) continue;
            for (double y : new double[]{0, .25, .5, .51, .75, 1}) {
                for (Direction f : H) {
                    click("Slab", it, f, y, .5, 0, 0, false, STONE);
                    // the clicked block is the slab itself
                    for (String type : new String[]{bottom, top}) {
                        double[] hit = f == N ? new double[]{.5, y, 0} : f == S ? new double[]{.5, y, 1} : f == W ? new double[]{0, y, .5} : new double[]{1, y, .5};
                        clickOrigin("Slab", it, f, hit[0], hit[1], hit[2], 0, 0, false, "0,0,0=" + type);
                    }
                }
            }
            for (Direction f : ALL) {
                clickOrigin("Slab", it, f, .5 + .5 * f.getStepX(), .5 + .5 * f.getStepY(), .5 + .5 * f.getStepZ(), 0, 0, false, "0,0,0=" + bottom);
                clickOrigin("Slab", it, f, .5 + .5 * f.getStepX(), .5 + .5 * f.getStepY(), .5 + .5 * f.getStepZ(), 0, 0, false, "0,0,0=" + top);
                clickOrigin("Slab", it, f, .5 + .5 * f.getStepX(), .5 + .5 * f.getStepY(), .5 + .5 * f.getStepZ(), 0, 0, false, "0,0,0=" + dbl);
                // a slab of this kind already in the target cell, clicked beside it: it joins
                click("Slab", it, f, .75, .5, 0, 0, false, STONE, "0,0,0=" + bottom);
                click("Slab", it, f, .25, .5, 0, 0, false, STONE, "0,0,0=" + top);
                click("Slab", it, f, .5, .5, 0, 0, false, STONE, "0,0,0=" + dbl);
                click("Slab", it, f, .5, .5, 0, 0, false, STONE, "0,0,0=minecraft:stone_slab[type=bottom]");
                click("Slab", it, f, .5, .5, 0, 0, false, STONE, "0,0,0=" + bottom.replace("[", "[waterlogged=true,"));
            }
        }
    }

    static String stair(String kind, Direction facing, String half) {
        return kind + "[facing=" + facing.getName() + ",half=" + half + "]";
    }

    // StairBlock: facing, half, water and the corner shape from the stairs in front and behind.
    static void stairs() {
        for (String it : items("Stairs")) {
            click("Stairs", it, U, .5, .5, 0, 0, false, STONE);
            click("Stairs", it, D, .5, .5, 90, 0, false, STONE);
            click("Stairs", it, E, .75, .5, 180, 0, false, STONE);
            click("Stairs", it, E, .25, .5, -90, 0, false, STONE);
            click("Stairs", it, N, .5, .5, 0, 0, false, STONE, "0,0,0=" + WATER);
            if (!rep("Stairs", it)) {
                // one outer and one inner corner, so every kind is checked for the shape rule
                click("Stairs", it, U, .5, .5, 0, 0, false, STONE, at(S, stair(it, E, "bottom")));
                click("Stairs", it, U, .5, .5, 0, 0, false, STONE, at(N, stair(it, W, "bottom")));
                continue;
            }
            for (float yaw : YAW_FULL) {
                click("Stairs", it, U, .5, .5, yaw, 0, false, STONE);
                click("Stairs", it, D, .5, .5, yaw, 0, false, STONE);
                click("Stairs", it, S, .5, .5, yaw, 0, false, STONE);
                click("Stairs", it, S, .75, .5, yaw, 0, false, STONE);
                click("Stairs", it, S, .25, .5, yaw, 0, false, STONE);
                click("Stairs", it, S, .5, .5, yaw, 0, false, STONE, "0,0,0=" + FLOWING);
            }
            // shape: every neighbour in front and behind, in each facing and half, for four looks
            for (Direction look : H) {
                float yaw = lookYaw(look);
                for (String half : new String[]{"bottom", "top"}) {
                    Direction face = half.equals("bottom") ? U : D;
                    List<String> opts = new ArrayList<>();
                    opts.add(null);
                    for (Direction d : H) { opts.add(stair(it, d, "bottom")); opts.add(stair(it, d, "top")); }
                    opts.add(STONE);
                    for (String front : opts) for (String behind : opts) {
                        List<String> cells = new ArrayList<>();
                        if (front != null) cells.add(at(look, front));
                        if (behind != null) cells.add(at(look.getOpposite(), behind));
                        click("Stairs", it, face, .5, .5, yaw, 0, false, STONE, cells.toArray(new String[0]));
                    }
                    // canTakeShape: a same-facing same-half stair beside the corner stair blocks the corner
                    for (Direction side : new Direction[]{look.getClockWise(), look.getCounterClockWise()}) {
                        for (String blocker : new String[]{null, stair(it, look, half), stair(it, look, half.equals("bottom") ? "top" : "bottom"),
                            stair(it, look.getOpposite(), half), stair(it, side, half)}) {
                            // outer corner: a stair in front turned to the side; the cell on its opposite side is checked
                            List<String> cells = new ArrayList<>();
                            cells.add(at(look, stair(it, side, half)));
                            if (blocker != null) cells.add(at(side.getOpposite(), blocker));
                            click("Stairs", it, face, .5, .5, yaw, 0, false, STONE, cells.toArray(new String[0]));
                            // inner corner: a stair behind turned to the side; the cell on that side is checked
                            cells = new ArrayList<>();
                            cells.add(at(look.getOpposite(), stair(it, side, half)));
                            if (blocker != null) cells.add(at(side, blocker));
                            click("Stairs", it, face, .5, .5, yaw, 0, false, STONE, cells.toArray(new String[0]));
                        }
                    }
                }
            }
        }
    }

    /** The yaw that looks along `d` (Direction#fromYRot(yaw) == d). */
    static float lookYaw(Direction d) { return d.toYRot(); }

    // Facing the opposite of the player's horizontal direction (furnaces and the like).
    static void horizontalFacing() {
        for (String it : items("HorizontalFacing")) {
            float[] yaws = rep("HorizontalFacing", it) ? YAW_FULL : YAW_6;
            for (float y : yaws) click("HorizontalFacing", it, U, .5, .5, y, 0, false, STONE);
            click("HorizontalFacing", it, E, .5, .5, 0, 0, false, STONE, "0,0,0=" + WATER);
        }
    }

    // AnvilBlock: the horizontal direction turned a quarter clockwise.
    static void clockwise() {
        for (String it : items("FacingClockwise")) {
            for (float y : YAW_FULL) click("FacingClockwise", it, U, .5, .5, y, 0, false, STONE);
            click("FacingClockwise", it, E, .5, .5, 0, 0, false, STONE, "0,0,0=" + WATER);
        }
    }

    // The nearest looking direction (all six): a grid of looks, plus the clicked face.
    static void looking() {
        float[] yaws = {0, 20, 45, 70, 90, 110, 135, 160, 180, 200, 250, 270, -20, -90};
        float[] pitches = {-85, -60, -45, -30, -20, 0, 20, 30, 45, 60, 85, 90, -90};
        for (String family : new String[]{"LookingOpposite", "LookingSame"}) {
            for (String it : items(family)) {
                boolean full = rep(family, it) || family.equals("LookingSame");
                for (float yaw : yaws) for (float pitch : pitches) {
                    if (!full && ((int) (yaw + pitch) % 3 != 0)) continue;
                    click(family, it, U, .5, .5, yaw, pitch, false, STONE);
                }
                for (Direction f : ALL) click(family, it, f, .5, .5, 30, 20, false, STONE);
                click(family, it, E, .5, .5, 30, 20, false, STONE, "0,0,0=" + WATER);
            }
        }
    }

    // VegetationBlock: the block below must be dirt-like (or farmland), else the place fails.
    static void vegetation() {
        String[] below = {"minecraft:dirt", "minecraft:grass_block", "minecraft:podzol", "minecraft:coarse_dirt", "minecraft:mycelium",
            "minecraft:rooted_dirt", "minecraft:moss_block", "minecraft:mud", "minecraft:muddy_mangrove_roots", "minecraft:farmland",
            "minecraft:stone", "minecraft:sand", "minecraft:glass", "minecraft:oak_leaves", "minecraft:dirt_path", "minecraft:soul_sand",
            "minecraft:netherrack", "minecraft:clay", "minecraft:gravel", "minecraft:water[level=0]"};
        for (String it : items("Vegetation")) {
            boolean full = rep("Vegetation", it);
            for (int i = 0; i < below.length; i++) {
                if (!full && i > 12) continue;
                click("Vegetation", it, U, .5, .5, 0, 0, false, below[i]);
            }
            // beside a wall with dirt below the target; replacing a plant clicked
            click("Vegetation", it, E, .5, .5, 0, 0, false, STONE, at(D, "minecraft:dirt"));
            clickOrigin("Vegetation", it, U, .5, 1, .5, 0, 0, false, "0,0,0=minecraft:short_grass", at(D, "minecraft:dirt"));
            click("Vegetation", it, U, .5, .5, 0, 0, false, "minecraft:dirt", "0,0,0=minecraft:short_grass");
        }
    }

    // SnowyDirtBlock: snowy when snow (layer, block or powder) lies above.
    static void snowy() {
        String[] above = {null, "minecraft:snow", "minecraft:snow[layers=8]", "minecraft:snow_block", "minecraft:powder_snow", STONE, "minecraft:ice", "minecraft:water[level=0]"};
        for (String it : items("SnowyTop")) for (String a : above) for (Direction f : new Direction[]{U, E}) {
            click("SnowyTop", it, f, .5, .5, 0, 0, false, STONE, a == null ? new String[0] : new String[]{at(U, a)});
        }
    }

    // HugeMushroomBlock: a side shows unless the block there is the same block.
    static void hugeMushroom() {
        for (String it : items("HugeMushroom")) {
            click("HugeMushroom", it, U, .5, .5, 0, 0, false, STONE);
            List<String> others = new ArrayList<>(items("HugeMushroom"));
            for (Direction d : ALL) {
                click("HugeMushroom", it, d, .5, .5, 0, 0, false, STONE, at(d.getOpposite(), it));
                for (String other : others) if (!other.equals(it)) click("HugeMushroom", it, d, .5, .5, 0, 0, false, STONE, at(d.getOpposite(), other));
            }
            for (int i = 0; i < 12; i++) {
                List<String> cells = new ArrayList<>();
                for (Direction d : ALL) if (rnd.nextBoolean()) cells.add(at(d, pick(it, it, others.get(rnd.nextInt(others.size())), STONE)));
                randomClick("HugeMushroom", it, cells, false);
            }
        }
    }

    static final String[] SUPPORT = {null, STONE, "minecraft:glass", "minecraft:oak_leaves", "minecraft:oak_fence", "minecraft:oak_slab[type=bottom]",
        "minecraft:oak_slab[type=top]", "minecraft:oak_stairs[facing=north,half=bottom]", "minecraft:oak_stairs[facing=north,half=top]", "minecraft:iron_chain[axis=y]",
        "minecraft:cobblestone_wall", "minecraft:hopper", "minecraft:snow[layers=4]", "minecraft:ice", "minecraft:oak_trapdoor[half=top,open=false]",
        "minecraft:oak_trapdoor[half=bottom,open=true,facing=north]", "minecraft:soul_sand", "minecraft:dirt_path", "minecraft:oak_fence_gate[open=false]", "minecraft:iron_bars"};

    // StandingAndWallBlockItem: a standing torch on centre support, else a wall torch for the first look direction that works.
    static void torch() {
        for (String it : items("Torch")) {
            for (String s : SUPPORT) {
                if (s == null) continue;
                click("Torch", it, U, .5, .5, 0, 0, false, s);
                for (Direction f : H) click("Torch", it, f, .5, .5, 0, 0, false, s, at(D, STONE));
            }
            for (int i = 0; i < (rep("Torch", it) ? 220 : 60); i++) {
                List<String> cells = new ArrayList<>();
                for (Direction d : new Direction[]{N, E, S, W, D}) {
                    String s = pick(SUPPORT);
                    if (s != null) cells.add(at(d, s));
                }
                if (rnd.nextInt(8) == 0) cells.add("0,0,0=" + pick(WATER, FLOWING, "minecraft:short_grass"));
                randomClick("Torch", it, cells, false);
            }
        }
    }

    // LadderBlock: faces away from the first horizontal look direction with a sturdy block behind.
    static void ladder() {
        for (String it : items("Ladder")) {
            for (int i = 0; i < 300; i++) {
                List<String> cells = new ArrayList<>();
                for (Direction d : H) {
                    String s = pick(SUPPORT);
                    if (s != null) cells.add(at(d, s));
                }
                if (rnd.nextInt(6) == 0) cells.add("0,0,0=" + pick(WATER, FLOWING));
                if (rnd.nextInt(5) == 0) cells.add(at(rnd.nextBoolean() ? U : D, "minecraft:ladder[facing=" + H[rnd.nextInt(4)].getName() + "]"));
                randomClick("Ladder", it, cells, false);
            }
            // clicking a ladder that already faces the clicked face
            for (Direction f : H) {
                click("Ladder", it, f, .5, .5, 0, 0, false, "minecraft:ladder[facing=" + f.getName() + "]");
                click("Ladder", it, f, .5, .5, 0, 0, false, "minecraft:ladder[facing=" + f.getOpposite().getName() + "]");
            }
        }
    }

    static final String[] CONNECT = {null, null, STONE, "minecraft:oak_fence", "minecraft:nether_brick_fence", "minecraft:spruce_fence", "minecraft:glass",
        "minecraft:oak_fence_gate[facing=north]", "minecraft:oak_fence_gate[facing=east]", "minecraft:oak_fence_gate[facing=east,open=true]",
        "minecraft:oak_slab[type=bottom]", "minecraft:oak_slab[type=top]", "minecraft:oak_stairs[facing=north]", "minecraft:oak_stairs[facing=east,half=top]",
        "minecraft:oak_leaves", "minecraft:cobblestone_wall", "minecraft:iron_bars", "minecraft:glass_pane", "minecraft:barrier", "minecraft:chest",
        "minecraft:hopper", "minecraft:oak_trapdoor[facing=north,open=false]", "minecraft:oak_trapdoor[facing=north,open=true]", "minecraft:lantern",
        "minecraft:snow[layers=3]", "minecraft:dirt_path", "minecraft:cactus", "minecraft:oak_door[half=lower,facing=north]", "minecraft:oak_button[face=wall,facing=north]",
        "minecraft:piston[facing=up,extended=false]", "minecraft:white_stained_glass_pane", "minecraft:copper_bars", "minecraft:redstone_wire[power=0]", "minecraft:water[level=0]", "minecraft:torch"};

    static List<String> around(String[] palette, Direction[] dirs) {
        List<String> cells = new ArrayList<>();
        for (Direction d : dirs) {
            String s = pick(palette);
            if (s != null) cells.add(at(d, s));
        }
        return cells;
    }

    // FenceBlock: four sides from sturdy faces and same-kind fences (wooden or not), gates across, water.
    static void fence() {
        for (String it : items("Fence")) {
            int n = rep("Fence", it) ? 240 : 10;
            for (int i = 0; i < n; i++) {
                List<String> cells = around(CONNECT, H);
                if (rnd.nextInt(6) == 0) cells.add("0,0,0=" + pick(WATER, FLOWING));
                randomClick("Fence", it, cells, false);
            }
        }
    }

    // IronBarsBlock: four sides from sturdy faces, panes, bars and walls, water.
    static void pane() {
        for (String it : items("Pane")) {
            int n = rep("Pane", it) ? 220 : 8;
            for (int i = 0; i < n; i++) {
                List<String> cells = around(CONNECT, H);
                if (rnd.nextInt(6) == 0) cells.add("0,0,0=" + pick(WATER, FLOWING));
                randomClick("Pane", it, cells, false);
            }
        }
    }

    static final String[] ABOVE = {null, null, STONE, "minecraft:cobblestone_wall", "minecraft:cobblestone_wall[up=false,north=tall]", "minecraft:oak_slab[type=bottom]",
        "minecraft:oak_slab[type=top]", "minecraft:oak_stairs[half=top]", "minecraft:oak_stairs[half=bottom]", "minecraft:iron_bars", "minecraft:glass_pane", "minecraft:torch",
        "minecraft:oak_fence", "minecraft:iron_chain[axis=y]", "minecraft:glass", "minecraft:hopper", "minecraft:lantern[hanging=true]", "minecraft:oak_trapdoor[half=bottom,open=false]",
        "minecraft:oak_trapdoor[half=top,open=false]", "minecraft:oak_fence_gate", "minecraft:lever[face=floor]", "minecraft:oak_pressure_plate"};

    // WallBlock: arms low or tall, the post, from neighbours and the block above, water.
    static void wall() {
        for (String it : items("Wall")) {
            int n = rep("Wall", it) ? 280 : 10;
            for (int i = 0; i < n; i++) {
                List<String> cells = around(CONNECT, H);
                String above = pick(ABOVE);
                if (above != null) cells.add(at(U, above));
                // sometimes a block above a neighbour, to move an arm height
                if (rnd.nextInt(5) == 0) {
                    String s = pick(ABOVE);
                    if (s != null) cells.add(cell(H[rnd.nextInt(4)].getStepX(), 1, H[rnd.nextInt(4)].getStepZ(), s));
                }
                if (rnd.nextInt(6) == 0) cells.add("0,0,0=" + pick(WATER, FLOWING));
                randomClick("Wall", it, cells, false);
            }
        }
    }

    // FenceGateBlock: facing from the look, in_wall from walls beside, open and powered from redstone.
    static void fenceGate() {
        String[] side = {null, null, null, STONE, "minecraft:cobblestone_wall", "minecraft:andesite_wall", "minecraft:oak_fence", "minecraft:oak_slab[type=bottom]", "minecraft:glass"};
        String[] power = {null, null, null, null, "minecraft:redstone_block", "minecraft:lever[powered=true,face=floor]", "minecraft:redstone_torch", "minecraft:oak_button[powered=true]", "minecraft:redstone_wire[power=5]", "minecraft:target"};
        for (String it : items("FenceGate")) {
            int n = rep("FenceGate", it) ? 160 : 10;
            for (int i = 0; i < n; i++) {
                List<String> cells = around(side, H);
                if (rnd.nextInt(3) == 0) {
                    String p = pick(power);
                    if (p != null) cells.add(at(ALL[rnd.nextInt(6)], p));
                }
                randomClick("FenceGate", it, cells, false);
            }
        }
    }

    // TrapDoorBlock: facing and half from the click and the look, open and powered from redstone, water.
    static void trapdoor() {
        for (String it : items("Trapdoor")) {
            click("Trapdoor", it, U, .5, .5, 0, 0, false, STONE);
            click("Trapdoor", it, D, .5, .5, 90, 0, false, STONE);
            click("Trapdoor", it, E, .25, .5, 90, 0, false, STONE);
            click("Trapdoor", it, E, .75, .5, 90, 0, false, STONE);
            click("Trapdoor", it, N, .75, .5, 180, 0, false, STONE, "0,0,0=" + WATER);
            click("Trapdoor", it, S, .25, .5, 0, 0, false, STONE, "0,0,0=" + FLOWING);
            if (!rep("Trapdoor", it)) continue;
            for (Direction f : ALL) for (double y : new double[]{0, .25, .5, .51, .75, 1}) for (float yaw : YAW_4) {
                String w = rnd.nextInt(3) == 0 ? "0,0,0=" + WATER : "0,0,0=minecraft:short_grass";
                click("Trapdoor", it, f, f.getAxis() == Direction.Axis.Y ? .5 : y, .5, yaw, 0, false, STONE, w);
            }
            for (int i = 0; i < 40; i++) {
                List<String> cells = new ArrayList<>();
                cells.add(at(ALL[rnd.nextInt(6)], pick("minecraft:redstone_block", "minecraft:lever[powered=true]", STONE, "minecraft:redstone_torch", "minecraft:redstone_wire[power=3]", "minecraft:repeater[facing=north,powered=true]")));
                randomClick("Trapdoor", it, cells, false);
            }
        }
    }

    static final String[] LEAF_NEIGH = {null, null, null, "minecraft:oak_log[axis=y]", "minecraft:oak_log[axis=x]", "minecraft:oak_wood", "minecraft:oak_leaves[distance=1]",
        "minecraft:oak_leaves[distance=2]", "minecraft:oak_leaves[distance=4]", "minecraft:oak_leaves[distance=6]", "minecraft:oak_leaves[distance=7]", "minecraft:oak_leaves[distance=3,persistent=true]",
        "minecraft:cherry_leaves[distance=2]", STONE, "minecraft:dirt", "minecraft:stripped_oak_log[axis=z]", "minecraft:mangrove_roots", "minecraft:glass", "minecraft:oak_planks"};

    // LeavesBlock: persistent, distance 1 to 7 from logs and leaves, water.
    static void leaves() {
        for (String it : items("Leaves")) {
            int n = rep("Leaves", it) ? 200 : 14;
            for (int i = 0; i < n; i++) {
                List<String> cells = around(LEAF_NEIGH, ALL);
                if (rnd.nextInt(6) == 0) cells.add("0,0,0=" + pick(WATER, FLOWING));
                randomClick("Leaves", it, cells, false);
            }
        }
    }

    static final String[] CHESTS = {null, null, null, STONE, "minecraft:chest[facing=north,type=single]", "minecraft:chest[facing=south,type=single]", "minecraft:chest[facing=east,type=single]",
        "minecraft:chest[facing=west,type=single]", "minecraft:chest[facing=north,type=left]", "minecraft:chest[facing=north,type=right]", "minecraft:chest[facing=east,type=left]",
        "minecraft:trapped_chest[facing=north,type=single]", "minecraft:trapped_chest[facing=east,type=single]", "minecraft:ender_chest[facing=north]", "minecraft:barrel[facing=up]"};

    // ChestBlock: facing from the look, single or double from chests beside and from sneaking, water.
    static void chest() {
        for (String it : items("Chest")) {
            for (int i = 0; i < 300; i++) {
                List<String> cells = around(CHESTS, H);
                if (rnd.nextInt(6) == 0) cells.add("0,0,0=" + pick(WATER, FLOWING));
                randomClick("Chest", it, cells, true);
            }
            for (Direction d : H) for (Direction f : H) for (boolean sneak : new boolean[]{false, true}) {
                click("Chest", it, f, .5, .5, lookYaw(d), 0, sneak, STONE, at(f.getClockWise(), it + "[facing=" + d.getOpposite().getName() + ",type=single]"));
            }
        }
    }

    static final String[] RAIL_BELOW = {STONE, "minecraft:glass", "minecraft:oak_slab[type=bottom]", "minecraft:oak_slab[type=top]", "minecraft:oak_stairs[half=top]",
        "minecraft:oak_stairs[half=bottom]", "minecraft:oak_fence", "minecraft:hopper", "minecraft:oak_leaves", "minecraft:ice", "minecraft:snow[layers=8]", "minecraft:snow[layers=2]",
        "minecraft:soul_sand", "minecraft:oak_trapdoor[half=top,open=false]", "minecraft:chest", "minecraft:cobblestone_wall", "minecraft:glass_pane", "minecraft:composter", "minecraft:oak_planks"};

    // RailBlock: straight along the look, water, needs a rigid block below.
    static void rail() {
        for (String it : items("Rail")) {
            for (String b : RAIL_BELOW) for (float yaw : YAW_FULL) {
                click("Rail", it, U, .5, .5, yaw, 0, false, b);
            }
            click("Rail", it, U, .5, .5, 0, 0, false, STONE, "0,0,0=" + WATER);
            click("Rail", it, U, .5, .5, 90, 0, false, STONE, "0,0,0=" + FLOWING);
            click("Rail", it, E, .5, .5, 0, 0, false, STONE, at(D, STONE));
            click("Rail", it, E, .5, .5, 0, 0, false, STONE);
        }
    }
}
