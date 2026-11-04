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
 * VD2 actions: {@code attack} sets the attack key (held while Null mines: vanilla's
 * {@code Minecraft#continueAttack}); the events press the use, swap-offhand or inventory key by
 * counting one click on the key mapping ({@code consumeClick}), the way a key event between two
 * ticks does, so {@code Minecraft#handleKeybinds} runs {@code startUseItem}, the offhand swap or
 * the inventory screen itself. Screen events ({@code click}, {@code close}) are the screen's frame
 * input in vanilla, which comes between two ticks: they run here at the head of the tick before
 * vanilla's tick body, so their packets come after the previous tick's {@code ClientTickEnd} and
 * before this tick's input packets, as in Null's tick order.
 *
 * Window: from the mark {@code start:<row>} to the mark {@code end:<row>} when {@code -Dvd.row} is
 * given, else the whole file. It starts when the control file {@code -Dvd.control} exists and the
 * player stands still on the ground (the scene is set by RCON before). It writes
 * {@code -Dvd.marks} with the client tick (count of ClientTickEnd packets sent before it) at which
 * the replay began, so the differ can line the two tap logs up.
 *
 * RCON actions inside the row ({@code act:<label>} marks between the row's marks) are not done here:
 * the mod appends {@code act:<label>} to {@code -Dvd.progress} at the head of the tick the mark
 * belongs to, and {@code null-abilitycheck --prepare-scene --progress} runs the row's commands for
 * that label; {@code end} closes the file. {@code -Dvd.phase=N} sets vanilla's position-reminder
 * counter ({@code LocalPlayer#positionReminder}) at the first replayed tick, to the value Null's
 * client had, since the 20-tick reminder depends on the ticks before the window.
 */
public final class Replay {
    private static final Path INPUT = path("vd.input");
    private static final String ROW = System.getProperty("vd.row");
    private static final Path CONTROL = path("vd.control");
    private static final Path MARKS = path("vd.marks");
    private static final Path PROGRESS = path("vd.progress");
    // Position-reminder counter at the first replayed tick (-1: leave it as the settle left it).
    private static final int PHASE = Integer.getInteger("vd.phase", -1);
    // Player ticks lived before the window may open (the player ticks only once vanilla marks the level loaded).
    private static final int MIN_AGE = Integer.getInteger("vd.minAge", 40);
    // Player ticks of stillness before the window opens (vanilla's own physics must have settled).
    private static final int QUIET_TICKS = Integer.getInteger("vd.quietTicks", 10);
    // Ticks after the window before the client quits (late server replies are still logged).
    private static final int TAIL_TICKS = Integer.getInteger("vd.tailTicks", 40);

    // The look of the last sample before the window (null: the log has none), applied at the first replayed tick.
    private static float[] preLook;

    /** One recorded tick. {@code events} are the discrete actions noted for it ({@code {"ev":..}} lines before it). */
    record Sample(long tick, boolean f, boolean b, boolean l, boolean r, boolean jump, boolean sneak, boolean sprint,
                  float yaw, float pitch, int slot, boolean attack, List<java.util.Map<String, String>> events) {}

    /** An RCON action inside the row: done by the scene process when the replay reaches {@code tick}. */
    record Act(long tick, String label) {}

    private static List<Sample> window;
    private static List<Act> acts = new ArrayList<>();
    private static int nextAct = 0;
    /** Ticks since the player died; vanilla's death screen enables its respawn button after 20. */
    private static int dead = 0;
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
        // The death screen's respawn button (DeathScreen), pressed 20 ticks after the screen opened.
        if (mc.currentScreen instanceof net.minecraft.client.gui.screen.DeathScreen) {
            if (++dead == 20) {
                mc.player.requestRespawn();
                mc.setScreen(null);
                Driver.log("respawn requested at client tick " + thisTick);
            }
        } else {
            dead = 0;
        }
        if (window == null) {
            if (CONTROL != null && !Files.exists(CONTROL)) return;
            window = load();
            Driver.log("replay: " + window.size() + " ticks loaded from " + INPUT + (ROW == null ? "" : " row " + ROW));
        }
        if (!started) {
            // Wait until the player is loaded (it ticks only then) and rests on the ground (vertical velocity stays at gravity's -0.0784 on the ground, so only x and z count).
            boolean still = mc.player.age >= MIN_AGE && mc.player.isOnGround()
                    && mc.player.getVelocity().x * mc.player.getVelocity().x + mc.player.getVelocity().z * mc.player.getVelocity().z < 1e-9;
            quiet = still ? quiet + 1 : 0;
            if (quiet < QUIET_TICKS) return;
            started = true;
            if (PROGRESS != null) {
                try {
                    Files.writeString(PROGRESS, "");
                } catch (IOException e) {
                    throw new RuntimeException(e);
                }
            }
            if (preLook != null) {
                // Null's client already had this look when the window opened (VD3): same look, same last-sent look.
                mc.player.setYaw(preLook[0]);
                mc.player.setPitch(preLook[1]);
                ((null_vd.mixin.ClientPlayerAccessor) mc.player).nullVdSetLastYaw(preLook[0]);
                ((null_vd.mixin.ClientPlayerAccessor) mc.player).nullVdSetLastPitch(preLook[1]);
            }
            if (PHASE >= 0) {
                ((null_vd.mixin.ClientPlayerAccessor) mc.player).nullVdSetPositionReminder(PHASE);
            }
            write("{\"replay_start_tick\":" + thisTick + ",\"ticks\":" + window.size() + ",\"first_input_tick\":"
                    + (window.isEmpty() ? -1 : window.get(0).tick()) + "}\n");
            Driver.log("replay start at client tick " + thisTick + ", " + window.size() + " ticks, player at "
                    + mc.player.getX() + " " + mc.player.getY() + " " + mc.player.getZ());
        }
        if (next < window.size()) {
            fireActs(window.get(next).tick(), false);
            apply(mc, window.get(next++));
            return;
        }
        if (next == window.size()) {
            fireActs(Long.MAX_VALUE, true);
            progress("end");
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

    /** Reports the acts whose mark is at or before this sample's tick (all that are left at the end). */
    // Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
    private static void fireActs(long tick, boolean all) {
        while (nextAct < acts.size() && (all || acts.get(nextAct).tick() <= tick)) {
            progress(acts.get(nextAct++).label());
        }
    }

    // Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
    private static void progress(String line) {
        if (PROGRESS == null) return;
        try {
            Files.writeString(PROGRESS, line + "\n", java.nio.file.StandardOpenOption.CREATE, java.nio.file.StandardOpenOption.APPEND);
        } catch (IOException e) {
            throw new RuntimeException(e);
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
        // The attack key is held while Null mines; the mouse must be grabbed, else vanilla's
        // continueAttack treats the key as up (Minecraft#continueAttack asks MouseHandler#isMouseGrabbed).
        // A window that never had focus (the GL stub) does not grab it by itself; this sets the flag only.
        if (mc.currentScreen == null) {
            ((null_vd.mixin.MouseAccessor) mc.mouse).nullVdSetCursorLocked(true);
        }
        o.attackKey.setPressed(s.attack());
        for (var ev : s.events()) {
            event(mc, ev);
        }
        // No key looks around; the mouse does, and the recorded look is the result of it.
        mc.player.setYaw(s.yaw());
        mc.player.setPitch(s.pitch());
        if (mc.player.getInventory().getSelectedSlot() != s.slot()) {
            mc.player.getInventory().setSelectedSlot(s.slot());
        }
    }

    /**
     * One discrete action of the log. Key events add one click to the key mapping and leave the key
     * up (a click between two ticks); {@code handleKeybinds} then consumes it. The screen events
     * go through the open screen's own path.
     */
    // Spec: specs/plans/VANILLA_DIFF_PLAN.md VD2
    private static void event(MinecraftClient mc, java.util.Map<String, String> ev) {
        var o = mc.options;
        String name = ev.get("ev").replace("\"", "");
        switch (name) {
            // use_item (Null sends UseItem whatever the crosshair shows) has no key of its own: the use key is the nearest, and the diff shows the difference.
            case "use_click", "use_item" -> click(o.useKey);
            case "attack_click" -> click(o.attackKey);
            case "swap_offhand" -> click(o.swapHandsKey);
            case "open_inventory" -> click(o.inventoryKey);
            case "click" -> {
                if (!(mc.currentScreen instanceof net.minecraft.client.gui.screen.ingame.HandledScreen<?> screen)) {
                    Driver.log("click event with no container screen open (replay differs from Null here)");
                    return;
                }
                // AbstractContainerScreen#slotClicked ends in MultiPlayerGameMode#handleInventoryMouseClick.
                mc.interactionManager.clickSlot(screen.getScreenHandler().syncId, Integer.parseInt(ev.get("slot")),
                        Integer.parseInt(ev.get("button")),
                        net.minecraft.screen.slot.SlotActionType.values()[Integer.parseInt(ev.get("type"))], mc.player);
            }
            case "close" -> {
                if (mc.currentScreen instanceof net.minecraft.client.gui.screen.ingame.HandledScreen<?> screen) {
                    screen.close();
                } else {
                    Driver.log("close event with no container screen open (replay differs from Null here)");
                }
            }
            default -> Driver.log("unknown event " + name);
        }
    }

    /** One press between two ticks, as a key event adds it ({@code KeyMapping#click}). */
    // Spec: specs/plans/VANILLA_DIFF_PLAN.md VD2
    private static void click(net.minecraft.client.option.KeyBinding key) {
        var acc = (null_vd.mixin.KeyBindingAccessor) key;
        acc.nullVdSetTimesPressed(acc.nullVdGetTimesPressed() + 1);
    }

    private static void release(MinecraftClient mc) {
        var o = mc.options;
        for (var k : new net.minecraft.client.option.KeyBinding[] {o.forwardKey, o.backKey, o.leftKey, o.rightKey, o.jumpKey, o.sneakKey, o.sprintKey, o.attackKey, o.useKey}) {
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
        boolean inRow = false;
        List<Sample> all = new ArrayList<>();
        List<java.util.Map<String, String>> pending = new ArrayList<>();
        for (String line : lines) {
            Matcher m = NUM.matcher(line);
            java.util.Map<String, String> kv = new java.util.HashMap<>();
            while (m.find()) kv.put(m.group(1), m.group(2));
            if (kv.containsKey("mark")) {
                String label = kv.get("mark").replace("\"", "");
                long tick = Long.parseLong(kv.get("tick"));
                if (ROW != null && label.equals("start:" + ROW)) {
                    from = tick;
                    inRow = true;
                } else if (ROW != null && label.equals("end:" + ROW)) {
                    to = tick;
                    inRow = false;
                } else if ((ROW == null || inRow) && label.startsWith("act:")) {
                    acts.add(new Act(tick, label));
                }
                continue;
            }
            if (!kv.containsKey("tick")) continue;
            if (kv.containsKey("ev")) {
                pending.add(kv);
                continue;
            }
            all.add(new Sample(Long.parseLong(kv.get("tick")), bool(kv, "f"), bool(kv, "b"), bool(kv, "l"), bool(kv, "r"),
                    bool(kv, "jump"), bool(kv, "sneak"), bool(kv, "sprint"),
                    Float.parseFloat(kv.get("yaw")), Float.parseFloat(kv.get("pitch")), Integer.parseInt(kv.getOrDefault("slot", "0")), bool(kv, "attack"), pending));
            pending = new ArrayList<>();
        }
        if (ROW != null && (from == Long.MIN_VALUE || to == Long.MAX_VALUE)) {
            throw new RuntimeException("row " + ROW + " has no start and end mark in " + INPUT);
        }
        List<Sample> out = new ArrayList<>();
        preLook = null;
        for (Sample s : all) {
            if (s.tick() < from) preLook = new float[] {s.yaw(), s.pitch()};
            else if (s.tick() < to) out.add(s);
        }
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
