import java.io.IOException;
import java.net.Proxy;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Optional;
import java.util.UUID;

import com.mojang.authlib.GameProfile;
import com.mojang.authlib.yggdrasil.ServicesKeySet;
import com.mojang.serialization.Lifecycle;
import net.minecraft.commands.Commands;
import net.minecraft.core.LayeredRegistryAccess;
import net.minecraft.core.MappedRegistry;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.Registries;
import net.minecraft.gizmos.GizmoCollector;
import net.minecraft.gizmos.Gizmos;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.RegistryLayer;
import net.minecraft.server.Services;
import net.minecraft.server.WorldLoader;
import net.minecraft.server.WorldStem;
import net.minecraft.server.level.progress.LoggingLevelLoadListener;
import net.minecraft.server.notifications.EmptyNotificationService;
import net.minecraft.server.packs.repository.PackRepository;
import net.minecraft.server.packs.repository.ServerPacksSource;
import net.minecraft.server.permissions.LevelBasedPermissionSet;
import net.minecraft.server.permissions.PermissionSet;
import net.minecraft.server.players.NameAndId;
import net.minecraft.server.players.PlayerList;
import net.minecraft.server.players.ProfileResolver;
import net.minecraft.server.players.UserNameToIdResolver;
import net.minecraft.util.Util;
import net.minecraft.util.datafix.DataFixers;
import net.minecraft.util.debugchart.LocalSampleLogger;
import net.minecraft.util.debugchart.SampleLogger;
import net.minecraft.world.Difficulty;
import net.minecraft.world.flag.FeatureFlagSet;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.level.DataPackConfig;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.LevelSettings;
import net.minecraft.world.level.WorldDataConfiguration;
import net.minecraft.world.level.dimension.LevelStem;
import net.minecraft.world.level.gamerules.GameRules;
import net.minecraft.world.level.levelgen.WorldDimensions;
import net.minecraft.world.level.levelgen.WorldOptions;
import net.minecraft.world.level.levelgen.presets.WorldPresets;
import net.minecraft.world.level.storage.LevelStorageSource;
import net.minecraft.world.level.storage.PrimaryLevelData;
import net.minecraft.SystemReport;

// A vanilla 1.21.11 MinecraftServer that opens a flat world and never listens on a port and never
// runs its tick loop: the harness calls vanilla code on the calling thread, which is the server
// thread this object is built with. Built the way the vanilla game-test server is built (our own
// code, it only calls vanilla's public API). Spec: specs/B6-placement-breaking.md §4.10 (the vanilla oracle)
public class OracleServer extends MinecraftServer {
    private final LocalSampleLogger sampleLogger = new LocalSampleLogger(4);

    // Spec: specs/B6-placement-breaking.md §4.10 (the vanilla oracle)
    public static OracleServer create(Path worldRoot) throws Exception {
        LevelStorageSource source = LevelStorageSource.createDefault(worldRoot);
        LevelStorageSource.LevelStorageAccess access = source.validateAndCreateAccess("oracle");
        PackRepository packs = ServerPacksSource.createPackRepository(access);
        packs.reload();
        ArrayList<String> ids = new ArrayList<>(packs.getAvailableIds());
        ids.remove("vanilla");
        ids.addFirst("vanilla");
        // Vanilla features only: the same set the player gets on a default server.
        FeatureFlagSet features = FeatureFlags.DEFAULT_FLAGS;
        WorldDataConfiguration dataConfig = new WorldDataConfiguration(new DataPackConfig(ids, java.util.List.of()), features);
        LevelSettings settings = new LevelSettings("oracle", GameType.CREATIVE, false, Difficulty.PEACEFUL, true, new GameRules(features), dataConfig);
        WorldLoader.PackConfig packConfig = new WorldLoader.PackConfig(packs, dataConfig, false, true);
        WorldLoader.InitConfig init = new WorldLoader.InitConfig(packConfig, Commands.CommandSelection.DEDICATED, LevelBasedPermissionSet.OWNER);
        WorldStem stem = Util.blockUntilDone(executor -> WorldLoader.load(
            init,
            ctx -> {
                Registry<LevelStem> reg = new MappedRegistry<>(Registries.LEVEL_STEM, Lifecycle.stable()).freeze();
                WorldDimensions.Complete complete = ctx.datapackWorldgen().lookupOrThrow(Registries.WORLD_PRESET)
                    .getOrThrow(WorldPresets.FLAT).value().createWorldDimensions().bake(reg);
                return new WorldLoader.DataLoadOutput<>(
                    new PrimaryLevelData(settings, new WorldOptions(0L, false, false), complete.specialWorldProperty(), complete.lifecycle()),
                    complete.dimensionsRegistryAccess());
            },
            WorldStem::new, Util.backgroundExecutor(), executor)).get();
        return new OracleServer(Thread.currentThread(), access, packs, stem);
    }

    private static final Services NO_SERVICES = new Services(
        null, ServicesKeySet.EMPTY, null, new NoNames(), new NoProfiles());

    private OracleServer(Thread thread, LevelStorageSource.LevelStorageAccess access, PackRepository packs, WorldStem stem) {
        super(thread, access, packs, stem, Proxy.NO_PROXY, DataFixers.getDataFixer(), NO_SERVICES, LoggingLevelLoadListener.forDedicatedServer());
    }

    /** Opens the overworld. The spawn chunks load on this thread, with no tick loop. */
    public void open() {
        this.setPlayerList(new PlayerList(this, this.registries(), this.playerDataStorage, new EmptyNotificationService()) {});
        Gizmos.withCollector(GizmoCollector.NOOP);
        this.loadLevel();
    }

    @Override protected boolean initServer() throws IOException { open(); return true; }
    @Override public void waitUntilNextTick() { this.runAllTasks(); }
    @Override public LevelBasedPermissionSet operatorUserPermissions() { return LevelBasedPermissionSet.ALL; }
    @Override public PermissionSet getFunctionCompilationPermissions() { return LevelBasedPermissionSet.OWNER; }
    @Override public boolean shouldRconBroadcast() { return false; }
    @Override protected SampleLogger getTickTimeLogger() { return sampleLogger; }
    @Override public boolean isTickTimeLoggingEnabled() { return false; }
    @Override public SystemReport fillServerSystemReport(SystemReport r) { return r; }
    @Override public boolean isDedicatedServer() { return false; }
    @Override public int getRateLimitPacketsPerSecond() { return 0; }
    @Override public boolean useNativeTransport() { return false; }
    @Override public boolean isPublished() { return false; }
    @Override public boolean shouldInformAdmins() { return false; }
    @Override public boolean isSingleplayerOwner(NameAndId id) { return false; }
    @Override public boolean isHardcore() { return false; }
    @Override public int getMaxPlayers() { return 1; }

    static class NoProfiles implements ProfileResolver {
        public Optional<GameProfile> fetchByName(String s) { return Optional.empty(); }
        public Optional<GameProfile> fetchById(UUID u) { return Optional.empty(); }
    }

    static class NoNames implements UserNameToIdResolver {
        public void add(NameAndId n) {}
        public Optional<NameAndId> get(String s) { return Optional.of(NameAndId.createOffline(s)); }
        public Optional<NameAndId> get(UUID u) { return Optional.empty(); }
        public void resolveOfflineUsers(boolean b) {}
        public void save() {}
    }
}
