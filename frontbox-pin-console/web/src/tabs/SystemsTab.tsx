import { For, Show } from 'solid-js'
import { machine } from '../state/console'
import './SystemsTab.css'

export default function SystemsTab() {
  return (
    <div class="systems-tab">
      <Show when={machine.groups.length > 0} fallback={<p class="empty">No system groups yet.</p>}>
        <For each={machine.groups}>
          {(group) => (
            <section class="group" classList={{ inactive: !group.active }}>
              <h2>{group.key}</h2>
              <ul>
                <For each={group.systems}>
                  {(system) => (
                    <li classList={{ inactive: !system.active }}>
                      <span class="id">#{system.id}</span> {system.name}
                    </li>
                  )}
                </For>
              </ul>
            </section>
          )}
        </For>
      </Show>
    </div>
  )
}
