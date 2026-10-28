package null_vd;

import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.screen.multiplayer.ConnectScreen;
import net.minecraft.client.gui.screen.TitleScreen;
import net.minecraft.client.network.ServerAddress;
import net.minecraft.client.network.ServerInfo;

/**
 * Minimal VD0 driver: joins a server, then holds the forward key for N client ticks through
 * vanilla's key mapping (so KeyboardInput builds the movement input), then quits.
 * The replay driver (VD1) replaces this.
 */
public final class Driver {
    private static final String SERVER = System.getProperty("vd.server", "127.0.0.1:25569");
    private static final int WALK_TICKS = Integer.getInteger("vd.walkTicks", 20);
    // Wait this many player ticks after the join so physics has settled (on the ground).
    private static final int SETTLE_TICKS = Integer.getInteger("vd.settleTicks", 40);

    private static long allTicks = 0;
    private static int connectCalls = 0;
    private static int settle = 0;
    private static int walked = -1; // -1 = not started
    private static int afterWalk = 0;
    private static long startNanos = 0;

    private Driver() {}

    // Spec: specs/plans/VANILLA_DIFF_PLAN.md VD0
    public static void onClientTick(MinecraftClient mc) {
        if (++allTicks % 100 == 1) {
            log("tick " + allTicks + " screen=" + (mc.currentScreen == null ? "none" : mc.currentScreen.getClass().getSimpleName())
                    + " overlay=" + (mc.getOverlay() == null ? "none" : mc.getOverlay().getClass().getSimpleName())
                    + " world=" + (mc.world != null) + " player=" + (mc.player != null));
        }
        if (mc.player == null || mc.world == null) {
            // Join through vanilla's own connect screen path once the title screen is up.
            if (mc.currentScreen instanceof TitleScreen && mc.getOverlay() == null && connectCalls == 0) {
                connectCalls++;
                log("connecting to " + SERVER);
                ConnectScreen.connect(mc.currentScreen, mc, ServerAddress.parse(SERVER),
                        new ServerInfo("tap", SERVER, ServerInfo.ServerType.OTHER), false, null);
            }
            return;
        }
        // The player ticks only once vanilla marks the client loaded (PlayerLoaded sent).
        if (walked < 0) {
            if (mc.player.age < SETTLE_TICKS) return;
            if (++settle < 1) return;
            walked = 0;
            startNanos = System.nanoTime();
            log("walk start, player age " + mc.player.age);
        }
        if (walked < WALK_TICKS) {
            mc.options.forwardKey.setPressed(true);
            walked++;
            return;
        }
        if (walked == WALK_TICKS) {
            mc.options.forwardKey.setPressed(false);
            walked++;
            double secs = (System.nanoTime() - startNanos) / 1e9;
            log(String.format("walk end after %d ticks in %.3f s (%.1f ticks/s) at %.3f %.3f %.3f",
                    WALK_TICKS, secs, WALK_TICKS / secs, mc.player.getX(), mc.player.getY(), mc.player.getZ()));
            return;
        }
        if (++afterWalk == 20) {
            log("stopping client");
            mc.scheduleStop();
        }
    }

    private static void log(String s) {
        System.out.println("[null-vd] " + s);
    }
}
