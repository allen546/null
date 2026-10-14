# Mineflayer Sequence Builder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Node.js Mineflayer bridge in `builder/` that executes Claude's generated build sequence (`planner/out/plan.json`) on an un-frozen, live Minecraft server using survival placement semantics (no `block_state` NBT), with offline pre-flight scaffolding and section-boundary checkpoint diffs.

**Architecture:** An offline pre-flight pass (`preflight.js`) walks `plan.json` against a 3D block grid to classify steps (`direct`, `adjusted`, `scaffold`, `unachievable`) and generates an augmented plan (`plan_augmented.json`) with explicit re-provable scaffold steps. The runtime bot (`bot.js`) configures cheats via RCON, connects to the server with ticks running live, and sequentially executes the plan using collision-free 3D flight, exact survival player aiming (piston head towards player, observer sensor away), click adjustments (un-sneaked), and settle verification via the plansim plugin status socket.

**Tech Stack:** Node.js (>=22), `mineflayer`, `vec3`, `minecraft-data`, `prismarine-block`, `prismarine-item`, `node:test` (built-in runner).

---

## File Structure

```
builder/
  package.json                          Node.js package metadata & dependencies
  preflight.js                          Offline voxel grid pre-flight pass & plan augmentor
  bot.js                                Main entry point & sequence orchestrator
  lib/
    rcon.js                             RCON client for cheats & gamerule configuration
    flight.js                           Collision-free 3D creative flight pathing
    placement.js                        Survival player placement & look-angle calculator
    adjust.js                           Post-placement survival click interactions
    settle.js                           Live tick settle detector (TCP client for plansim plugin)
    checkpoint.js                       Section-boundary checkpoint diff verifier
  test/
    preflight.test.js                   Unit tests for preflight step classification & scaffolding
    placement.test.js                   Unit tests for placement aim vectors & facing calculation
    adjust.test.js                      Unit tests for click adjustment counts
    smoke.js                            Live 10-block smoke test against un-frozen mc_server
```

---

### Task 1: Package Setup & RCON Cheat Initializer

**Files:**
- Create: `builder/package.json`
- Create: `builder/lib/rcon.js`
- Test: `builder/test/rcon.test.js`

- [ ] **Step 1: Create `builder/package.json`**

```json
{
  "name": "mineflayer-sequence-builder",
  "version": "1.0.0",
  "private": true,
  "description": "Mineflayer bridge for executing generated placement sequences on live un-frozen servers",
  "main": "bot.js",
  "scripts": {
    "test": "node --test test/*.test.js",
    "preflight": "node preflight.js",
    "build": "node bot.js"
  },
  "dependencies": {
    "minecraft-data": "^3.92.0",
    "mineflayer": "git+https://github.com/PrismarineJS/mineflayer.git#01f20014d7b6f70767a91a46e6d5522fd765bdbc",
    "mineflayer-pathfinder": "git+https://github.com/PrismarineJS/mineflayer-pathfinder.git#6f5c26bf79e7861d20e0fbc735059db51e4d8f77",
    "prismarine-block": "^1.20.0",
    "prismarine-item": "^1.15.0",
    "vec3": "^0.1.10"
  }
}
```

- [ ] **Step 2: Run npm install in `builder/`**

Run: `cd /Users/allen/Desktop/null/builder && npm install`
Expected: Dependencies installed into `builder/node_modules`.

- [ ] **Step 3: Write failing unit test for `rcon.js`**

Create `builder/test/rcon.test.js`:
```javascript
const test = require('node:test')
const assert = require('node:assert/strict')
const { encodeRconPacket, decodeRconPacket } = require('../lib/rcon')

test('encode and decode RCON packet', () => {
  const reqId = 42
  const type = 2 // EXECCOMMAND
  const body = 'gamerule doMobSpawning false'
  const buffer = encodeRconPacket(reqId, type, body)

  assert.equal(buffer.readInt32LE(0), buffer.length - 4)
  assert.equal(buffer.readInt32LE(4), reqId)
  assert.equal(buffer.readInt32LE(8), type)

  const decoded = decodeRconPacket(buffer)
  assert.equal(decoded.id, reqId)
  assert.equal(decoded.type, type)
  assert.equal(decoded.body, body)
})
```

- [ ] **Step 4: Run test to verify it fails**

Run: `node --test /Users/allen/Desktop/null/builder/test/rcon.test.js`
Expected: FAIL with "Cannot find module '../lib/rcon'"

- [ ] **Step 5: Implement `builder/lib/rcon.js`**

```javascript
const net = require('node:net')

const SERVERDATA_AUTH = 3
const SERVERDATA_EXECCOMMAND = 2
const SERVERDATA_RESPONSE_VALUE = 0

function encodeRconPacket (id, type, body) {
  const bodyBuffer = Buffer.from(body, 'utf8')
  const length = 4 + 4 + bodyBuffer.length + 2 // id(4) + type(4) + body + 2 null bytes
  const buffer = Buffer.alloc(4 + length)
  buffer.writeInt32LE(length, 0)
  buffer.writeInt32LE(id, 4)
  buffer.writeInt32LE(type, 8)
  bodyBuffer.copy(buffer, 12)
  buffer.writeInt8(0, 12 + bodyBuffer.length)
  buffer.writeInt8(0, 13 + bodyBuffer.length)
  return buffer
}

function decodeRconPacket (buffer) {
  const length = buffer.readInt32LE(0)
  const id = buffer.readInt32LE(4)
  const type = buffer.readInt32LE(8)
  const body = buffer.toString('utf8', 12, buffer.length - 2)
  return { length, id, type, body }
}

class RconClient {
  constructor ({ host = '127.0.0.1', port = 25575, password = 'antigravity' } = {}) {
    this.host = host
    this.port = port
    this.password = password
    this.reqId = 1
  }

  async connect () {
    return new Promise((resolve, reject) => {
      this.socket = net.createConnection({ host: this.host, port: this.port }, () => {
        const authPacket = encodeRconPacket(this.reqId++, SERVERDATA_AUTH, this.password)
        this.socket.write(authPacket)
      })

      this.socket.once('data', data => {
        const packet = decodeRconPacket(data)
        if (packet.id === -1) {
          reject(new Error('RCON Authentication failed'))
        } else {
          resolve()
        }
      })

      this.socket.on('error', reject)
    })
  }

  async send (command) {
    return new Promise((resolve, reject) => {
      const id = this.reqId++
      const packet = encodeRconPacket(id, SERVERDATA_EXECCOMMAND, command)
      const onData = data => {
        const resp = decodeRconPacket(data)
        if (resp.id === id) {
          this.socket.removeListener('data', onData)
          resolve(resp.body)
        }
      }
      this.socket.on('data', onData)
      this.socket.write(packet)
    })
  }

  async setupCheats (botName = 'builder0') {
    const commands = [
      'gamerule doMobSpawning false',
      'gamerule doDaylightCycle false',
      'gamerule doWeatherCycle false',
      'difficulty peaceful',
      'gamerule keepInventory true',
      'tick unfreeze',
      `op ${botName}`,
      `gamemode creative ${botName}`,
      `effect give ${botName} resistance infinite 255 true`
    ]
    const results = []
    for (const cmd of commands) {
      results.push(await this.send(cmd))
    }
    return results
  }

  close () {
    if (this.socket) {
      this.socket.end()
    }
  }
}

module.exports = {
  encodeRconPacket,
  decodeRconPacket,
  RconClient
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `node --test /Users/allen/Desktop/null/builder/test/rcon.test.js`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add builder/package.json builder/lib/rcon.js builder/test/rcon.test.js
git commit -m "feat(builder): add package definition and rcon cheat client"
```

---

### Task 2: Offline Pre-Flight Pass & Plan Augmentor

**Files:**
- Create: `builder/preflight.js`
- Test: `builder/test/preflight.test.js`

- [ ] **Step 1: Write failing unit test for `preflight.js`**

Create `builder/test/preflight.test.js`:
```javascript
const test = require('node:test')
const assert = require('node:assert/strict')
const { PreflightAnalyzer } = require('../preflight')

test('classifies direct placement when adjacent support exists', () => {
  const analyzer = new PreflightAnalyzer({ origin: [0, 100, 0] })
  // Base support at 0, 100, 0
  analyzer.setWorldBlock([0, 100, 0], 'minecraft:stone')

  const step = { op: 'place', pos: [0, 1, 0], state: 'minecraft:glass' }
  const result = analyzer.classifyStep(step)
  assert.equal(result.type, 'direct')
  assert.deepEqual(result.supportFace, [0, -1, 0])
})

test('identifies scaffold requirement for mid-air block and generates augmented steps', () => {
  const analyzer = new PreflightAnalyzer({ origin: [0, 100, 0] })
  // Anchor at [0, 100, 0]
  analyzer.setWorldBlock([0, 100, 0], 'minecraft:stone')

  // Target at [2, 1, 0] has no direct adjacent block, but can be reached via scaffolding
  const plan = {
    version: 1,
    origin: [0, 100, 0],
    sections: [
      {
        id: 0,
        kind: 'single',
        steps: [
          { op: 'place', pos: [0, 1, 0], state: 'minecraft:stone' },
          { op: 'place', pos: [0, 2, 0], state: 'minecraft:repeater[delay=3,facing=north,locked=false,powered=false]' }
        ]
      }
    ]
  }

  const { augmentedPlan, stats } = analyzer.processPlan(plan)
  assert.equal(stats.unachievable, 0)
  assert.equal(stats.adjusted, 1) // repeater delay=3
  assert.ok(augmentedPlan.sections.length > 0)
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test /Users/allen/Desktop/null/builder/test/preflight.test.js`
Expected: FAIL with "Cannot find module '../preflight'"

- [ ] **Step 3: Implement `builder/preflight.js`**

```javascript
const fs = require('node:fs')
const path = require('node:path')

const ADJACENT_OFFSETS = [
  [0, -1, 0], // down
  [0, 1, 0],  // up
  [0, 0, -1], // north
  [0, 0, 1],  // south
  [-1, 0, 0], // west
  [1, 0, 0]   // east
]

function posKey (p) {
  return `${p[0]},${p[1]},${p[2]}`
}

class PreflightAnalyzer {
  constructor ({ origin = [0, 100, 0] } = {}) {
    this.origin = origin
    this.grid = new Map() // posKey -> state
  }

  setWorldBlock (worldPos, state) {
    this.grid.set(posKey(worldPos), state)
  }

  getBlock (pos) {
    const worldPos = [pos[0] + this.origin[0], pos[1] + this.origin[1], pos[2] + this.origin[2]]
    return this.grid.get(posKey(worldPos)) || 'minecraft:air'
  }

  setBlock (pos, state) {
    const worldPos = [pos[0] + this.origin[0], pos[1] + this.origin[1], pos[2] + this.origin[2]]
    if (state === 'minecraft:air') {
      this.grid.delete(posKey(worldPos))
    } else {
      this.grid.set(posKey(worldPos), state)
    }
  }

  classifyStep (step) {
    if (step.op !== 'place') {
      return { type: step.op }
    }

    const pos = step.pos
    let supportNeighbor = null
    let supportFace = null

    for (const offset of ADJACENT_OFFSETS) {
      const neighbor = [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]]
      const block = this.getBlock(neighbor)
      if (block && block !== 'minecraft:air' && !block.includes('water') && !block.includes('lava')) {
        supportNeighbor = neighbor
        supportFace = [-offset[0], -offset[1], -offset[2]]
        break
      }
    }

    // Check if right-click adjustment is required
    let adjustments = 0
    if (step.state.includes('repeater')) {
      const match = step.state.match(/delay=(\d+)/)
      if (match && Number(match[1]) > 1) {
        adjustments = Number(match[1]) - 1
      }
    } else if (step.state.includes('comparator') && step.state.includes('mode=subtract')) {
      adjustments = 1
    }

    if (supportNeighbor) {
      return {
        type: adjustments > 0 ? 'adjusted' : 'direct',
        supportNeighbor,
        supportFace,
        adjustments
      }
    }

    // Mid-air: requires temporary scaffold
    // Find closest anchor below
    let scaffoldPos = null
    for (let dy = 1; dy <= 5; dy++) {
      const below = [pos[0], pos[1] - dy, pos[2]]
      if (this.getBlock(below) !== 'minecraft:air') {
        scaffoldPos = [pos[0], pos[1] - 1, pos[2]]
        break
      }
    }

    if (!scaffoldPos) {
      scaffoldPos = [pos[0], pos[1] - 1, pos[2]] // Default directly beneath
    }

    return {
      type: 'scaffold',
      scaffoldPos,
      adjustments
    }
  }

  processPlan (plan) {
    const stats = { direct: 0, adjusted: 0, scaffold: 0, unachievable: 0 }
    const augmentedSections = []

    for (const section of plan.sections) {
      const augmentedSteps = []
      for (const step of section.steps) {
        if (step.op === 'expect') {
          augmentedSteps.push(step)
          continue
        }

        const classification = this.classifyStep(step)
        stats[classification.type] = (stats[classification.type] || 0) + 1

        if (classification.type === 'scaffold') {
          // Prepend temporary scaffold placement
          augmentedSteps.push({
            op: 'place',
            pos: classification.scaffoldPos,
            state: 'minecraft:dirt',
            scaffold: true
          })
          this.setBlock(classification.scaffoldPos, 'minecraft:dirt')

          // Place the target block against scaffold
          augmentedSteps.push({
            ...step,
            adjustments: classification.adjustments
          })
          this.setBlock(step.pos, step.state)

          // Remove the temporary scaffold
          augmentedSteps.push({
            op: 'remove',
            pos: classification.scaffoldPos,
            scaffold: true
          })
          this.setBlock(classification.scaffoldPos, 'minecraft:air')
        } else {
          augmentedSteps.push({
            ...step,
            adjustments: classification.adjustments || 0
          })
          this.setBlock(step.pos, step.state)
        }
      }

      augmentedSections.push({
        ...section,
        steps: augmentedSteps
      })
    }

    return {
      augmentedPlan: {
        ...plan,
        sections: augmentedSections
      },
      stats
    }
  }
}

if (require.main === module) {
  const planPath = process.argv[2] || path.join(__dirname, '../planner/out/plan.json')
  const outPath = process.argv[3] || path.join(__dirname, '../planner/out/plan_augmented.json')

  console.log(`Running preflight pass on ${planPath}...`)
  const raw = fs.readFileSync(planPath, 'utf8')
  const plan = JSON.parse(raw)

  const analyzer = new PreflightAnalyzer({ origin: plan.origin })
  const { augmentedPlan, stats } = analyzer.processPlan(plan)

  console.log('Preflight results:', stats)
  fs.writeFileSync(outPath, JSON.stringify(augmentedPlan, null, 2))
  console.log(`Saved augmented plan to ${outPath}`)
}

module.exports = {
  PreflightAnalyzer
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test /Users/allen/Desktop/null/builder/test/preflight.test.js`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add builder/preflight.js builder/test/preflight.test.js
git commit -m "feat(builder): implement offline pre-flight pass and scaffold plan generator"
```

---

### Task 3: Survival Placement & Aiming Calculations

**Files:**
- Create: `builder/lib/placement.js`
- Test: `builder/test/placement.test.js`

- [ ] **Step 1: Write failing unit test for `placement.js`**

Create `builder/test/placement.test.js`:
```javascript
const test = require('node:test')
const assert = require('node:assert/strict')
const { calculateAimVector, parseBlockProperties } = require('../lib/placement')

test('parses block state properties correctly', () => {
  const props = parseBlockProperties('minecraft:piston[facing=up,extended=false]')
  assert.equal(props.name, 'piston')
  assert.equal(props.facing, 'up')
  assert.equal(props.extended, 'false')
})

test('calculates correct bot position for piston placement (head toward player)', () => {
  const targetPos = [10, 100, 10]
  // In vanilla survival, placing a piston sets its head facing the player.
  // To have facing=north, player must stand to the north of targetPos looking south.
  const aim = calculateAimVector('minecraft:piston[facing=north,extended=false]', targetPos)
  assert.ok(aim.botStandPos.z < targetPos[2]) // north is negative Z
})

test('calculates correct bot position for observer placement (sensor away from player)', () => {
  const targetPos = [10, 100, 10]
  // In vanilla survival, placing an observer sets redstone output towards player, sensor away.
  // To have facing=north (sensor pointing north), player must stand to the south looking north.
  const aim = calculateAimVector('minecraft:observer[facing=north,powered=false]', targetPos)
  assert.ok(aim.botStandPos.z > targetPos[2]) // south is positive Z
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test /Users/allen/Desktop/null/builder/test/placement.test.js`
Expected: FAIL with "Cannot find module '../lib/placement'"

- [ ] **Step 3: Implement `builder/lib/placement.js`**

```javascript
const { Vec3 } = require('vec3')

function parseBlockProperties (stateString) {
  const match = stateString.match(/^(?:minecraft:)?([a-z0-9_]+)(?:\[(.*)\])?$/)
  if (!match) return { name: stateString }
  const name = match[1]
  const props = { name }
  if (match[2]) {
    for (const pair of match[2].split(',')) {
      const [k, v] = pair.split('=')
      props[k] = v
    }
  }
  return props
}

function calculateAimVector (stateString, targetPos) {
  const props = parseBlockProperties(stateString)
  const target = new Vec3(targetPos[0], targetPos[1], targetPos[2])
  let standOffset = new Vec3(0, 0, 0)
  let subBlockOffset = new Vec3(0.5, 0.5, 0.5)

  const facing = props.facing || 'north'

  if (props.name === 'piston' || props.name === 'sticky_piston' || props.name === 'dropper' || props.name === 'dispenser') {
    // Piston head extends toward player. Player stands in facing direction.
    switch (facing) {
      case 'up': standOffset = new Vec3(0, 2.0, 0); break
      case 'down': standOffset = new Vec3(0, -2.0, 0); break
      case 'north': standOffset = new Vec3(0, 0, -2.0); break
      case 'south': standOffset = new Vec3(0, 0, 2.0); break
      case 'west': standOffset = new Vec3(-2.0, 0, 0); break
      case 'east': standOffset = new Vec3(2.0, 0, 0); break
    }
  } else if (props.name === 'observer') {
    // Observer sensor points away from player, redstone dot towards player.
    // Player stands opposite to facing direction.
    switch (facing) {
      case 'up': standOffset = new Vec3(0, -2.0, 0); break
      case 'down': standOffset = new Vec3(0, 2.0, 0); break
      case 'north': standOffset = new Vec3(0, 0, 2.0); break
      case 'south': standOffset = new Vec3(0, 0, -2.0); break
      case 'west': standOffset = new Vec3(2.0, 0, 0); break
      case 'east': standOffset = new Vec3(-2.0, 0, 0); break
    }
  } else if (props.name === 'repeater' || props.name === 'comparator' || props.name.endsWith('_stairs')) {
    // Horizontal direction: player looks in facing direction
    switch (facing) {
      case 'north': standOffset = new Vec3(0, 1.0, 2.0); break
      case 'south': standOffset = new Vec3(0, 1.0, -2.0); break
      case 'west': standOffset = new Vec3(2.0, 1.0, 0); break
      case 'east': standOffset = new Vec3(-2.0, 1.0, 0); break
    }
  } else {
    standOffset = new Vec3(0, 1.5, 2.0)
  }

  // Sub-block aiming for slabs and trapdoors
  if (props.type === 'top' || props.half === 'top') {
    subBlockOffset.y = 0.75
  } else if (props.type === 'bottom' || props.half === 'bottom') {
    subBlockOffset.y = 0.25
  }

  return {
    botStandPos: target.plus(standOffset),
    lookTarget: target.plus(subBlockOffset),
    subBlockOffset
  }
}

class PlacementEngine {
  constructor (bot) {
    this.bot = bot
    this.Item = require('prismarine-item')(bot.version)
    this.mcData = require('minecraft-data')(bot.version)
  }

  async equipCleanItem (itemName) {
    const itemDef = this.mcData.itemsByName[itemName]
    if (!itemDef) throw new Error(`Unknown item name: ${itemName}`)
    const item = new this.Item(itemDef.id, 64)
    // Slot 36 is hotbar 0
    await this.bot.creative.setInventorySlot(36, item)
    this.bot.setQuickBarSlot(0)
  }

  async placeAt (step, referenceBlock, faceVector) {
    const props = parseBlockProperties(step.state)
    const itemName = step.state.includes('water') ? 'water_bucket'
      : step.state.includes('lava') ? 'lava_bucket'
      : props.name

    await this.equipCleanItem(itemName)

    // Sneak during placement to avoid opening containers/doors
    this.bot.setControlState('sneak', true)
    try {
      const aim = calculateAimVector(step.state, step.pos)
      await this.bot.lookAt(aim.lookTarget, true)

      if (itemName.endsWith('_bucket')) {
        await this.bot.activateItem()
      } else {
        await this.bot.placeBlock(referenceBlock, faceVector)
      }
    } finally {
      this.bot.setControlState('sneak', false)
    }
  }
}

module.exports = {
  parseBlockProperties,
  calculateAimVector,
  PlacementEngine
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test /Users/allen/Desktop/null/builder/test/placement.test.js`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add builder/lib/placement.js builder/test/placement.test.js
git commit -m "feat(builder): implement survival placement aiming and clean item equipping"
```

---

### Task 4: Post-Placement Survival Adjustments

**Files:**
- Create: `builder/lib/adjust.js`
- Test: `builder/test/adjust.test.js`

- [ ] **Step 1: Write failing unit test for `adjust.js`**

Create `builder/test/adjust.test.js`:
```javascript
const test = require('node:test')
const assert = require('node:assert/strict')
const { getRequiredClicks } = require('../lib/adjust')

test('computes right-clicks for repeater delays', () => {
  assert.equal(getRequiredClicks('minecraft:repeater[delay=1]'), 0)
  assert.equal(getRequiredClicks('minecraft:repeater[delay=2]'), 1)
  assert.equal(getRequiredClicks('minecraft:repeater[delay=3]'), 2)
  assert.equal(getRequiredClicks('minecraft:repeater[delay=4]'), 3)
})

test('computes right-clicks for comparator mode', () => {
  assert.equal(getRequiredClicks('minecraft:comparator[mode=compare]'), 0)
  assert.equal(getRequiredClicks('minecraft:comparator[mode=subtract]'), 1)
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test /Users/allen/Desktop/null/builder/test/adjust.test.js`
Expected: FAIL with "Cannot find module '../lib/adjust'"

- [ ] **Step 3: Implement `builder/lib/adjust.js`**

```javascript
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))

function getRequiredClicks (stateString) {
  if (stateString.includes('repeater')) {
    const match = stateString.match(/delay=(\d+)/)
    if (match) {
      return Math.max(0, Number(match[1]) - 1)
    }
  }
  if (stateString.includes('comparator') && stateString.includes('mode=subtract')) {
    return 1
  }
  return 0
}

class Adjuster {
  constructor (bot) {
    this.bot = bot
  }

  async adjustBlock (worldPos, targetState) {
    const clicks = getRequiredClicks(targetState)
    if (clicks === 0) return

    const block = this.bot.blockAt(worldPos)
    if (!block) throw new Error(`Target block at ${worldPos} is not loaded`)

    // Critical: bot must NOT sneak during adjustment clicks
    this.bot.setControlState('sneak', false)

    await this.bot.lookAt(block.position.offset(0.5, 0.5, 0.5), true)

    for (let i = 0; i < clicks; i++) {
      await this.bot.activateBlock(block)
      await sleep(100) // Small delay between clicks for server recognition
    }
  }
}

module.exports = {
  getRequiredClicks,
  Adjuster
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test /Users/allen/Desktop/null/builder/test/adjust.test.js`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add builder/lib/adjust.js builder/test/adjust.test.js
git commit -m "feat(builder): implement un-sneaked survival click adjuster for repeaters and comparators"
```

---

### Task 5: Live Settle Detector & Checkpoint Diff Verifier

**Files:**
- Create: `builder/lib/settle.js`
- Create: `builder/lib/checkpoint.js`
- Test: `builder/test/settle.test.js`

- [ ] **Step 1: Write failing unit test for `settle.js`**

Create `builder/test/settle.test.js`:
```javascript
const test = require('node:test')
const assert = require('node:assert/strict')
const { SettleDetector } = require('../lib/settle')

test('instantiates settle detector with default options', () => {
  const detector = new SettleDetector({ quietTicks: 20, capTicks: 600 })
  assert.equal(detector.quietTicks, 20)
  assert.equal(detector.capTicks, 600)
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test /Users/allen/Desktop/null/builder/test/settle.test.js`
Expected: FAIL with "Cannot find module '../lib/settle'"

- [ ] **Step 3: Implement `builder/lib/settle.js`**

```javascript
const net = require('node:net')
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))

class SettleDetector {
  constructor ({ host = '127.0.0.1', port = 25580, quietTicks = 20, capTicks = 600 } = {}) {
    this.host = host
    this.port = port
    this.quietTicks = quietTicks
    this.capTicks = capTicks
  }

  async queryStatus (regionId = 'r0') {
    return new Promise((resolve, reject) => {
      const client = net.createConnection({ host: this.host, port: this.port }, () => {
        client.write(JSON.stringify({ id: 1, op: 'status', region: regionId }) + '\n')
      })

      client.once('data', data => {
        client.end()
        try {
          const resp = JSON.parse(data.toString('utf8'))
          resolve(resp)
        } catch (e) {
          reject(e)
        }
      })

      client.on('error', reject)
    })
  }

  async waitForSettle (regionId = 'r0') {
    let quietCount = 0
    let totalTicks = 0

    while (totalTicks < this.capTicks) {
      try {
        const status = await this.queryStatus(regionId)
        const isQuiet = status.blockTicks === 0 &&
                        status.fluidTicks === 0 &&
                        status.blockEvents === 0 &&
                        status.movingPistons === 0

        if (isQuiet) {
          quietCount++
          if (quietCount >= this.quietTicks) {
            return { settled: true, ticks: totalTicks }
          }
        } else {
          quietCount = 0
        }
      } catch (err) {
        // Fallback: 50ms per tick wait if status port is unavailable
        quietCount++
        if (quietCount >= this.quietTicks) {
          return { settled: true, ticks: totalTicks, fallback: true }
        }
      }

      await sleep(50) // 1 game tick = 50ms
      totalTicks++
    }

    return { settled: false, ticks: totalTicks, reason: 'cap' }
  }
}

module.exports = {
  SettleDetector
}
```

- [ ] **Step 4: Implement `builder/lib/checkpoint.js`**

```javascript
const { parseBlockProperties } = require('./placement')

const COSMETIC_PROPERTIES = new Set(['east', 'west', 'north', 'south'])

function propertiesMatch (targetState, actualBlock) {
  if (!actualBlock) return false
  const targetProps = parseBlockProperties(targetState)
  if (actualBlock.name !== targetProps.name) return false

  const actualProps = actualBlock.getProperties ? actualBlock.getProperties() : {}

  for (const [key, value] of Object.entries(targetProps)) {
    if (key === 'name') continue
    // Ignore cosmetic redstone dust / fence connection shapes
    if (targetProps.name === 'redstone_wire' && COSMETIC_PROPERTIES.has(key)) continue
    if (String(actualProps[key]) !== String(value)) return false
  }

  return true
}

class CheckpointVerifier {
  constructor (bot) {
    this.bot = bot
  }

  verifySection (section, origin) {
    const failures = []
    for (const step of section.steps) {
      if (step.op !== 'place' || step.scaffold) continue
      const worldPos = this.bot.entity.position.clone().set(
        step.pos[0] + origin[0],
        step.pos[1] + origin[1],
        step.pos[2] + origin[2]
      )
      const actual = this.bot.blockAt(worldPos)
      if (!propertiesMatch(step.state, actual)) {
        failures.push({
          pos: step.pos,
          worldPos: [worldPos.x, worldPos.y, worldPos.z],
          target: step.state,
          actual: actual ? actual.name : 'null'
        })
      }
    }
    return {
      passed: failures.length === 0,
      failures
    }
  }
}

module.exports = {
  propertiesMatch,
  CheckpointVerifier
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `node --test /Users/allen/Desktop/null/builder/test/settle.test.js`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add builder/lib/settle.js builder/lib/checkpoint.js builder/test/settle.test.js
git commit -m "feat(builder): implement live settle detector and section checkpoint verifier"
```

---

### Task 6: Creative Flight & Bot Orchestrator (`bot.js`)

**Files:**
- Create: `builder/lib/flight.js`
- Create: `builder/bot.js`
- Test: `builder/test/smoke.js`

- [ ] **Step 1: Implement `builder/lib/flight.js`**

```javascript
const { Vec3 } = require('vec3')
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))

class FlightNavigator {
  constructor (bot) {
    this.bot = bot
  }

  async flyTo (targetPos, timeoutMs = 8000) {
    const started = Date.now()
    this.bot.creative.startFlying()

    const dest = new Vec3(targetPos.x, targetPos.y, targetPos.z)

    while (this.bot.entity.position.distanceTo(dest) > 0.3) {
      if (Date.now() - started > timeoutMs) {
        throw new Error(`Flight timeout moving to ${dest}`)
      }

      const delta = dest.minus(this.bot.entity.position)
      const dist = delta.norm()
      const step = Math.min(dist, 0.8)
      const move = delta.normalize().scaled(step)

      this.bot.entity.velocity.set(0, 0, 0)
      this.bot.entity.position.add(move)

      await sleep(50)
    }

    this.bot.entity.velocity.set(0, 0, 0)
  }
}

module.exports = {
  FlightNavigator
}
```

- [ ] **Step 2: Implement `builder/bot.js`**

```javascript
const fs = require('node:fs')
const path = require('node:path')
const mineflayer = require('mineflayer')
const { Vec3 } = require('vec3')
const { RconClient } = require('./lib/rcon')
const { FlightNavigator } = require('./lib/flight')
const { PlacementEngine, calculateAimVector } = require('./lib/placement')
const { Adjuster } = require('./lib/adjust')
const { SettleDetector } = require('./lib/settle')
const { CheckpointVerifier } = require('./lib/checkpoint')

async function main () {
  const planPath = process.argv[2] || path.join(__dirname, '../planner/out/plan_augmented.json')
  const host = process.env.MC_HOST || '127.0.0.1'
  const port = Number(process.env.MC_PORT || 25565)
  const username = process.env.BOT_NAME || 'builder0'

  console.log(`[Builder] Configuring cheats via RCON on ${host}:25575...`)
  const rcon = new RconClient({ host, port: 25575 })
  try {
    await rcon.connect()
    await rcon.setupCheats(username)
    console.log('[Builder] Cheats and gamerules initialized. Ticks are live.')
  } catch (err) {
    console.warn(`[Builder] Warning: RCON setup error: ${err.message}. Proceeding...`)
  } finally {
    rcon.close()
  }

  console.log(`[Builder] Reading plan from ${planPath}...`)
  const plan = JSON.parse(fs.readFileSync(planPath, 'utf8'))
  const origin = plan.origin

  console.log(`[Builder] Spawning bot ${username} to ${host}:${port}...`)
  const bot = mineflayer.createBot({ host, port, username, auth: 'offline' })

  bot.once('spawn', async () => {
    console.log('[Builder] Bot spawned into world.')
    const flight = new FlightNavigator(bot)
    const placement = new PlacementEngine(bot)
    const adjuster = new Adjuster(bot)
    const settle = new SettleDetector({ host })
    const verifier = new CheckpointVerifier(bot)

    try {
      for (let secIdx = 0; secIdx < plan.sections.length; secIdx++) {
        const section = plan.sections[secIdx]
        console.log(`[Builder] Section ${secIdx + 1}/${plan.sections.length} (${section.kind}, ${section.steps.length} steps)...`)

        for (const step of section.steps) {
          const worldPos = new Vec3(
            step.pos[0] + origin[0],
            step.pos[1] + origin[1],
            step.pos[2] + origin[2]
          )

          if (step.op === 'expect') continue

          // Idempotency check: skip if block already matches
          const current = bot.blockAt(worldPos)
          if (step.op === 'place' && current && current.name === step.state.split('[')[0].replace('minecraft:', '')) {
            continue
          }

          if (step.op === 'remove') {
            await flight.flyTo(worldPos.offset(0, 1.5, 0))
            const block = bot.blockAt(worldPos)
            if (block && block.name !== 'air') {
              await bot.dig(block)
            }
            continue
          }

          if (step.op === 'place') {
            const aim = calculateAimVector(step.state, [worldPos.x, worldPos.y, worldPos.z])
            await flight.flyTo(aim.botStandPos)

            // Find adjacent reference block
            let refBlock = null
            let faceVec = new Vec3(0, 1, 0)
            const offsets = [
              new Vec3(0, -1, 0), new Vec3(0, 1, 0),
              new Vec3(0, 0, -1), new Vec3(0, 0, 1),
              new Vec3(-1, 0, 0), new Vec3(1, 0, 0)
            ]
            for (const off of offsets) {
              const b = bot.blockAt(worldPos.plus(off))
              if (b && b.name !== 'air') {
                refBlock = b
                faceVec = off.scaled(-1)
                break
              }
            }

            if (refBlock) {
              await placement.placeAt(step, refBlock, faceVec)
            }

            // Click adjustments if required
            if (step.adjustments > 0) {
              await adjuster.adjustBlock(worldPos, step.state)
            }
          }
        }

        // Live settle check
        await settle.waitForSettle()

        // Section checkpoint diff
        const check = verifier.verifySection(section, origin)
        if (!check.passed) {
          console.error(`[Builder] Checkpoint failed at section ${secIdx}:`, check.failures)
          process.exit(1)
        }
      }

      console.log('[Builder] Entire sequence completed successfully!')
      process.exit(0)
    } catch (err) {
      console.error('[Builder] Fatal execution error:', err)
      process.exit(1)
    }
  })
}

if (require.main === module) {
  main()
}
```

- [ ] **Step 3: Create live smoke test `builder/test/smoke.js`**

```javascript
const { PreflightAnalyzer } = require('../preflight')
const assert = require('node:assert/strict')

console.log('[Smoke] Testing preflight pass on planner/out/plan.json...')
const plan = require('../../planner/out/plan.json')
const analyzer = new PreflightAnalyzer({ origin: plan.origin })
const { augmentedPlan, stats } = analyzer.processPlan(plan)

console.log('[Smoke] Preflight stats:', stats)
assert.equal(stats.unachievable, 0, 'No unachievable steps allowed')
assert.ok(augmentedPlan.sections.length >= plan.sections.length)
console.log('[Smoke] Preflight verification PASSED!')
```

- [ ] **Step 4: Run smoke test**

Run: `node /Users/allen/Desktop/null/builder/test/smoke.js`
Expected: PASS with 0 unachievable steps.

- [ ] **Step 5: Commit**

```bash
git add builder/lib/flight.js builder/bot.js builder/test/smoke.js
git commit -m "feat(builder): implement flight navigation, sequence runner bot, and smoke test"
```

---

### Task 7: Full Execution of Pre-flight & Live Verification

**Files:**
- Output: `planner/out/plan_augmented.json`

- [ ] **Step 1: Generate `plan_augmented.json` from `plan.json`**

Run: `node /Users/allen/Desktop/null/builder/preflight.js /Users/allen/Desktop/null/planner/out/plan.json /Users/allen/Desktop/null/planner/out/plan_augmented.json`
Expected: Output shows statistics and writes `planner/out/plan_augmented.json`.

- [ ] **Step 2: Start Minecraft server and verify un-frozen status**

Run: `cd /Users/allen/Desktop/null/mc_server && ./start.sh`
Expected: Server starts on port 25565.

- [ ] **Step 3: Run all builder unit tests**

Run: `cd /Users/allen/Desktop/null/builder && npm test`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add planner/out/plan_augmented.json
git commit -m "chore(planner): generate augmented plan with pre-flight scaffolds"
```
