import { Accordion } from '@ark-ui/solid/accordion'
import { ToggleGroup } from '@ark-ui/solid/toggle-group'
import { type Accessor, createMemo, createSignal, For, Show } from 'solid-js'
import InfoTip from '../../components/InfoTip'
import Tip from '../../components/Tip'
import { machine } from '../../state/console'
import { defaultHardwareKinds, type HardwareKind, hardwareKinds, type HardwareRow, hardwareRows } from './hardware'
import HardwareDetail from './HardwareDetail'

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
                <h2 class="pane-title">
                  {info.label}
                  <Show when={info.note}>{(note) => <InfoTip text={note()} />}</Show>
                </h2>
                <Show when={rows().length > 0} fallback={<p class="empty">{info.empty}</p>}>
                  {/* details only render while open */}
                  <Accordion.Root multiple collapsible lazyMount unmountOnExit>
                    <For each={rows()}>{(row) => <HardwareItem row={row} />}</For>
                  </Accordion.Root>
                </Show>
              </section>
            )
          }}
        </For>
      </Show>
    </div>
  )
}

function HardwareItem(props: { row: HardwareRow }) {
  const row = props.row
  return (
    <Accordion.Item value={row.key} classList={{ active: row.active() }}>
      <Accordion.ItemTrigger>
        <Accordion.ItemIndicator>›</Accordion.ItemIndicator>
        <span class="name">{row.name}</span>
        <span class="tags">
          <For each={row.tags}>{(tag) => <span class="badge">{tag}</span>}</For>
        </span>
        <span class="address mono">
          <Show when={row.addressTip} fallback={row.address}>
            {(tip) => <Tip text={tip()}>{row.address}</Tip>}
          </Show>
        </span>
        <span class="state">
          <Show when={row.state()} fallback={<span class="detail">{row.detail}</span>}>
            <span class="dot" />
            {row.state()}
          </Show>
        </span>
      </Accordion.ItemTrigger>
      <Accordion.ItemContent>
        <HardwareDetail item={row.ref} />
      </Accordion.ItemContent>
    </Accordion.Item>
  )
}
