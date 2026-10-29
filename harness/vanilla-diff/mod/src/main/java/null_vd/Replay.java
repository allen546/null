package null_vd;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import net.minecraft.client.MinecraftClient;

/**
 * VD1 replay driver. Reads an input log (the fork's {@code input_log.rs}: one JSON line per client
 * tick) and, on each client tick of the window, sets vanilla's key mappings and the player's look,
 * so vanilla's own {@code KeyboardInput#tick} builds the movement input and vanilla decides what
 * to send. It sends no packet itself.
 *
 * Window: from the mark {@code start:<row>} to the mark {@code end:<row>} when {@code -Dvd.row} is
 * given, else the whole file. It starts when the control file {@code -Dvd.control} exists and the
 * player stands still on the ground (the scene is set by RCON before). It writes
 * {@code -Dvd.marks} with the client tick (count of ClientTickEnd packets sent before it) at which
 * the replay began, so the differ can line the two tap logs up.
 */
public final class Replay {
    private static final Path INPUT = path("vd.input");
    private static final String ROW = System.getProperty("vd.row");
    private static final Path CONTROL = path("vd.control");
    private static final Path MARKS = path("vd.marks");
    // Player ticks of stillness before the window opens (vanilla's own physics must have settled).
    private static final int QUIET_TICKS = Integer.getInteger("vd.quietTicks", 10);
    // Ticks after the window before the client quits (late server replies are still logged).
    private static final int TAIL_TICKS = Integer.getInteger("vd.tailTicks", 40);

    /** One recorded tick. */
    record Sample(long tick, boolean f, boolean b, boolean l, boolean r, boolean jump, boolean sneak, boolean sprint,
                  float yaw, float pitch, int slot) {}

    private static List<Sample> window;
    private static boolean started = false;
    private static int next = 0;
    private static int quiet = 0;
    private static int tail = 0;
    /** ClientTickEnd packets vanilla has sent so far: it sends one at the end of every tick with a level. */
    private static long tickEnds = 0;

    private Replay() {}

    public static boolean enabled() {
        return INPUT != null;
    }

    private static Path path(String key) {
        String v = System.getProperty(key);
        return v == null ? null : Path.of(v);
    }

    // Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
    public static void onClientTick(MinecraftClient mc) {
        boolean inGame = mc.player != null && mc.world != null && mc.getNetworkHandler() != null;
        // Minecraft#tick sends ClientTickEnd at its end whenever a level exists (head test = end test).
        long thisTick = tickEnds;
        if (inGame) tickEnds++;
        if (!inGame) return;
        if (window == null) {
            if (CONTROL != null && !Files.exists(CONTROL)) return;
            window = load();
            Driver.log("replay: " + window.size() + " ticks loaded from " + INPUT + (ROW == null ? "" : " row " + ROW));
        }
        if (!started) {
            // Wait until the player is loaded (it ticks only then) and rests on the ground (vertical velocity stays at gravity's -0.0784 on the ground, so only x and z count).
            boolean still = mc.player.age >= 40 && mc.player.isOnGround()
                    && mc.player.getVelocity().x * mc.player.getVelocity().x + mc.player.getVelocity().z * mc.player.getVelocity().z < 1e-9;
            quiet = still ? quiet + 1 : 0;
            if (quiet < QUIET_TICKS) return;
            started = true;
            write("{\"replay_start_tick\":" + thisTick + ",\"ticks\":" + window.size() + ",\"first_input_tick\":"
                    + (window.isEmpty() ? -1 : window.get(0).tick()) + "}\n");
            Driver.log("replay start at client tick " + thisTick + ", " + window.size() + " ticks, player at "
                    + mc.player.getX() + " " + mc.player.getY() + " " + mc.player.getZ());
        }
        if (next < window.size()) {
            apply(mc, window.get(next++));
            return;
        }
        if (next == window.size()) {
            release(mc);
            next++;
            Driver.log("replay end at client tick " + thisTick + " at " + mc.player.getX() + " " + mc.player.getY() + " " + mc.player.getZ());
            return;
        }
        if (++tail == TAIL_TICKS) {
            Driver.log("stopping client");
            mc.scheduleStop();
        }
    }

    /** Sets the keys and the look the way input between two ticks would, before this tick reads them. */
    // Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
    private static void apply(MinecraftClient mc, Sample s) {
        var o = mc.options;
        o.forwardKey.setPressed(s.f());
        o.backKey.setPressed(s.b());
        o.leftKey.setPressed(s.l());
        o.rightKey.setPressed(s.r());
        o.jumpKey.setPressed(s.jump());
        o.sneakKey.setPressed(s.sneak());
        o.sprintKey.setPressed(s.sprint());
        // No key looks around; the mouse does, and the recorded look is the result of it.
        mc.player.setYaw(s.yaw());
        mc.player.setPitch(s.pitch());
        if (mc.player.getInventory().getSelectedSlot() != s.slot()) {
            mc.player.getInventory().setSelectedSlot(s.slot());
        }
    }

    private static void release(MinecraftClient mc) {
        var o = mc.options;
        for (var k : new net.minecraft.client.option.KeyBinding[] {o.forwardKey, o.backKey, o.leftKey, o.rightKey, o.jumpKey, o.sneakKey, o.sprintKey}) {
            k.setPressed(false);
        }
    }

    private static final Pattern NUM = Pattern.compile("\"(\\w+)\":(\"[^\"]*\"|[-0-9.eE+]+|true|false)");

    private static List<Sample> load() {
        List<String> lines;
        try {
            lines = Files.readAllLines(INPUT);
        } catch (IOException e) {
            throw new RuntimeException("cannot read " + INPUT, e);
        }
        long from = Long.MIN_VALUE, to = Long.MAX_VALUE;
        List<Sample> all = new ArrayList<>();
        for (String line : lines) {
            Matcher m = NUM.matcher(line);
            java.util.Map<String, String> kv = new java.util.HashMap<>();
            while (m.find()) kv.put(m.group(1), m.group(2));
            if (kv.containsKey("mark")) {
                String label = kv.get("mark").replace("\"", "");
                long tick = Long.parseLong(kv.get("tick"));
                if (ROW != null && label.equals("start:" + ROW)) from = tick;
                if (ROW != null && label.equals("end:" + ROW)) to = tick;
                continue;
            }
            if (!kv.containsKey("tick")) continue;
            all.add(new Sample(Long.parseLong(kv.get("tick")), bool(kv, "f"), bool(kv, "b"), bool(kv, "l"), bool(kv, "r"),
                    bool(kv, "jump"), bool(kv, "sneak"), bool(kv, "sprint"),
                    Float.parseFloat(kv.get("yaw")), Float.parseFloat(kv.get("pitch")), Integer.parseInt(kv.getOrDefault("slot", "0"))));
        }
        if (ROW != null && (from == Long.MIN_VALUE || to == Long.MAX_VALUE)) {
            throw new RuntimeException("row " + ROW + " has no start and end mark in " + INPUT);
        }
        List<Sample> out = new ArrayList<>();
        for (Sample s : all) if (s.tick() >= from && s.tick() < to) out.add(s);
        return out;
    }

    private static boolean bool(java.util.Map<String, String> kv, String k) {
        return "true".equals(kv.get(k));
    }

    private static void write(String text) {
        if (MARKS == null) return;
        try {
            Files.writeString(MARKS, text);
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }
}
