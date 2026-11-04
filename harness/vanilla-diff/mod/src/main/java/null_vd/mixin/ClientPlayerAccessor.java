package null_vd.mixin;

import net.minecraft.client.network.ClientPlayerEntity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

// Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
// Sets vanilla's position-reminder counter (Mojang: LocalPlayer#positionReminder, Yarn:
// ticksSinceLastPositionPacketSent) so the 20-tick reminder at the window start has the phase Null's
// client had. It is client state only; it sends nothing.
@Mixin(ClientPlayerEntity.class)
public interface ClientPlayerAccessor {
    @Accessor("ticksSinceLastPositionPacketSent")
    void nullVdSetPositionReminder(int ticks);

    // Mojang: LocalPlayer#yRotLast / #xRotLast, the look last sent to the server. VD3: the look Null's client had
    // before the window, so the first replayed tick sends a look packet only if Null's did.
    @Accessor("lastYawClient")
    void nullVdSetLastYaw(float yaw);

    @Accessor("lastPitchClient")
    void nullVdSetLastPitch(float pitch);
}
