import { batch } from 'solid-js'
import { createStore, produce, reconcile } from 'solid-js/store'
import type { ServerMessage } from '../types/generated/ServerMessage'
import type { Snapshot } from '../types/generated/Snapshot'
import type { TraceRecord } from '../types/generated/TraceRecord'
import type { HistoryEntry } from '../types/generated/HistoryEntry'
import type { StepEnding } from '../types/generated/StepEnding'
import type { System } from '../types/generated/System'
import { shortName } from '../lib/format'
import type { ClientMessage } from '../types/generated/ClientMessage'
import type { DriverMode } from '../types/generated/DriverMode'
import { receiveMetricTotals } from './metricsQuery'
import { BINARY_CLIP, BINARY_DISPLAY_FRAME, receiveDisplayFrame } from './displays'
import { receiveSceneClip, receiveSceneClipFailed, receiveSceneRender } from './sceneRender'
import { type Connection, connect, type ConnectionStatus } from './socket'

/** How many trace records the log keeps. Mirrors the hub; LED color changes aren't logged. */
const LOG_CAPACITY = 2500
const RECENT_EVENTS_PER_SYSTEM = 10
/** Mirrors the hub */
const RECENT_STATUSES_PER_SYSTEM = 10
/** Mirrors the hub */
const HISTORY_PER_SYSTEM = 300
/** Despawned systems kept to look back at. Mirrors the hub */
const DESPAWNED_KEPT = 50

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
  systems: [],
  states: {},
  switches: {},
  drivers: {},
  driver_modes: {},
  game: null,
  routines: [],
  planes: [],
  displays: [],
  led_colors: {},
  configs: {},
  log: [],
  firing: {},
})

/** Read-only: it only changes from server messages */
export const machine = state

let connection: Connection | null = null

/** Returns a cleanup function */
export function startConsole(): () => void {
  connection = connect(handleMessage, handleBinary, (status) => setState('connection', status))
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
      const { hardware, systems, states, switches, drivers, driver_modes, game, planes, displays, led_colors, configs, log } =
        message
      // Hardware rarely changes between snapshots (they also arrive on every game start and end, and when a client
      // falls behind), so reconcile it and the LED colors into what's already there instead of replacing them, which
      // would rebuild everything that reads them.
      batch(() => {
        setState('hardware', reconcile(hardware))
        setState('led_colors', reconcile(led_colors))
        setState('configs', reconcile(configs))
        setState({ systems, states, switches, drivers, driver_modes, game, planes, displays, log })
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
    case 'Configs':
      setState(
        produce((s) => {
          for (const config of message.configs) s.configs[config.key] = config
        }),
      )
      break
    case 'MetricTotals':
      receiveMetricTotals(message)
      break
    case 'SceneRendered':
      receiveSceneRender({ rendered: message })
      break
    case 'SceneRenderFailed':
      receiveSceneRender({ error: message.error })
      break
    case 'SceneClipFailed':
      receiveSceneClipFailed(message.id, message.error)
      break
  }
}

/** Its first byte says what it is */
function handleBinary(message: ArrayBuffer) {
  switch (new Uint8Array(message)[0]) {
    case BINARY_CLIP:
      receiveSceneClip(message.slice(1))
      break
    case BINARY_DISPLAY_FRAME:
      receiveDisplayFrame(message)
      break
  }
}

/** What a driver should display as, with "Fired" lasting only briefly */
export function driverState(id: number): 'Fired' | 'On' | 'Off' {
  if (state.firing[id]) return 'Fired'
  return state.drivers[id] === 'On' ? 'On' : 'Off'
}

/** The mode a driver is in: the last one it was configured with while running, else its defined mode */
export function driverMode(id: number): DriverMode | undefined {
  return state.driver_modes[id] ?? state.hardware?.drivers.modes[id]
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
  const at_ms = record.at_ms
  // despawned systems are kept for a while, but nothing more happens to them
  const running = (id: number) => s.systems.find((sys) => sys.id === id && sys.despawned_ms === null)

  if ('StateChange' in event) {
    const { name, value } = event.StateChange
    if (value === null) delete s.states[name]
    else s.states[name] = value
    // not logged: the `StateChanged` event it comes with is
    return
  } else if ('SystemSpawned' in event) {
    const { id, name, parent, active, group, description } = event.SystemSpawned
    s.systems.push({
      id,
      name,
      parent,
      active,
      group,
      description,
      spawned_ms: at_ms,
      despawned_ms: null,
      history: [{ kind: 'Spawned', at_ms, active }],
      recent_events: [],
      recent_statuses: [],
    })
  } else if ('SystemDespawned' in event) {
    // its children are despawned with it, each traced on its own
    const { id, cause } = event.SystemDespawned
    const sys = running(id)
    if (sys) {
      sys.despawned_ms = at_ms
      sys.active = false
      endOpenSteps(sys, at_ms, 'Stopped')
      pushHistory(sys, { kind: 'Despawned', at_ms, cause })
    }
    forgetOldDespawned(s)
  } else if ('SystemActiveStateChange' in event) {
    const { id, active, cause } = event.SystemActiveStateChange
    const sys = running(id)
    if (sys) {
      sys.active = active
      pushHistory(sys, { kind: 'ActivityChanged', at_ms, seq: record.seq, active, cause })
    }
  } else if ('SystemStatusReported' in event) {
    const sys = running(event.SystemStatusReported.id)
    if (sys) {
      sys.recent_statuses.push(record)
      if (sys.recent_statuses.length > RECENT_STATUSES_PER_SYSTEM) sys.recent_statuses.shift()
      pushHistory(sys, { kind: 'StatusReported', at_ms, seq: record.seq })
    }
  } else if ('RoutineStepEnded' in event) {
    const { routine, participant, ending } = event.RoutineStepEnded
    const sys = s.systems.find((sys) => sys.id === participant)
    const open = sys?.history.findLast(
      (entry): entry is Extract<HistoryEntry, { kind: 'RoutineStep' }> =>
        entry.kind === 'RoutineStep' && entry.routine === routine && entry.ended_ms === null,
    )
    if (open) {
      open.ended_ms = at_ms
      open.ending = ending
    }
  } else if ('SwitchStateChange' in event) {
    const { switch_id, state } = event.SwitchStateChange
    s.switches[switch_id] = state
  } else if ('DriverStateChange' in event) {
    const { driver_id, state } = event.DriverStateChange
    s.drivers[driver_id] = state
  } else if ('DriverModeChange' in event) {
    const { driver_id, mode } = event.DriverModeChange
    s.driver_modes[driver_id] = mode
  } else if ('RoutineStarted' in event) {
    const { routine, origin } = event.RoutineStarted
    s.routines.push({ routine, origin, current: null })
  } else if ('RoutineStep' in event) {
    const { routine, participant } = event.RoutineStep
    const progress = s.routines.find((r) => r.routine === routine)
    if (progress) progress.current = participant
    const sys = running(participant)
    if (sys) pushHistory(sys, { kind: 'RoutineStep', routine, started_ms: at_ms, ended_ms: null, ending: null })
  } else if ('RoutinePaused' in event || 'RoutineHalted' in event) {
    // paused once the step in progress is over, and a halt ends it, so no one holds it
    const { routine } = 'RoutinePaused' in event ? event.RoutinePaused : event.RoutineHalted
    const progress = s.routines.find((r) => r.routine === routine)
    if (progress) progress.current = null
  } else if ('RoutineCompleted' in event || 'RoutineAbandoned' in event) {
    const { routine } = 'RoutineCompleted' in event ? event.RoutineCompleted : event.RoutineAbandoned
    s.routines = s.routines.filter((r) => r.routine !== routine)
  } else if ('RoutineRejected' in event || 'RoutineSkipped' in event || 'RoutineResumed' in event) {
    // logged only
  } else if ('LedsRGBChange' in event || 'ConfigsInUse' in event || 'ConfigChange' in event) {
    // never sent as traces: the hub keeps LEDs and configs as current state and sends them as `Leds` and `Configs`
    return
  } else if (event.Event.sender !== null) {
    const sys = running(event.Event.sender)
    if (sys) {
      sys.recent_events.push(record)
      if (sys.recent_events.length > RECENT_EVENTS_PER_SYSTEM) sys.recent_events.shift()
      pushHistory(sys, { kind: 'Emitted', at_ms, seq: record.seq, event: event.Event.type_name })
    }
  }

  s.game = record.game
  s.log.push(record)
  if (s.log.length > LOG_CAPACITY) s.log.splice(0, s.log.length - LOG_CAPACITY)
}

function pushHistory(system: System, entry: HistoryEntry) {
  system.history.push(entry)
  if (system.history.length > HISTORY_PER_SYSTEM) system.history.shift()
}

/** Ends the routine steps the system still holds, as when it's despawned mid-step */
function endOpenSteps(system: System, at_ms: number, how: StepEnding) {
  for (const entry of system.history) {
    if (entry.kind === 'RoutineStep' && entry.ended_ms === null) {
      entry.ended_ms = at_ms
      entry.ending = how
    }
  }
}

/** Drops the earliest despawned systems beyond the number kept, ties going by place in the list */
function forgetOldDespawned(s: ConsoleState) {
  const despawned = s.systems
    .map((sys, index) => ({ at: sys.despawned_ms, index }))
    .filter((d): d is { at: number; index: number } => d.at !== null)
  if (despawned.length <= DESPAWNED_KEPT) return
  despawned.sort((a, b) => a.at - b.at || a.index - b.index)
  const dropped = new Set(despawned.slice(0, despawned.length - DESPAWNED_KEPT).map((d) => d.index))
  s.systems = s.systems.filter((_, index) => !dropped.has(index))
}

/**
 * A system's short name by id: from the running systems, or, for one that has since despawned,
 * from its spawn record in the log.
 */
export function systemName(id: number): string {
  const system = state.systems.find((s) => s.id === id)
  if (system) return shortName(system.name)
  const spawned = state.log.find((r) => 'SystemSpawned' in r.event && r.event.SystemSpawned.id === id)
  if (spawned && 'SystemSpawned' in spawned.event) return shortName(spawned.event.SystemSpawned.name)
  return `System #${id}`
}

