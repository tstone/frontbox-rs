import { For, Show } from 'solid-js'
import { machine } from '../state/console'
import type { TraceRecord } from '../types/generated/TraceRecord'
import './LogTab.css'

export default function LogTab() {
  // newest first
  const records = () => [...machine.log].reverse()

  return (
    <div class="log-tab">
      <Show when={machine.log.length > 0} fallback={<p class="empty">Nothing traced yet.</p>}>
        <table>
          <tbody>
            <For each={records()}>
              {(record) => (
                <tr>
                  <td class="time">{formatTime(record.at_ms)}</td>
                  <td class="kind">{kind(record)}</td>
                  <td class="detail">{detail(record)}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>
    </div>
  )
}

function kind(record: TraceRecord): string {
  return Object.keys(record.event)[0]
}

function detail(record: TraceRecord): string {
  const event = record.event
  if ('Event' in event) {
    const body = event.Event.event
    return body == null ? event.Event.type_name : `${event.Event.type_name} ${JSON.stringify(body)}`
  }
  return JSON.stringify(Object.values(event)[0])
}

function formatTime(ms: number): string {
  return (ms / 1000).toFixed(3) + 's'
}
