package null_vd.mixin;

import net.minecraft.client.MinecraftClient;
import null_vd.Driver;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

// Spec: specs/plans/VANILLA_DIFF_PLAN.md VD0
// Hook at the start of vanilla's client tick: key state set here is read by the same tick's
// input update, the way a key event between two ticks would be. Adds no behaviour of its own.
@Mixin(MinecraftClient.class)
public class ClientTickMixin {
    @Inject(method = "tick", at = @At("HEAD"))
    private void nullVdTick(CallbackInfo ci) {
        Driver.onClientTick((MinecraftClient) (Object) this);
    }
}
