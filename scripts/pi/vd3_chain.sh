#!/bin/bash
# Runs on the Pi, detached: the T2 rerun, one pass, each leg once. Spec: specs/plans/VD3_BRIEF.md T2
# Status: vd-logs/rerun/chain.status (one line per leg, then done).
H=/mnt/pi-data/null-harness; L=$H/vd-logs/rerun; mkdir -p "$L"; cd $H/vd
leg() { n=$1; shift; ./vd3_leg.sh "$1" "$2" "$L/$n" "$3" "${4:-}" > "$L/$n.out" 2>&1; echo "$n exit $?" >> "$L/chain.status"; }
leg t1_vanilla vanilla A1,A2,A3,A4,A22 null_vd3v
leg t1_grim grim A1,A2,A3,A4,A22 null_vd3g
leg t2_grim grim A4S,A2S null_vd3t2g "A4S:place_anvil_faces_clockwise_of_the_player,A4S:place_banner,A4S:place_bed_both_parts,A4S:place_bell_on_floor,A4S:place_brewing_stand,A4S:place_cake,A4S:place_campfire,A4S:place_cauldron,A4S:place_chain_on_floor_is_axis_y,A4S:place_chest_faces_the_player,A4S:place_composter,A4S:place_daylight_detector,A4S:place_enchanting_table,A4S:place_flower_pot,A4S:place_glass_pane,A4S:place_grindstone_on_floor,A4S:place_hopper,A4S:place_iron_bars,A4S:place_lightning_rod,A4S:place_poppy_on_grass,A4S:place_pressure_plate,A4S:place_rail,A4S:place_redstone_wire_on_stone,A4S:place_repeater_faces_back_to_the_player,A4S:place_skull_rotation_from_yaw,A4S:place_slab_on_floor_is_bottom,A4S:place_snow_layer,A4S:place_stonecutter_faces_the_player,A4S:place_torch_on_floor,A4S:place_wall,A2S:break_brewing_stand,A2S:break_pressure_plate"
echo done >> "$L/chain.status"
