import { createMemo, createSignal, For, Show } from 'solid-js'
import FilterList, { type FilterOption } from '../../components/FilterList'
import TraceRow, { categoryColor, traceSender } from '../../components/TraceRow'
import { playerLabel, traceCategory, traceType } from '../../lib/format'
import { machine, systemName } from '../../state/console'
import type { TraceRecord } from '../../types/generated/TraceRecord'
import './EventsTab.css'

const playerKey = (record: TraceRecord) => String(record.game?.player ?? 'none')

/** "none" for events emitted by the framework */
const senderKey = (record: TraceRecord) => String(traceSender(record) ?? 'none')

/** Counts records into filter options, `describe` supplying the rest of a new option */
function countOptions(
  records: TraceRecord[],
  key: (record: TraceRecord) => string,
  describe: (record: TraceRecord) => Omit<FilterOption, 'value' | 'count'>) {
  const options = new Map<string, FilterOption>()
  for (const record of records) {
    const value = key(record)
    const option = options.get(value)
    if (option) option.count! += 1
    else options.set(value, { value, count: 1, ...describe(record) })
  }
  return [...options.values()]
}

const noneFirst = (by: (a: FilterOption, b: FilterOption) => number) => (a: FilterOption, b: FilterOption) =>
  a.value === 'none' ? -1 : b.value === 'none' ? 1 : by(a, b)

export default function EventsTab() {
  // Filters hold what's hidden, so values that show up later are visible by default
  const [hiddenTypes, setHiddenTypes] = createSignal<string[]>([])
  const [hiddenPlayers, setHiddenPlayers] = createSignal<string[]>([])
  const [hiddenSenders, setHiddenSenders] = createSignal<string[]>([])

  const toggle = (list: string[], value: string, visible: boolean) =>
    visible ? list.filter((v) => v !== value) : [...list, value]

  // the log also holds switch, driver and system traces; this tab is only about emitted events
  const events = createMemo(() => machine.log.filter((record) => 'Event' in record.event))

  const typeOptions = createMemo(() =>
    countOptions(events(), traceType, (record) => ({
      label: traceType(record),
      color: categoryColor(traceCategory(record)),
    })).sort((a, b) => a.label.localeCompare(b.label)),
  )

  const playerOptions = createMemo(() =>
    countOptions(events(), playerKey, (record) => ({ label: playerLabel(record.game?.player ?? null) })).sort(
      noneFirst((a, b) => +a.value - +b.value),
    ),
  )

  const senderOptions = createMemo(() =>
    countOptions(events(), senderKey, (record) => {
      const sender = traceSender(record)
      return { label: sender === null ? 'Framework' : systemName(sender) }
    }).sort(noneFirst((a, b) => a.label.localeCompare(b.label))),
  )

  // newest first
  const visible = createMemo(() => {
    const types = hiddenTypes()
    const players = hiddenPlayers()
    const senders = hiddenSenders()
    const out: TraceRecord[] = []
    const all = events()
    for (let i = all.length - 1; i >= 0; i--) {
      const record = all[i]
      if (types.includes(traceType(record))) continue
      if (players.includes(playerKey(record))) continue
      if (senders.includes(senderKey(record))) continue
      out.push(record)
    }
    return out
  })

  return (
    <div class="events-tab">
      <div class="events-list pane">
        <Show
          when={visible().length > 0}
          fallback={<p class="empty">{events().length > 0 ? 'Everything is filtered out.' : 'No events yet.'}</p>}
        >
          <ol>
            <For each={visible()}>{(record) => <TraceRow record={record} />}</For>
          </ol>
        </Show>
      </div>

      <aside class="events-sidebar pane">
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
        <FilterList
          title="Sent by"
          options={senderOptions()}
          isChecked={(sender) => !hiddenSenders().includes(sender)}
          onChange={(sender, checked) => setHiddenSenders((hidden) => toggle(hidden, sender, checked))}
        />
      </aside>
    </div>
  )
}
