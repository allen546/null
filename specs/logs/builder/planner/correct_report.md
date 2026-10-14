# Correction loop report

Final status: **unresolved** after 2 iterations, 544 learned edges, 474.9 s.

## Iterations

| it | resumed | section | kind | block | experiments | r | how | edges | s |
|---|---|---|---|---|---|---|---|---|---|
| 1 | None | 802 | unreachable | [3, 6, 7] minecraft:observer[facing=up,powered=fal | 0 | access | access | 4 | 292.1 |
| 2 | 250 | 434 | irreversible | [1, 3, 4] minecraft:lava[level=0] | 284 | None | None | 0 | 182.8 |

## Learned edges by failure kind

### mismatch (99)
- [2, 12, 7] -> [2, 12, 6]: mismatch: placing minecraft:repeater[delay=2,facing=south,locked=false,powered=true] at [2, 12, 6] needs all of 2 blocks first (ddmin from 31, r=2)
- [3, 12, 7] -> [2, 12, 6]: mismatch: placing minecraft:repeater[delay=2,facing=south,locked=false,powered=true] at [2, 12, 6] needs all of 2 blocks first (ddmin from 31, r=2)
- [3, 12, 7] -> [3, 12, 6]: mismatch: placing minecraft:repeater[delay=1,facing=south,locked=false,powered=true] at [3, 12, 6] needs minecraft:redstone_wire[east=none,north=side,power=15,south=side,west=side] at [3, 12, 7] first (single-candidate experiment, r=2, closure=1)
- [11, 12, 7] -> [11, 12, 6]: mismatch: placing minecraft:repeater[delay=1,facing=south,locked=false,powered=true] at [11, 12, 6] needs minecraft:redstone_wire[east=side,north=side,power=15,south=side,west=none] at [11, 12, 7] first (single-candidate experiment, r=2, closure=1)
- [11, 12, 7] -> [12, 12, 6]: mismatch: placing minecraft:repeater[delay=2,facing=south,locked=false,powered=true] at [12, 12, 6] needs all of 2 blocks first (ddmin from 28, r=2)
- [12, 12, 7] -> [12, 12, 6]: mismatch: placing minecraft:repeater[delay=2,facing=south,locked=false,powered=true] at [12, 12, 6] needs all of 2 blocks first (ddmin from 28, r=2)
- [3, 12, 7] -> [2, 12, 7]: mismatch: placing minecraft:redstone_wire[east=side,north=side,power=14,south=side,west=none] at [2, 12, 7] needs minecraft:redstone_wire[east=none,north=side,power=15,south=side,west=side] at [3, 12, 7] first (single-candidate experiment, r=2, closure=1)
- [9, 13, 6] -> [9, 13, 5]: mismatch: placing minecraft:repeater[delay=1,facing=south,locked=false,powered=true] at [9, 13, 5] needs all of 8 blocks first (ddmin from 8, knockout-influencers)

### irreversible (31)
- [1, 6, 0] -> [1, 4, 0]: irreversible: placing minecraft:iron_trapdoor[facing=north,half=top,open=false,powered=false,waterlogged=false] at [1, 6, 0] after minecraft:snow_block at [1, 4, 0] fails; b first passes (swap experiment)
- [13, 6, 0] -> [13, 4, 0]: irreversible: placing minecraft:iron_trapdoor[facing=north,half=top,open=false,powered=false,waterlogged=false] at [13, 6, 0] after minecraft:snow_block at [13, 4, 0] fails; b first passes (swap experiment)
- [0, 6, 1] -> [0, 4, 1]: irreversible: placing minecraft:iron_trapdoor[facing=west,half=top,open=false,powered=false,waterlogged=false] at [0, 6, 1] after minecraft:snow_block at [0, 4, 1] fails; b first passes (swap experiment)
- [1, 8, 0] -> [1, 4, 0]: irreversible: placing minecraft:dropper[facing=down,triggered=false] at [1, 8, 0] after minecraft:snow_block at [1, 4, 0] fails; b first passes (swap experiment)
- [7, 9, 1] -> [7, 6, 1]: irreversible: placing minecraft:iron_trapdoor[facing=north,half=bottom,open=false,powered=false,waterlogged=false] at [7, 9, 1] after minecraft:snow_block at [7, 6, 1] fails; b first passes (swap experiment)
- [1, 9, 7] -> [1, 6, 7]: irreversible: placing minecraft:iron_trapdoor[facing=west,half=bottom,open=false,powered=false,waterlogged=false] at [1, 9, 7] after minecraft:snow_block at [1, 6, 7] fails; b first passes (swap experiment)
- [2, 11, 1] -> [1, 11, 1]: irreversible: placing minecraft:iron_trapdoor[facing=south,half=top,open=false,powered=false,waterlogged=false] at [2, 11, 1] after minecraft:observer[facing=east,powered=false] at [1, 11, 1] fails; replaying the plan with b moved before x passes
- [3, 11, 1] -> [3, 9, 1]: irreversible: placing minecraft:dropper[facing=south,triggered=false] at [3, 11, 1] after minecraft:piston[extended=false,facing=down] at [3, 9, 1] fails; b first passes (swap experiment)

### ? (410)
- [0, 6, 13] -> [0, 4, 13]: structural image of learned edge [1, 6, 0] -> [1, 4, 0] (same block ids in the surrounding box under a horizontal isometry; unverified hint)
- [1, 6, 14] -> [1, 4, 14]: structural image of learned edge [1, 6, 0] -> [1, 4, 0] (same block ids in the surrounding box under a horizontal isometry; unverified hint)
- [13, 6, 14] -> [13, 4, 14]: structural image of learned edge [1, 6, 0] -> [1, 4, 0] (same block ids in the surrounding box under a horizontal isometry; unverified hint)
- [14, 6, 1] -> [14, 4, 1]: structural image of learned edge [1, 6, 0] -> [1, 4, 0] (same block ids in the surrounding box under a horizontal isometry; unverified hint)
- [14, 6, 13] -> [14, 4, 13]: structural image of learned edge [1, 6, 0] -> [1, 4, 0] (same block ids in the surrounding box under a horizontal isometry; unverified hint)
- [0, 8, 1] -> [0, 4, 1]: structural image of learned edge [1, 8, 0] -> [1, 4, 0] (same block ids in the surrounding box under a horizontal isometry; unverified hint)
- [0, 8, 13] -> [0, 4, 13]: structural image of learned edge [1, 8, 0] -> [1, 4, 0] (same block ids in the surrounding box under a horizontal isometry; unverified hint)
- [1, 8, 14] -> [1, 4, 14]: structural image of learned edge [1, 8, 0] -> [1, 4, 0] (same block ids in the surrounding box under a horizontal isometry; unverified hint)

### unreachable (4)
- [3, 6, 7] -> [1, 2, 5]: unreachable: a player can only place minecraft:observer[facing=up,powered=false] at [3, 6, 7] while minecraft:piston[extended=false,facing=up] at [1, 2, 5] is not there yet
- [3, 6, 7] -> [1, 3, 5]: unreachable: a player can only place minecraft:observer[facing=up,powered=false] at [3, 6, 7] while minecraft:cobblestone at [1, 3, 5] is not there yet
- [3, 6, 7] -> [1, 5, 6]: unreachable: a player can only place minecraft:observer[facing=up,powered=false] at [3, 6, 7] while minecraft:piston[extended=false,facing=east] at [1, 5, 6] is not there yet
- [3, 7, 7] -> [3, 6, 7]: unreachable: a player can only place minecraft:observer[facing=up,powered=false] at [3, 6, 7] by clicking minecraft:observer[facing=west,powered=false] at [3, 7, 7], placed first

## Radius distribution (Chebyshev distance between c and b)

{1: 270, 2: 68, 3: 21, 4: 18, 5: 4, 6: 3, 7: 1, None: 159}

## Set radius at which each fix was first found

{2: 29, 4: 10, None: 417, 'access': 4, 'knockout': 65, 'replay': 1, 'swap': 18}

## Every learned edge (from -> to, Chebyshev radius, set radius, how)

- [2, 12, 7] -> [2, 12, 6]: radius 1, set radius None, None
- [3, 12, 7] -> [2, 12, 6]: radius 1, set radius None, None
- [3, 12, 7] -> [3, 12, 6]: radius 1, set radius None, None
- [11, 12, 7] -> [11, 12, 6]: radius 1, set radius None, None
- [11, 12, 7] -> [12, 12, 6]: radius 1, set radius None, None
- [12, 12, 7] -> [12, 12, 6]: radius 1, set radius None, None
- [3, 12, 7] -> [2, 12, 7]: radius 1, set radius None, None
- [9, 13, 6] -> [9, 13, 5]: radius 1, set radius knockout, ddmin
- [9, 13, 7] -> [9, 13, 5]: radius 2, set radius knockout, ddmin
- [9, 13, 8] -> [9, 13, 5]: radius 3, set radius knockout, ddmin
- [8, 13, 9] -> [9, 13, 5]: radius 4, set radius knockout, ddmin
- [9, 13, 9] -> [9, 13, 5]: radius 4, set radius knockout, ddmin
- [8, 13, 10] -> [9, 13, 5]: radius 5, set radius knockout, ddmin
- [8, 13, 11] -> [9, 13, 5]: radius 6, set radius knockout, ddmin
- [8, 13, 12] -> [9, 13, 5]: radius 7, set radius knockout, ddmin
- [9, 13, 6] -> [8, 13, 6]: radius 1, set radius knockout, ddmin
- [9, 13, 7] -> [8, 13, 6]: radius 1, set radius knockout, ddmin
- [9, 13, 8] -> [8, 13, 6]: radius 2, set radius knockout, ddmin
- [8, 13, 9] -> [8, 13, 6]: radius 3, set radius knockout, ddmin
- [9, 13, 9] -> [8, 13, 6]: radius 3, set radius knockout, ddmin
- [8, 13, 10] -> [8, 13, 6]: radius 4, set radius knockout, ddmin
- [8, 13, 11] -> [8, 13, 6]: radius 5, set radius knockout, ddmin
- [8, 13, 12] -> [8, 13, 6]: radius 6, set radius knockout, ddmin
- [9, 13, 7] -> [9, 13, 6]: radius 1, set radius knockout, ddmin
- [9, 13, 8] -> [9, 13, 6]: radius 2, set radius knockout, ddmin
- [8, 13, 9] -> [9, 13, 6]: radius 3, set radius knockout, ddmin
- [9, 13, 9] -> [9, 13, 6]: radius 3, set radius knockout, ddmin
- [8, 13, 10] -> [9, 13, 6]: radius 4, set radius knockout, ddmin
- [8, 13, 11] -> [9, 13, 6]: radius 5, set radius knockout, ddmin
- [8, 13, 12] -> [9, 13, 6]: radius 6, set radius knockout, ddmin
- [9, 13, 8] -> [9, 13, 7]: radius 1, set radius knockout, ddmin
- [8, 13, 9] -> [9, 13, 7]: radius 2, set radius knockout, ddmin
- [9, 13, 9] -> [9, 13, 7]: radius 2, set radius knockout, ddmin
- [8, 13, 10] -> [9, 13, 7]: radius 3, set radius knockout, ddmin
- [8, 13, 11] -> [9, 13, 7]: radius 4, set radius knockout, ddmin
- [8, 13, 12] -> [9, 13, 7]: radius 5, set radius knockout, ddmin
- [8, 13, 9] -> [9, 13, 8]: radius 1, set radius knockout, ddmin
- [9, 13, 9] -> [9, 13, 8]: radius 1, set radius knockout, ddmin
- [8, 13, 10] -> [9, 13, 8]: radius 2, set radius knockout, ddmin
- [8, 13, 11] -> [9, 13, 8]: radius 3, set radius knockout, ddmin
- [8, 13, 12] -> [9, 13, 8]: radius 4, set radius knockout, ddmin
- [8, 13, 10] -> [8, 13, 9]: radius 1, set radius knockout, ddmin
- [8, 13, 11] -> [8, 13, 9]: radius 2, set radius knockout, ddmin
- [8, 13, 12] -> [8, 13, 9]: radius 3, set radius knockout, ddmin
- [8, 13, 10] -> [9, 13, 9]: radius 1, set radius knockout, ddmin
- [8, 13, 11] -> [9, 13, 9]: radius 2, set radius knockout, ddmin
- [8, 13, 12] -> [9, 13, 9]: radius 3, set radius knockout, ddmin
- [8, 13, 9] -> [9, 13, 9]: radius 1, set radius knockout, ddmin
- [5, 13, 11] -> [4, 13, 10]: radius 1, set radius 4, ddmin
- [4, 13, 12] -> [4, 13, 10]: radius 2, set radius 4, ddmin
- [5, 13, 12] -> [4, 13, 10]: radius 2, set radius 4, ddmin
- [4, 13, 13] -> [4, 13, 10]: radius 3, set radius 4, ddmin
- [5, 13, 13] -> [4, 13, 10]: radius 3, set radius 4, ddmin
- [8, 13, 11] -> [8, 13, 10]: radius 1, set radius knockout, ddmin
- [8, 13, 12] -> [8, 13, 10]: radius 2, set radius knockout, ddmin
- [9, 13, 11] -> [10, 13, 10]: radius 1, set radius 4, ddmin
- [9, 13, 12] -> [10, 13, 10]: radius 2, set radius 4, ddmin
- [10, 13, 12] -> [10, 13, 10]: radius 2, set radius 4, ddmin
- [9, 13, 13] -> [10, 13, 10]: radius 3, set radius 4, ddmin
- [10, 13, 13] -> [10, 13, 10]: radius 3, set radius 4, ddmin
- [4, 13, 12] -> [5, 13, 11]: radius 1, set radius knockout, ddmin
- [4, 13, 13] -> [5, 13, 11]: radius 2, set radius knockout, ddmin
- [5, 13, 13] -> [5, 13, 11]: radius 2, set radius knockout, ddmin
- [8, 13, 12] -> [8, 13, 11]: radius 1, set radius knockout, single
- [10, 13, 12] -> [9, 13, 11]: radius 1, set radius knockout, ddmin
- [9, 13, 13] -> [9, 13, 11]: radius 2, set radius knockout, ddmin
- [10, 13, 13] -> [9, 13, 11]: radius 2, set radius knockout, ddmin
- [4, 13, 13] -> [4, 13, 12]: radius 1, set radius knockout, ddmin
- [5, 13, 13] -> [4, 13, 12]: radius 1, set radius knockout, ddmin
- [4, 13, 13] -> [5, 13, 12]: radius 1, set radius knockout, ddmin
- [5, 13, 13] -> [5, 13, 12]: radius 1, set radius knockout, ddmin
- [4, 13, 12] -> [5, 13, 12]: radius 1, set radius knockout, ddmin
- [5, 13, 11] -> [5, 13, 12]: radius 1, set radius knockout, ddmin
- [10, 13, 12] -> [9, 13, 12]: radius 1, set radius knockout, ddmin
- [9, 13, 13] -> [9, 13, 12]: radius 1, set radius knockout, ddmin
- [10, 13, 13] -> [9, 13, 12]: radius 1, set radius knockout, ddmin
- [9, 13, 11] -> [9, 13, 12]: radius 1, set radius knockout, ddmin
- [9, 13, 13] -> [10, 13, 12]: radius 1, set radius knockout, ddmin
- [10, 13, 13] -> [10, 13, 12]: radius 1, set radius knockout, ddmin
- [5, 13, 13] -> [4, 13, 13]: radius 1, set radius knockout, single
- [2, 12, 5] -> [2, 12, 3]: radius 2, set radius knockout, single
- [12, 12, 5] -> [12, 12, 3]: radius 2, set radius knockout, single
- [1, 6, 0] -> [1, 4, 0]: radius 2, set radius swap, swap
- [13, 6, 0] -> [13, 4, 0]: radius 2, set radius swap, swap
- [0, 6, 1] -> [0, 4, 1]: radius 2, set radius swap, swap
- [0, 6, 13] -> [0, 4, 13]: radius 2, set radius None, analog
- [1, 6, 14] -> [1, 4, 14]: radius 2, set radius None, analog
- [13, 6, 14] -> [13, 4, 14]: radius 2, set radius None, analog
- [14, 6, 1] -> [14, 4, 1]: radius 2, set radius None, analog
- [14, 6, 13] -> [14, 4, 13]: radius 2, set radius None, analog
- [1, 8, 0] -> [1, 4, 0]: radius 4, set radius swap, swap
- [0, 8, 1] -> [0, 4, 1]: radius 4, set radius None, analog
- [0, 8, 13] -> [0, 4, 13]: radius 4, set radius None, analog
- [1, 8, 14] -> [1, 4, 14]: radius 4, set radius None, analog
- [13, 8, 0] -> [13, 4, 0]: radius 4, set radius None, analog
- [13, 8, 14] -> [13, 4, 14]: radius 4, set radius None, analog
- [14, 8, 1] -> [14, 4, 1]: radius 4, set radius None, analog
- [14, 8, 13] -> [14, 4, 13]: radius 4, set radius None, analog
- [7, 9, 1] -> [7, 6, 1]: radius 3, set radius swap, swap
- [7, 9, 13] -> [7, 6, 13]: radius 3, set radius None, analog
- [1, 9, 7] -> [1, 6, 7]: radius 3, set radius swap, swap
- [13, 9, 7] -> [13, 6, 7]: radius 3, set radius None, analog
- [2, 11, 1] -> [1, 11, 1]: radius 1, set radius replay, replay
- [2, 11, 13] -> [1, 11, 13]: radius 1, set radius None, analog
- [12, 11, 1] -> [13, 11, 1]: radius 1, set radius None, analog
- [12, 11, 13] -> [13, 11, 13]: radius 1, set radius None, analog
- [3, 11, 1] -> [3, 9, 1]: radius 2, set radius swap, swap
- [11, 11, 1] -> [11, 9, 1]: radius 2, set radius None, analog
- [6, 11, 1] -> [6, 9, 1]: radius 2, set radius swap, swap
- [8, 11, 1] -> [8, 9, 1]: radius 2, set radius None, analog
- [1, 11, 3] -> [1, 9, 3]: radius 2, set radius swap, swap
- [1, 11, 11] -> [1, 9, 11]: radius 2, set radius None, analog
- [13, 11, 3] -> [13, 9, 3]: radius 2, set radius None, analog
- [13, 11, 11] -> [13, 9, 11]: radius 2, set radius None, analog
- [1, 11, 6] -> [1, 9, 6]: radius 2, set radius swap, swap
- [1, 11, 8] -> [1, 9, 8]: radius 2, set radius None, analog
- [13, 11, 6] -> [13, 9, 6]: radius 2, set radius None, analog
- [13, 11, 8] -> [13, 9, 8]: radius 2, set radius None, analog
- [3, 11, 13] -> [3, 9, 13]: radius 2, set radius swap, swap
- [11, 11, 13] -> [11, 9, 13]: radius 2, set radius None, analog
- [5, 11, 13] -> [5, 11, 12]: radius 1, set radius swap, swap
- [6, 11, 13] -> [6, 9, 13]: radius 2, set radius swap, swap
- [8, 11, 13] -> [8, 9, 13]: radius 2, set radius swap, swap
- [9, 11, 13] -> [9, 10, 13]: radius 1, set radius swap, swap
- [3, 12, 3] -> [3, 12, 4]: radius 1, set radius swap, swap
- [11, 12, 3] -> [11, 12, 4]: radius 1, set radius None, analog
- [3, 12, 11] -> [3, 11, 11]: radius 1, set radius swap, swap
- [11, 12, 11] -> [11, 11, 11]: radius 1, set radius swap, swap
- [1, 3, 2] -> [2, 3, 1]: radius 1, set radius 2, ddmin
- [3, 3, 2] -> [2, 3, 1]: radius 1, set radius 2, ddmin
- [2, 3, 3] -> [2, 3, 1]: radius 2, set radius 2, ddmin
- [1, 3, 12] -> [2, 3, 13]: radius 1, set radius None, analog
- [2, 3, 1] -> [1, 3, 2]: radius 1, set radius None, analog
- [12, 3, 1] -> [13, 3, 2]: radius 1, set radius None, analog
- [12, 3, 13] -> [13, 3, 12]: radius 1, set radius None, analog
- [2, 3, 3] -> [1, 3, 2]: radius 1, set radius None, analog
- [2, 3, 11] -> [1, 3, 12]: radius 1, set radius None, analog
- [3, 3, 12] -> [2, 3, 13]: radius 1, set radius None, analog
- [11, 3, 12] -> [12, 3, 13]: radius 1, set radius None, analog
- [12, 3, 3] -> [13, 3, 2]: radius 1, set radius None, analog
- [12, 3, 11] -> [13, 3, 12]: radius 1, set radius None, analog
- [2, 3, 11] -> [2, 3, 13]: radius 2, set radius None, analog
- [3, 3, 2] -> [1, 3, 2]: radius 2, set radius None, analog
- [3, 3, 12] -> [1, 3, 12]: radius 2, set radius None, analog
- [11, 3, 12] -> [13, 3, 12]: radius 2, set radius None, analog
- [12, 3, 3] -> [12, 3, 1]: radius 2, set radius None, analog
- [12, 3, 11] -> [12, 3, 13]: radius 2, set radius None, analog
- [8, 3, 1] -> [6, 3, 1]: radius 2, set radius 2, ddmin
- [7, 3, 2] -> [6, 3, 1]: radius 1, set radius 2, ddmin
- [6, 3, 1] -> [8, 3, 1]: radius 2, set radius None, analog
- [6, 3, 13] -> [8, 3, 13]: radius 2, set radius None, analog
- [7, 3, 2] -> [8, 3, 1]: radius 1, set radius None, analog
- [7, 3, 12] -> [6, 3, 13]: radius 1, set radius None, analog
- [7, 3, 12] -> [8, 3, 13]: radius 1, set radius None, analog
- [7, 3, 2] -> [7, 3, 1]: radius 1, set radius 2, ddmin
- [6, 3, 1] -> [7, 3, 1]: radius 1, set radius 2, ddmin
- [8, 3, 1] -> [7, 3, 1]: radius 1, set radius 2, ddmin
- [7, 3, 12] -> [7, 3, 13]: radius 1, set radius None, analog
- [6, 3, 13] -> [7, 3, 13]: radius 1, set radius None, analog
- [8, 3, 13] -> [7, 3, 13]: radius 1, set radius None, analog
- [3, 3, 2] -> [2, 3, 2]: radius 1, set radius 2, ddmin
- [2, 3, 3] -> [2, 3, 2]: radius 1, set radius 2, ddmin
- [2, 3, 1] -> [2, 3, 2]: radius 1, set radius 2, ddmin
- [1, 3, 2] -> [2, 3, 2]: radius 1, set radius 2, ddmin
- [2, 3, 11] -> [2, 3, 12]: radius 1, set radius None, analog
- [3, 3, 12] -> [2, 3, 12]: radius 1, set radius None, analog
- [11, 3, 12] -> [12, 3, 12]: radius 1, set radius None, analog
- [12, 3, 3] -> [12, 3, 2]: radius 1, set radius None, analog
- [12, 3, 11] -> [12, 3, 12]: radius 1, set radius None, analog
- [1, 3, 12] -> [2, 3, 12]: radius 1, set radius None, analog
- [2, 3, 13] -> [2, 3, 12]: radius 1, set radius None, analog
- [12, 3, 1] -> [12, 3, 2]: radius 1, set radius None, analog
- [12, 3, 13] -> [12, 3, 12]: radius 1, set radius None, analog
- [13, 3, 2] -> [12, 3, 2]: radius 1, set radius None, analog
- [13, 3, 12] -> [12, 3, 12]: radius 1, set radius None, analog
- [2, 3, 4] -> [3, 3, 2]: radius 2, set radius 2, single
- [2, 3, 10] -> [3, 3, 12]: radius 2, set radius None, analog
- [4, 3, 2] -> [2, 3, 3]: radius 2, set radius None, analog
- [4, 3, 12] -> [2, 3, 11]: radius 2, set radius None, analog
- [10, 3, 12] -> [12, 3, 11]: radius 2, set radius None, analog
- [12, 3, 10] -> [11, 3, 12]: radius 2, set radius None, analog
- [12, 3, 4] -> [11, 3, 2]: radius 2, set radius 2, single
- [2, 3, 4] -> [2, 3, 3]: radius 1, set radius 2, single
- [2, 3, 10] -> [2, 3, 11]: radius 1, set radius None, analog
- [4, 3, 2] -> [3, 3, 2]: radius 1, set radius None, analog
- [4, 3, 12] -> [3, 3, 12]: radius 1, set radius None, analog
- [10, 3, 12] -> [11, 3, 12]: radius 1, set radius None, analog
- [12, 3, 4] -> [12, 3, 3]: radius 1, set radius None, analog
- [12, 3, 10] -> [12, 3, 11]: radius 1, set radius None, analog
- [2, 3, 4] -> [1, 3, 4]: radius 1, set radius 2, single
- [2, 3, 10] -> [1, 3, 10]: radius 1, set radius None, analog
- [4, 3, 2] -> [4, 3, 1]: radius 1, set radius None, analog
- [4, 3, 12] -> [4, 3, 13]: radius 1, set radius None, analog
- [10, 3, 12] -> [10, 3, 13]: radius 1, set radius None, analog
- [12, 3, 4] -> [13, 3, 4]: radius 1, set radius None, analog
- [12, 3, 10] -> [13, 3, 10]: radius 1, set radius None, analog
- [2, 3, 4] -> [2, 3, 5]: radius 1, set radius 2, single
- [2, 3, 10] -> [2, 3, 9]: radius 1, set radius None, analog
- [12, 3, 4] -> [12, 3, 5]: radius 1, set radius None, analog
- [12, 3, 10] -> [12, 3, 9]: radius 1, set radius None, analog
- [2, 4, 5] -> [2, 4, 7]: radius 2, set radius 2, ddmin
- [2, 4, 9] -> [2, 4, 7]: radius 2, set radius 2, ddmin
- [12, 4, 5] -> [12, 4, 7]: radius 2, set radius None, analog
- [12, 4, 9] -> [12, 4, 7]: radius 2, set radius None, analog
- [4, 8, 0] -> [4, 8, 1]: radius 1, set radius 2, single
- [0, 8, 4] -> [1, 8, 4]: radius 1, set radius None, analog
- [0, 8, 10] -> [1, 8, 10]: radius 1, set radius None, analog
- [4, 8, 14] -> [4, 8, 13]: radius 1, set radius None, analog
- [10, 8, 0] -> [10, 8, 1]: radius 1, set radius None, analog
- [10, 8, 14] -> [10, 8, 13]: radius 1, set radius None, analog
- [14, 8, 4] -> [13, 8, 4]: radius 1, set radius None, analog
- [14, 8, 10] -> [13, 8, 10]: radius 1, set radius None, analog
- [5, 7, 4] -> [5, 8, 4]: radius 1, set radius 2, ddmin
- [5, 8, 5] -> [5, 8, 4]: radius 1, set radius 2, ddmin
- [4, 7, 5] -> [4, 8, 5]: radius 1, set radius None, analog
- [4, 7, 9] -> [4, 8, 9]: radius 1, set radius None, analog
- [5, 7, 10] -> [5, 8, 10]: radius 1, set radius None, analog
- [9, 7, 4] -> [9, 8, 4]: radius 1, set radius None, analog
- [9, 7, 10] -> [9, 8, 10]: radius 1, set radius None, analog
- [10, 7, 5] -> [10, 8, 5]: radius 1, set radius None, analog
- [10, 7, 9] -> [10, 8, 9]: radius 1, set radius None, analog
- [5, 8, 5] -> [4, 8, 5]: radius 1, set radius None, analog
- [5, 8, 9] -> [4, 8, 9]: radius 1, set radius None, analog
- [5, 8, 9] -> [5, 8, 10]: radius 1, set radius None, analog
- [9, 8, 5] -> [9, 8, 4]: radius 1, set radius None, analog
- [9, 8, 5] -> [10, 8, 5]: radius 1, set radius None, analog
- [9, 8, 9] -> [9, 8, 10]: radius 1, set radius None, analog
- [9, 8, 9] -> [10, 8, 9]: radius 1, set radius None, analog
- [5, 8, 5] -> [6, 8, 5]: radius 1, set radius 2, single
- [9, 8, 5] -> [8, 8, 5]: radius 1, set radius None, analog
- [5, 8, 5] -> [5, 8, 6]: radius 1, set radius 2, single
- [5, 8, 9] -> [5, 8, 8]: radius 1, set radius None, analog
- [9, 8, 5] -> [9, 8, 6]: radius 1, set radius 2, single
- [9, 8, 9] -> [9, 8, 8]: radius 1, set radius None, analog
- [6, 8, 4] -> [6, 8, 3]: radius 1, set radius 2, single
- [8, 8, 4] -> [8, 8, 3]: radius 1, set radius None, analog
- [6, 8, 4] -> [5, 8, 4]: radius 1, set radius 2, single
- [8, 8, 4] -> [9, 8, 4]: radius 1, set radius None, analog
- [5, 8, 7] -> [5, 8, 8]: radius 1, set radius 2, single
- [5, 8, 7] -> [5, 8, 6]: radius 1, set radius None, analog
- [13, 0, 5] -> [12, 0, 5]: radius None, set radius None, neighbour
- [2, 0, 7] -> [1, 0, 7]: radius None, set radius None, neighbour
- [12, 0, 7] -> [11, 0, 7]: radius None, set radius None, neighbour
- [2, 0, 9] -> [1, 0, 9]: radius None, set radius None, neighbour
- [4, 0, 13] -> [3, 0, 13]: radius None, set radius None, neighbour
- [10, 0, 13] -> [9, 0, 13]: radius None, set radius None, neighbour
- [4, 1, 2] -> [3, 1, 2]: radius None, set radius None, neighbour
- [6, 1, 1] -> [6, 1, 2]: radius None, set radius None, neighbour
- [10, 1, 2] -> [11, 1, 2]: radius None, set radius None, neighbour
- [13, 1, 3] -> [13, 1, 2]: radius None, set radius None, neighbour
- [4, 1, 12] -> [3, 1, 12]: radius None, set radius None, neighbour
- [10, 1, 12] -> [11, 1, 12]: radius None, set radius None, neighbour
- [13, 1, 11] -> [13, 1, 12]: radius None, set radius None, neighbour
- [8, 1, 13] -> [8, 1, 14]: radius None, set radius None, neighbour
- [4, 2, 2] -> [5, 2, 2]: radius None, set radius None, neighbour
- [8, 2, 2] -> [9, 2, 2]: radius None, set radius None, neighbour
- [2, 2, 2] -> [2, 2, 3]: radius None, set radius None, neighbour
- [2, 2, 10] -> [2, 2, 11]: radius None, set radius None, neighbour
- [11, 2, 12] -> [11, 2, 11]: radius None, set radius None, neighbour
- [4, 2, 12] -> [5, 2, 12]: radius None, set radius None, neighbour
- [8, 2, 12] -> [9, 2, 12]: radius None, set radius None, neighbour
- [2, 2, 2] -> [2, 2, 1]: radius None, set radius None, neighbour
- [2, 2, 12] -> [2, 2, 13]: radius None, set radius None, neighbour
- [4, 2, 2] -> [4, 2, 1]: radius None, set radius None, neighbour
- [10, 2, 2] -> [10, 2, 1]: radius None, set radius None, neighbour
- [1, 2, 3] -> [1, 2, 4]: radius None, set radius None, neighbour
- [13, 2, 3] -> [13, 2, 4]: radius None, set radius None, neighbour
- [1, 2, 9] -> [1, 2, 10]: radius None, set radius None, neighbour
- [13, 2, 9] -> [13, 2, 10]: radius None, set radius None, neighbour
- [10, 4, 0] -> [10, 4, 1]: radius None, set radius None, neighbour
- [9, 4, 3] -> [9, 4, 2]: radius None, set radius None, neighbour
- [6, 5, 1] -> [6, 5, 0]: radius None, set radius None, neighbour
- [0, 5, 5] -> [0, 5, 6]: radius None, set radius None, neighbour
- [14, 5, 5] -> [14, 5, 6]: radius None, set radius None, neighbour
- [13, 5, 7] -> [12, 5, 7]: radius None, set radius None, neighbour
- [2, 4, 9] -> [2, 5, 9]: radius None, set radius None, neighbour
- [12, 4, 9] -> [12, 5, 9]: radius None, set radius None, neighbour
- [7, 5, 13] -> [7, 5, 12]: radius None, set radius None, neighbour
- [6, 5, 13] -> [6, 5, 14]: radius None, set radius None, neighbour
- [10, 6, 1] -> [11, 6, 1]: radius None, set radius None, neighbour
- [6, 6, 1] -> [6, 6, 2]: radius None, set radius None, neighbour
- [8, 6, 1] -> [8, 6, 2]: radius None, set radius None, neighbour
- [3, 5, 5] -> [3, 6, 5]: radius None, set radius None, neighbour
- [11, 5, 5] -> [11, 6, 5]: radius None, set radius None, neighbour
- [13, 6, 4] -> [13, 6, 5]: radius None, set radius None, neighbour
- [2, 6, 5] -> [2, 6, 6]: radius None, set radius None, neighbour
- [12, 6, 5] -> [12, 6, 6]: radius None, set radius None, neighbour
- [2, 6, 9] -> [2, 6, 8]: radius None, set radius None, neighbour
- [12, 6, 9] -> [12, 6, 8]: radius None, set radius None, neighbour
- [3, 5, 9] -> [3, 6, 9]: radius None, set radius None, neighbour
- [11, 5, 9] -> [11, 6, 9]: radius None, set radius None, neighbour
- [13, 6, 8] -> [13, 6, 9]: radius None, set radius None, neighbour
- [1, 6, 10] -> [1, 6, 11]: radius None, set radius None, neighbour
- [13, 6, 10] -> [13, 6, 11]: radius None, set radius None, neighbour
- [6, 6, 11] -> [6, 6, 12]: radius None, set radius None, neighbour
- [8, 6, 11] -> [8, 6, 12]: radius None, set radius None, neighbour
- [5, 6, 12] -> [5, 6, 13]: radius None, set radius None, neighbour
- [9, 6, 12] -> [9, 6, 13]: radius None, set radius None, neighbour
- [10, 6, 13] -> [11, 6, 13]: radius None, set radius None, neighbour
- [2, 2, 4] -> [2, 1, 4]: radius None, set radius None, neighbour
- [2, 2, 10] -> [2, 1, 10]: radius None, set radius None, neighbour
- [12, 2, 4] -> [12, 1, 4]: radius None, set radius None, neighbour
- [12, 2, 10] -> [12, 1, 10]: radius None, set radius None, neighbour
- [3, 7, 1] -> [2, 7, 1]: radius None, set radius None, neighbour
- [1, 7, 3] -> [1, 7, 2]: radius None, set radius None, neighbour
- [13, 7, 3] -> [13, 7, 2]: radius None, set radius None, neighbour
- [9, 7, 4] -> [9, 7, 5]: radius None, set radius None, neighbour
- [3, 7, 13] -> [2, 7, 13]: radius None, set radius None, neighbour
- [0, 8, 1] -> [0, 8, 2]: radius None, set radius None, neighbour
- [14, 8, 1] -> [14, 8, 2]: radius None, set radius None, neighbour
- [1, 8, 14] -> [2, 8, 14]: radius None, set radius None, neighbour
- [2, 9, 2] -> [3, 9, 2]: radius None, set radius None, neighbour
- [12, 9, 2] -> [11, 9, 2]: radius None, set radius None, neighbour
- [2, 9, 2] -> [2, 9, 3]: radius None, set radius None, neighbour
- [12, 9, 2] -> [12, 9, 3]: radius None, set radius None, neighbour
- [2, 9, 12] -> [2, 9, 11]: radius None, set radius None, neighbour
- [12, 9, 12] -> [12, 9, 11]: radius None, set radius None, neighbour
- [2, 9, 12] -> [3, 9, 12]: radius None, set radius None, neighbour
- [12, 9, 12] -> [11, 9, 12]: radius None, set radius None, neighbour
- [2, 9, 2] -> [2, 9, 1]: radius None, set radius None, neighbour
- [12, 9, 2] -> [12, 9, 1]: radius None, set radius None, neighbour
- [2, 9, 2] -> [1, 9, 2]: radius None, set radius None, neighbour
- [13, 9, 3] -> [13, 9, 2]: radius None, set radius None, neighbour
- [1, 9, 11] -> [1, 9, 12]: radius None, set radius None, neighbour
- [13, 9, 11] -> [13, 9, 12]: radius None, set radius None, neighbour
- [2, 9, 12] -> [2, 9, 13]: radius None, set radius None, neighbour
- [12, 9, 12] -> [12, 9, 13]: radius None, set radius None, neighbour
- [11, 10, 14] -> [10, 10, 14]: radius None, set radius None, neighbour
- [3, 11, 2] -> [4, 11, 2]: radius None, set radius None, neighbour
- [11, 11, 2] -> [10, 11, 2]: radius None, set radius None, neighbour
- [2, 11, 3] -> [2, 11, 4]: radius None, set radius None, neighbour
- [4, 11, 3] -> [4, 11, 4]: radius None, set radius None, neighbour
- [10, 11, 3] -> [10, 11, 4]: radius None, set radius None, neighbour
- [12, 11, 3] -> [12, 11, 4]: radius None, set radius None, neighbour
- [2, 11, 9] -> [2, 11, 10]: radius None, set radius None, neighbour
- [4, 11, 9] -> [4, 11, 10]: radius None, set radius None, neighbour
- [10, 11, 9] -> [10, 11, 10]: radius None, set radius None, neighbour
- [12, 11, 9] -> [12, 11, 10]: radius None, set radius None, neighbour
- [3, 11, 12] -> [4, 11, 12]: radius None, set radius None, neighbour
- [9, 11, 12] -> [10, 11, 12]: radius None, set radius None, neighbour
- [13, 11, 0] -> [12, 11, 0]: radius None, set radius None, neighbour
- [3, 12, 6] -> [4, 12, 6]: radius None, set radius None, neighbour
- [6, 12, 7] -> [6, 12, 8]: radius None, set radius None, neighbour
- [9, 12, 9] -> [8, 12, 9]: radius None, set radius None, neighbour
- [9, 13, 5] -> [9, 13, 4]: radius None, set radius None, neighbour
- [10, 13, 10] -> [9, 13, 10]: radius None, set radius None, neighbour
- [4, 13, 10] -> [4, 13, 11]: radius None, set radius None, neighbour
- [10, 13, 10] -> [10, 13, 11]: radius None, set radius None, neighbour
- [7, 13, 8] -> [7, 13, 9]: radius None, set radius None, neighbour
- [3, 2, 12] -> [3, 2, 11]: radius None, set radius None, neighbour
- [12, 2, 10] -> [12, 2, 11]: radius None, set radius None, neighbour
- [7, 5, 1] -> [7, 5, 0]: radius None, set radius None, neighbour
- [1, 5, 7] -> [0, 5, 7]: radius None, set radius None, neighbour
- [8, 5, 1] -> [8, 5, 0]: radius None, set radius None, neighbour
- [5, 3, 0] -> [6, 3, 0]: radius None, set radius None, neighbour
- [9, 3, 0] -> [10, 3, 0]: radius None, set radius None, neighbour
- [6, 3, 3] -> [5, 3, 3]: radius None, set radius None, neighbour
- [6, 3, 3] -> [7, 3, 3]: radius None, set radius None, neighbour
- [8, 3, 3] -> [9, 3, 3]: radius None, set radius None, neighbour
- [0, 3, 3] -> [0, 3, 4]: radius None, set radius None, neighbour
- [6, 3, 3] -> [6, 3, 4]: radius None, set radius None, neighbour
- [8, 3, 3] -> [8, 3, 4]: radius None, set radius None, neighbour
- [14, 3, 3] -> [14, 3, 4]: radius None, set radius None, neighbour
- [0, 3, 5] -> [0, 3, 6]: radius None, set radius None, neighbour
- [3, 3, 6] -> [4, 3, 6]: radius None, set radius None, neighbour
- [11, 3, 6] -> [10, 3, 6]: radius None, set radius None, neighbour
- [14, 3, 5] -> [14, 3, 6]: radius None, set radius None, neighbour
- [3, 3, 8] -> [4, 3, 8]: radius None, set radius None, neighbour
- [3, 3, 8] -> [3, 3, 9]: radius None, set radius None, neighbour
- [11, 3, 8] -> [11, 3, 9]: radius None, set radius None, neighbour
- [0, 3, 9] -> [0, 3, 10]: radius None, set radius None, neighbour
- [14, 3, 9] -> [14, 3, 10]: radius None, set radius None, neighbour
- [6, 3, 11] -> [5, 3, 11]: radius None, set radius None, neighbour
- [6, 3, 11] -> [7, 3, 11]: radius None, set radius None, neighbour
- [8, 3, 11] -> [9, 3, 11]: radius None, set radius None, neighbour
- [3, 3, 14] -> [4, 3, 14]: radius None, set radius None, neighbour
- [5, 3, 14] -> [6, 3, 14]: radius None, set radius None, neighbour
- [9, 3, 14] -> [10, 3, 14]: radius None, set radius None, neighbour
- [3, 3, 0] -> [2, 3, 0]: radius None, set radius None, neighbour
- [11, 3, 0] -> [12, 3, 0]: radius None, set radius None, neighbour
- [0, 3, 3] -> [0, 3, 2]: radius None, set radius None, neighbour
- [14, 3, 3] -> [14, 3, 2]: radius None, set radius None, neighbour
- [3, 3, 6] -> [3, 3, 7]: radius None, set radius None, neighbour
- [11, 3, 6] -> [11, 3, 7]: radius None, set radius None, neighbour
- [0, 3, 11] -> [0, 3, 12]: radius None, set radius None, neighbour
- [14, 3, 11] -> [14, 3, 12]: radius None, set radius None, neighbour
- [3, 3, 14] -> [2, 3, 14]: radius None, set radius None, neighbour
- [11, 3, 14] -> [12, 3, 14]: radius None, set radius None, neighbour
- [7, 8, 2] -> [7, 8, 3]: radius None, set radius None, neighbour
- [0, 8, 5] -> [0, 8, 6]: radius None, set radius None, neighbour
- [14, 8, 5] -> [14, 8, 6]: radius None, set radius None, neighbour
- [4, 8, 7] -> [5, 8, 7]: radius None, set radius None, neighbour
- [7, 8, 10] -> [7, 8, 11]: radius None, set radius None, neighbour
- [0, 8, 11] -> [0, 8, 12]: radius None, set radius None, neighbour
- [14, 8, 11] -> [14, 8, 12]: radius None, set radius None, neighbour
- [5, 8, 14] -> [6, 8, 14]: radius None, set radius None, neighbour
- [9, 8, 14] -> [8, 8, 14]: radius None, set radius None, neighbour
- [11, 8, 14] -> [12, 8, 14]: radius None, set radius None, neighbour
- [8, 13, 7] -> [8, 13, 8]: radius None, set radius None, neighbour
- [2, 3, 6] -> [2, 3, 5]: radius 1, set radius 2, single
- [2, 3, 8] -> [2, 3, 9]: radius 1, set radius None, analog
- [12, 3, 6] -> [12, 3, 5]: radius 1, set radius None, analog
- [12, 3, 8] -> [12, 3, 9]: radius 1, set radius None, analog
- [1, 0, 5] -> [2, 0, 5]: radius 1, set radius None, analog
- [13, 0, 9] -> [12, 0, 9]: radius 1, set radius None, analog
- [12, 0, 7] -> [13, 0, 7]: radius 1, set radius None, analog
- [2, 0, 7] -> [3, 0, 7]: radius 1, set radius None, analog
- [4, 0, 1] -> [3, 0, 1]: radius 1, set radius None, analog
- [4, 0, 1] -> [5, 0, 1]: radius 1, set radius None, analog
- [4, 0, 13] -> [5, 0, 13]: radius 1, set radius None, analog
- [10, 0, 1] -> [9, 0, 1]: radius 1, set radius None, analog
- [6, 1, 13] -> [6, 1, 12]: radius 1, set radius None, analog
- [8, 1, 1] -> [8, 1, 2]: radius 1, set radius None, analog
- [8, 1, 13] -> [8, 1, 12]: radius 1, set radius None, analog
- [6, 1, 1] -> [6, 1, 0]: radius 1, set radius None, analog
- [6, 1, 13] -> [6, 1, 14]: radius 1, set radius None, analog
- [8, 1, 1] -> [8, 1, 0]: radius 1, set radius None, analog
- [10, 2, 2] -> [9, 2, 2]: radius 1, set radius None, analog
- [10, 2, 12] -> [9, 2, 12]: radius 1, set radius None, analog
- [6, 2, 2] -> [5, 2, 2]: radius 1, set radius None, analog
- [6, 2, 12] -> [5, 2, 12]: radius 1, set radius None, analog
- [2, 2, 12] -> [2, 2, 11]: radius 1, set radius None, analog
- [2, 2, 4] -> [2, 2, 3]: radius 1, set radius None, analog
- [12, 2, 4] -> [12, 2, 3]: radius 1, set radius None, analog
- [11, 2, 2] -> [11, 2, 3]: radius 1, set radius None, analog
- [4, 2, 12] -> [4, 2, 13]: radius 1, set radius None, analog
- [10, 2, 12] -> [10, 2, 13]: radius 1, set radius None, analog
- [1, 2, 11] -> [1, 2, 10]: radius 1, set radius None, analog
- [13, 2, 11] -> [13, 2, 10]: radius 1, set radius None, analog
- [1, 2, 5] -> [1, 2, 4]: radius 1, set radius None, analog
- [13, 2, 5] -> [13, 2, 4]: radius 1, set radius None, analog
- [8, 5, 13] -> [8, 5, 14]: radius 1, set radius None, analog
- [0, 5, 9] -> [0, 5, 8]: radius 1, set radius None, analog
- [5, 5, 0] -> [6, 5, 0]: radius 1, set radius None, analog
- [5, 5, 14] -> [6, 5, 14]: radius 1, set radius None, analog
- [9, 5, 14] -> [8, 5, 14]: radius 1, set radius None, analog
- [14, 5, 9] -> [14, 5, 8]: radius 1, set radius None, analog
- [1, 5, 7] -> [2, 5, 7]: radius 1, set radius None, analog
- [2, 4, 5] -> [2, 5, 5]: radius 1, set radius None, analog
- [12, 4, 5] -> [12, 5, 5]: radius 1, set radius None, analog
- [7, 5, 1] -> [7, 5, 2]: radius 1, set radius None, analog
- [1, 6, 4] -> [1, 6, 3]: radius 1, set radius None, analog
- [4, 6, 1] -> [3, 6, 1]: radius 1, set radius None, analog
- [4, 6, 13] -> [3, 6, 13]: radius 1, set radius None, analog
- [13, 6, 4] -> [13, 6, 3]: radius 1, set radius None, analog
- [6, 6, 13] -> [6, 6, 12]: radius 1, set radius None, analog
- [8, 6, 13] -> [8, 6, 12]: radius 1, set radius None, analog
- [1, 6, 4] -> [1, 6, 5]: radius 1, set radius None, analog
- [1, 6, 10] -> [1, 6, 9]: radius 1, set radius None, analog
- [4, 6, 1] -> [5, 6, 1]: radius 1, set radius None, analog
- [4, 6, 13] -> [5, 6, 13]: radius 1, set radius None, analog
- [10, 6, 1] -> [9, 6, 1]: radius 1, set radius None, analog
- [10, 6, 13] -> [9, 6, 13]: radius 1, set radius None, analog
- [13, 6, 10] -> [13, 6, 9]: radius 1, set radius None, analog
- [1, 6, 6] -> [1, 6, 5]: radius 1, set radius None, analog
- [1, 6, 8] -> [1, 6, 9]: radius 1, set radius None, analog
- [13, 6, 6] -> [13, 6, 5]: radius 1, set radius None, analog
- [6, 6, 3] -> [6, 6, 2]: radius 1, set radius None, analog
- [8, 6, 3] -> [8, 6, 2]: radius 1, set radius None, analog
- [2, 6, 5] -> [1, 6, 5]: radius 1, set radius None, analog
- [2, 6, 9] -> [1, 6, 9]: radius 1, set radius None, analog
- [5, 6, 2] -> [5, 6, 1]: radius 1, set radius None, analog
- [9, 6, 2] -> [9, 6, 1]: radius 1, set radius None, analog
- [12, 6, 5] -> [13, 6, 5]: radius 1, set radius None, analog
- [12, 6, 9] -> [13, 6, 9]: radius 1, set radius None, analog
- [1, 7, 11] -> [1, 7, 12]: radius 1, set radius None, analog
- [11, 7, 1] -> [12, 7, 1]: radius 1, set radius None, analog
- [11, 7, 13] -> [12, 7, 13]: radius 1, set radius None, analog
- [13, 7, 11] -> [13, 7, 12]: radius 1, set radius None, analog
- [4, 7, 5] -> [5, 7, 5]: radius 1, set radius None, analog
- [4, 7, 9] -> [5, 7, 9]: radius 1, set radius None, analog
- [5, 7, 4] -> [5, 7, 5]: radius 1, set radius None, analog
- [5, 7, 10] -> [5, 7, 9]: radius 1, set radius None, analog
- [9, 7, 10] -> [9, 7, 9]: radius 1, set radius None, analog
- [10, 7, 5] -> [9, 7, 5]: radius 1, set radius None, analog
- [10, 7, 9] -> [9, 7, 9]: radius 1, set radius None, analog
- [0, 8, 13] -> [0, 8, 12]: radius 1, set radius None, analog
- [1, 8, 0] -> [2, 8, 0]: radius 1, set radius None, analog
- [13, 8, 0] -> [12, 8, 0]: radius 1, set radius None, analog
- [13, 8, 14] -> [12, 8, 14]: radius 1, set radius None, analog
- [14, 8, 13] -> [14, 8, 12]: radius 1, set radius None, analog
- [2, 9, 12] -> [1, 9, 12]: radius 1, set radius None, analog
- [12, 9, 2] -> [13, 9, 2]: radius 1, set radius None, analog
- [12, 9, 12] -> [13, 9, 12]: radius 1, set radius None, analog
- [1, 9, 3] -> [1, 9, 2]: radius 1, set radius None, analog
- [3, 9, 1] -> [2, 9, 1]: radius 1, set radius None, analog
- [3, 9, 13] -> [2, 9, 13]: radius 1, set radius None, analog
- [11, 9, 1] -> [12, 9, 1]: radius 1, set radius None, analog
- [11, 9, 13] -> [12, 9, 13]: radius 1, set radius None, analog
- [3, 10, 0] -> [4, 10, 0]: radius 1, set radius None, analog
- [3, 10, 14] -> [4, 10, 14]: radius 1, set radius None, analog
- [11, 10, 0] -> [10, 10, 0]: radius 1, set radius None, analog
- [12, 11, 11] -> [12, 11, 10]: radius 1, set radius None, analog
- [2, 11, 5] -> [2, 11, 4]: radius 1, set radius None, analog
- [12, 11, 5] -> [12, 11, 4]: radius 1, set radius None, analog
- [1, 11, 0] -> [2, 11, 0]: radius 1, set radius None, analog
- [1, 11, 14] -> [2, 11, 14]: radius 1, set radius None, analog
- [13, 11, 14] -> [12, 11, 14]: radius 1, set radius None, analog
- [3, 2, 2] -> [3, 2, 3]: radius 1, set radius None, analog
- [7, 5, 13] -> [7, 5, 14]: radius 1, set radius None, analog
- [13, 5, 7] -> [14, 5, 7]: radius 1, set radius None, analog
- [0, 3, 9] -> [0, 3, 8]: radius 1, set radius None, analog
- [9, 3, 14] -> [8, 3, 14]: radius 1, set radius None, analog
- [14, 3, 9] -> [14, 3, 8]: radius 1, set radius None, analog
- [8, 3, 3] -> [7, 3, 3]: radius 1, set radius None, analog
- [8, 3, 11] -> [7, 3, 11]: radius 1, set radius None, analog
- [0, 3, 11] -> [0, 3, 10]: radius 1, set radius None, analog
- [3, 3, 0] -> [4, 3, 0]: radius 1, set radius None, analog
- [11, 3, 14] -> [10, 3, 14]: radius 1, set radius None, analog
- [14, 3, 11] -> [14, 3, 10]: radius 1, set radius None, analog
- [6, 3, 11] -> [6, 3, 10]: radius 1, set radius None, analog
- [8, 3, 11] -> [8, 3, 10]: radius 1, set radius None, analog
- [11, 3, 8] -> [10, 3, 8]: radius 1, set radius None, analog
- [3, 3, 6] -> [3, 3, 5]: radius 1, set radius None, analog
- [11, 3, 6] -> [11, 3, 5]: radius 1, set radius None, analog
- [0, 3, 5] -> [0, 3, 4]: radius 1, set radius None, analog
- [5, 3, 0] -> [4, 3, 0]: radius 1, set radius None, analog
- [5, 3, 14] -> [4, 3, 14]: radius 1, set radius None, analog
- [14, 3, 5] -> [14, 3, 4]: radius 1, set radius None, analog
- [3, 3, 8] -> [3, 3, 7]: radius 1, set radius None, analog
- [11, 3, 8] -> [11, 3, 7]: radius 1, set radius None, analog
- [0, 8, 9] -> [0, 8, 8]: radius 1, set radius None, analog
- [5, 8, 0] -> [6, 8, 0]: radius 1, set radius None, analog
- [9, 8, 0] -> [8, 8, 0]: radius 1, set radius None, analog
- [14, 8, 9] -> [14, 8, 8]: radius 1, set radius None, analog
- [0, 8, 3] -> [0, 8, 2]: radius 1, set radius None, analog
- [3, 8, 0] -> [2, 8, 0]: radius 1, set radius None, analog
- [3, 8, 14] -> [2, 8, 14]: radius 1, set radius None, analog
- [11, 8, 0] -> [12, 8, 0]: radius 1, set radius None, analog
- [14, 8, 3] -> [14, 8, 2]: radius 1, set radius None, analog
- [3, 6, 7] -> [1, 2, 5]: radius 4, set radius access, access
- [3, 6, 7] -> [1, 3, 5]: radius 3, set radius access, access
- [3, 6, 7] -> [1, 5, 6]: radius 2, set radius access, access
- [3, 7, 7] -> [3, 6, 7]: radius 1, set radius access, access
- [3, 6, 7] -> [1, 2, 9]: radius 4, set radius None, analog
- [11, 6, 7] -> [13, 2, 5]: radius 4, set radius None, analog
- [11, 6, 7] -> [13, 2, 9]: radius 4, set radius None, analog
- [3, 6, 7] -> [1, 3, 9]: radius 3, set radius None, analog
- [11, 6, 7] -> [13, 3, 5]: radius 3, set radius None, analog
- [11, 6, 7] -> [13, 3, 9]: radius 3, set radius None, analog
- [3, 6, 7] -> [1, 5, 8]: radius 2, set radius None, analog
- [11, 6, 7] -> [13, 5, 6]: radius 2, set radius None, analog
- [11, 6, 7] -> [13, 5, 8]: radius 2, set radius None, analog
- [11, 7, 7] -> [11, 6, 7]: radius 1, set radius None, analog

## Candidates refused because the edge would close a cycle (0)


## Unresolved failure

```json
{
 "note": "no ordering fix up to radius 4",
 "failure": {
  "section": 434,
  "step": 0,
  "kind": "irreversible",
  "diff": [],
  "irreversible": [
   {
    "type": "flow",
    "from": [
     1,
     3,
     4
    ],
    "to": [
     1,
     2,
     4
    ],
    "tick": 1536132
   },
   {
    "type": "flow",
    "from": [
     1,
     2,
     4
    ],
    "to": [
     0,
     2,
     4
    ],
    "tick": 1536162
   },
   {
    "type": "flow",
    "from": [
     1,
     3,
     4
    ],
    "to": [
     1,
     3,
     5
    ],
    "tick": 1536162
   },
   {
    "type": "flow",
    "from": [
     0,
     2,
     4
    ],
    "to": [
     -1,
     2,
     4
    ],
    "tick": 1536192
   },
   {
    "type": "flow",
    "from": [
     1,
     3,
     5
    ],
    "to": [
     1,
     2,
     5
    ],
    "tick": 1536192
   },
   {
    "type": "flow",
    "from": [
     -1,
     2,
     4
    ],
    "to": [
     -1,
     1,
     4
    ],
    "tick": 1536222
   },
   {
    "type": "flow",
    "from": [
     1,
     2,
     5
    ],
    "to": [
     0,
     2,
     5
    ],
    "tick": 1536222
   },
   {
    "type": "flow",
    "from": [
     -1,
     1,
     4
    ],
    "to": [
     -1,
     0,
     4
    ],
    "tick": 1536252
   },
   {
    "type": "flow",
    "from": [
     0,
     2,
     5
    ],
    "to": [
     -1,
     2,
     5
    ],
    "tick": 1536252
   },
   {
    "type": "flow",
    "from": [
     -1,
     0,
     4
    ],
    "to": [
     -1,
     -1,
     4
    ],
    "tick": 1536282
   },
   {
    "type": "flow",
    "from": [
     -1,
     2,
     5
    ],
    "to": [
     -1,
     1,
     5
    ],
    "tick": 1536282
   },
   {
    "type": "flow",
    "from": [
     -1,
     -1,
     4
    ],
    "to": [
     -1,
     -2,
     4
    ],
    "tick": 1536312
   },
   {
    "type": "flow",
    "from": [
     -1,
     1,
     5
    ],
    "to": [
     -1,
     0,
     5
    ],
    "tick": 1536312
   },
   {
    "type": "flow",
    "from": [
     -1,
     -2,
     4
    ],
    "to": [
     -1,
     -3,
     4
    ],
    "tick": 1536342
   },
   {
    "type": "flow",
    "from": [
     -1,
     0,
     5
    ],
    "to": [
     -1,
     -1,
     5
    ],
    "tick": 1536342
   },
   {
    "type": "flow",
    "from": [
     -1,
     -3,
     4
    ],
    "to": [
     -1,
     -4,
     4
    ],
    "tick": 1536372
   },
   {
    "type": "flow",
    "from": [
     -1,
     -1,
     5
    ],
    "to": [
     -1,
     -2,
     5
    ],
    "tick": 1536372
   },
   {
    "type": "flow",
    "from": [
     -1,
     -4,
     4
    ],
    "to": [
     -1,
     -5,
     4
    ],
    "tick": 1536402
   },
   {
    "type": "flow",
    "from": [
     -1,
     -2,
     5
    ],
    "to": [
     -1,
     -3,
     5
    ],
    "tick": 1536402
   },
   {
    "type": "flow",
    "from": [
     -1,
     -5,
     4
    ],
    "to": [
     -1,
     -6,
     4
    ],
    "tick": 1536432
   },
   {
    "type": "flow",
    "from": [
     -1,
     -3,
     5
    ],
    "to": [
     -1,
     -4,
     5
    ],
    "tick": 1536432
   },
   {
    "type": "flow",
    "from": [
     -1,
     -6,
     4
    ],
    "to": [
     -1,
     -7,
     4
    ],
    "tick": 1536462
   },
   {
    "type": "flow",
    "from": [
     -1,
     -4,
     5
    ],
    "to": [
     -1,
     -5,
     5
    ],
    "tick": 1536462
   },
   {
    "type": "flow",
    "from": [
     -1,
     -7,
     4
    ],
    "to": [
     -1,
     -7,
     3
    ],
    "tick": 1536492
   },
   {
    "type": "flow",
    "from": [
     -1,
     -7,
     4
    ],
    "to": [
     -2,
     -7,
     4
    ],
    "tick": 1536492
   },
   {
    "type": "flow",
    "from": [
     -1,
     -7,
     4
    ],
    "to": [
     0,
     -7,
     4
    ],
    "tick": 1536492
   },
   {
    "type": "flow",
    "from": [
     -1,
     -5,
     5
    ],
    "to": [
     -1,
     -6,
     5
    ],
    "tick": 1536492
   },
   {
    "type": "flow",
    "from": [
     -2,
     -7,
     4
    ],
    "to": [
     -2,
     -8,
     4
    ],
    "tick": 1536522
   },
   {
    "type": "flow",
    "from": [
     0,
     -7,
     4
    ],
    "to": [
     0,
     -8,
     4
    ],
    "tick": 1536522
   },
   {
    "type": "flow",
    "from": [
     -1,
     -6,
     5
    ],
    "to": [
     -1,
     -7,
     5
    ],
    "tick": 1536522
   },
   {
    "type": "flow",
    "from": [
     -1,
     -7,
     5
    ],
    "to": [
     -1,
     -7,
     6
    ],
    "tick": 1536552
   },
   {
    "type": "flow",
    "from": [
     -1,
     -7,
     5
    ],
    "to": [
     -2,
     -7,
     5
    ],
    "tick": 1536552
   },
   {
    "type": "flow",
    "from": [
     -1,
     -7,
     5
    ],
    "to": [
     0,
     -7,
     5
    ],
    "tick": 1536552
   },
   {
    "type": "flow",
    "from": [
     -1,
     -7,
     6
    ],
    "to": [
     -1,
     -8,
     6
    ],
    "tick": 1536582
   },
   {
    "type": "flow",
    "from": [
     -2,
     -7,
     5
    ],
    "to": [
     -2,
     -8,
     5
    ],
    "tick": 1536582
   },
   {
    "type": "flow",
    "from": [
     0,
     -7,
     5
    ],
    "to": [
     0,
     -8,
     5
    ],
    "tick": 1536582
   }
  ]
 },
 "events_head": [
  {
   "type": "flow",
   "from": [
    1,
    3,
    4
   ],
   "to": [
    1,
    2,
    4
   ],
   "tick": 1536132
  },
  {
   "type": "flow",
   "from": [
    1,
    2,
    4
   ],
   "to": [
    0,
    2,
    4
   ],
   "tick": 1536162
  },
  {
   "type": "flow",
   "from": [
    1,
    3,
    4
   ],
   "to": [
    1,
    3,
    5
   ],
   "tick": 1536162
  },
  {
   "type": "flow",
   "from": [
    0,
    2,
    4
   ],
   "to": [
    -1,
    2,
    4
   ],
   "tick": 1536192
  },
  {
   "type": "flow",
   "from": [
    1,
    3,
    5
   ],
   "to": [
    1,
    2,
    5
   ],
   "tick": 1536192
  },
  {
   "type": "flow",
   "from": [
    -1,
    2,
    4
   ],
   "to": [
    -1,
    1,
    4
   ],
   "tick": 1536222
  },
  {
   "type": "flow",
   "from": [
    1,
    2,
    5
   ],
   "to": [
    0,
    2,
    5
   ],
   "tick": 1536222
  },
  {
   "type": "flow",
   "from": [
    -1,
    1,
    4
   ],
   "to": [
    -1,
    0,
    4
   ],
   "tick": 1536252
  },
  {
   "type": "flow",
   "from": [
    0,
    2,
    5
   ],
   "to": [
    -1,
    2,
    5
   ],
   "tick": 1536252
  },
  {
   "type": "flow",
   "from": [
    -1,
    0,
    4
   ],
   "to": [
    -1,
    -1,
    4
   ],
   "tick": 1536282
  },
  {
   "type": "flow",
   "from": [
    -1,
    2,
    5
   ],
   "to": [
    -1,
    1,
    5
   ],
   "tick": 1536282
  },
  {
   "type": "flow",
   "from": [
    -1,
    -1,
    4
   ],
   "to": [
    -1,
    -2,
    4
   ],
   "tick": 1536312
  },
  {
   "type": "flow",
   "from": [
    -1,
    1,
    5
   ],
   "to": [
    -1,
    0,
    5
   ],
   "tick": 1536312
  },
  {
   "type": "flow",
   "from": [
    -1,
    -2,
    4
   ],
   "to": [
    -1,
    -3,
    4
   ],
   "tick": 1536342
  },
  {
   "type": "flow",
   "from": [
    -1,
    0,
    5
   ],
   "to": [
    -1,
    -1,
    5
   ],
   "tick": 1536342
  },
  {
   "type": "flow",
   "from": [
    -1,
    -3,
    4
   ],
   "to": [
    -1,
    -4,
    4
   ],
   "tick": 1536372
  },
  {
   "type": "flow",
   "from": [
    -1,
    -1,
    5
   ],
   "to": [
    -1,
    -2,
    5
   ],
   "tick": 1536372
  },
  {
   "type": "flow",
   "from": [
    -1,
    -4,
    4
   ],
   "to": [
    -1,
    -5,
    4
   ],
   "tick": 1536402
  },
  {
   "type": "flow",
   "from": [
    -1,
    -2,
    5
   ],
   "to": [
    -1,
    -3,
    5
   ],
   "tick": 1536402
  },
  {
   "type": "flow",
   "from": [
    -1,
    -5,
    4
   ],
   "to": [
    -1,
    -6,
    4
   ],
   "tick": 1536432
  },
  {
   "type": "flow",
   "from": [
    -1,
    -3,
    5
   ],
   "to": [
    -1,
    -4,
    5
   ],
   "tick": 1536432
  },
  {
   "type": "flow",
   "from": [
    -1,
    -6,
    4
   ],
   "to": [
    -1,
    -7,
    4
   ],
   "tick": 1536462
  },
  {
   "type": "flow",
   "from": [
    -1,
    -4,
    5
   ],
   "to": [
    -1,
    -5,
    5
   ],
   "tick": 1536462
  },
  {
   "type": "flow",
   "from": [
    -1,
    -7,
    4
   ],
   "to": [
    -1,
    -7,
    3
   ],
   "tick": 1536492
  },
  {
   "type": "flow",
   "from": [
    -1,
    -7,
    4
   ],
   "to": [
    -2,
    -7,
    4
   ],
   "tick": 1536492
  },
  {
   "type": "flow",
   "from": [
    -1,
    -7,
    4
   ],
   "to": [
    0,
    -7,
    4
   ],
   "tick": 1536492
  },
  {
   "type": "flow",
   "from": [
    -1,
    -5,
    5
   ],
   "to": [
    -1,
    -6,
    5
   ],
   "tick": 1536492
  },
  {
   "type": "flow",
   "from": [
    -2,
    -7,
    4
   ],
   "to": [
    -2,
    -8,
    4
   ],
   "tick": 1536522
  },
  {
   "type": "flow",
   "from": [
    0,
    -7,
    4
   ],
   "to": [
    0,
    -8,
    4
   ],
   "tick": 1536522
  },
  {
   "type": "flow",
   "from": [
    -1,
    -6,
    5
   ],
   "to": [
    -1,
    -7,
    5
   ],
   "tick": 1536522
  },
  {
   "type": "flow",
   "from": [
    -1,
    -7,
    5
   ],
   "to": [
    -1,
    -7,
    6
   ],
   "tick": 1536552
  },
  {
   "type": "flow",
   "from": [
    -1,
    -7,
    5
   ],
   "to": [
    -2,
    -7,
    5
   ],
   "tick": 1536552
  },
  {
   "type": "flow",
   "from": [
    -1,
    -7,
    5
   ],
   "to": [
    0,
    -7,
    5
   ],
   "tick": 1536552
  },
  {
   "type": "flow",
   "from": [
    -1,
    -7,
    6
   ],
   "to": [
    -1,
    -8,
    6
   ],
   "tick": 1536582
  },
  {
   "type": "flow",
   "from": [
    -2,
    -7,
    5
   ],
   "to": [
    -2,
    -8,
    5
   ],
   "tick": 1536582
  },
  {
   "type": "flow",
   "from": [
    0,
    -7,
    5
   ],
   "to": [
    0,
    -8,
    5
   ],
   "tick": 1536582
  }
 ],
 "n_events": 36,
 "block": [
  1,
  3,
  4
 ],
 "state": "minecraft:lava[level=0]",
 "F": [
  [
   -2,
   -8,
   4
  ],
  [
   -2,
   -8,
   5
  ],
  [
   -2,
   -7,
   4
  ],
  [
   -2,
   -7,
   5
  ],
  [
   -1,
   -8,
   6
  ],
  [
   -1,
   -7,
   3
  ],
  [
   -1,
   -7,
   4
  ],
  [
   -1,
   -7,
   5
  ],
  [
   -1,
   -7,
   6
  ],
  [
   -1,
   -6,
   4
  ],
  [
   -1,
   -6,
   5
  ],
  [
   -1,
   -5,
   4
  ],
  [
   -1,
   -5,
   5
  ],
  [
   -1,
   -4,
   4
  ],
  [
   -1,
   -4,
   5
  ],
  [
   -1,
   -3,
   4
  ],
  [
   -1,
   -3,
   5
  ],
  [
   -1,
   -2,
   4
  ],
  [
   -1,
   -2,
   5
  ],
  [
   -1,
   -1,
   4
  ],
  [
   -1,
   -1,
   5
  ],
  [
   -1,
   0,
   4
  ],
  [
   -1,
   0,
   5
  ],
  [
   -1,
   1,
   4
  ],
  [
   -1,
   1,
   5
  ],
  [
   -1,
   2,
   4
  ],
  [
   -1,
   2,
   5
  ],
  [
   0,
   -8,
   4
  ],
  [
   0,
   -8,
   5
  ],
  [
   0,
   -7,
   4
  ],
  [
   0,
   -7,
   5
  ],
  [
   0,
   2,
   4
  ],
  [
   0,
   2,
   5
  ],
  [
   1,
   2,
   4
  ],
  [
   1,
   2,
   5
  ],
  [
   1,
   3,
   4
  ],
  [
   1,
   3,
   5
  ]
 ],
 "neighbourhood": [
  {
   "pos": [
    0,
    1,
    2
   ],
   "target": "minecraft:white_stained_glass",
   "placed": true
  },
  {
   "pos": [
    1,
    1,
    2
   ],
   "target": "minecraft:snow_block",
   "placed": true
  },
  {
   "pos": [
    2,
    1,
    2
   ],
   "target": "minecraft:redstone_wire[east=side,north=side,power=0,south=side,west=side]",
   "placed": true
  },
  {
   "pos": [
    3,
    1,
    2
   ],
   "target": "minecraft:snow_block",
   "placed": true
  },
  {
   "pos": [
    0,
    1,
    3
   ],
   "target": "minecraft:white_stained_glass",
   "placed": true
  },
  {
   "pos": [
    1,
    1,
    3
   ],
   "target": "minecraft:observer[facing=south,powered=false]",
   "placed": true
  },
  {
   "pos": [
    0,
    1,
    4
   ],
   "target": "minecraft:white_stained_glass",
   "placed": true
  },
  {
   "pos": [
    1,
    1,
    4
   ],
   "target": "minecraft:activator_rail[powered=false,shape=north_south,waterlogged=false]",
   "placed": true
  },
  {
   "pos": [
    2,
    1,
    4
   ],
   "target": "minecraft:iron_trapdoor[facing=north,half=top,open=false,powered=false,waterlogged=false]",
   
```

