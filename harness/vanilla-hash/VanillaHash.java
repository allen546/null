import java.util.List;
import java.util.Optional;

import com.google.common.hash.HashCode;
import net.minecraft.SharedConstants;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.HolderSet;
import net.minecraft.core.component.DataComponentType;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.component.TypedDataComponent;
import net.minecraft.core.registries.Registries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.resources.RegistryOps;
import net.minecraft.server.Bootstrap;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.tags.BannerPatternTags;
import net.minecraft.tags.BlockTags;
import net.minecraft.tags.DamageTypeTags;
import net.minecraft.tags.ItemTags;
import net.minecraft.util.HashOps;
import net.minecraft.world.damagesource.DamageTypes;
import net.minecraft.world.item.component.BlocksAttacks;
import net.minecraft.world.item.component.DamageResistant;
import net.minecraft.world.item.component.Tool;
import net.minecraft.world.item.enchantment.Repairable;

// Prints vanilla 1.21.11 item component hashes, as the client's decoratedHashOpsGenerator makes them.
public class VanillaHash {
    static RegistryOps<HashCode> ops;

    public static void main(String[] a) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        HolderLookup.Provider lookup = VanillaRegistries.createLookup();
        ops = lookup.createSerializationContext(HashOps.CRC32C_INSTANCE);
        var damage = lookup.lookupOrThrow(Registries.DAMAGE_TYPE);
        var blocks = lookup.lookupOrThrow(Registries.BLOCK);
        var items = lookup.lookupOrThrow(Registries.ITEM);

        print("damage_resistant_is_fire", DataComponents.DAMAGE_RESISTANT, new DamageResistant(DamageTypeTags.IS_FIRE));
        print("provides_banner_patterns_flower", DataComponents.PROVIDES_BANNER_PATTERNS, BannerPatternTags.PATTERN_ITEM_FLOWER);
        print("blocks_attacks_slow_shield", DataComponents.BLOCKS_ATTACKS, new BlocksAttacks(1.0F, 1.0F,
            List.of(new BlocksAttacks.DamageReduction(90.0F, Optional.empty(), 0.0F, 1.0F)),
            new BlocksAttacks.ItemDamageFunction(3.0F, 1.0F, 1.0F), Optional.of(DamageTypeTags.BYPASSES_SHIELD),
            Optional.of(SoundEvents.SHIELD_BLOCK), Optional.of(SoundEvents.SHIELD_BREAK)));
        print("blocks_attacks_reduction_tag", DataComponents.BLOCKS_ATTACKS, new BlocksAttacks(0.25F, 1.0F,
            List.of(new BlocksAttacks.DamageReduction(90.0F, Optional.of(HolderSet.emptyNamed(ops.owner(Registries.DAMAGE_TYPE).orElseThrow(), DamageTypeTags.IS_FIRE)), 0.0F, 1.0F)),
            BlocksAttacks.ItemDamageFunction.DEFAULT, Optional.empty(), Optional.empty(), Optional.empty()));
        print("blocks_attacks_reduction_direct", DataComponents.BLOCKS_ATTACKS, new BlocksAttacks(0.25F, 1.0F,
            List.of(new BlocksAttacks.DamageReduction(90.0F, Optional.of(HolderSet.direct(damage.getOrThrow(DamageTypes.IN_FIRE))), 0.0F, 1.0F)),
            BlocksAttacks.ItemDamageFunction.DEFAULT, Optional.empty(), Optional.empty(), Optional.empty()));
        print("tool_rule_tag", DataComponents.TOOL, new Tool(
            List.of(Tool.Rule.minesAndDrops(HolderSet.emptyNamed(ops.owner(Registries.BLOCK).orElseThrow(), BlockTags.MINEABLE_WITH_PICKAXE), 6.0F)), 1.0F, 1, true));
        print("repairable_tag", DataComponents.REPAIRABLE, new Repairable(HolderSet.emptyNamed(ops.owner(Registries.ITEM).orElseThrow(), ItemTags.WOODEN_TOOL_MATERIALS)));
    }

    static <T> void print(String name, DataComponentType<T> type, T value) {
        HashCode h = new TypedDataComponent<>(type, value).encodeValue(ops).getOrThrow();
        System.out.println(name + " " + Integer.toUnsignedString(h.asInt()));
    }
}
