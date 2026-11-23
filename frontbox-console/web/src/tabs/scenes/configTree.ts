import { batch, createContext, createEffect, createMemo, createSignal, on, untrack, type Accessor, type Signal } from 'solid-js'
import type { Prop } from '../../types/generated/Prop'
import type { Stop } from '../../types/generated/Stop'
import { ancestry, remapped, selectionNode, selectionOf, type ConfigNode, type ListPlace, type NodeKind } from './configNodes'
import { moved, removed, withTiming, type Lens, type Scene, type SceneLayer, type SceneValue, type Selection } from './sceneModel'

/**
 * The scene's nodes, which one is selected, and which are open. The tree on the timeline and the properties panel
 * share one, and it keeps its ids in step as the lists in the scene change. Selecting a node selects the layer
 * it's in for the preview, and a layer selected in the preview is selected here.
 */
export type ConfigTree = {
  nodes(): ConfigNode[]
  /** The node with this id, if the scene has it */
  find(id: string): ConfigNode | undefined
  selected(): string | null
  isSelected(id: string): boolean
  select(id: string | null): void
  isExpanded(id: string): boolean
  anyExpanded(): boolean
  expand(...ids: string[]): void
  toggle(id: string): void
  /** Opens every node in the tree with rows under it, and every group of layers */
  expandAll(): void
  /** Whether every node in the tree with rows under it, and every group of layers, is open */
  allExpanded(): boolean
  /** Whether a group of layers in the tree is open, by its id. Groups start open. */
  isGroupOpen(id: string): boolean
  setGroupOpen(id: string, open: boolean): void
  collapseAll(): void
  move(place: ListPlace, by: number): void
  remove(place: ListPlace): void
  /** Puts a copy of an item, as `change` makes it, just after it in its list, and selects the copy */
  duplicate(place: ListPlace, change?: (copy: unknown) => unknown): void
  /** Shifts the nodes of a list from `index` on down a place, for an item put in at `index` */
  insert(listId: string, index: number): void
  /** The place of the row being dragged, which can be dropped elsewhere in its list */
  dragging(): ListPlace | null
  setDragging(place: ListPlace | null): void
  /** Adds a layer to the end of the scene's, and selects it */
  addLayer(layer: SceneLayer): void
  /** Adds a sound layer playing `sound` from the start, and selects it */
  addSound(sound: string): void
}

export const ConfigTreeContext = createContext<ConfigTree>()

/** Whether a prop changes along the timeline, and so has a track */
export const hasTrack = (prop: Prop<SceneValue>) => 'Keyframes' in prop || 'Random' in prop || 'Modulated' in prop

/** Nodes that always have a row of their own: what a layer is made of that's set apart from it */
const OWN_ROWS: NodeKind[] = ['layer', 'sound', 'traversal', 'path', 'copy', 'fit', 'stroke']

/**
 * Whether a node has a row in the tree. A layer is one row, with its shape, its fill and their settings in its
 * properties. What's set apart from those has a row under it: a modifier, a prop that changes along the timeline, a
 * traversal, and a sweep's path.
 */
export function shownInTree(node: ConfigNode) {
  if (node.kind === 'prop') return (node.modifier !== undefined && !node.placement) || hasTrack(node.lens.get())
  return OWN_ROWS.includes(node.kind)
}

/** A row under another, and `context`, what it's in that has no row of its own, where that's worth saying, such as a fill */
export type TreeChild = { node: ConfigNode; context?: string }

/** The rows under a node: its children that have rows, and in the place of those that don't, the rows under them */
export function treeChildren(node: ConfigNode): TreeChild[] {
  const found: TreeChild[] = []
  const visit = (list: ConfigNode[], context: string | undefined) =>
    list.forEach((child) => {
      if (shownInTree(child)) found.push({ node: child, context })
      else visit(child.children ?? [], child.kind === 'fill' ? 'Fill' : context)
    })
  visit(node.children ?? [], undefined)
  return found
}

/** Spaces stops evenly in their order, from 0 to 1 */
export function spreadStops(stops: Stop[]): Stop[] {
  return stops.map((stop, i) => ({ ...stop, at: { Fixed: evenAt(i, stops.length) } }))
}

/** Whether stops are where spreading them would put them, so none has been placed by hand */
export function evenlySpread(stops: Stop[]) {
  return stops.every(({ at }, i) => 'Fixed' in at && Math.abs(at.Fixed - evenAt(i, stops.length)) < 1e-6)
}

/** Where the `index`th of `count` stops spaced evenly is */
const evenAt = (index: number, count: number) => (count > 1 ? index / (count - 1) : 0)

/** Stops after one is taken out: spaced evenly again if they were, or else left where they are */
function respaced(before: Stop[], after: Stop[]) {
  return evenlySpread(before) ? spreadStops(after) : after
}

/** Stops after one is moved in the list: the positions stay in their places, so its color is what moves */
function positionsKept(before: Stop[], after: Stop[]) {
  return after.map((stop, i) => ({ ...stop, at: before[i].at }))
}

/** `nodes` is empty until the scene can be read */
export function createConfigTree(scene: Lens<Scene>, nodes: Accessor<ConfigNode[]>, selection: Signal<Selection | null>): ConfigTree {
  const [open, setOpen] = selection

  const byId = createMemo(() => {
    const found = new Map<string, ConfigNode>()
    const visit = (list: ConfigNode[]) =>
      list.forEach((node) => {
        found.set(node.id, node)
        visit(node.children ?? [])
      })
    visit(nodes())
    return found
  })
  const find = (id: string) => byId().get(id)

  /** The nodes in the tree that have rows under them */
  const branches = createMemo(() => {
    const found: string[] = []
    const visit = (list: ConfigNode[]) =>
      list.filter(shownInTree).forEach((node) => {
        const children = treeChildren(node).map(({ node }) => node)
        if (children.length > 0) found.push(node.id)
        visit(children)
      })
    visit(find('scene')?.children ?? [])
    return found
  })

  const initial = untrack(open)
  const [selected, setSelected] = createSignal<string | null>(initial && selectionNode(initial))
  const [expanded, setExpanded] = createSignal<string[]>(initial ? ancestry(selectionNode(initial)) : [])
  const [dragging, setDragging] = createSignal<ListPlace | null>(null)
  // what's closed rather than what's open, so a new group starts open
  const [closedGroups, setClosedGroups] = createSignal<string[]>([])

  const expand = (...ids: string[]) => setExpanded((current) => [...new Set([...current, ...ids])])
  /** The node itself if it has a row, or else the nearest it's in that does, as what's selected is a row */
  const withRow = (id: string) =>
    ancestry(id)
      .reverse()
      .find((up) => {
        const node = untrack(() => find(up))
        return node !== undefined && shownInTree(node)
      }) ?? id
  // what's under the row is opened up to, so it can be seen
  const select = (id: string | null) =>
    batch(() => {
      const row = id === null || untrack(nodes).length === 0 ? id : withRow(id)
      setSelected(row)
      if (row !== null) expand(...ancestry(row).slice(0, -1))
      const next = selectionOf(row)
      if (!sameSelection(next, untrack(open))) setOpen(next)
    })

  /**
   * The ids are positions, so they follow a list's items as they move and go. Taking out what's selected selects its
   * list, or what the list is in where it has no node, such as transforms. The scene's layers and sounds are in nothing.
   */
  const relocate = (listId: string, to: (index: number) => number | null) => {
    setExpanded((ids) => remapped(ids, listId, to))
    const id = untrack(selected)
    if (id === null) return
    const owner = untrack(() => find(listId)) ? listId : listId.slice(0, listId.lastIndexOf('/'))
    select(remapped([id], listId, to)[0] ?? (listId.includes('/') ? owner : null))
  }

  // a layer selected elsewhere, such as by a click in the preview, is selected and opened up to
  createEffect(
    on(open, (current) => {
      if (sameSelection(selectionOf(untrack(selected)), current)) return
      if (current === null) return setSelected(null)
      const id = selectionNode(current)
      setSelected(id)
      expand(...ancestry(id))
    }),
  )

  // what's selected can leave the scene, such as a circle's radius once it's a rectangle, which selects what it was in.
  // It can also lose its row, such as a prop set back to a fixed value, which selects the row it's now part of.
  createEffect(() => {
    const id = selected()
    if (id === null || nodes().length === 0) return
    const node = find(id)
    if (node && !shownInTree(node)) return select(id)
    if (node) return
    select(
      ancestry(id)
        .reverse()
        .find((up) => up !== 'scene' && find(up) !== undefined) ?? null,
    )
  })

  const addLayer = (layer: SceneLayer) => {
    const layers = scene.get().layers
    const id = selectionNode({ layer: layers.length })
    batch(() => {
      scene.set({ ...scene.get(), layers: [...layers, layer] })
      select(id)
      expand(id)
    })
  }

  const addSound = (sound: string) => {
    // moving its start needs a timing to be in, and the timeline to be put on
    const timed = withTiming(scene.get())
    const id = `sounds/${timed.sounds.length}`
    batch(() => {
      scene.set({ ...timed, sounds: [...timed.sounds, { sound, start: 0 }] })
      select(id)
    })
  }

  return {
    nodes,
    find,
    selected,
    isSelected: (id) => selected() === id,
    select,
    isExpanded: (id) => expanded().includes(id),
    anyExpanded: () => expanded().length > 0,
    expand,
    toggle: (id) => setExpanded((ids) => (ids.includes(id) ? ids.filter((open) => open !== id) : [...ids, id])),
    expandAll: () =>
      batch(() => {
        setExpanded(branches())
        setClosedGroups([])
      }),
    allExpanded: () => closedGroups().length === 0 && branches().every((id) => expanded().includes(id)),
    isGroupOpen: (id) => !closedGroups().includes(id),
    setGroupOpen: (id, open) => setClosedGroups((ids) => (open ? ids.filter((closed) => closed !== id) : ids.includes(id) ? ids : [...ids, id])),
    collapseAll: () => setExpanded([]),
    move: ({ list, listId, index, spread }, by) =>
      batch(() => {
        const after = moved(list.get(), index, by)
        list.set(spread ? positionsKept(list.get() as Stop[], after as Stop[]) : after)
        // the ones it passes each shift a place toward where it was
        relocate(listId, (i) => (i === index ? index + by : by > 0 && i > index && i <= index + by ? i - 1 : by < 0 && i < index && i >= index + by ? i + 1 : i))
      }),
    remove: ({ list, listId, index, spread }) =>
      batch(() => {
        const after = removed(list.get(), index)
        list.set(spread ? respaced(list.get() as Stop[], after as Stop[]) : after)
        relocate(listId, (i) => (i === index ? null : i > index ? i - 1 : i))
      }),
    duplicate: ({ list, listId, index }, change = (copy) => copy) =>
      batch(() => {
        const items = list.get()
        const copy = change(JSON.parse(JSON.stringify(items[index])))
        list.set([...items.slice(0, index + 1), copy, ...items.slice(index + 1)])
        relocate(listId, (i) => (i > index ? i + 1 : i))
        select(`${listId}/${index + 1}`)
      }),
    insert: (listId, index) => relocate(listId, (i) => (i >= index ? i + 1 : i)),
    dragging,
    setDragging,
    addLayer,
    addSound,
  }
}

const sameSelection = (a: Selection | null, b: Selection | null) => a?.layer === b?.layer
