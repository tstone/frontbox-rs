import { JsonTreeView } from '@ark-ui/solid/json-tree-view'
import { createMemo, createSignal, For, Show } from 'solid-js'
import FilterList, { type FilterOption } from '../../components/FilterList'
import {
  formatElapsed,
  groupLabel,
  playerLabel,
  shortName,
  traceBody,
  traceCategory,
  traceType,
  type TraceCategory,
} from '../../lib/format'
import { machine } from '../../state/console'
import type { TraceRecord } from '../../types/generated/TraceRecord'
import './LogTab.css'

const categoryColor = (category: TraceCategory) => `var(--cat-${category})`

/** Filter key for the player a record belongs to */
const playerKey = (record: TraceRecord) => String(record.game?.player ?? 'none')

export default function LogTab() {
  // Filters hold what's hidden, so types and players that show up later are visible by default
  const [hiddenTypes, setHiddenTypes] = createSignal<string[]>([])
  const [hiddenPlayers, setHiddenPlayers] = createSignal<string[]>([])
  const [expanded, setExpanded] = createSignal<number[]>([])

  const toggle = (list: string[], value: string, visible: boolean) =>
    visible ? list.filter((v) => v !== value) : [...list, value]
  const toggleExpanded = (seq: number) =>
    setExpanded((seqs) => (seqs.includes(seq) ? seqs.filter((s) => s !== seq) : [...seqs, seq]))

  const typeOptions = createMemo<FilterOption[]>(() => {
    const types = new Map<string, FilterOption>()
    for (const record of machine.log) {
      const type = traceType(record)
      const option = types.get(type)
      if (option) option.count! += 1
      else types.set(type, { value: type, label: type, count: 1, color: categoryColor(traceCategory(record)) })
    }
    return [...types.values()].sort((a, b) => a.label.localeCompare(b.label))
  })

  const playerOptions = createMemo<FilterOption[]>(() => {
    const players = new Map<string, FilterOption>()
    for (const record of machine.log) {
      const key = playerKey(record)
      const option = players.get(key)
      if (option) option.count! += 1
      else players.set(key, { value: key, label: playerLabel(record.game?.player ?? null), count: 1 })
    }
    // "No player" first, then players in order
    return [...players.values()].sort((a, b) => (a.value === 'none' ? -1 : b.value === 'none' ? 1 : +a.value - +b.value))
  })

  // newest first
  const visible = createMemo(() => {
    const types = hiddenTypes()
    const players = hiddenPlayers()
    const out: TraceRecord[] = []
    for (let i = machine.log.length - 1; i >= 0; i--) {
      const record = machine.log[i]
      if (!types.includes(traceType(record)) && !players.includes(playerKey(record))) out.push(record)
    }
    return out
  })

  return (
    <div class="log-tab">
      <div class="log-list pane">
        <Show
          when={visible().length > 0}
          fallback={<p class="empty">{machine.log.length > 0 ? 'Everything is filtered out.' : 'Nothing traced yet.'}</p>}
        >
          <ol>
            <For each={visible()}>
              {(record) => {
                const isExpanded = () => expanded().includes(record.seq)
                return (
                  <li classList={{ expanded: isExpanded() }}>
                    <button type="button" class="row" aria-expanded={isExpanded()} onClick={() => toggleExpanded(record.seq)}>
                      <span class="dot" style={{ background: categoryColor(traceCategory(record)) }} />
                      <span class="time mono">{formatElapsed(record.at_ms)}</span>
                      <span class="type">{traceType(record)}</span>
                      <span class="summary">{summary(record)}</span>
                      <Show when={record.game?.player != null}>
                        <span class="badge">P{record.game!.player! + 1}</span>
                      </Show>
                    </button>
                    <Show when={isExpanded()}>
                      <div class="detail">
                        <Show when={'Event' in record.event && record.event.Event.type_name}>
                          {(typeName) => <p class="type-name mono">{typeName()}</p>}
                        </Show>
                        <Show when={traceBody(record) != null} fallback={<p class="empty">No data</p>}>
                          <JsonTreeView.Root data={traceBody(record)} defaultExpandedDepth={3}>
                            <JsonTreeView.Tree arrow={<span>›</span>} />
                          </JsonTreeView.Root>
                        </Show>
                      </div>
                    </Show>
                  </li>
                )
              }}
            </For>
          </ol>
        </Show>
      </div>

      <aside class="log-sidebar pane">
        <section class="game-status">
          <h3 class="pane-title">Game</h3>
          <Show when={machine.game} fallback={<p class="empty">No game in progress</p>}>
            {(game) => (
              <p>
                In progress
                <Show when={game().player !== null}>
                  {' · '}
                  {playerLabel(game().player)}
                  {game().turn !== null && `, turn ${game().turn}`}
                </Show>
              </p>
            )}
          </Show>
        </section>
        <FilterList
          title="Type"
          options={typeOptions()}
          isChecked={(type) => !hiddenTypes().includes(type)}
          onChange={(type, checked) => setHiddenTypes((hidden) => toggle(hidden, type, checked))}
        />
        <FilterList
          title="Player"
          options={playerOptions()}
          isChecked={(player) => !hiddenPlayers().includes(player)}
          onChange={(player, checked) => setHiddenPlayers((hidden) => toggle(hidden, player, checked))}
        />
      </aside>
    </div>
  )
}

/** A one-line description, using hardware names where the console knows them */
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
  if ('SystemSpawned' in event) return shortName(event.SystemSpawned.name)
  if ('SystemDespawned' in event) return `#${event.SystemDespawned.id}`
  if ('SystemActiveStateChange' in event) {
    const { id, active } = event.SystemActiveStateChange
    return `#${id} ${active ? 'activated' : 'deactivated'}`
  }
  if ('SystemGroupSpawned' in event) return groupLabel(event.SystemGroupSpawned.key)
  if ('SystemGroupDespawned' in event) return groupLabel(event.SystemGroupDespawned.key)
  if ('SystemGroupActiveStateChange' in event) {
    const { key, active } = event.SystemGroupActiveStateChange
    return `${groupLabel(key)} ${active ? 'activated' : 'deactivated'}`
  }
  const body = event.Event.event
  return body == null ? '' : JSON.stringify(body)
}
