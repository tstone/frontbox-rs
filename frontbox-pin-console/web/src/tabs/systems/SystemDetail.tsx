import { For, Show } from 'solid-js'
import TraceRow from '../../components/TraceRow'
import { groupLabel, shortName } from '../../lib/format'
import type { System } from '../../types/generated/System'
import type { SystemGroup } from '../../types/generated/SystemGroup'

/** Everything the console knows about one system. This is the place to grow system details. */
export default function SystemDetail(props: { system: System; group: SystemGroup }) {
  // newest first
  const recentEvents = () => [...props.system.recent_events].reverse()

  return (
    <article class="detail">
      <h2>
        {shortName(props.system.name)}
        <span class={`badge ${props.system.active ? 'ok' : 'warn'}`}>
          {props.system.active ? 'Active' : 'Inactive'}
        </span>
      </h2>
      <dl>
        <dt>Full name</dt>
        <dd class="mono">{props.system.name}</dd>
        <dt>ID</dt>
        <dd class="mono">{props.system.id}</dd>
        <dt>Group</dt>
        <dd>{groupLabel(props.group.key)}</dd>
      </dl>

      <section class="recent-events">
        <h3 class="pane-title">Recent events</h3>
        <Show when={recentEvents().length > 0} fallback={<p class="empty">This system hasn't emitted any events yet.</p>}>
          <ol>
            <For each={recentEvents()}>{(record) => <TraceRow record={record} hideSender />}</For>
          </ol>
        </Show>
      </section>
    </article>
  )
}
