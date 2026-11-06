import { Checkbox } from '@ark-ui/solid/checkbox'
import { Popover } from '@ark-ui/solid/popover'
import { SegmentGroup } from '@ark-ui/solid/segment-group'
import { createMemo, createSignal, For, Index, Show } from 'solid-js'
import { Portal } from 'solid-js/web'
import { ledIndex, ledName, ledStrip } from '../../lib/format'
import { machine } from '../../state/console'
import type { LED } from '../../types/generated/LED'
import type { SceneLedQuery } from '../../types/generated/SceneLedQuery'
import { byName } from '../machine/hardware'
import { moved, removed } from './sceneModel'

const PICK_KINDS = ['Every', 'By name', 'By tag'] as const

/** What's been picked each way is held while another way is in use, to come back to */
export type LedPick = { kind: (typeof PICK_KINDS)[number]; names: string[]; tags: string[] }

export const EVERY_LED: LedPick = { kind: 'Every', names: [], tags: [] }

/** Having nothing chosen is every LED */
export function pickedQuery(pick: LedPick): SceneLedQuery {
  if (pick.kind === 'By name' && pick.names.length > 0) return { Names: pick.names }
  if (pick.kind === 'By tag' && pick.tags.length > 0) return { Tags: pick.tags }
  return 'Every'
}

export function ledsLabel(query: SceneLedQuery): string {
  if (query === 'Every') return 'Every LED'
  if ('Tags' in query) return query.Tags.join(', ')
  return query.Names.length === 1 ? ledName(query.Names[0]) : `${query.Names.length} LEDs`
}

/**
 * A button that opens a choice of the machine's LEDs: every one, those of some tags, or a search of them to choose from,
 * a strip at a time or an LED at a time. Names and tags stay in the order they're ticked, a strip from its first LED to
 * its last, and that order is shown beneath them to change, as it's the order a sequence runs along them.
 */
export default function LedPicker(props: { pick: LedPick; onChange: (pick: LedPick) => void }) {
  const [filter, setFilter] = createSignal('')
  const search = () => filter().trim().toLowerCase()
  const matches = (name: string) => ledName(name).toLowerCase().includes(search())

  const change = (pick: Partial<LedPick>) => props.onChange({ ...props.pick, ...pick })

  const leds = createMemo(() => Object.values(machine.hardware?.leds.by_name ?? {}))
  const rows = createMemo(() => ledRows(leds().map((led) => led.name)))
  const tags = createMemo(() => ledTags(leds()))
  const chosen = createMemo(() => new Set(props.pick.names))

  const choose = (names: string[], on: boolean) => {
    const next = new Set(props.pick.names)
    for (const name of names) {
      if (on) next.add(name)
      else next.delete(name)
    }
    change({ names: [...next] })
  }

  const query = () => pickedQuery(props.pick)
  const byTag = () => props.pick.kind === 'By tag'
  const order = createMemo(() => (byTag() ? props.pick.tags.map((tag) => ({ label: tag, names: [tag] })) : ledRuns(props.pick.names, rows())))
  const setOrder = (next: LedRow[]) => {
    const names = next.flatMap((row) => row.names)
    change(byTag() ? { tags: names } : { names })
  }
  const noMatch = (none: string) => <p class="scene-note">{search() === '' ? none : `Nothing matches "${filter().trim()}"`}</p>

  return (
    <Popover.Root lazyMount unmountOnExit positioning={{ placement: 'bottom-start' }}>
      <Popover.Trigger class="scene-led-button" classList={{ chosen: query() !== 'Every' }} aria-label="Choose LEDs" title="Choose LEDs">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2 3h12L9.5 8.5v5l-3-1.5V8.5z" />
        </svg>
      </Popover.Trigger>
      <Portal>
        <Popover.Positioner>
          <Popover.Content class="scene-led-picker">
            <SegmentGroup.Root
              class="scene-prop-kind"
              aria-label="Choose LEDs by"
              value={props.pick.kind}
              onValueChange={(details) => {
                setFilter('')
                change({ kind: details.value as LedPick['kind'] })
              }}
            >
              <SegmentGroup.Indicator />
              <For each={PICK_KINDS}>
                {(kind) => (
                  <SegmentGroup.Item value={kind}>
                    <SegmentGroup.ItemText>{kind}</SegmentGroup.ItemText>
                    <SegmentGroup.ItemControl />
                    <SegmentGroup.ItemHiddenInput />
                  </SegmentGroup.Item>
                )}
              </For>
            </SegmentGroup.Root>
            <Show when={props.pick.kind !== 'Every'}>
              <input
                type="search"
                placeholder={byTag() ? 'Search by tag' : 'Search by name'}
                aria-label={byTag() ? 'Search LED tags' : 'Search LEDs by name'}
                autocomplete="off"
                spellcheck={false}
                value={filter()}
                onInput={(e) => setFilter(e.currentTarget.value)}
              />
              <div class="scene-led-list">
                <Show
                  when={byTag()}
                  fallback={
                    <For each={rows().filter((row) => row.names.some(matches))} fallback={noMatch('The machine has no LEDs')}>
                      {(row) => <Row row={row} chosen={chosen()} matches={matches} searching={search() !== ''} onChoose={choose} />}
                    </For>
                  }
                >
                  <For each={tags().filter((tag) => tag.name.toLowerCase().includes(search()))} fallback={noMatch('No LED has a tag')}>
                    {(tag) => (
                      <div class="scene-led-row">
                        <LedCheck
                          label={tag.name}
                          count={tag.count}
                          checked={props.pick.tags.includes(tag.name)}
                          onChange={(on) => change({ tags: on ? [...props.pick.tags, tag.name] : props.pick.tags.filter((name) => name !== tag.name) })}
                        />
                      </div>
                    )}
                  </For>
                </Show>
              </div>
              <h3 class="scene-led-order-title">Runs in this order</h3>
              <ol class="scene-led-order">
                <Index each={order()} fallback={<p class="scene-note">{byTag() ? 'Tick tags to put them in order' : 'Tick LEDs to put them in order'}</p>}>
                  {(row, index) => (
                    <li>
                      <span class="scene-led-place">{index + 1}</span>
                      <span class="scene-led-order-label">{row().label}</span>
                      <Show when={row().names.length > 1}>
                        <span class="scene-led-count">{row().names.length}</span>
                      </Show>
                      <button type="button" aria-label={`Move ${row().label} up`} disabled={index === 0} onClick={() => setOrder(moved(order(), index, -1))}>
                        ↑
                      </button>
                      <button
                        type="button"
                        aria-label={`Move ${row().label} down`}
                        disabled={index === order().length - 1}
                        onClick={() => setOrder(moved(order(), index, 1))}
                      >
                        ↓
                      </button>
                      <button type="button" aria-label={`Remove ${row().label}`} onClick={() => setOrder(removed(order(), index))}>
                        ×
                      </button>
                    </li>
                  )}
                </Index>
              </ol>
            </Show>
            <footer>
              <span class="scene-note">{ledsLabel(query())}</span>
              <Show when={query() !== 'Every'}>
                <button type="button" onClick={() => change(byTag() ? { tags: [] } : { names: [] })}>
                  Clear
                </button>
              </Show>
            </footer>
          </Popover.Content>
        </Popover.Positioner>
      </Portal>
    </Popover.Root>
  )
}

/** An LED, or a strip that opens out to its LEDs. A search for particular LEDs of a strip opens it to those. */
function Row(props: {
  row: LedRow
  chosen: Set<string>
  matches: (name: string) => boolean
  searching: boolean
  onChoose: (names: string[], on: boolean) => void
}) {
  const [open, setOpen] = createSignal(false)
  const isStrip = () => props.row.names.length > 1
  const allMatch = () => props.row.names.every(props.matches)
  const expanded = () => isStrip() && (open() || (props.searching && !allMatch()))
  const shown = () => (open() ? props.row.names : props.row.names.filter(props.matches))

  const count = () => props.row.names.filter((name) => props.chosen.has(name)).length
  const checked = () => (count() === 0 ? false : count() === props.row.names.length ? true : 'indeterminate')

  return (
    <>
      <div class="scene-led-row">
        <LedCheck
          label={props.row.label}
          count={isStrip() ? props.row.names.length : undefined}
          checked={checked()}
          onChange={(on) => props.onChoose(props.row.names, on)}
        />
        <Show when={isStrip()}>
          <button
            type="button"
            class="scene-led-expand"
            aria-label={`LEDs of ${props.row.label}`}
            aria-expanded={expanded()}
            onClick={() => setOpen(!expanded())}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M6 4l4 4-4 4" />
            </svg>
          </button>
        </Show>
      </div>
      <Show when={expanded()}>
        <div class="scene-led-children">
          <For each={shown()}>
            {(name) => (
              <div class="scene-led-row">
                <LedCheck label={ledName(name)} checked={props.chosen.has(name)} onChange={(on) => props.onChoose([name], on)} />
              </div>
            )}
          </For>
        </div>
      </Show>
    </>
  )
}

function LedCheck(props: { label: string; count?: number; checked: boolean | 'indeterminate'; onChange: (on: boolean) => void }) {
  return (
    <Checkbox.Root checked={props.checked} onCheckedChange={(details) => props.onChange(details.checked === true)}>
      <Checkbox.Control>
        <Checkbox.Indicator>✓</Checkbox.Indicator>
        <Checkbox.Indicator indeterminate>–</Checkbox.Indicator>
      </Checkbox.Control>
      <Checkbox.Label>{props.label}</Checkbox.Label>
      <Show when={props.count !== undefined}>
        <span class="scene-led-count">{props.count}</span>
      </Show>
      <Checkbox.HiddenInput />
    </Checkbox.Root>
  )
}

/** The LEDs of a definition in order, which reach the console with only their names to tie them together. One is an LED on its own. */
type LedRow = { label: string; names: string[] }

function ledRows(names: string[]): LedRow[] {
  const rows = new Map<string, string[]>()
  for (const name of names.toSorted((a, b) => byName.compare(ledName(a), ledName(b)))) {
    const label = ledStrip(name) ?? name
    rows.set(label, [...(rows.get(label) ?? []), name])
  }
  return [...rows].map(([label, names]) => ({ label, names }))
}

/** The chosen LEDs in order, with those that follow on along a strip taken together, as a whole strip is many names */
function ledRuns(names: string[], rows: LedRow[]): LedRow[] {
  const runs: string[][] = []
  for (const name of names) {
    const run = runs.at(-1)
    if (run && followsOn(run.at(-1)!, name)) run.push(name)
    else runs.push([name])
  }
  return runs.map((names) => ({ label: runLabel(names, rows), names }))
}

function followsOn(led: string, next: string): boolean {
  const index = ledIndex(led)
  return index !== null && ledStrip(led) === ledStrip(next) && ledIndex(next) === index + 1
}

function runLabel(names: string[], rows: LedRow[]): string {
  const strip = ledStrip(names[0]) ?? names[0]
  if (rows.find((row) => row.label === strip)?.names.length === names.length) return strip
  return names.length === 1 ? ledName(names[0]) : `${strip}[${ledIndex(names[0])}..${ledIndex(names.at(-1)!)}]`
}

/** Every tag on the machine's LEDs, with how many have it */
function ledTags(leds: LED[]): { name: string; count: number }[] {
  const counts = new Map<string, number>()
  for (const name of leds.flatMap((led) => led.tags)) counts.set(name, (counts.get(name) ?? 0) + 1)
  return [...counts].map(([name, count]) => ({ name, count })).toSorted((a, b) => byName.compare(a.name, b.name))
}
