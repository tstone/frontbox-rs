import { JsonTreeView } from '@ark-ui/solid/json-tree-view'
import { createSignal, Show } from 'solid-js'
import { formatElapsed, groupLabel, shortName, traceBody, traceCategory, traceType, type TraceCategory } from '../lib/format'
import { hexAddress } from '../lib/hardwareConfig'
import { machine, systemName } from '../state/console'
import type { TraceRecord } from '../types/generated/TraceRecord'
import './TraceRow.css'

export const categoryColor = (category: TraceCategory) => `var(--cat-${category})`

/** The system that emitted an event, or `null` for framework events and other traces */
export const traceSender = (record: TraceRecord) => ('Event' in record.event ? record.event.Event.sender : null)

type Props = {
  record: TraceRecord
  hideSender?: boolean
}

export default function TraceRow(props: Props) {
  const [expanded, setExpanded] = createSignal(false)
  const record = props.record
  const sender = traceSender(record)

  return (
    <li class="trace-row" classList={{ expanded: expanded() }}>
      <button type="button" class="row" aria-expanded={expanded()} onClick={() => setExpanded((open) => !open)}>
        <span class="dot" style={{ background: categoryColor(traceCategory(record)) }} />
        <span class="time mono">{formatElapsed(record.at_ms)}</span>
        <span class="type">{traceType(record)}</span>
        <span class="summary">{summary(record)}</span>
        <Show when={!props.hideSender && sender !== null}>
          <span class="sender">by {systemName(sender!)}</span>
        </Show>
        <Show when={record.game?.player != null}>
          <span class="badge">P{record.game!.player! + 1}</span>
        </Show>
      </button>
      <Show when={expanded()}>
        <div class="detail">
          <Show when={'Event' in record.event && record.event.Event.type_name}>
            {(typeName) => <p class="meta mono">{typeName()}</p>}
          </Show>
          <Show when={'Event' in record.event}>
            <p class="meta">Sent by {sender === null ? 'the framework' : `${systemName(sender)} (#${sender})`}</p>
          </Show>
          <Show when={traceBody(record) != null} fallback={<p class="meta">No data</p>}>
            <JsonTreeView.Root data={traceBody(record)} defaultExpandedDepth={3}>
              <JsonTreeView.Tree arrow={<span>›</span>} />
            </JsonTreeView.Root>
          </Show>
        </div>
      </Show>
    </li>
  )
}

function summary(record: TraceRecord): string {
  const event = record.event
  const hw = machine.hardware
  if ('SwitchStateChange' in event) {
    const { switch_id, state } = event.SwitchStateChange
    return `${hw?.switches.by_id[switch_id]?.name ?? `Switch ${switch_id}`} ${state.toLowerCase()}`
  }
  if ('DriverStateChange' in event) {
    const { driver_id, state } = event.DriverStateChange
    return `${hw?.drivers.by_id[driver_id]?.name ?? `Driver ${driver_id}`} ${state.toLowerCase()}`
  }
  if ('DriverModeChange' in event) {
    const { driver_id, mode } = event.DriverModeChange
    return `${hw?.drivers.by_id[driver_id]?.name ?? `Driver ${driver_id}`} set to ${Object.keys(mode)[0]}`
  }
  if ('SystemSpawned' in event) return shortName(event.SystemSpawned.name)
  if ('SystemDespawned' in event) return systemName(event.SystemDespawned.id)
  if ('SystemActiveStateChange' in event) {
    const { id, active } = event.SystemActiveStateChange
    return `${systemName(id)} ${active ? 'activated' : 'deactivated'}`
  }
  if ('SystemGroupSpawned' in event) return groupLabel(event.SystemGroupSpawned.key)
  if ('SystemGroupDespawned' in event) return groupLabel(event.SystemGroupDespawned.key)
  if ('SystemGroupActiveStateChange' in event) {
    const { key, active } = event.SystemGroupActiveStateChange
    return `${groupLabel(key)} ${active ? 'activated' : 'deactivated'}`
  }
  if ('LedsRGBChange' in event) {
    const { expansion, states } = event.LedsRGBChange
    return `${states.length} LED${states.length === 1 ? '' : 's'} on ${hexAddress(expansion)}`
  }
  if ('RoutineStarted' in event) {
    const { routine, origin } = event.RoutineStarted
    return `${shortName(routine)} by ${systemName(origin)}`
  }
  if ('RoutineStep' in event) {
    const { routine, participant } = event.RoutineStep
    return `${shortName(routine)} handed to ${systemName(participant)}`
  }
  if ('RoutineSkipped' in event) {
    const { routine, participant, reason } = event.RoutineSkipped
    const why = reason === 'Missing' ? 'not in reach' : 'inactive'
    return `${shortName(routine)} skipped ${shortName(participant)} (${why})`
  }
  if ('RoutineRejected' in event) {
    const { routine, origin, running_origin } = event.RoutineRejected
    return `${shortName(routine)} by ${systemName(origin)} ignored: already running for ${systemName(running_origin)}`
  }
  if ('RoutinePaused' in event) return `${shortName(event.RoutinePaused.routine)} waiting for its origin`
  if ('RoutineResumed' in event) return shortName(event.RoutineResumed.routine)
  if ('RoutineFinished' in event) {
    const { routine, origin } = event.RoutineFinished
    return `${shortName(routine)} for ${systemName(origin)}`
  }
  if ('RoutineAbandoned' in event) {
    const { routine, origin } = event.RoutineAbandoned
    return `${shortName(routine)}: ${systemName(origin)} was despawned`
  }
  // kept as state by the hub, never logged
  if ('ConfigsInUse' in event || 'ConfigChange' in event) return ''
  const body = event.Event.event
  return body == null ? '' : JSON.stringify(body)
}
