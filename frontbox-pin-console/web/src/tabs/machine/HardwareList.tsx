import { ToggleGroup } from '@ark-ui/solid/toggle-group'
import { type Accessor, createMemo, createSignal, For, Show } from 'solid-js'
import { machine } from '../../state/console'
import { defaultHardwareKinds, type HardwareKind, hardwareKinds, type HardwareRow, hardwareRows } from './hardware'

export default function HardwareList() {
  const [kinds, setKinds] = createSignal<HardwareKind[]>(defaultHardwareKinds)
  const visibleKinds = () => hardwareKinds.filter((info) => kinds().includes(info.kind))

  // rebuilt only when the hardware definition changes; live state is read inside each row
  const rowsByKind = Object.fromEntries(
    hardwareKinds.map((info) => [info.kind, createMemo(() => hardwareRows(machine, info.kind))]),
  ) as Record<HardwareKind, Accessor<HardwareRow[]>>

  return (
    <div class="hardware-list pane">
      <ToggleGroup.Root multiple value={kinds()} onValueChange={(details) => setKinds(details.value as HardwareKind[])}>
        <For each={hardwareKinds}>
          {(info) => (
            <ToggleGroup.Item value={info.kind}>
              {info.label}
              <span class="count">{rowsByKind[info.kind]().length}</span>
            </ToggleGroup.Item>
          )}
        </For>
      </ToggleGroup.Root>

      <Show when={machine.hardware} fallback={<p class="empty">Waiting for the machine to report its hardware…</p>}>
        <For each={visibleKinds()} fallback={<p class="empty">Select a hardware type above.</p>}>
          {(info) => {
            const rows = rowsByKind[info.kind]
            return (
              <section>
                <h2 class="pane-title">{info.label}</h2>
                <Show when={rows().length > 0} fallback={<p class="empty">{info.empty}</p>}>
                  <table>
                    <tbody>
                      <For each={rows()}>
                        {(row) => (
                          <tr classList={{ active: row.active() }}>
                            <td class="name">{row.name}</td>
                            <td class="tags">
                              <For each={row.tags}>{(tag) => <span class="badge">{tag}</span>}</For>
                            </td>
                            <td class="address mono">{row.address}</td>
                            <td class="state">
                              <Show when={row.state()}>
                                <span class="dot" />
                                {row.state()}
                              </Show>
                            </td>
                          </tr>
                        )}
                      </For>
                    </tbody>
                  </table>
                </Show>
              </section>
            )
          }}
        </For>
      </Show>
    </div>
  )
}
