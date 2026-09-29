import { createStore, produce } from 'solid-js/store'
import type { ServerMessage } from '../types/generated/ServerMessage'
import type { Snapshot } from '../types/generated/Snapshot'
import type { TraceRecord } from '../types/generated/TraceRecord'
import { connect, type ConnectionStatus } from './socket'

const LOG_CAPACITY = 1000

export type ConsoleState = Snapshot & { connection: ConnectionStatus }

const [state, setState] = createStore<ConsoleState>({
  connection: 'connecting',
  hardware: null,
  groups: [],
  switches: {},
  drivers: {},
  game: null,
  log: [],
})

/**
 * Read-only, reactive view of the machine. Any component that reads a field of this inside JSX
 * (or an effect/memo) re-renders just that part when the field changes.
 */
export const machine = state

/** Connects to the console server. Returns a cleanup function. */
export function startConsole(): () => void {
  return connect(handleMessage, (connection) => setState('connection', connection))
}

function handleMessage(message: ServerMessage) {
  switch (message.type) {
    case 'Init': {
      const { hardware, groups, switches, drivers, game, log } = message
      setState({ hardware, groups, switches, drivers, game, log })
      break
    }
    case 'Trace':
      setState(produce((s) => applyTrace(s, message)))
      break
  }
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
    group(parent_key)?.systems.push({ id, name, active: true })
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
  }

  s.game = record.game
  s.log.push(record)
  if (s.log.length > LOG_CAPACITY) s.log.splice(0, s.log.length - LOG_CAPACITY)
}
