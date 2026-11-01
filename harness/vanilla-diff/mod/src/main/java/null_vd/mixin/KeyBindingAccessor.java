package null_vd.mixin;

import net.minecraft.client.option.KeyBinding;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

// Spec: specs/plans/VANILLA_DIFF_PLAN.md VD2
// The click counter of a key mapping (Mojang: KeyMapping#clickCount, Yarn: timesPressed). A key
// event adds one; Minecraft#handleKeybinds takes them with consumeClick. The replay adds a click
// the way a key event between two ticks does, without a real key.
@Mixin(KeyBinding.class)
public interface KeyBindingAccessor {
    @Accessor("timesPressed")
    int nullVdGetTimesPressed();

    @Accessor("timesPressed")
    void nullVdSetTimesPressed(int times);
}
