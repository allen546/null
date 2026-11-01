package null_vd.mixin;

import net.minecraft.client.Mouse;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

// Spec: specs/plans/VANILLA_DIFF_PLAN.md VD2
// Sets vanilla's "mouse is grabbed" flag (Mojang: MouseHandler#mouseGrabbed, Yarn: cursorLocked).
// A headless window never gets focus, so the flag stays false and Minecraft#continueAttack would
// treat a held attack key as up. Client state only; it sends nothing.
@Mixin(Mouse.class)
public interface MouseAccessor {
    @Accessor("cursorLocked")
    void nullVdSetCursorLocked(boolean locked);
}
