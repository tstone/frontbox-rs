import { JsonTreeView } from '@ark-ui/solid/json-tree-view'
import { createEffect, createSignal, For, Show, type JSX } from 'solid-js'
import { CirclePlus, CircleX, Mail, Route, ToggleLeft, ToggleRight } from '../../components/icons'
import { formatElapsed, shortName, traceBody } from '../../lib/format'
import { machine } from '../../state/console'
import type { HistoryEntry } from '../../types/generated/HistoryEntry'
import type { System } from '../../types/generated/System'
import type { TraceRecord } from '../../types/generated/TraceRecord'
import SystemTimeline, { describe, entryKey, entryTime, type Hovered } from './SystemTimeline'

/** How the activity is ordered. Shared by every system, so switching systems keeps it. */
const [newestFirst, setNewestFirst] = createSignal(true)

export default function SystemDetail(props: { system: System; parent: System | undefined; children: number }) {
  // everything that happened to it, and the events it emitted
  const activity = () => (newestFirst() ? [...props.system.history].reverse() : props.system.history)
  const status = () => (props.system.despawned_ms !== null ? 'Despawned' : props.system.active ? 'Active' : 'Inactive')

  // what the pointer is on, in the timeline or the activity list, so the other one points it out
  const [hovered, setHovered] = createSignal<Hovered | null>(null)

  // an entry hovered in the timeline is scrolled to in the list, which scrolls on its own so the page doesn't move
  let list: HTMLOListElement | undefined
  createEffect(() => {
    const h = hovered()
    if (!list || h?.from !== 'timeline') return
    const row = list.querySelector<HTMLElement>(`[data-key="${CSS.escape(h.key)}"]`)
    if (!row) return
    const top = row.offsetTop - list.offsetTop
    if (top < list.scrollTop) list.scrollTop = top
    else if (top + row.offsetHeight > list.scrollTop + list.clientHeight)
      list.scrollTop = top + row.offsetHeight - list.clientHeight
  })

  return (
    <article class="detail">
      <h2>
        {props.system.group ? `${props.system.name} (Group)` : shortName(props.system.name)}
        <span class={`badge ${status() === 'Active' ? 'ok' : 'warn'}`}>{status()}</span>
      </h2>
      <Show when={props.system.description}>{(description) => <p class="description">{description()}</p>}</Show>
      <dl>
        <dt>{props.system.group ? 'Kind' : 'Full name'}</dt>
        <dd class="mono">{props.system.group ? 'Group' : props.system.name}</dd>
        <dt>ID</dt>
        <dd class="mono">{props.system.id}</dd>
        <dt>Parent</dt>
        <dd>{props.parent ? shortName(props.parent.name) : 'None (top level)'}</dd>
        <dt>Children</dt>
        <dd>{props.children}</dd>
        <dt>Spawned</dt>
        <dd class="mono">{formatElapsed(props.system.spawned_ms)}</dd>
        <Show when={props.system.despawned_ms}>
          {(at) => (
            <>
              <dt>Despawned</dt>
              <dd class="mono">{formatElapsed(at())}</dd>
            </>
          )}
        </Show>
      </dl>

      <SystemTimeline system={props.system} hovered={hovered()} onHover={setHovered} />

      <section class="activity-log">
        <div class="section-header">
          <h3 class="pane-title">Activity</h3>
          <div class="list-order" role="group" aria-label="Order of the activity">
            <button classList={{ selected: newestFirst() }} onClick={() => setNewestFirst(true)}>
              Newest first
            </button>
            <button classList={{ selected: !newestFirst() }} onClick={() => setNewestFirst(false)}>
              Oldest first
            </button>
          </div>
        </div>
        <ol ref={list} onMouseLeave={() => setHovered(null)}>
          <For each={activity()}>
            {(entry) => (
              <ActivityRow
                entry={entry}
                system={props.system}
                highlighted={hovered()?.key === entryKey(entry)}
                onHover={() => setHovered({ key: entryKey(entry), entry, from: 'list' })}
              />
            )}
          </For>
        </ol>
      </section>
    </article>
  )
}

/** One thing that happened to the system. An event it emitted opens to show what it carried. */
function ActivityRow(props: { entry: HistoryEntry; system: System; highlighted: boolean; onHover: () => void }) {
  const [open, setOpen] = createSignal(false)
  // the event, while the console still has it: the system's latest few are kept, older ones only while in the log
  const record = (): TraceRecord | undefined => {
    const entry = props.entry
    if (entry.kind !== 'Emitted') return undefined
    return (
      props.system.recent_events.find((r) => r.seq === entry.seq) ?? machine.log.find((r) => r.seq === entry.seq)
    )
  }
  const opens = () => props.entry.kind === 'Emitted'

  return (
    <li
      class={props.entry.kind}
      classList={{ highlighted: props.highlighted, open: open() }}
      data-key={entryKey(props.entry)}
      onMouseEnter={props.onHover}
    >
      <button type="button" class="row" disabled={!opens()} onClick={() => setOpen((o) => !o)}>
        <span class="icon" title={KIND_LABELS[props.entry.kind]}>
          {icon(props.entry)}
        </span>
        <span class="time mono">{formatElapsed(entryTime(props.entry))}</span>
        <span>{describe(props.entry)}</span>
      </button>
      <Show when={open()}>
        <div class="event-body">
          <Show when={record()} fallback={<p class="meta">No longer kept by the console</p>}>
            {(r) => (
              <Show when={traceBody(r()) != null} fallback={<p class="meta">No data</p>}>
                <JsonTreeView.Root data={traceBody(r())} defaultExpandedDepth={3}>
                  <JsonTreeView.Tree arrow={<span>›</span>} />
                </JsonTreeView.Root>
              </Show>
            )}
          </Show>
        </div>
      </Show>
    </li>
  )
}

const KIND_LABELS: Record<HistoryEntry['kind'], string> = {
  Spawned: 'Spawned',
  ActivityChanged: 'Activity changed',
  RoutineStep: 'Routine step',
  Emitted: 'Event emitted',
  Despawned: 'Despawned',
}

function icon(entry: HistoryEntry): JSX.Element {
  switch (entry.kind) {
    case 'Spawned':
      return <CirclePlus size={14} />
    case 'ActivityChanged':
      return entry.active ? <ToggleRight size={14} /> : <ToggleLeft size={14} />
    case 'RoutineStep':
      return <Route size={14} />
    case 'Emitted':
      return <Mail size={14} />
    case 'Despawned':
      return <CircleX size={14} />
  }
}
