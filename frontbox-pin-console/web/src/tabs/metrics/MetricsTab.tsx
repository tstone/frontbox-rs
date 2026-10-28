import { createEffect, createMemo, For, Match, Show, Switch } from 'solid-js'
import { machine } from '../../state/console'
import { metricTotals, queryMetrics } from '../../state/metricsQuery'
import type { CounterTotal } from '../../types/generated/CounterTotal'
import './MetricsTab.css'

const UNCATEGORIZED = 'Other'

export default function MetricsTab() {
  // the reply goes back on the socket that asked, so ask again after reconnecting
  createEffect(() => {
    if (machine.connection === 'connected') queryMetrics()
  })

  const groups = createMemo(() => {
    const byCategory = new Map<string, CounterTotal[]>()
    for (const counter of metricTotals()?.counters ?? []) {
      const category = counter.category ?? UNCATEGORIZED
      byCategory.set(category, [...(byCategory.get(category) ?? []), counter])
    }
    return [...byCategory.entries()].sort(([a], [b]) => a.localeCompare(b))
  })

  return (
    <div class="metrics-tab pane">
      <header>
        <p class="hint">
          All-time totals from the metrics database. Recordings reach it within 30 seconds, and when a turn or game ends.
        </p>
        <button type="button" onClick={queryMetrics}>
          Refresh
        </button>
      </header>
      <Switch>
        <Match when={!metricTotals()}>
          <p class="empty">Loading…</p>
        </Match>
        <Match when={!metricTotals()!.stored}>
          <p class="empty">
            The machine keeps no metrics database. Set <code>metrics_path</code> in its boot config to keep history.
          </p>
        </Match>
        <Match when={groups().length === 0}>
          <p class="empty">Nothing counted yet.</p>
        </Match>
        <Match when={true}>
          <For each={groups()}>
            {([category, counters]) => (
              <section>
                <h3 class="pane-title">{category}</h3>
                <table>
                  <tbody>
                    <For each={counters}>{(counter) => <CounterRow counter={counter} />}</For>
                  </tbody>
                </table>
              </section>
            )}
          </For>
        </Match>
      </Switch>
    </div>
  )
}

function CounterRow(props: { counter: CounterTotal }) {
  return (
    <tr>
      <td>
        <span class="label">{props.counter.label}</span>
        <Show when={props.counter.metric !== props.counter.label}>
          <span class="key mono">{props.counter.metric}</span>
        </Show>
        <Show when={props.counter.desc}>
          <p class="desc">{props.counter.desc}</p>
        </Show>
      </td>
      <td class="total mono">{props.counter.total.toLocaleString()}</td>
    </tr>
  )
}
