#!/bin/bash
# Runs on the Pi, detached: the first live trial of the VD3 T3 to T6 scenes, vanilla server only, each leg once. Spec: specs/plans/VD3_BRIEF.md T3
# Status: vd-logs/trial/chain.status (one line per leg, then done). Row lists: the burst scene notes (specs/plans/BURST_2026-11-10.md).
H=/mnt/pi-data/null-harness; L=$H/vd-logs/trial; mkdir -p "$L"; cd $H/vd
leg() { n=$1; shift; ./vd3_leg.sh "$1" "$2" "$L/$n" "$3" "${4:-}" > "$L/$n.out" 2>&1; echo "$n exit $?" >> "$L/chain.status"; }
leg t4_vanilla vanilla A5,A6,A7,A8,A18,A20 null_itm "A5:eat_takes_32_ticks,A5:full_food_bread_is_refused,A6:blocks_from_the_front,A6:movement_slowed_while_raised,A7:critical_hit,A8:damage_without_an_entity_names_nobody,A8:melee_by_named_player,A18:mob_moved_by_rcon_is_reached_in_three_steps,A18:server_move_of_a_bot_shows_in_the_other_bots_view_in_three_steps,A20:broken_elytra_jump_not_refused"
leg t3_vanilla vanilla A10,A10E,A11 null_vt3 "A10:open_single_chest,A10:server_change_while_open,A10:ender_chest_is_the_players,A10E:chest_minecart_open_and_read,A10E:chest_minecart_killed_while_open,A10E:chest_boat_open_and_read,A11:planks_from_log_2x2,A11:table_shift_click_crafts_all,A11:repair_damaged_pickaxes"
leg t5_vanilla vanilla A14,A16,A19,A21 null_vt5 "A16:movement_speed_modifier_halves_the_walk,A16:jump_strength_base_0.84_jumps_higher,A16:block_break_speed_x3_breaks_in_a_third_of_the_ticks,A19:pushed_by_mob,A19:team_never_no_push,A21:goto_across_beds_goes_around_them,A21:goto_inside_the_world_border_margin_stays_outside_it,A14:oak_felled_with_an_axe,A19:pushed_by_bot"
leg t6_vanilla vanilla A12,A23,AT null_vt6 "A12:kill_and_respawn_twice_in_a_row,A12:immediate_respawn_has_no_delay,A12:death_while_crouching_and_eating,A12:death_in_the_nether_respawns_in_the_overworld,AT:stall_400_ms_runs_vanillas_count_and_drops_the_rest,AT:stall_250_ms_then_look_and_click_in_one_update_places_at_the_new_look,A23:fall_into_unsent_chunk"
echo done >> "$L/chain.status"
