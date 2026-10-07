import { batch } from 'solid-js'
import { createStore, produce, reconcile } from 'solid-js/store'
import type { ServerMessage } from '../types/generated/ServerMessage'
import type { Snapshot } from '../types/generated/Snapshot'
import type { TraceRecord } from '../types/generated/TraceRecord'
import { shortName } from '../lib/format'
import type { ClientMessage } from '../types/generated/ClientMessage'
import { type Connection, connect, type ConnectionStatus } from './socket'

/** How many trace records the log keeps. Mirrors the hub; LED color changes aren't logged. */
const LOG_CAPACITY = 2500
const RECENT_EVENTS_PER_SYSTEM = 10

export type ConsoleState = Snapshot & {
  connection: ConnectionStatus
  /** Drivers that were fired within the last `FIRED_DISPLAY_MS`, by id */
  firing: Record<number, boolean>
}

/**
 * How long a driver shows as fired. Tracers only hear that a pulse started, not when it ended, so
 * this is an approximation that's plenty for a console.
 */
const FIRED_DISPLAY_MS = 750
const firedTimers = new Map<number, ReturnType<typeof setTimeout>>()

const [state, setState] = createStore<ConsoleState>({
  connection: 'connecting',
  hardware: null,
  groups: [],
  switches: {},
  drivers: {},
  game: null,
  planes: [],
  led_colors: {},
  log: [],
  firing: {},
})

/** Read-only: it only changes from server messages */
export const machine = state

let connection: Connection | null = null

/** Returns a cleanup function */
export function startConsole(): () => void {
  connection = connect(handleMessage, (status) => setState('connection', status))
  return () => {
    connection?.close()
    connection = null
  }
}

export function sendToMachine(message: ClientMessage) {
  connection?.send(message)
}

function handleMessage(message: ServerMessage) {
  switch (message.type) {
    case 'Init': {
      const { hardware, groups, switches, drivers, game, planes, led_colors, log } = message
      // Hardware rarely changes between snapshots (they also arrive on every game start and end, and when a client
      // falls behind), so reconcile it and the LED colors into what's already there instead of replacing them, which
      // would rebuild everything that reads them.
      batch(() => {
        setState('hardware', reconcile(hardware))
        setState('led_colors', reconcile(led_colors))
        setState({ groups, switches, drivers, game, planes, log })
      })
      break
    }
    case 'Trace':
      setState(produce((s) => applyTrace(s, message)))
      if ('DriverStateChange' in message.event && message.event.DriverStateChange.state === 'Fired') {
        markFired(message.event.DriverStateChange.driver_id)
      }
      break
    case 'Leds':
      setState(
        produce((s) => {
          for (const [name, color] of Object.entries(message.colors)) s.led_colors[name] = color
        }),
      )
      break
  }
}

/** What a driver should display as, with "Fired" lasting only briefly */
export function driverState(id: number): 'Fired' | 'On' | 'Off' {
  if (state.firing[id]) return 'Fired'
  return state.drivers[id] === 'On' ? 'On' : 'Off'
}

function markFired(id: number) {
  clearTimeout(firedTimers.get(id))
  setState('firing', id, true)
  firedTimers.set(
    id,
    setTimeout(() => {
      setState('firing', id, false)
      firedTimers.delete(id)
    }, FIRED_DISPLAY_MS),
  )
}

/**
 * Mirror of `apply` in `src/console_hub.rs`. Game tracking is not mirrored: the hub tags each
 * record with the game in progress, and sends a fresh `Init` when a game starts or ends.
 */
function applyTrace(s: ConsoleState, record: TraceRecord) {
  const event = record.event
  const group = (key: string) => s.groups.find((g) => g.key === key)

  if ('SystemGroupSpawned' in event) {
    const { key } = event.SystemGroupSpawned
    if (!group(key)) s.groups.push({ key, active: true, systems: [] })
  } else if ('SystemGroupDespawned' in event) {
    const { key } = event.SystemGroupDespawned
    s.groups = s.groups.filter((g) => g.key !== key)
  } else if ('SystemGroupActiveStateChange' in event) {
    const { key, active } = event.SystemGroupActiveStateChange
    const g = group(key)
    if (g) g.active = active
  } else if ('SystemSpawned' in event) {
    const { id, name, parent_key } = event.SystemSpawned
    group(parent_key)?.systems.push({ id, name, active: true, recent_events: [] })
  } else if ('SystemDespawned' in event) {
    const { id, parent_key } = event.SystemDespawned
    const g = group(parent_key)
    if (g) g.systems = g.systems.filter((sys) => sys.id !== id)
  } else if ('SystemActiveStateChange' in event) {
    const { id, active } = event.SystemActiveStateChange
    const sys = s.groups.flatMap((g) => g.systems).find((sys) => sys.id === id)
    if (sys) sys.active = active
  } else if ('SwitchStateChange' in event) {
    const { switch_id, state } = event.SwitchStateChange
    s.switches[switch_id] = state
  } else if ('DriverStateChange' in event) {
    const { driver_id, state } = event.DriverStateChange
    s.drivers[driver_id] = state
  } else if ('LedsRGBChange' in event) {
    // never sent as a trace: the hub collects LED changes and sends them as `Leds`
    return
  } else if (event.Event.sender !== null) {
    const sender = event.Event.sender
    const sys = s.groups.flatMap((g) => g.systems).find((sys) => sys.id === sender)
    if (sys) {
      sys.recent_events.push(record)
      if (sys.recent_events.length > RECENT_EVENTS_PER_SYSTEM) sys.recent_events.shift()
    }
  }

  s.game = record.game
  s.log.push(record)
  if (s.log.length > LOG_CAPACITY) s.log.splice(0, s.log.length - LOG_CAPACITY)
}

/**
 * A system's short name by id: from the running systems, or, for one that has since despawned,
 * from its spawn record in the log.
 */
export function systemName(id: number): string {
  for (const group of state.groups) {
    const system = group.systems.find((s) => s.id === id)
    if (system) return shortName(system.name)
  }
  const spawned = state.log.find((r) => 'SystemSpawned' in r.event && r.event.SystemSpawned.id === id)
  if (spawned && 'SystemSpawned' in spawned.event) return shortName(spawned.event.SystemSpawned.name)
  return `System #${id}`
}

