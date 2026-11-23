import { Popover } from '@ark-ui/solid/popover'
import { batch, createEffect, createMemo, createSignal, For, Index, Match, on, onCleanup, onMount, Show, Switch, untrack, useContext } from 'solid-js'
import { Dynamic } from 'solid-js/web'
import * as icons from '../../components/icons'
import ScenePortal, { LayerColorContext, layerColorClass, type LayerColor } from './ScenePortal'
import type { Cycle } from '../../types/generated/Cycle'
import type { Key } from '../../types/generated/Key'
import type { Modulated } from '../../types/generated/Modulated'
import type { Modulator } from '../../types/generated/Modulator'
import type { Pick as RandomPick } from '../../types/generated/Pick'
import type { Prop } from '../../types/generated/Prop'
import type { RenderedGraph } from '../../types/generated/RenderedGraph'
import type { Timing } from '../../types/generated/Timing'
import type { Traversal } from '../../types/generated/Traversal'
import type { Playback } from './playback'
import { is, type ConfigNode, type ListPlace } from './configNodes'
import { ConfigTreeContext, hasTrack, shownInTree, treeChildren, type ConfigTree, type TreeChild } from './configTree'
import { AddMenu, NodeIcon, NodeSummary } from './PropertiesPanel'
import { LayerMenuContext } from './LayerMenu'
import { CurveField, Dropdown, EyeButton, FieldRow, LayerSurfacesContext, NumberBox, NumberField, SceneContext, ValueField } from './SceneFields'
import { contentSoundLength } from '../../state/contentSounds'
import type { SoundLayer } from '../../types/generated/SoundLayer'
import { curveHeight, field, isHidden, item, moveMoment, removed, repeats, sceneTracks, soundEnd, soundNaturalEnd, sceneTraversals, snap, startingValue, TIME_STEP, trackTimes, traversalLength, UNIT, variant, envelopeHeight, wavePosition, withKey, withKeyCopy, withPick, withHidden, withTimesInLength, withTimesScaled, type Lens, type Ranges, type Scene, type SceneLayer, type SceneValue } from './sceneModel'

/** How close in pixels a click can be to a key before it's taken to mean that key */
const KEY_REACH = 12

/** How far in pixels a press on a dot has to move before it's a drag and not a click */
const DRAG_START = 3

/** How many pixels either side of the playhead a press in the lanes can take hold of it */
const PLAYHEAD_REACH = 5

/** What in the lanes has a drag of its own, which a press on keeps from the playhead */
const OWN_DRAGS = '.scene-key, .scene-wave, .scene-sweep, .scene-sound-bar, .scene-timeline-ruler, .scene-layers-name, input, textarea'

/** How far along the view a copy of a key goes from it, given the room */
const COPY_DISTANCE = 0.05

/** How many milliseconds the pointer rests on the button for the time scale menu to open */
const SCALE_MENU_OPEN_DELAY = 400

/** How many milliseconds the time scale menu stays open once the pointer has left it */
const SCALE_MENU_CLOSE_DELAY = 1000

/** The narrowest the tree beside the lanes can be dragged to, in pixels, and the most of the timeline's width it can take */
const TREE_MIN_WIDTH = 140
const TREE_MAX_SHARE = 0.6

type TimeScale = { label: string; by: number }

const SHORTER: TimeScale[] = [
  { label: '÷ 4', by: 1 / 4 },
  { label: '÷ 2', by: 1 / 2 },
  { label: '÷ 1.5', by: 1 / 1.5 },
]

const LONGER: TimeScale[] = [
  { label: '× 1.5', by: 1.5 },
  { label: '× 2', by: 2 },
  { label: '× 4', by: 4 },
]

const TICK_SPACINGS = [0.05, 0.1, 0.25, 0.5, 1, 2, 5, 10, 30, 60, 120, 300]
const MOST_TICKS = 10

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max)

/** Where a pointer is along an element, from 0 to 1 */
function along(e: PointerEvent, element: HTMLElement) {
  const { left, width } = element.getBoundingClientRect()
  return clamp((e.clientX - left) / width, 0, 1)
}

/** How wide the tree is, in pixels, as it was last dragged to */
const TREE_WIDTH_KEY = 'frontbox-console.scene-tree-width'
function readTreeWidth(): number | null {
  try {
    const width = Number(localStorage.getItem(TREE_WIDTH_KEY))
    return width > 0 ? width : null
  } catch {
    return null
  }
}
function writeTreeWidth(width: number) {
  try {
    localStorage.setItem(TREE_WIDTH_KEY, String(Math.round(width)))
  } catch {
    // storage can be unavailable (private windows); the width still holds for this session
  }
}

/**
 * The scene's layers as a tree, each node on one row with its time along the rest of it: a keyframed prop's keys, a
 * sound's bar, or a traversal's. A closed node shows when the props under it change. Selecting a row selects the node
 * for the properties panel. `graphs` are the layers' graphs as last drawn, which measure how long a traversal with a
 * rate takes. A preview scene has a row of its own at the top, which can't be opened or selected.
 */
export default function Timeline(props: { playback: Playback; rendering: boolean; error: string | null; graphs: (RenderedGraph | null)[]; preview: PreviewLayer }) {
  const scene = useContext(SceneContext)!
  const surfaces = useContext(LayerSurfacesContext)!
  const tree = useContext(ConfigTreeContext)!
  const timing = () => scene.get().timing
  const tracks = createMemo(() => sceneTracks(scene, surfaces))
  const traversals = createMemo(() => sceneTraversals(scene))
  const groups = createMemo(() => layerGroups(tree))
  const rows = createMemo(() => treeRows(tree, groups()))
  const nodeRows = () => rows().filter((row): row is TreeRowData => 'node' in row)
  const [treeWidth, setTreeWidth] = createSignal(readTreeWidth())
  /** The tracks whose lanes are kept short, by node. The rest that can be are made taller to show their values over time. */
  const [collapsed, setCollapsed] = createSignal<ReadonlySet<string>>(new Set())
  let lanes!: HTMLDivElement

  const measured = (traversal: Traversal, layer: number, index: number) => traversalLength(traversal, scene.get(), props.graphs[layer]?.lengths[index] ?? null)

  /** How long a traversal's node takes, which its id says the layer and place of */
  const lengthOf = (node: Extract<ConfigNode, { kind: 'traversal' }>) => {
    const [, layer, index] = node.id.match(/^layers\/(\d+)\/.*\/(\d+)$/) ?? []
    return layer === undefined ? null : measured(node.lens.get(), Number(layer), Number(index))
  }

  /** How much time the view shows. A key, a traversal or a sound past the end is still there to be found and moved. */
  const span = createMemo(() =>
    Math.max(
      timing()?.duration ?? 1,
      ...tracks().map(({ prop }) => trackTimes(prop.get()).at(-1) ?? 0),
      ...traversals().map(({ traversal, layer, index }) => traversal.get().start + (measured(traversal.get(), layer, index) ?? 0)),
      ...scene.get().sounds.map((layer) => soundEnd(layer, contentSoundLength(layer.sound)) ?? layer.start),
    ),
  )

  const percent = (seconds: number) => `${(seconds / span()) * 100}%`

  const ticks = () => {
    const spacing = TICK_SPACINGS.find((candidate) => span() / candidate <= MOST_TICKS) ?? span()
    return Array.from({ length: Math.floor(span() / spacing) + 1 }, (_, index) => index * spacing)
  }

  let ruler: HTMLDivElement | undefined
  /** Moves the playhead to the time under the pointer, measured along the ruler */
  const scrubTo = (e: PointerEvent) => {
    if (ruler) props.playback.scrub(Math.min(snap(along(e, ruler) * span()), timing()!.duration))
  }
  /** A press on the ruler drags the playhead */
  const scrubbing = {
    onPointerDown(e: PointerEvent & { currentTarget: HTMLElement }) {
      e.currentTarget.setPointerCapture(e.pointerId)
      scrubTo(e)
    },
    onPointerMove(e: PointerEvent & { currentTarget: HTMLElement }) {
      if (e.currentTarget.hasPointerCapture(e.pointerId)) scrubTo(e)
    },
  }

  /**
   * A press along the playhead's length, in the lanes, that might become a drag of it. It's left to go on to what's under
   * it, so one let go without moving is still a click there, such as adding a key, and one that moves drags the playhead.
   */
  let playheadPress: { x: number; pointer: number; dragging: boolean } | null = null
  const nearPlayhead = (e: PointerEvent) => {
    if (!ruler || !timing()) return false
    const { left, width } = ruler.getBoundingClientRect()
    return Math.abs(e.clientX - (left + (props.playback.position() / span()) * width)) <= PLAYHEAD_REACH
  }
  onMount(() => {
    const press = (e: PointerEvent) => {
      if (e.button !== 0 || !nearPlayhead(e) || (e.target as Element).closest(OWN_DRAGS)) return
      playheadPress = { x: e.clientX, pointer: e.pointerId, dragging: false }
    }
    const move = (e: PointerEvent) => {
      if (playheadPress?.pointer !== e.pointerId) return
      if (!playheadPress.dragging) {
        if (Math.abs(e.clientX - playheadPress.x) < DRAG_START) return
        // from here on the lanes have the pointer, so its release isn't a click on what it was pressed on
        playheadPress.dragging = true
        lanes.setPointerCapture(e.pointerId)
      }
      e.stopPropagation()
      scrubTo(e)
    }
    const release = (e: PointerEvent) => {
      if (playheadPress?.pointer !== e.pointerId) return
      if (playheadPress.dragging) e.stopPropagation()
      playheadPress = null
    }
    lanes.addEventListener('pointerdown', press, { capture: true })
    lanes.addEventListener('pointermove', move, { capture: true })
    lanes.addEventListener('pointerup', release, { capture: true })
    lanes.addEventListener('pointercancel', release, { capture: true })
    onCleanup(() => {
      lanes.removeEventListener('pointerdown', press, { capture: true })
      lanes.removeEventListener('pointermove', move, { capture: true })
      lanes.removeEventListener('pointerup', release, { capture: true })
      lanes.removeEventListener('pointercancel', release, { capture: true })
    })
  })

  const rowElement = (id: string) => lanes.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`)

  // what's selected, here or by a click in the preview, is scrolled to once its row is up, opening its group if it's closed
  createEffect(
    on(tree.selected, (id) => {
      if (id === null) return
      const group = untrack(groups).find(({ nodes }) => nodes.some((node) => id === node.id || id.startsWith(`${node.id}/`)))
      if (group) tree.setGroupOpen(group.id, true)
      requestAnimationFrame(() => rowElement(id)?.scrollIntoView({ block: 'nearest' }))
    }),
  )

  /** Up and down go between rows, right opens a node or goes into it, and left closes it or goes to what it's in */
  const navigate = (e: KeyboardEvent) => {
    if ((e.target as HTMLElement).getAttribute('role') !== 'treeitem') return
    const list = nodeRows()
    const index = list.findIndex(({ node }) => node.id === tree.selected())
    const row = list[index]
    if (!row) return
    let next: string | undefined
    if (e.key === 'ArrowDown') next = list[index + 1]?.node.id
    else if (e.key === 'ArrowUp') next = list[index - 1]?.node.id
    else if (e.key === 'Home') next = list[0]?.node.id
    else if (e.key === 'End') next = list.at(-1)?.node.id
    else if (e.key === 'ArrowRight') {
      if (row.branch && !row.open) tree.toggle(row.node.id)
      else if (row.open) next = list[index + 1]?.node.id
    } else if (e.key === 'ArrowLeft') {
      if (row.open) tree.toggle(row.node.id)
      else next = list.slice(0, index).findLast(({ depth }) => depth < row.depth)?.node.id
    } else if (e.key === 'Enter' && row.branch) tree.toggle(row.node.id)
    else return
    e.preventDefault()
    if (next === undefined) return
    const id = next
    tree.select(id)
    requestAnimationFrame(() => rowElement(id)?.focus({ preventScroll: true }))
  }

  /** Dragging the edge of the tree, which is kept for next time once it's let go */
  const resize = {
    onPointerDown(e: PointerEvent & { currentTarget: HTMLElement }) {
      e.currentTarget.setPointerCapture(e.pointerId)
    },
    onPointerMove(e: PointerEvent & { currentTarget: HTMLElement }) {
      if (!e.currentTarget.hasPointerCapture(e.pointerId)) return
      const { left, width } = lanes.getBoundingClientRect()
      setTreeWidth(clamp(e.clientX - left, TREE_MIN_WIDTH, width * TREE_MAX_SHARE))
    },
    onPointerUp() {
      const width = treeWidth()
      if (width !== null) writeTreeWidth(width)
    },
  }

  return (
    <section class="scene-timeline" aria-label="Layers and timeline" style={{ '--label-width': treeWidth() === null ? undefined : `${treeWidth()}px` }}>
      {/* between the tree and the lanes, from the heading over the tree down */}
      <span class="scene-layers-resize" role="separator" aria-orientation="vertical" aria-label="Resize the layers" {...resize} />
      {/* over the tree, what it shows; over the lanes, playing them and how long they are */}
      <header>
        <div class="scene-layers-header">
          <span class="scene-layers-title">Layers</span>
          <AddMenu
            label="Preview another scene with this one"
            class="scene-icon-button"
            groups={[props.preview.choices.map((name) => ({ value: name, label: name }))]}
            onPick={props.preview.set}
          >
            <icons.Clapperboard aria-hidden="true" />
          </AddMenu>
          <button type="button" class="scene-icon-button" aria-label="Expand all" title="Expand all" disabled={tree.allExpanded()} onClick={tree.expandAll}>
            <icons.ChevronsUpDown aria-hidden="true" />
          </button>
          <button type="button" class="scene-icon-button" aria-label="Collapse all" title="Collapse all" disabled={!tree.anyExpanded()} onClick={tree.collapseAll}>
            <icons.ChevronsDownUp aria-hidden="true" />
          </button>
        </div>
        <div class="scene-timeline-controls">
          <Show
            when={props.playback.playing()}
            fallback={
              <button type="button" aria-label="Play" disabled={!timing()} onClick={props.playback.play}>
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M4 2.5v11l9-5.5z" />
                </svg>
              </button>
            }
          >
            <button type="button" aria-label="Pause" onClick={props.playback.pause}>
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M3.5 2.5h3v11h-3zM9.5 2.5h3v11h-3z" />
              </svg>
            </button>
          </Show>
          <button type="button" aria-label="Stop" disabled={!timing()} onClick={props.playback.stop}>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3 3h10v10H3z" />
            </svg>
          </button>
          <Show when={props.rendering}>
            <span class="scene-spinner" role="status" aria-label="Rendering" />
          </Show>
          <Show when={timing()} fallback={<span class="scene-note">Set any value to Keyframes to animate this scene.</span>}>
            <span class="scene-timeline-time">
              {props.playback.position().toFixed(2)} / {timing()!.duration.toFixed(2)} s
            </span>
            <TimingFields lens={field(scene, 'timing') as Lens<Timing>} playback={props.playback} />
          </Show>
          <Show when={props.error}>
            <span class="scene-note scene-note-warn">{props.error}</span>
          </Show>
        </div>
      </header>
      <div class="scene-timeline-tracks">
        <div
          class="scene-timeline-lanes"
          ref={lanes}
          classList={{ 'scene-timeline-untimed': !timing() }}
          style={timing() ? { '--played-fraction': props.playback.position() / span(), '--end-fraction': timing()!.duration / span() } : undefined}
        >
          <div class="scene-timeline-row scene-timeline-times">
            <span class="scene-timeline-times-gap" />
            <Show when={timing()} fallback={<span />}>
              <div class="scene-timeline-ruler" ref={ruler} {...scrubbing}>
                <span class="scene-timeline-playhead-head" aria-hidden="true" />
                <For each={ticks()}>
                  {(tick) => (
                    <span class="scene-timeline-tick" classList={{ 'scene-timeline-tick-end': tick / span() > 0.9 }} style={{ left: percent(tick) }}>
                      {Number(tick.toFixed(2))}
                    </span>
                  )}
                </For>
              </div>
            </Show>
          </div>
          <Show when={scene.get().layers.length === 0 && scene.get().sounds.length === 0}>
            <p class="scene-note scene-layers-empty">This scene has no layers yet.</p>
          </Show>
          <div role="tree" aria-label="Layers" onKeyDown={navigate}>
            <Show when={props.preview.name}>
              {(name) => <PreviewRow name={name()} error={props.preview.error} onRemove={() => props.preview.set(null)} />}
            </Show>
            {/* by position, so an edit doesn't tear down a key's editor that's open on its row */}
            <Index each={rows()}>
              {(row) => (
                <Switch>
                  <Match when={'group' in row() && (row() as GroupRowData)}>
                    {(group) => <GroupRow row={group()} timed={timing() !== undefined} percent={percent} />}
                  </Match>
                  <Match when={'node' in row() && (row() as TreeRowData)}>
                    {(node) => <TreeRow
                        row={node()}
                        timed={timing() !== undefined}
                        end={timing()?.duration ?? 1}
                        span={span()}
                        percent={percent}
                        lengthOf={lengthOf}
                        expanded={!collapsed().has(node().node.id)}
                        scrub={(seconds) => props.playback.scrub(Math.min(seconds, timing()!.duration))}
                        onExpand={(open) => setCollapsed((ids) => (open ? new Set([...ids].filter((id) => id !== node().node.id)) : new Set(ids).add(node().node.id)))}
                      />}
                  </Match>
                </Switch>
              )}
            </Index>
          </div>
          <Show when={timing()}>
            <div class="scene-timeline-playhead" />
          </Show>
        </div>
      </div>
    </section>
  )
}

/**
 * Another scene drawn with the one being edited, to see how the two look together, which the tree shows but can't
 * change. Their layers are stacked by z, with its layers under this scene's on the same z. `error` is why it couldn't be
 * drawn, and `choices` are the scenes it can be.
 */
export type PreviewLayer = { name: string | null; error: string | null; choices: string[]; set(name: string | null): void }

/** The preview scene's row, faded, with a button to take it away */
function PreviewRow(props: { name: string; error: string | null; onRemove: () => void }) {
  return (
    <div
      class="scene-timeline-row scene-layers-row scene-layers-preview"
      role="treeitem"
      aria-level={1}
      aria-disabled="true"
      aria-label={`${props.name}, previewed with this scene`}
      title={props.error ?? `${props.name} is drawn with this scene, stacked in by z. It can't be edited here.`}
    >
      <div class="scene-timeline-name scene-layers-name" style={{ '--depth': 0 }}>
        <span class="scene-layers-indicator" />
        <icons.Clapperboard class="scene-tree-icon" aria-hidden="true" />
        <span class="scene-layers-label">{props.name}</span>
        <Show when={props.error}>
          <icons.CircleX class="scene-layers-preview-error" aria-label={props.error!} />
        </Show>
        <button type="button" class="scene-icon-button scene-layers-preview-remove" aria-label={`Stop previewing ${props.name}`} title="Stop previewing" onClick={props.onRemove}>
          <icons.X aria-hidden="true" />
        </button>
      </div>
      <div class="scene-timeline-lane" />
    </div>
  )
}

/** A row of the tree: its node, how deep it is, whether it has rows under it and they're showing, and its group's color */
/** `context` is what the node is in that has no row of its own, which its row names */
type TreeRowData = { node: ConfigNode; context?: string; depth: number; branch: boolean; open: boolean; color: LayerColor }

/** A row heading a group of layers, and whether the group's layers are showing */
type GroupRowData = { group: LayerGroup; open: boolean }

/**
 * Layers that draw on the same thing, together in the tree: the light layers, each display's layers, or the sound
 * layers. In the file they're in one list. A group of drawn layers lists them in the order they're drawn in, backwards,
 * so the one on top is at the top, as in an image editor: by z, then later in the list over earlier. Sounds keep the
 * list's order.
 */
type LayerGroup = { id: string; label: string; icon: typeof icons.Circle; color: LayerColor; nodes: ConfigNode[] }

/** The scene's layers in groups: the lights, then each display in the order the scene first draws on it, then the sounds */
function layerGroups(tree: ConfigTree): LayerGroup[] {
  const layers = (tree.nodes().find((node) => node.id === 'scene')?.children ?? []).filter(shownInTree)
  const groups: LayerGroup[] = [{ id: 'lights', label: 'Lights', icon: icons.Lightbulb, color: 'lights', nodes: [] }]
  const sounds: LayerGroup = { id: 'sounds', label: 'Sounds', icon: icons.AudioLines, color: 'sound', nodes: [] }
  for (const node of layers) {
    if (node.kind !== 'layer') {
      sounds.nodes.push(node)
      continue
    }
    const id = layerGroupId(node.lens.get())
    let group = groups.find((found) => found.id === id)
    if (!group) {
      group = { id, label: node.lens.get().display!, icon: icons.Monitor, color: 'display', nodes: [] }
      groups.push(group)
    }
    group.nodes.push(node)
  }
  groups.forEach((group) => group.nodes.reverse().sort((a, b) => layerZ(b) - layerZ(a)))
  return [...groups, sounds].filter(({ nodes }) => nodes.length > 0)
}

/** The group of layers a layer is in, by its id */
const layerGroupId = (layer: SceneLayer) => (layer.display === undefined ? 'lights' : `display:${layer.display}`)

/** The rows that are showing, in order: each group's row, then its layers. The variables and values are in the sidebar. */
function treeRows(tree: ConfigTree, groups: LayerGroup[]): (TreeRowData | GroupRowData)[] {
  const rows: (TreeRowData | GroupRowData)[] = []
  const visit = (nodes: TreeChild[], depth: number, color: LayerColor) => {
    for (const { node, context } of nodes) {
      const children = treeChildren(node)
      const open = children.length > 0 && tree.isExpanded(node.id)
      rows.push({ node, context, depth, branch: children.length > 0, open, color })
      if (open) visit(children, depth + 1, color)
    }
  }
  for (const group of groups) {
    const open = tree.isGroupOpen(group.id)
    rows.push({ group, open })
    if (open) visit(group.nodes.map((node) => ({ node })), 1, group.color)
  }
  return rows
}

/** A group's row, which opens and closes it. Closed, its lane shows when the props of its layers change. */
function GroupRow(props: { row: GroupRowData; timed: boolean; percent: (seconds: number) => string }) {
  const tree = useContext(ConfigTreeContext)!
  const group = () => props.row.group
  const toggle = () => tree.setGroupOpen(group().id, !props.row.open)
  return (
    <div class={`scene-timeline-row scene-layers-row scene-layers-group ${layerColorClass(group().color) ?? ''}`} role="treeitem" aria-level={1} aria-expanded={props.row.open} aria-label={`${group().label} layers`} tabIndex={-1}>
      <div class="scene-timeline-name scene-layers-name" style={{ '--depth': 0 }} onDblClick={(e) => !(e.target as Element).closest('button') && toggle()}>
        <button type="button" class="scene-layers-indicator" classList={{ 'scene-layers-open': props.row.open }} tabIndex={-1} aria-label={`${props.row.open ? 'Close' : 'Open'} ${group().label}`} onClick={toggle}>
          <icons.ChevronRight aria-hidden="true" />
        </button>
        <Dynamic component={group().icon} class="scene-tree-icon" aria-hidden="true" />
        <span class="scene-layers-label">{group().label}</span>
        <span class="scene-tree-summary">{group().nodes.length}</span>
      </div>
      <div class="scene-timeline-lane">
        <Show when={props.timed && !props.row.open}>
          <KeySummary nodes={group().nodes} percent={props.percent} />
        </Show>
      </div>
    </div>
  )
}

/** A prop that changes along the timeline, which has a track */
type TrackNode = Extract<ConfigNode, { kind: 'prop' }>
const trackOf = (node: ConfigNode): TrackNode | false => (node.kind === 'prop' && hasTrack(node.lens.get()) ? node : false)

/** Whether a node's lane can be made taller to show its value over time: a keyframed or modulated prop's can */
const expandable = (node: ConfigNode) => {
  const track = trackOf(node)
  return track !== false && ('Keyframes' in track.lens.get() || 'Modulated' in track.lens.get())
}

/**
 * A node's row: its name, indented under what it's in, with what it is at a glance, then its lane. A press anywhere on
 * it selects the node, and its name can be dragged to elsewhere in its list.
 */
function TreeRow(props: {
  row: TreeRowData
  timed: boolean
  end: number
  span: number
  percent: (seconds: number) => string
  lengthOf: (node: Extract<ConfigNode, { kind: 'traversal' }>) => number | null
  expanded: boolean
  onExpand: (expanded: boolean) => void
  /** Moves the playhead to a time, such as a key's that's just been added, so what's changed next changes it */
  scrub: (seconds: number) => void
}) {
  const tree = useContext(ConfigTreeContext)!
  const menu = useContext(LayerMenuContext)!
  const node = () => props.row.node
  const selected = () => tree.isSelected(node().id)
  const named = () => {
    const current = node()
    return is(current, 'layer') || is(current, 'sound')
  }
  const drag = rowDrag(node, tree)
  const color = () => props.row.color
  const layerHidden = () => {
    const current = node()
    return current.kind === 'layer' && current.lens.get().hidden === true
  }
  const expanded = () => props.timed && props.expanded && expandable(node())
  const trackHidden = () => {
    const track = trackOf(node())
    return track !== false && isHidden(track.lens.get())
  }

  return (
    <LayerColorContext.Provider value={color()}>
      <div
        class={`scene-timeline-row scene-layers-row ${layerColorClass(color()) ?? ''}`}
        classList={{
          'scene-layers-selected': selected(),
          'scene-timeline-hidden': trackHidden(),
          'scene-timeline-expanded': expanded(),
          'scene-timeline-expanded-keys': expanded() && trackOf(node()) !== false && 'Keyframes' in (trackOf(node()) as TrackNode).lens.get(),
        }}
        role="treeitem"
        aria-level={props.row.depth + 1}
        aria-selected={selected()}
        aria-expanded={props.row.branch ? props.row.open : undefined}
        aria-label={node().label}
        data-id={node().id}
        tabIndex={selected() ? 0 : -1}
        onPointerDown={(e) => {
          drag.grab(e)
          tree.select(node().id)
        }}
        onContextMenu={(e) => {
          e.preventDefault()
          menu.open(node().id, e.clientX, e.clientY)
        }}
        {...drag.handlers}
      >
        <div
          class="scene-timeline-name scene-layers-name"
          classList={{ 'scene-item-hidden': layerHidden() }}
          style={{ '--depth': props.row.depth }}
          onDblClick={(e) => {
            if (props.row.branch && !(e.target as Element).closest('button')) tree.toggle(node().id)
          }}
        >
          <Show when={props.row.branch} fallback={<span class="scene-layers-indicator" />}>
            <button
              type="button"
              class="scene-layers-indicator"
              classList={{ 'scene-layers-open': props.row.open }}
              tabIndex={-1}
              aria-label={`${props.row.open ? 'Close' : 'Open'} ${node().label}`}
              onClick={() => tree.toggle(node().id)}
            >
              <icons.ChevronRight aria-hidden="true" />
            </button>
          </Show>
          <NodeIcon node={node()} />
          <Show
            when={menu.renaming() === node().id && named()}
            fallback={
              <span class="scene-layers-label" title={props.row.context ? `${props.row.context} · ${node().label}` : node().label}>
                <Show when={props.row.context}>{(context) => <span class="scene-layers-context">{context()} · </span>}</Show>
                {node().label}
              </span>
            }
          >
            {(layer) => <NameEditor node={layer()} onDone={() => menu.setRenaming(null)} />}
          </Show>
          <NodeSummary node={node()} />
          <Show when={props.timed && expandable(node())}>
            <button
              type="button"
              class="scene-eye scene-track-expand"
              aria-label={`${expanded() ? 'Collapse' : 'Expand'} ${node().label}`}
              aria-pressed={expanded()}
              title={expanded() ? 'Collapse' : 'Show its value over time'}
              onClick={() => props.onExpand(!expanded())}
            >
              <Show when={expanded()} fallback={<icons.ChevronsUpDown aria-hidden="true" />}>
                <icons.ChevronsDownUp aria-hidden="true" />
              </Show>
            </button>
          </Show>
          <RowEye node={node()} />
        </div>
        <div class="scene-timeline-lane" classList={{ 'scene-timeline-lane-track': trackOf(node()) !== false }}>
          <Show when={props.timed}>
            <Switch>
              <Match when={trackOf(node())}>{(track) => <TrackLane node={track()} percent={props.percent} span={props.span} expanded={expanded()} scrub={props.scrub} />}</Match>
              <Match when={is(node(), 'sound')}>{(sound) => <SoundBar label={sound().label} lens={sound().lens} end={props.end} percent={props.percent} span={props.span} />}</Match>
              <Match when={is(node(), 'traversal')}>
                {(traversal) => <TraversalLane node={traversal()} length={props.lengthOf(traversal())} end={props.end} percent={props.percent} span={props.span} />}
              </Match>
              <Match when={props.row.branch && !props.row.open}>
                <KeySummary nodes={node().children ?? []} percent={props.percent} />
              </Match>
            </Switch>
          </Show>
        </div>
      </div>
    </LayerColorContext.Provider>
  )
}

/**
 * A layer's name being edited in its row, which Enter or leaving the field keeps and Escape puts back. An empty name
 * goes back to what the layer is called unnamed.
 */
function NameEditor(props: { node: Extract<ConfigNode, { kind: 'layer' | 'sound' }>; onDone: () => void }) {
  const name = () => field(props.node.lens as Lens<{ name?: string }>, 'name')
  let done = false
  const finish = (keep: string | null) => {
    if (done) return
    done = true
    if (keep !== null) name().set(keep.trim() || undefined)
    props.onDone()
  }
  return (
    <input
      class="scene-layers-rename"
      aria-label="Name"
      spellcheck={false}
      placeholder={props.node.unnamed}
      value={name().get() ?? ''}
      ref={(input) => requestAnimationFrame(() => (input.focus(), input.select()))}
      onPointerDown={(e) => e.stopPropagation()}
      onKeyDown={(e) => {
        e.stopPropagation()
        if (e.key === 'Enter') finish(e.currentTarget.value)
        else if (e.key === 'Escape') finish(null)
      }}
      onBlur={(e) => finish(e.currentTarget.value)}
    />
  )
}

/** Shows or hides a layer, or holds a track still on its first value */
function RowEye(props: { node: ConfigNode }) {
  return (
    <Switch>
      <Match when={is(props.node, 'layer')}>
        {(layer) => <EyeButton label={layer().label} hidden={layer().lens.get().hidden === true} onChange={(hidden) => field(layer().lens, 'hidden').set(hidden || undefined)} />}
      </Match>
      <Match when={trackOf(props.node)}>
        {(track) => <EyeButton label={track().label} hidden={isHidden(track().lens.get())} onChange={(hidden) => track().lens.set(withHidden(track().lens.get(), hidden))} />}
      </Match>
    </Switch>
  )
}

/** When the props under closed nodes change, as marks that can't be moved from here */
function KeySummary(props: { nodes: ConfigNode[]; percent: (seconds: number) => string }) {
  const times = createMemo(() => {
    const found = new Set<number>()
    const visit = (nodes: ConfigNode[]) =>
      nodes.forEach((node) => {
        if (node.kind === 'prop') trackTimes(node.lens.get()).forEach((time) => found.add(time))
        visit(node.children ?? [])
      })
    visit(props.nodes)
    return [...found]
  })
  return <For each={times()}>{(time) => <span class="scene-key-summary" style={{ left: props.percent(time) }} aria-hidden="true" />}</For>
}

/** Whether a list's rows are shown in the opposite order to the list: the drawn layers', topmost first */
const listedBackwards = ({ listId }: ListPlace) => listId === 'layers'

/** The z of a drawn layer's node, which orders it among the others in its group before their order in the list does */
const layerZ = (node: ConfigNode) => (node.kind === 'layer' ? (node.lens.get().z ?? 0) : 0)

/** The group of layers a place in the scene's layers is in. Places in other lists are all in one. */
const placeGroup = ({ list, listId, index }: ListPlace) => (listId === 'layers' ? layerGroupId(list.get()[index] as SceneLayer) : null)

/**
 * Dragging a row in a list to elsewhere in it, among the rows of its group, dropped before or after the row it's let go on. Only a press on its name
 * starts a drag, and not one on a button there, so the lane's keys and bars can still be dragged along it.
 */
function rowDrag(node: () => ConfigNode, tree: ConfigTree) {
  const [grabbable, setGrabbable] = createSignal(false)
  const [drop, setDrop] = createSignal<'before' | 'after' | null>(null)
  const place = () => node().place
  const isDragged = () => {
    const from = tree.dragging()
    return !!place() && from?.listId === place()!.listId && from.index === place()!.index
  }
  const accepts = () => {
    const from = tree.dragging()
    return !!place() && from?.listId === place()!.listId && from.index !== place()!.index && placeGroup(from) === placeGroup(place()!)
  }
  return {
    grab: (e: PointerEvent) => {
      const target = e.target as HTMLElement
      setGrabbable(!!place() && target.closest('.scene-layers-name') !== null && !target.closest('input, textarea, button, [contenteditable]'))
    },
    handlers: {
      get draggable() {
        return grabbable()
      },
      get 'data-dragging'() {
        return isDragged() || undefined
      },
      get 'data-drop'() {
        return drop() ?? undefined
      },
      onDragStart: (e: DragEvent) => {
        const from = place()
        if (!from) return
        e.dataTransfer!.effectAllowed = 'move'
        e.dataTransfer!.setData('text/plain', node().id)
        tree.setDragging(from)
      },
      onDragOver: (e: DragEvent) => {
        if (!accepts()) return
        e.preventDefault()
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
        setDrop(e.clientY < rect.top + rect.height / 2 ? 'before' : 'after')
      },
      onDragLeave: () => setDrop(null),
      onDrop: (e: DragEvent) => {
        const from = tree.dragging()
        const to = place()
        const side = drop()
        setDrop(null)
        if (!from || !to || !side || !accepts()) return
        e.preventDefault()
        // where it goes once it's out of the list, which moves those after it up one. Below a row in a list shown
        // backwards is before it in the list.
        const after = (side === 'after') !== listedBackwards(to)
        const at = to.index + (after ? 1 : 0)
        const index = from.index < at ? at - 1 : at
        // a drawn layer dropped among others on another z takes theirs, so it's drawn where it's dropped
        const z = listedBackwards(to) ? (to.list.get()[to.index] as SceneLayer).z : undefined
        batch(() => {
          if (index !== from.index) tree.move(from, index - from.index)
          if (listedBackwards(to) && ((from.list.get()[index] as SceneLayer).z ?? 0) !== (z ?? 0)) field(item(from.list as Lens<SceneLayer[]>, index), 'z').set(z || undefined)
        })
      },
      onDragEnd: () => {
        tree.setDragging(null)
        setGrabbable(false)
      },
    },
  }
}

/** The length of a cycle, and how many times it plays */
function TimingFields(props: { lens: Lens<Timing>; playback: Playback }) {
  const scene = useContext(SceneContext)!
  const cycle = field(props.lens, 'cycle')
  const kind = () => {
    const value = cycle.get()
    return typeof value === 'string' ? value : 'Times'
  }

  const scale = (by: number) => {
    batch(() => {
      scene.set(withTimesScaled(scene.get(), by))
      props.playback.rescale(by)
    })
  }

  return (
    <>
      <label>
        Length
        <NumberBox
          min={TIME_STEP}
          step="0.1"
          // a scaled length can run on for more digits than this box has room for
          value={Number(props.lens.get().duration.toFixed(3))}
          onNumber={(duration) => {
            if (duration > 0) field(props.lens, 'duration').set(duration)
          }}
          // not on each keystroke, as typing a length passes through shorter ones on the way to it
          onChange={() => scene.set(withTimesInLength(scene.get()))}
        />
      </label>
      <TimeScaleMenu duration={props.lens.get().duration} scale={scale} />
      <Dropdown<'Once' | 'Times' | 'Forever'>
        label="Repeat"
        options={[
          { value: 'Once', label: 'Once' },
          { value: 'Times', label: 'Times' },
          { value: 'Forever', label: 'Forever' },
        ]}
        value={kind()}
        onChange={(next) => {
          if (next !== kind()) cycle.set((next === 'Times' ? { Times: 2 } : next) as Cycle)
        }}
      />
      <Show when={kind() === 'Times'}>
        <NumberBox
          min="1"
          step="1"
          aria-label="Times to play"
          value={(cycle.get() as { Times: number }).Times}
          onNumber={(times) => {
            if (Number.isInteger(times) && times > 0) cycle.set({ Times: times })
          }}
        />
      </Show>
    </>
  )
}

/**
 * A button with a menu for scaling a scene's times, which opens by resting the pointer on it as well as by clicking it.
 * It leaves the keyboard's focus alone, as it can come and go while the length is being typed in.
 */
function TimeScaleMenu(props: { duration: number; scale: (by: number) => void }) {
  const [open, setOpen] = createSignal(false)
  let timer: ReturnType<typeof setTimeout> | undefined

  const stay = () => clearTimeout(timer)
  const after = (delay: number, next: boolean) => {
    stay()
    timer = setTimeout(() => setOpen(next), delay)
  }
  const leave = () => after(SCALE_MENU_CLOSE_DELAY, false)
  onCleanup(stay)

  const option = ({ label, by }: TimeScale) => (
    <button
      type="button"
      disabled={props.duration * by < TIME_STEP}
      onClick={() => {
        props.scale(by)
        stay()
        setOpen(false)
      }}
    >
      {label}
    </button>
  )

  return (
    <Popover.Root
      lazyMount
      unmountOnExit
      autoFocus={false}
      restoreFocus={false}
      positioning={{ placement: 'top' }}
      open={open()}
      onOpenChange={(details) => {
        stay()
        setOpen(details.open)
      }}
    >
      <Popover.Trigger
        aria-label="Scale the length and everything on the timeline"
        onPointerEnter={() => (open() ? stay() : after(SCALE_MENU_OPEN_DELAY, true))}
        onPointerLeave={() => (open() ? leave() : stay())}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2 3h1.25v10H2zM12.75 3H14v10h-1.25zM4.25 8 7 5.25v2h2v-2L11.75 8 9 10.75v-2H7v2z" />
        </svg>
      </Popover.Trigger>
      <ScenePortal>
        <Popover.Positioner>
          <Popover.Content class="scene-time-scales" onPointerEnter={stay} onPointerLeave={leave}>
            <For each={SHORTER}>{option}</For>
            <span class="scene-time-scales-divider" />
            <For each={LONGER}>{option}</For>
          </Popover.Content>
        </Popover.Positioner>
      </ScenePortal>
    </Popover.Root>
  )
}

/** A prop's keys, its picks, or its wave along its lane, with a button that follows the mouse for adding a key or a pick */
function TrackLane(props: { node: TrackNode; percent: (seconds: number) => string; span: number; expanded: boolean; scrub: (seconds: number) => void }) {
  const scene = useContext(SceneContext)!
  // read through `props`, as a lane goes to whichever prop comes to be at its row
  const prop: Lens<Prop<SceneValue>> = { get: () => props.node.lens.get(), set: (next) => props.node.lens.set(next) }
  const label = () => props.node.label
  const times = () => trackTimes(prop.get())
  const waved = () => 'Modulated' in prop.get()
  /** The time under the mouse, and whether that's a dot's rather than one to add a dot at */
  const [hovering, setHovering] = createSignal<{ at: number; onDot: boolean } | null>(null)
  let lane!: HTMLDivElement

  /** On the page, as the tracks scroll and cut off whatever hangs out of them, and the times would cover the first */
  const addTimePlace = () => {
    const { left, top, width } = lane.getBoundingClientRect()
    return { '--x': `${left + (hovering()!.at / props.span) * width}px`, top: `${top}px` }
  }

  const hover = (e: PointerEvent & { currentTarget: HTMLElement }) => {
    if (waved()) return
    const { width } = e.currentTarget.getBoundingClientRect()
    const at = along(e, e.currentTarget) * props.span
    const nearest = times().reduce((near, time) => (Math.abs(time - at) < Math.abs(near - at) ? time : near), Infinity)
    const onDot = (Math.abs(nearest - at) / props.span) * width < KEY_REACH
    setHovering(onDot ? { at: nearest, onDot } : { at: snap(at), onDot })
  }

  const add = (at: number) => {
    const current = prop.get()
    if ('Keyframes' in current) prop.set({ Keyframes: withKey(current.Keyframes, at) })
    if ('Random' in current) prop.set({ Random: withPick(current.Random, at) })
    props.scrub(at)
  }

  return (
    <div class="scene-track" ref={lane} onPointerMove={hover} onPointerLeave={() => setHovering(null)}>
      <Show when={hovering()?.onDot === false}>
        <button
          type="button"
          class="scene-key-add"
          aria-label={`Add a key to ${label()}`}
          style={{ left: props.percent(hovering()!.at) }}
          onClick={() => {
            add(hovering()!.at)
            setHovering(null)
          }}
        >
          +
        </button>
      </Show>
      <Show when={hovering() !== null}>
        <ScenePortal>
          <span class="scene-key-add-time" style={addTimePlace()}>
            {hovering()!.at.toFixed(2)} s
          </span>
        </ScenePortal>
      </Show>
      <Show when={waved()}>
        <Show when={scene.get().modulators?.[variant(prop, 'Modulated').get().modulator]}>
          {(modulator) => (
            <WaveLine modulated={variant(prop, 'Modulated').get()} modulator={modulator()} scene={scene.get()} duration={scene.get().timing?.duration ?? props.span} percent={props.percent} />
          )}
        </Show>
      </Show>
      <Show when={props.expanded && 'Keyframes' in prop.get()}>
        <KeyedPreview keys={variant(prop, 'Keyframes').get().keys} cycle={variant(prop, 'Keyframes').get().cycle} span={props.span} />
      </Show>
      <Index each={times()}>
        {(time, index) => (
          <Show
            when={'Keyframes' in prop.get()}
            fallback={<PickDot label={label()} picks={field(variant(prop, 'Random'), 'picks')} index={index} left={props.percent(time())} times={times()} span={props.span} />}
          >
            <KeyDot label={label()} ranges={props.node.ranges} keys={field(variant(prop, 'Keyframes'), 'keys')} index={index} left={props.percent(time())} times={times()} span={props.span} />
          </Show>
        )}
      </Index>
    </div>
  )
}

/** Most points drawn along a wave, however many times it goes by */
const WAVE_SAMPLES = 2048

/** A modulated prop's wave by `modulator` over the scene's first cycle, from its low end at the bottom to its high end at the top */
function WaveLine(props: { modulated: Modulated; modulator: Modulator; scene: Scene; duration: number; percent: (seconds: number) => string }) {
  const points = () => {
    const samples = Math.min(Math.ceil(96 * Math.max(props.modulator.rate, 1)), WAVE_SAMPLES)
    return Array.from({ length: samples + 1 }, (_, index) => {
      const at = index / samples
      const phase = phaseAt(props.modulated.phase, props.scene, at * props.duration)
      return `${at},${1 - envelopeHeight(props.modulator.envelope, wavePosition(props.modulator, phase, at))}`
    }).join(' ')
  }
  return (
    <svg class="scene-wave" style={{ width: props.percent(props.duration) }} viewBox="0 0 1 1" preserveAspectRatio="none" aria-hidden="true">
      <polyline points={points()} />
    </svg>
  )
}

/**
 * A wave's phase `seconds` into the cycle, as `Keyframes::resolve` in frontbox-gfx has it for keyframes, with a curve written
 * by hand taken as linear. Only the machine knows what one from anywhere else comes to, so that stays where it starts.
 */
function phaseAt(phase: Prop<number>, scene: Scene, seconds: number): number {
  if (!('Keyframes' in phase)) return startingValue(phase, scene, 0)
  return keyedAt(phase.Keyframes.keys, phase.Keyframes.cycle, seconds, (a, b, height) => a + (b - a) * height) ?? 0
}

/**
 * A keyframed value `seconds` into the scene's cycle, as `Keyframes::resolve` in frontbox-gfx has it, with a curve
 * written by hand taken as linear. `mix` goes from one key's value to the next's by the curve's height between them.
 */
function keyedAt<T>(keys: Key<T>[], cycle: Cycle, seconds: number, mix: (a: T, b: T, height: number) => T): T | undefined {
  const [first, last] = [keys[0], keys[keys.length - 1]]
  if (!first || !last) return undefined
  const laps = cycle === 'Once' ? 1 : cycle === 'Forever' ? Infinity : cycle.Times
  const time = last.at <= 0 ? seconds : seconds >= laps * last.at ? last.at : cycle === 'Once' ? seconds : seconds % last.at
  if (time <= first.at) return first.value
  for (let index = 1; index < keys.length; index++) {
    const [a, b] = [keys[index - 1], keys[index]]
    if (time < b.at) {
      const span = b.at - a.at
      const through = span > 0 ? (time - a.at) / span : 1
      return mix(a.value, b.value, curveHeight(b.curve, through))
    }
  }
  return last.value
}

/** How many points a keyframed value's preview is drawn with across the view */
const KEYED_SAMPLES = 400

/** Two keyed values `height` of the way from `a` to `b`. A boolean switches on the key it arrives at. */
function mixValues(a: SceneValue, b: SceneValue, height: number): SceneValue {
  if (typeof a === 'number' && typeof b === 'number') return a + (b - a) * height
  if (Array.isArray(a) && Array.isArray(b)) return a.map((value, index) => value + ((b[index] ?? value) - value) * height) as SceneValue
  return height >= 1 ? b : a
}

/** The numbers a value is drawn as: a number or a boolean as one line, a point as two */
const lineValues = (value: SceneValue): number[] => (typeof value === 'boolean' ? [value ? 1 : 0] : typeof value === 'number' ? [value] : value)

/**
 * A keyframed prop's value over the view, below its keys: a line from its lowest value at the bottom to its highest at
 * the top, one for each of a point's x and y, or a color as a band of the colors it goes through.
 */
function KeyedPreview(props: { keys: Key<SceneValue>[]; cycle: Cycle; span: number }) {
  const samples = createMemo(() =>
    Array.from({ length: KEYED_SAMPLES + 1 }, (_, index) => {
      const at = index / KEYED_SAMPLES
      return { at, value: keyedAt(props.keys, props.cycle, at * props.span, mixValues) }
    }).filter((sample): sample is { at: number; value: SceneValue } => sample.value !== undefined),
  )
  const isColor = () => {
    const first = props.keys[0]?.value
    return Array.isArray(first) && first.length === 4
  }
  /** Each line's points, each fit to the height between its own lowest and highest values */
  const lines = () => {
    const values = samples().map(({ value }) => lineValues(value))
    return (values[0] ?? []).map((_, line) => {
      const of = values.map((value) => value[line])
      const [low, high] = [Math.min(...of), Math.max(...of)]
      const height = (value: number) => (high - low > 1e-6 ? (value - low) / (high - low) : 0.5)
      return samples()
        .map(({ at }, index) => `${at},${1 - height(of[index])}`)
        .join(' ')
    })
  }
  const id = `scene-keyed-${Math.random().toString(36).slice(2)}`
  return (
    <svg class="scene-keyed" viewBox="0 0 1 1" preserveAspectRatio="none" aria-hidden="true">
      <Show
        when={isColor()}
        fallback={<For each={lines()}>{(points, line) => <polyline classList={{ 'scene-keyed-second': line() > 0 }} points={points} />}</For>}
      >
        <defs>
          <linearGradient id={id}>
            <For each={samples().filter((_, index) => index % 8 === 0)}>
              {({ at, value }) => {
                const [r, g, b] = value as number[]
                return <stop offset={at} stop-color={`rgb(${r} ${g} ${b})`} />
              }}
            </For>
          </linearGradient>
        </defs>
        <rect class="scene-keyed-band" x="0" y="0.2" width="1" height="0.6" fill={`url(#${id})`} />
      </Show>
    </svg>
  )
}

/**
 * When a sound plays: a bar from its start to where it ends or is cut off, between the parts of the sound that are
 * skipped and cut off. Its left edge skips more or less of the sound's beginning and its right edge moves the end, each
 * leaving the other where it is, and the rest moves the whole of it. It can't start at `end`, as that's the start of the
 * next cycle, but it can play on past it.
 */
function SoundBar(props: { label: string; lens: Lens<SoundLayer>; end: number; percent: (seconds: number) => string; span: number }) {
  const layer = () => props.lens.get()
  const length = () => contentSoundLength(layer().sound)
  const skip = () => layer().skip ?? 0
  /** Where the sound ends on its own, once that's known */
  const natural = () => soundNaturalEnd(layer(), length())
  // until its length is known, it's given some room to be grabbed by
  const shownEnd = () => soundEnd(layer(), length()) ?? layer().start + props.span / 20
  /** Where the sound's beginning would be on the timeline if none of it were skipped, which can't be before the top */
  const head = () => Math.max(layer().start - skip(), 0)
  const latest = () => Math.max(props.end - TIME_STEP, 0)
  let bar!: HTMLDivElement

  type Times = { start: number; skip: number; end: number | undefined }

  /** `layer` with its times set, playing to the end of its sound if `end` reaches it */
  const set = ({ start, skip, end }: Times) => {
    const { end: _, skip: __, ...rest } = layer()
    const natural = soundNaturalEnd({ ...rest, start, skip }, length())
    const ends = end !== undefined && (natural === undefined || end < natural - TIME_STEP / 2)
    props.lens.set({ ...rest, start, ...(skip > 0 && { skip }), ...(ends && { end }) })
  }

  /** Dragging something that sets the times from how far it's come, and from how they were when it was pressed */
  const dragged = (times: (by: number, pressed: Times) => Times) => {
    // kept from the press, as the view's length can change with where the ends are
    let press: { x: number; span: number; times: Times } | null = null
    const lane = () => bar.parentElement!
    return {
      onPointerDown(e: PointerEvent & { currentTarget: HTMLElement }) {
        // an edge is inside the bar, which would take the drag as its own
        e.stopPropagation()
        e.currentTarget.setPointerCapture(e.pointerId)
        press = { x: along(e, lane()), span: props.span, times: { start: layer().start, skip: skip(), end: layer().end ?? natural() } }
      },
      onPointerMove(e: PointerEvent) {
        if (!press) return
        set(times((along(e, lane()) - press.x) * press.span, press.times))
      },
      onPointerUp: () => (press = null),
      onPointerCancel: () => (press = null),
    }
  }

  // made once, as each of these holds its own press
  // the sound stays put on the timeline, so its start moves with how much of its beginning is skipped
  const startEdge = dragged((by, { start, skip, end }) => {
    const next = clamp(snap(start + by), Math.max(start - skip, 0), Math.min(latest(), (end ?? shownEnd()) - TIME_STEP))
    return { start: next, skip: Math.max(snap(skip + next - start), 0), end }
  })
  const endEdge = dragged((by, { start, skip, end }) => ({ start, skip, end: clamp(snap((end ?? shownEnd()) + by), start + TIME_STEP, natural() ?? Infinity) }))
  const middle = dragged((by, { start, skip, end }) => {
    const moved = clamp(snap(by), -start, latest() - start)
    return { start: snap(start + moved), skip, end: end === undefined ? undefined : snap(end + moved) }
  })

  return (
    <>
      <Show when={skip() > 0 && head() < layer().start}>
        <span
          class="scene-sound-skipped"
          title={`The first ${skip().toFixed(2)} s of ${props.label} is skipped`}
          style={{ left: props.percent(head()), width: props.percent(layer().start - head()) }}
        />
      </Show>
      <Show when={natural() !== undefined && natural()! > shownEnd()}>
        <span
          class="scene-sound-cut"
          title={`The last ${(natural()! - shownEnd()).toFixed(2)} s of ${props.label} is cut off`}
          style={{ left: props.percent(shownEnd()), width: props.percent(natural()! - shownEnd()) }}
        />
      </Show>
      <div
        class="scene-sound-bar"
        ref={bar}
        title={`${layer().sound}${length() ? `, ${length()!.toFixed(2)} s long` : ''}`}
        style={{ left: props.percent(layer().start), width: props.percent(shownEnd() - layer().start) }}
        {...middle}
      >
        <button type="button" class="scene-sound-edge scene-sound-edge-start" aria-label={`${props.label}, ${skip().toFixed(2)} seconds skipped at its beginning`} {...startEdge} />
        <button type="button" class="scene-sound-edge scene-sound-edge-end" aria-label={`${props.label} end, ${shownEnd().toFixed(2)} seconds`} {...endEdge} />
      </div>
    </>
  )
}

/** When a traversal plays out. `length` is `null` while that isn't known. */
function TraversalLane(props: { node: Extract<ConfigNode, { kind: 'traversal' }>; length: number | null; end: number; percent: (seconds: number) => string; span: number }) {
  // read through `props`, as a lane goes to whichever traversal comes to be at its row
  const traversal: Lens<Traversal> = { get: () => props.node.lens.get(), set: (next) => props.node.lens.set(next) }
  const over = () => (props.length === null ? 0 : traversal.get().start + props.length - props.end)
  const overText = () => (over() > TIME_STEP / 2 ? `${props.node.label} runs ${over().toFixed(2)} s past the end of the scene, so it's cut off before it finishes` : null)
  return <TraversalBar label={props.node.label} lens={traversal} length={props.length} end={props.end} over={overText()} percent={props.percent} span={props.span} />
}

/**
 * When a traversal plays out, with a grip for moving its start and, for one that finishes in a time, an arrow for
 * changing that time. The part past `end` is marked, with `over` saying what that means.
 */
function TraversalBar(props: {
  label: string
  lens: Lens<Traversal>
  length: number | null
  end: number
  over: string | null
  percent: (seconds: number) => string
  span: number
}) {
  const traversal = () => props.lens.get()
  const length = () => props.length ?? 0
  let bar!: HTMLDivElement

  /** Dragging something that changes the traversal by how far it's come, from how it was when it was pressed */
  const dragged = (moved: (by: number, pressed: { start: number; length: number; span: number }) => Partial<Traversal>) => {
    // kept from the press, as the view's length can change with where the bar is
    let press: { x: number; start: number; length: number; span: number } | null = null
    const lane = () => bar.parentElement!
    return {
      onPointerDown(e: PointerEvent & { currentTarget: HTMLElement }) {
        e.currentTarget.setPointerCapture(e.pointerId)
        press = { x: along(e, lane()), start: traversal().start, length: length(), span: props.span }
      },
      onPointerMove(e: PointerEvent) {
        if (!press) return
        props.lens.set({ ...traversal(), ...moved((along(e, lane()) - press.x) * press.span, press) })
      },
      onPointerUp: () => (press = null),
      onPointerCancel: () => (press = null),
    }
  }

  // made once, as each of these holds its own press
  const grip = dragged((by, { start }) => ({ start: clamp(snap(start + by), 0, Math.max(props.end - TIME_STEP, 0)) }))
  const endArrow = dragged((by, { start, length, span }) => ({ speed: { Duration: clamp(snap(length + by), TIME_STEP, span - start) } }))

  /** Where the end of the scene falls along the bar, as a share of it */
  const endAlong = () => (length() > 0 ? clamp((props.end - traversal().start) / length(), 0, 1) : 0)

  return (
    <div class="scene-sweep scene-traversal" ref={bar} style={{ left: props.percent(traversal().start), width: props.percent(length()) }} title={props.over ?? undefined}>
      <Show when={props.over}>
        <span class="scene-traversal-over" style={{ left: `${endAlong() * 100}%` }} />
      </Show>
      <button type="button" class="scene-sweep-grip" aria-label={`Move ${props.label} along the timeline, starting at ${traversal().start.toFixed(2)} seconds`} {...grip} />
      <Show when={'Duration' in traversal().speed}>
        <button type="button" class="scene-sweep-end scene-sweep-finish" aria-label={`${props.label} finishes in ${length().toFixed(2)} seconds`} {...endArrow}>
          ›
        </button>
      </Show>
    </div>
  )
}

/** Where a dot sits among the others on its track */
type DotPlace = { index: number; left: string; times: number[]; span: number }

/** A free time just after a dot, or just before it if it has no room after. There's none if it's hemmed in both ways. */
function roomNear({ index, times, span }: DotPlace) {
  const toward = (end: number) => {
    const room = end - times[index]
    return snap(times[index] + Math.sign(room) * Math.min(span * COPY_DISTANCE, Math.abs(room) / 2))
  }
  const candidates = [toward(times[index + 1] ?? span), toward(times[index - 1] ?? 0)]
  return candidates.find((candidate) => times.every((time) => Math.abs(time - candidate) >= TIME_STEP / 2))
}

/** A dot on a track. Clicking it opens what can be changed about it, and dragging it moves it along the track, past the others if it comes to them. */
function Dot<T extends { at: number }>(props: DotPlace & { label: string; moments: Lens<T[]>; pick?: boolean; background?: string; children: any }) {
  const [open, setOpen] = createSignal(false)
  // kept from the press, as the view's length can change with where the dot is. Dots are matched to moments by position,
  // so this is what follows the moment being dragged as it passes others, while the press stays with this dot.
  let press: { x: number; index: number; span: number } | null = null
  // until the next press, as the click that ends a drag comes after the pointer is let go
  let dragged = false

  return (
    <Popover.Root
      lazyMount
      unmountOnExit
      positioning={{ placement: 'top' }}
      open={open()}
      onOpenChange={(details) => {
        if (!dragged) setOpen(details.open)
      }}
    >
      <Popover.Trigger
        class="scene-key"
        classList={{ 'scene-key-value': !props.background, 'scene-key-pick': props.pick }}
        aria-label={props.label}
        style={{ left: props.left, background: props.background }}
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId)
          dragged = false
          press = { x: e.clientX, index: props.index, span: props.span }
        }}
        onPointerMove={(e) => {
          if (!press || (!dragged && Math.abs(e.clientX - press.x) < DRAG_START)) return
          // what's open would otherwise be left showing whichever moment takes this one's place
          if (!dragged) setOpen(false)
          dragged = true
          const at = clamp(snap(along(e, e.currentTarget.parentElement!) * press.span), 0, press.span)
          press.index = moveMoment(props.moments, press.index, at)
        }}
        onPointerUp={() => (press = null)}
        onPointerCancel={() => (press = null)}
      />
      <ScenePortal>
        <Popover.Positioner>
          <Popover.Content class="scene-key-editor">{props.children}</Popover.Content>
        </Popover.Positioner>
      </ScenePortal>
    </Popover.Root>
  )
}

/** A moment on a random prop's track when a new value is picked */
function PickDot(props: DotPlace & { label: string; picks: Lens<RandomPick[]> }) {
  const scene = useContext(SceneContext)!
  const pick = () => item(props.picks, props.index)
  const last = () => props.index === props.picks.get().length - 1

  const smoothing = (label: string, name: 'smooth_in' | 'smooth_out') => (
    <FieldRow label={label}>
      <NumberField label={label} range={UNIT} value={pick().get()[name]} onChange={field(pick(), name).set} />
    </FieldRow>
  )

  return (
    <Dot {...props} label={`${props.label} pick at ${pick().get().at.toFixed(2)} seconds`} moments={props.picks} pick>
      {/* the first moment of a scene has nothing to smooth from, nor the last anything to smooth to */}
      <Show when={props.index > 0 || repeats(scene.get())}>{smoothing('Smooth in', 'smooth_in')}</Show>
      <Show when={!last() || repeats(scene.get())}>{smoothing('Smooth out', 'smooth_out')}</Show>
      {/* with no picks there'd be nothing to show, and leaving it is a matter of switching it away from Random */}
      <button
        type="button"
        class="scene-key-remove"
        disabled={props.picks.get().length === 1}
        onClick={() => props.picks.set(removed(props.picks.get(), props.index))}
      >
        Remove pick
      </button>
    </Dot>
  )
}

/** A key on a track */
function KeyDot(props: DotPlace & { label: string; ranges: Ranges; keys: Lens<Key<SceneValue>[]> }) {
  const key = () => item(props.keys, props.index)
  const at = () => key().get().at
  const room = () => roomNear(props)

  const background = () => {
    const value = key().get().value
    return Array.isArray(value) && value.length === 4 ? `rgb(${value.slice(0, 3).join(' ')})` : undefined
  }

  return (
    <Dot {...props} label={`${props.label} key at ${at().toFixed(2)} seconds`} moments={props.keys} background={background()}>
      <FieldRow label="Value">
        <ValueField label="Value" lens={field(key(), 'value')} ranges={props.ranges} />
      </FieldRow>
      <Show when={props.index > 0}>
        <FieldRow label="Curve into this key">
          <CurveField label="Curve into this key" lens={field(key(), 'curve')} />
        </FieldRow>
      </Show>
      <div class="scene-key-actions">
        <button
          type="button"
          class="scene-key-duplicate"
          disabled={room() === undefined}
          onClick={() => props.keys.set(withKeyCopy(props.keys.get(), props.index, room()!))}
        >
          Duplicate
        </button>
        {/* a track with no keys reads as zero, and leaving it is a matter of switching it away from Keyframes */}
        <button
          type="button"
          class="scene-key-remove"
          disabled={props.keys.get().length === 1}
          onClick={() => props.keys.set(removed(props.keys.get(), props.index))}
        >
          Remove keyframe
        </button>
      </div>
    </Dot>
  )
}
