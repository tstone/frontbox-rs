import { Checkbox } from '@ark-ui/solid/checkbox'
import { Popover } from '@ark-ui/solid/popover'
import { SegmentGroup } from '@ark-ui/solid/segment-group'
import { createMemo, createSignal, For, Index, Show } from 'solid-js'
import { Portal } from 'solid-js/web'
import { ledIndex, ledName, ledStrip } from '../../lib/format'
import { machine } from '../../state/console'
import type { Light } from '../../types/generated/Light'
import type { LightQ } from '../../types/generated/LightQ'
import { byName } from '../machine/hardware'
import { moved, removed } from './sceneModel'

const PICK_KINDS = ['Every', 'By name', 'By tag'] as const

/** What's been picked each way is held while another way is in use, to come back to */
export type LightPick = { kind: (typeof PICK_KINDS)[number]; names: string[]; tags: string[] }

export const EVERY_LIGHT: LightPick = { kind: 'Every', names: [], tags: [] }

/** Having nothing chosen is every light */
export function pickedQuery(pick: LightPick): LightQ {
  if (pick.kind === 'By name' && pick.names.length > 0) return { Names: pick.names }
  if (pick.kind === 'By tag' && pick.tags.length === 1) return { Tag: pick.tags[0] }
  if (pick.kind === 'By tag' && pick.tags.length > 1) return { Or: pick.tags.map((tag) => ({ Tag: tag })) }
  return 'Every'
}

/** The tags of a query that's only tags */
function queryTags(query: LightQ): string[] | null {
  if (query !== 'Every' && 'Tag' in query) return [query.Tag]
  if (query === 'Every' || !('Or' in query)) return null
  const tags = query.Or.map(queryTags)
  return tags.every((tag) => tag !== null) ? tags.flat() : null
}

/** The names of a query that's only names */
function queryNames(query: LightQ): string[] | null {
  if (query === 'Every') return null
  if ('Name' in query) return [query.Name]
  return 'Names' in query ? query.Names : null
}

/** Whether a query is one this can show. A file can say more, such as reversing or skipping lights. */
export const canPick = (query: LightQ) => query === 'Every' || queryNames(query) !== null || queryTags(query) !== null

/** A query that this can't show starts over from every light */
export function lightPick(query: LightQ): LightPick {
  const [names, tags] = [queryNames(query), queryTags(query)]
  if (names) return { ...EVERY_LIGHT, kind: 'By name', names }
  if (tags) return { ...EVERY_LIGHT, kind: 'By tag', tags }
  return EVERY_LIGHT
}

export function lightsLabel(query: LightQ): string {
  if (query === 'Every') return 'Every light'
  const [names, tags] = [queryNames(query), queryTags(query)]
  if (tags) return tags.join(', ')
  if (names) return names.length === 1 ? ledName(names[0]) : `${names.length} lights`
  return 'A custom query'
}

/**
 * A button that opens a choice of the machine's lights: every one, those of some tags, or a search of them to choose from,
 * a strip at a time or a light at a time. Names and tags stay in the order they're ticked, a strip from its first light to
 * its last, and that order is shown beneath them to change, as it's the order a sequence runs along them.
 */
export default function LightPicker(props: { pick: LightPick; onChange: (pick: LightPick) => void }) {
  const [filter, setFilter] = createSignal('')
  const search = () => filter().trim().toLowerCase()
  const matches = (name: string) => ledName(name).toLowerCase().includes(search())

  const change = (pick: Partial<LightPick>) => props.onChange({ ...props.pick, ...pick })

  const lights = createMemo(() => Object.values(machine.hardware?.lights.by_name ?? {}))
  const rows = createMemo(() => lightRows(lights().map((light) => light.name)))
  const tags = createMemo(() => lightTags(lights()))
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
  const order = createMemo(() => (byTag() ? props.pick.tags.map((tag) => ({ label: tag, names: [tag] })) : lightRuns(props.pick.names, rows())))
  const setOrder = (next: LightRow[]) => {
    const names = next.flatMap((row) => row.names)
    change(byTag() ? { tags: names } : { names })
  }
  const noMatch = (none: string) => <p class="scene-note">{search() === '' ? none : `Nothing matches "${filter().trim()}"`}</p>

  return (
    <Popover.Root lazyMount unmountOnExit positioning={{ placement: 'bottom-start' }}>
      <Popover.Trigger class="scene-light-button" classList={{ chosen: query() !== 'Every' }} aria-label="Choose lights" title="Choose lights">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2 3h12L9.5 8.5v5l-3-1.5V8.5z" />
        </svg>
      </Popover.Trigger>
      <Portal>
        <Popover.Positioner>
          <Popover.Content class="scene-light-picker">
            <SegmentGroup.Root
              class="scene-prop-kind"
              aria-label="Choose lights by"
              value={props.pick.kind}
              onValueChange={(details) => {
                setFilter('')
                change({ kind: details.value as LightPick['kind'] })
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
                aria-label={byTag() ? 'Search light tags' : 'Search lights by name'}
                autocomplete="off"
                spellcheck={false}
                value={filter()}
                onInput={(e) => setFilter(e.currentTarget.value)}
              />
              <div class="scene-light-list">
                <Show
                  when={byTag()}
                  fallback={
                    <For each={rows().filter((row) => row.names.some(matches))} fallback={noMatch('The machine has no lights')}>
                      {(row) => <Row row={row} chosen={chosen()} matches={matches} searching={search() !== ''} onChoose={choose} />}
                    </For>
                  }
                >
                  <For each={tags().filter((tag) => tag.name.toLowerCase().includes(search()))} fallback={noMatch('No light has a tag')}>
                    {(tag) => (
                      <div class="scene-light-row">
                        <LightCheck
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
              <h3 class="scene-light-order-title">Runs in this order</h3>
              <ol class="scene-light-order">
                <Index each={order()} fallback={<p class="scene-note">{byTag() ? 'Tick tags to put them in order' : 'Tick lights to put them in order'}</p>}>
                  {(row, index) => (
                    <li>
                      <span class="scene-light-place">{index + 1}</span>
                      <span class="scene-light-order-label">{row().label}</span>
                      <Show when={row().names.length > 1}>
                        <span class="scene-light-count">{row().names.length}</span>
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
              <span class="scene-note">{lightsLabel(query())}</span>
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

/** A light, or a strip that opens out to its lights. A search for particular lights of a strip opens it to those. */
function Row(props: {
  row: LightRow
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
      <div class="scene-light-row">
        <LightCheck
          label={props.row.label}
          count={isStrip() ? props.row.names.length : undefined}
          checked={checked()}
          onChange={(on) => props.onChoose(props.row.names, on)}
        />
        <Show when={isStrip()}>
          <button
            type="button"
            class="scene-light-expand"
            aria-label={`lights of ${props.row.label}`}
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
        <div class="scene-light-children">
          <For each={shown()}>
            {(name) => (
              <div class="scene-light-row">
                <LightCheck label={ledName(name)} checked={props.chosen.has(name)} onChange={(on) => props.onChoose([name], on)} />
              </div>
            )}
          </For>
        </div>
      </Show>
    </>
  )
}

function LightCheck(props: { label: string; count?: number; checked: boolean | 'indeterminate'; onChange: (on: boolean) => void }) {
  return (
    <Checkbox.Root checked={props.checked} onCheckedChange={(details) => props.onChange(details.checked === true)}>
      <Checkbox.Control>
        <Checkbox.Indicator>✓</Checkbox.Indicator>
        <Checkbox.Indicator indeterminate>–</Checkbox.Indicator>
      </Checkbox.Control>
      <Checkbox.Label>{props.label}</Checkbox.Label>
      <Show when={props.count !== undefined}>
        <span class="scene-light-count">{props.count}</span>
      </Show>
      <Checkbox.HiddenInput />
    </Checkbox.Root>
  )
}

/** The lights of a definition in order, which reach the console with only their names to tie them together. One is a light on its own. */
type LightRow = { label: string; names: string[] }

function lightRows(names: string[]): LightRow[] {
  const rows = new Map<string, string[]>()
  for (const name of names.toSorted((a, b) => byName.compare(ledName(a), ledName(b)))) {
    const label = ledStrip(name) ?? name
    rows.set(label, [...(rows.get(label) ?? []), name])
  }
  return [...rows].map(([label, names]) => ({ label, names }))
}

/** The chosen lights in order, with those that follow on along a strip taken together, as a whole strip is many names */
function lightRuns(names: string[], rows: LightRow[]): LightRow[] {
  const runs: string[][] = []
  for (const name of names) {
    const run = runs.at(-1)
    if (run && followsOn(run.at(-1)!, name)) run.push(name)
    else runs.push([name])
  }
  return runs.map((names) => ({ label: runLabel(names, rows), names }))
}

function followsOn(light: string, next: string): boolean {
  const index = ledIndex(light)
  return index !== null && ledStrip(light) === ledStrip(next) && ledIndex(next) === index + 1
}

function runLabel(names: string[], rows: LightRow[]): string {
  const strip = ledStrip(names[0]) ?? names[0]
  if (rows.find((row) => row.label === strip)?.names.length === names.length) return strip
  return names.length === 1 ? ledName(names[0]) : `${strip}[${ledIndex(names[0])}..${ledIndex(names.at(-1)!)}]`
}

/** Every tag on the machine's lights, with how many have it */
function lightTags(lights: Light[]): { name: string; count: number }[] {
  const counts = new Map<string, number>()
  for (const name of lights.flatMap((light) => light.tags)) counts.set(name, (counts.get(name) ?? 0) + 1)
  return [...counts].map(([name, count]) => ({ name, count })).toSorted((a, b) => byName.compare(a.name, b.name))
}
