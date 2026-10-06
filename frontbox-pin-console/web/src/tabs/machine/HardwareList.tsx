import { Accordion } from '@ark-ui/solid/accordion'
import { ToggleGroup } from '@ark-ui/solid/toggle-group'
import { type Accessor, createMemo, createSignal, For, Show } from 'solid-js'
import InfoTip from '../../components/InfoTip'
import Tip from '../../components/Tip'
import { machine } from '../../state/console'
import { cancelPlacing, movedPositions, placing, resetPosition, startPlacing } from '../../state/placement'
import { defaultHardwareKinds, type HardwareKind, hardwareKinds, type HardwareRow, hardwareRows } from './hardware'
import HardwareDetail from './HardwareDetail'

export default function HardwareList() {
  const [kinds, setKinds] = createSignal<HardwareKind[]>(defaultHardwareKinds)
  const [filter, setFilter] = createSignal('')
  const query = () => filter().trim().toLowerCase()

  // rebuilt only when the hardware definition changes; live state is read inside each row
  const rowsByKind = Object.fromEntries(
    hardwareKinds.map((info) => [info.kind, createMemo(() => hardwareRows(machine, info.kind))]),
  ) as Record<HardwareKind, Accessor<HardwareRow[]>>

  // Filtering keeps the same row objects, so matching rows keep their open/closed state as the filter changes
  const matchesByKind = Object.fromEntries(
    hardwareKinds.map((info) => [
      info.kind,
      createMemo(() => {
        const q = query()
        const rows = rowsByKind[info.kind]()
        return q ? rows.filter((row) => row.name.toLowerCase().includes(q)) : rows
      }),
    ]),
  ) as Record<HardwareKind, Accessor<HardwareRow[]>>

  // while filtering, sections without a match are left out instead of listing nothing
  const visibleKinds = () =>
    hardwareKinds.filter((info) => kinds().includes(info.kind) && (!query() || matchesByKind[info.kind]().length > 0))

  return (
    <div class="hardware-list pane">
      <input
        id="hardware-filter"
        class="filter-input"
        type="search"
        placeholder="Filter by name"
        aria-label="Filter hardware by name"
        autocomplete="off"
        spellcheck={false}
        value={filter()}
        onInput={(e) => setFilter(e.currentTarget.value)}
      />

      <ToggleGroup.Root multiple value={kinds()} onValueChange={(details) => setKinds(details.value as HardwareKind[])}>
        <For each={hardwareKinds}>
          {(info) => (
            <ToggleGroup.Item value={info.kind}>
              {info.label}
              <span class="count">{matchesByKind[info.kind]().length}</span>
            </ToggleGroup.Item>
          )}
        </For>
      </ToggleGroup.Root>

      <Show when={machine.hardware} fallback={<p class="empty">Waiting for the machine to report its hardware…</p>}>
        <For
          each={visibleKinds()}
          fallback={<p class="empty">{query() ? `Nothing matches "${filter().trim()}".` : 'Select a hardware type above.'}</p>}
        >
          {(info) => {
            const rows = matchesByKind[info.kind]
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
        <Show when={movedPositions[row.key]}>
          <span class="badge warn">moved</span>
        </Show>
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
            <span class="dot" style={{ background: row.color?.() ?? undefined }} />
            {row.state()}
          </Show>
        </span>
      </Accordion.ItemTrigger>
      <Accordion.ItemContent>
        <HardwareDetail item={row.ref} />
        <Show when={['driver', 'switch', 'led'].includes(row.ref.kind)}>
          <PositionControl hwKey={row.key} />
        </Show>
      </Accordion.ItemContent>
    </Accordion.Item>
  )
}

function PositionControl(props: { hwKey: string }) {
  const isPlacing = () => placing() === props.hwKey
  const moved = () => movedPositions[props.hwKey] !== undefined
  return (
    <div class="position-control">
      <button type="button" class="action" onClick={() => (isPlacing() ? cancelPlacing() : startPlacing(props.hwKey))}>
        {isPlacing() ? 'Cancel' : 'Position'}
      </button>
      <Show when={moved() && !isPlacing()}>
        <button type="button" class="action" onClick={() => resetPosition(props.hwKey)}>
          Reset
        </button>
      </Show>
      <span class="note">
        {isPlacing()
          ? 'Click a plane in the 3D view to drop it'
          : moved()
            ? 'Moved here only. Right-click its dot to copy the coordinates into your code.'
            : ''}
      </span>
    </div>
  )
}
