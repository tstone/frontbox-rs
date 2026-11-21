import { Editable } from '@ark-ui/solid/editable'
import { Menu } from '@ark-ui/solid/menu'
import { createTreeCollection, TreeView } from '@ark-ui/solid/tree-view'
import { batch, createContext, createEffect, createMemo, createSignal, For, Index, Match, on, onCleanup, onMount, Show, Switch, untrack, useContext, type JSX, type Signal } from 'solid-js'
import { Dynamic, Portal } from 'solid-js/web'
import * as icons from '../../components/icons'
import { selectThenToggle } from '../../components/treeClick'
import ScenePortal, { LayerColorContext, layerColorClass } from './ScenePortal'
import { NAMED_VALUE_LABELS, namedValueKind, ValueFields } from './ValueFields'
import { VariableFields, VariableSource } from './VariableFields'
import { contentImageInfo } from '../../state/contentImages'
import { contentFonts, fontLabel, refreshContentFonts } from '../../state/contentFonts'
import { contentSoundLength, contentSounds, refreshContentSounds } from '../../state/contentSounds'

import type { BezierPoint } from '../../types/generated/BezierPoint'
import type { Color } from '../../types/generated/Color'
import type { GradientStyle } from '../../types/generated/GradientStyle'
import type { FillNode } from '../../types/generated/FillNode'
import type { ImageInfo } from '../../types/generated/ImageInfo'
import type { ImageSize } from '../../types/generated/ImageSize'
import type { Mask } from '../../types/generated/Mask'
import type { NodeRef } from '../../types/generated/NodeRef'
import type { NumberFormat } from '../../types/generated/NumberFormat'
import type { Prop } from '../../types/generated/Prop'
import type { ShapeNode } from '../../types/generated/ShapeNode'
import type { SoundLayer } from '../../types/generated/SoundLayer'
import type { Speed } from '../../types/generated/Speed'
import type { Stop } from '../../types/generated/Stop'
import type { StrokeAlign } from '../../types/generated/StrokeAlign'
import type { TextAlign } from '../../types/generated/TextAlign'
import type { TextSource } from '../../types/generated/TextSource'
import type { Transform } from '../../types/generated/Transform'
import type { Traversal } from '../../types/generated/Traversal'
import type { TraversalAlgorithm } from '../../types/generated/TraversalAlgorithm'
import type { TextOverflow } from '../../types/generated/TextOverflow'
import type { VerticalAlign } from '../../types/generated/VerticalAlign'
import {
  algorithmKind,
  ancestry,
  configNodes,
  pathTo,
  remapped,
  selectionNode,
  selectionOf,
  TRANSFORM_LABELS,
  WANDERING,
  type AlgorithmKind,
  type ConfigNode,
  type Fill,
  type ListPlace,
  type NodeKind,
  type PointsShape,
  type PropSpec,
  type Shape,
  type TransformKind,
  type Wandering,
} from './configNodes'
import { ImagePicker } from './ImagePicker'
import { SoundPicker } from './SoundPicker'
import { anchorPresets, writer } from './ShapeHandles'
import { displayNames, LayerLights, NEW_LAYER_OPTIONS, newPlace, planeNames, type NewLayerPlace } from './LayerPlace'
import {
  ColorField,
  Dropdown,
  EyeButton,
  FieldRow,
  LayerSurfacesContext,
  NodePickingContext,
  NumberBox,
  PropField,
  propKind,
  PropSource,
  RegionVisibilityContext,
  RevealContext,
  SceneContext,
  ShapePosesContext,
  SurfaceContext,
  TraversalsContext,
  type PropKind,
} from './SceneFields'
import {
  ADJUSTS,
  BULGE,
  BRANCHING,
  COPIES,
  COPY_KINDS,
  crispTextSize,
  FAN_ANGLE,
  FAN_COUNT,
  field,
  framesTrack,
  freshVariableName,
  freshTraversalName,
  GROWTH_DENSITY,
  imageLength,
  layerPlane,
  LEG_WIDTH,
  LEGS,
  moved,
  newAdjust,
  newLayerPlace,
  newShapeSize,
  newStroke,
  NOTCH,
  orEmpty,
  RATE,
  reachRange,
  removed,
  REVOLVE_ANGLE,
  shapePoints,
  snap,
  soundEnd,
  soundVolume,
  startingValue,
  TIME_STEP,
  traversalNames,
  traversalTarget,
  usedTraversals,
  variant,
  WANDER,
  RESHAPE_KINDS,
  SHIFT,
  withDuration,
  withVariableValue,
  withTiming,
  withTraversalRenamed,
  type AdjustKind,
  type CopyKind,
  type ReshapeKind,
  type Lens,
  type Scene,
  type SceneLayer,
  type Selection,
  type Surface,
  type NamedValueKind,
  NAMED_VALUE_KINDS,
  withNewValue,
  withoutValue,
  type SceneValue,
  type Vec2,
} from './sceneModel'

type FillKind = 'Solid' | GradientStyle | 'Image' | 'Spritesheet'
type ImageFill = Extract<FillNode, { Image: unknown }>['Image']
type SpritesheetFill = Extract<FillNode, { Spritesheet: unknown }>['Spritesheet']
type ShapeKind = 'None' | 'Circle' | 'Ellipse' | 'Rect' | 'Diamond' | 'Star' | 'Chevron' | 'Text' | 'Polygon' | 'Polyline' | 'Bezier'
/** A shape set up from the others, which is a shape of their kind once it's picked */
type TemplateKind = 'Pinwheel' | 'Propeller' | 'Triangle'
/** What a layer's region can be: a shape, or on a light scene its graph */
type RegionKind = ShapeKind | 'Graph'
type Icon = typeof icons.Circle

/** How wide a triangle is, as a share of how tall it is, so that its sides are all the same length */
const TRIANGLE_WIDTH = 2 / Math.sqrt(3)
/** How wide a pinwheel's blades are where they're widest, as a share of how long they are */
const PINWHEEL_BLADE_WIDTH = 0.8
/** How wide a propeller's blades are, as a share of how long they are */
const PROPELLER_BLADE_WIDTH = 0.125
/** How many blades a template has */
const TEMPLATE_BLADES = 4

const WHITE: Color = [255, 255, 255, 255]
const BLACK: Color = [0, 0, 0, 255]

const FILL_OPTIONS: { value: FillKind; label: string }[] = [
  { value: 'Solid', label: 'Solid' },
  { value: 'Linear', label: 'Linear gradient' },
  { value: 'Radial', label: 'Radial gradient' },
  { value: 'Conical', label: 'Conical gradient' },
  { value: 'Image', label: 'Image' },
  { value: 'Spritesheet', label: 'Sprite sheet' },
]
const byLabel = (a: { label: string }, b: { label: string }) => a.label.localeCompare(b.label)
type ShapeOption = { value: RegionKind | TemplateKind; label: string; group?: string }
const SHAPES: ShapeOption[] = ([
  { value: 'Chevron', label: 'Chevron' },
  { value: 'Circle', label: 'Circle' },
  { value: 'Diamond', label: 'Diamond' },
  { value: 'Ellipse', label: 'Ellipse' },
  { value: 'Polygon', label: 'Polygon' },
  { value: 'Rect', label: 'Rectangle' },
  { value: 'Star', label: 'Star' },
  { value: 'Text', label: 'Text' },
] as ShapeOption[]).map((option) => ({ ...option, group: 'Shapes' }))
const TEMPLATES: ShapeOption[] = [
  { value: 'Pinwheel', label: 'Pinwheel', group: 'Templates' },
  { value: 'Propeller', label: 'Propeller', group: 'Templates' },
  { value: 'Triangle', label: 'Triangle', group: 'Templates' },
]
const isTemplate = (kind: RegionKind | TemplateKind): kind is TemplateKind => TEMPLATES.some((option) => option.value === kind)
const ALL: ShapeOption = { value: 'None', label: 'All' }
/** "All", then the shapes by label, then the templates made of them */
const SHAPE_OPTIONS: ShapeOption[] = [ALL, ...SHAPES, ...TEMPLATES]
/**
 * A path has to be something, and can also be a line, which has no inside to draw as a shape. A sweep follows only a
 * path's own outline, so it has no templates. Text isn't offered, as only its box is an outline to follow.
 */
const PATH_OPTIONS: ShapeOption[] = [
  ...SHAPES.filter((option) => option.value !== 'Text'),
  { value: 'Polyline', label: 'Polyline', group: 'Shapes' } as ShapeOption,
  { value: 'Bezier', label: 'Bezier', group: 'Shapes' } as ShapeOption,
].sort(byLabel)
const TRANSFORM_OPTIONS: { value: TransformKind; label: string }[] = (['Translate', 'Rotate', 'Scale'] as const).map((value) => ({ value, label: TRANSFORM_LABELS[value] }))
/** Only for a shape that's drawn, rather than a sweep's path */
const SHAPE_TRANSFORM_OPTIONS: { value: TransformKind; label: string }[] = [...COPY_KINDS, ...RESHAPE_KINDS].map((value) => ({ value, label: TRANSFORM_LABELS[value] }))
/** Only for a layer on a light scene */
const GRAPH_REGION_OPTIONS: ShapeOption[] = [ALL, { value: 'Graph', label: 'Graph' }, ...SHAPES, ...TEMPLATES]
/** Only for a layer with traversals to fit to */
const FIT_OPTION: { value: TransformKind; label: string } = { value: 'Fit', label: TRANSFORM_LABELS.Fit }
const ALGORITHM_OPTIONS: { value: AlgorithmKind; label: string }[] = [
  { value: 'DepthFirst', label: 'Depth first' },
  { value: 'ShortestPath', label: 'Direct path (targeted)' },
  { value: 'AStar', label: 'Exploratory path (targeted)' },
  { value: 'FanOut', label: 'Fan out (targeted)' },
  { value: 'Growth', label: 'Growth (targeted)' },
  { value: 'Dijkstra', label: 'Fill by distance' },
  { value: 'BreadthFirst', label: 'Fill by steps' },
  { value: 'RandomWalk', label: 'Random walk' },
]
const NODE_OPTIONS: { value: 'Name' | 'Nearest'; label: string }[] = [
  { value: 'Name', label: 'Light' },
  { value: 'Nearest', label: 'Nearest to a position' },
]
const MASK_SHOWS_OPTIONS: { value: 'Inside' | 'Outside'; label: string }[] = [
  { value: 'Outside', label: 'Outside the mask' },
  { value: 'Inside', label: 'Inside the mask' },
]
const ADJUST_OPTIONS = (Object.keys(ADJUSTS) as AdjustKind[]).map((value) => ({ value, label: ADJUSTS[value].label }))
/** Only for a shape that's drawn, rather than a sweep's path */
const STROKE_OPTION: { value: 'Stroke'; label: string } = { value: 'Stroke', label: 'Stroke' }
type TextSourceKind = 'Fixed' | 'Variable' | 'Number'
const TEXT_SOURCE_OPTIONS: { value: TextSourceKind; label: string }[] = [
  { value: 'Fixed', label: 'Fixed' },
  { value: 'Variable', label: 'Variable' },
  { value: 'Number', label: 'Number' },
]
type NumberFormatKind = 'Grouped' | 'Plain' | 'Abbreviated'
const NUMBER_FORMAT_OPTIONS: { value: NumberFormatKind; label: string }[] = [
  { value: 'Grouped', label: 'Grouped (12,942)' },
  { value: 'Plain', label: 'Plain (12942)' },
  { value: 'Abbreviated', label: 'Abbreviated (12.9k)' },
]
/** A new number variable's default, long enough to show grouping and abbreviating */
const NUMBER_PREVIEW = 1_234_567
/** Characters an abbreviated number starts out fitting in */
const ABBREVIATED_LENGTH = 5
const TEXT_ALIGN_OPTIONS: { value: TextAlign; label: string }[] = [
  { value: 'Left', label: 'Left' },
  { value: 'Center', label: 'Center' },
  { value: 'Right', label: 'Right' },
]
const VERTICAL_ALIGN_OPTIONS: { value: VerticalAlign; label: string }[] = [
  { value: 'Top', label: 'Top' },
  { value: 'Middle', label: 'Middle' },
  { value: 'Bottom', label: 'Bottom' },
]
const TEXT_OVERFLOW_OPTIONS: { value: TextOverflow; label: string }[] = [
  { value: 'Wrap', label: 'Wraps onto more lines' },
  { value: 'Ellipsis', label: 'Cut short with …' },
]
const IMAGE_SIZE_OPTIONS: { value: ImageSize; label: string }[] = [
  { value: 'Stretch', label: 'Stretch to fit' },
  { value: 'Native', label: 'Native (a pixel per unit)' },
]
const STROKE_ALIGN_OPTIONS: { value: StrokeAlign; label: string }[] = [
  { value: 'Inside', label: 'Inside the edge' },
  { value: 'Centered', label: 'Centered on the edge' },
  { value: 'Outside', label: 'Outside the edge' },
]
const STROKE_DRAWS_OPTIONS: { value: 'Fill' | 'Color'; label: string }[] = [
  { value: 'Fill', label: "The layer's fill, without the inside" },
  { value: 'Color', label: 'A color, over the inside' },
]
const SPEED_OPTIONS: { value: SpeedKind; label: string }[] = [
  { value: 'Duration', label: 'Finish in a time' },
  { value: 'Rate', label: 'Rate' },
]

const SHAPE_ICONS: Record<RegionKind, Icon> = {
  None: icons.Scan,
  Circle: icons.Circle,
  Ellipse: icons.Egg,
  Rect: icons.Square,
  Polygon: icons.Pentagon,
  Diamond: icons.Diamond,
  Star: icons.Star,
  Chevron: icons.ChevronUp,
  Text: icons.Type,
  Polyline: icons.Activity,
  Bezier: icons.Spline,
  Graph: icons.Network,
}
const FILL_ICONS: Record<FillKind, Icon> = { Solid: icons.PaintBucket, Linear: icons.Blend, Radial: icons.Blend, Conical: icons.Blend, Image: icons.ImageIcon, Spritesheet: icons.ImageIcon }
/** A transform's or an adjust's icon says which kind it is, in place of where its value comes from */
const MODIFIER_ICONS: Record<TransformKind | AdjustKind, Icon> = {
  Translate: icons.Move,
  Rotate: icons.RotateCw,
  Scale: icons.Scaling,
  Fit: icons.Maximize2,
  Mirror: icons.SquareSplitHorizontal,
  Revolve: icons.RefreshCw,
  Bisect: icons.Scissors,
  Shift: icons.MoveVertical,
  Bulge: icons.Squircle,
  Round: icons.SquareRoundCorner,
  HueShift: icons.Palette,
  Opacity: icons.Droplet,
  Brightness: icons.Sun,
  Contrast: icons.Contrast,
}
/** A prop's icon says where its value comes from */
const SOURCE_ICONS: Record<PropKind, Icon> = {
  Fixed: icons.Pin,
  Variable: icons.Variable,
  Keyframes: icons.ChartSpline,
  Random: icons.Dices,
  Modulated: icons.AudioWaveform,
  Sweep: icons.Orbit,
  Visit: icons.Footprints,
}
const KIND_ICONS: Partial<Record<NodeKind, Icon>> = {
  place: icons.MapIcon,
  lights: icons.Lightbulb,
  z: icons.Layers2,
  mask: icons.SquareDashed,
  stroke: icons.PenLine,
  fit: icons.Maximize2,
  points: icons.Waypoints,
  'bezier-point': icons.Dot,
  traversals: icons.Route,
  traversal: icons.Footprints,
  'node-ref': icons.MapPin,
  variable: icons.Variable,
  value: icons.Palette,
}

type SpeedKind = 'Rate' | 'Duration'
type NewLayer = NewLayerPlace | 'Sound'

/** A node of a kind, or `false` for one of another */
function is<K extends NodeKind>(node: ConfigNode, kind: K) {
  return node.kind === kind ? (node as Extract<ConfigNode, { kind: K }>) : false
}

/** Moving around the config tree, and keeping it in step as the lists in it change */
type ConfigTree = {
  /** Selects a node, which selects the layer or mask it's in for the preview */
  select(id: string | null): void
  expand(...ids: string[]): void
  toggle(id: string): void
  /** Whether the node is the one selected */
  isSelected(id: string): boolean
  move(place: ListPlace, by: number): void
  remove(place: ListPlace): void
  /** Shifts the nodes of a list from `index` on down a place, for an item put in at `index` */
  insert(listId: string, index: number): void
  /** The place of the row being dragged, which can be dropped elsewhere in its list */
  dragging(): ListPlace | null
  setDragging(place: ListPlace | null): void
  /** The kinds of layer the scene can have added, and adding one */
  layerOptions(): { value: NewLayer; label: string }[]
  addLayer(kind: NewLayer): void
}

const ConfigTreeContext = createContext<ConfigTree>()

/**
 * The scene as a tree, with three roots: its layers and sound layers, its variables, and its named values. Each layer's
 * settings and props are nodes under it, and any of those with settings of its own has them under it in turn. Selecting a node selects the layer, or the mask, it's in. Its
 * button for closing every node goes in `toolbar`, and `onSelect` hears which node is selected, by id, and `null` once
 * the panel is gone.
 */
export default function ConfigPanel(props: { selection: Signal<Selection | null>; toolbar?: HTMLElement; onSelect?: (id: string | null) => void }) {
  const scene = useContext(SceneContext)!
  const surfaces = useContext(LayerSurfacesContext)!
  const [revealing] = useContext(RevealContext)!
  const graphNodes = useGraphNodes()
  const [open, setOpen] = props.selection
  onMount(refreshContentSounds)
  onMount(refreshContentFonts)

  const nodes = createMemo(() => configNodes(scene, surfaces))
  const collection = createMemo(() =>
    createTreeCollection<ConfigNode>({
      nodeToValue: (node) => node.id,
      nodeToString: (node) => node.label,
      rootNode: { id: '', label: '', children: nodes() } as unknown as ConfigNode,
    }),
  )

  // A branch's content only opens with it if it's open from the start or opened once it's up, so what's open to begin
  // with is set here rather than by the effect below, and anything added and opened at once is opened in the same batch.
  const initial = untrack(open)
  const [selected, setSelected] = createSignal<string | null>(initial && selectionNode(initial))
  const [expanded, setExpanded] = createSignal<string[]>(initial ? ancestry(selectionNode(initial)) : ['scene'])
  createEffect(() => props.onSelect?.(selected()))
  onCleanup(() => props.onSelect?.(null))

  const select = (id: string | null) =>
    batch(() => {
      setSelected(id)
      const next = selectionOf(id)
      if (!sameSelection(next, untrack(open))) setOpen(next)
    })
  const expand = (...ids: string[]) => setExpanded((current) => [...new Set([...current, ...ids])])
  const [dragging, setDragging] = createSignal<ListPlace | null>(null)

  /**
   * The tree's ids are positions, so they follow a list's items as they move and go. Taking out what's selected selects
   * its list, or what the list is in where it has no node, such as transforms. The scene's layers and sounds are in nothing.
   */
  const relocate = (listId: string, to: (index: number) => number | null) => {
    setExpanded((ids) => remapped(ids, listId, to))
    const id = untrack(selected)
    if (id === null) return
    const owner = untrack(collection).findNode(listId) ? listId : listId.slice(0, listId.lastIndexOf('/'))
    select(remapped([id], listId, to)[0] ?? (listId.includes('/') ? owner : null))
  }

  const tree: ConfigTree = {
    select,
    expand,
    isSelected: (id) => selected() === id,
    toggle: (id) => setExpanded((ids) => (ids.includes(id) ? ids.filter((open) => open !== id) : [...ids, id])),
    move: ({ list, listId, index, spread }, by) =>
      batch(() => {
        const after = moved(list.get(), index, by)
        list.set(spread ? spreadStops(after as Stop[]) : after)
        // the ones it passes each shift a place toward where it was
        relocate(listId, (i) => (i === index ? index + by : by > 0 && i > index && i <= index + by ? i - 1 : by < 0 && i < index && i >= index + by ? i + 1 : i))
      }),
    remove: ({ list, listId, index, spread }) =>
      batch(() => {
        const after = removed(list.get(), index)
        list.set(spread ? spreadStops(after as Stop[]) : after)
        relocate(listId, (i) => (i === index ? null : i > index ? i - 1 : i))
      }),
    insert: (listId, index) => relocate(listId, (i) => (i >= index ? i + 1 : i)),
    dragging,
    setDragging,
    layerOptions: () => options(),
    addLayer: (kind) => add(kind),
  }

  // a layer opened from elsewhere, such as by a click in the preview, is selected, opened up, and scrolled to in the tree
  let treeElement!: HTMLDivElement
  createEffect(
    on(open, (current) => {
      if (sameSelection(selectionOf(untrack(selected)), current)) return
      if (current === null) return setSelected(null)
      const id = selectionNode(current)
      setSelected(id)
      expand(...ancestry(id))
      scrollToRow(treeElement, id)
    }),
  )

  // a light layer from before layers started on every light has none, which the game can't play, so it's given every light
  createEffect(() => {
    const current = scene.get()
    if (current.layers.every((layer) => layer.display !== undefined || layer.lights !== undefined)) return
    scene.set({ ...current, layers: current.layers.map((layer) => (layer.display === undefined && layer.lights === undefined ? { ...layer, lights: 'Every' } : layer)) })
  })

  // a prop's field only comes up once every node it's under is open
  createEffect(() => {
    const prop = revealing()
    if (prop === null) return
    const path = untrack(() => pathTo(nodes(), prop))
    if (path) expand(...path)
  })

  /** A new layer is picked by kind, and goes on a plane or display that "Draws on" can change. An area needs a plane to cover. */
  const options = (): { value: NewLayer; label: string }[] => [
    ...NEW_LAYER_OPTIONS.filter(({ value }) => value !== 'Area' || planeNames().length > 0),
    { value: 'Sound', label: 'Sound' },
  ]

  const add = (kind: NewLayer) => {
    if (kind === 'Sound') {
      // moving its start needs a timing to be in, and the timeline to be put on
      const timed = withTiming(scene.get())
      const id = `sounds/${timed.sounds.length}`
      batch(() => {
        scene.set({ ...timed, sounds: [...timed.sounds, { sound: timed.sounds.at(-1)?.sound ?? contentSounds()[0] ?? '', start: 0 }] })
        select(id)
        expand('scene', id)
      })
      return
    }
    const place = newLayerPlace(scene.get(), newPlace(scene.get(), kind))
    const layers = scene.get().layers
    const id = selectionNode({ layer: layers.length, mask: false })
    batch(() => {
      scene.set({ ...scene.get(), layers: [...layers, { ...place, fill: { base: { Solid: { Fixed: WHITE } } } }] })
      select(id)
      expand('scene', id)
    })
  }

  return (
    <div class="scene-config">
      <Show when={props.toolbar}>
        {(toolbar) => (
          <Portal mount={toolbar()}>
            <button type="button" class="scene-icon-button" aria-label="Collapse all" title="Collapse all" disabled={expanded().length === 0} onClick={() => setExpanded([])}>
              <icons.ChevronsDownUp aria-hidden="true" />
            </button>
          </Portal>
        )}
      </Show>
      <ConfigTreeContext.Provider value={tree}>
        <TreeView.Root
          ref={treeElement}
          class="scene-config-tree"
          collection={collection()}
          selectionMode="single"
          selectedValue={selected() === null ? [] : [selected()!]}
          onSelectionChange={(details) => select(details.selectedValue[0] ?? null)}
          expandedValue={expanded()}
          onExpandedChange={(details) => setExpanded(details.expandedValue)}
          // a branch opens or closes on a click once it's selected, or on its arrow (`selectThenToggle`)
          expandOnClick={false}
          lazyMount
          unmountOnExit
        >
          <TreeView.Tree>
            <Index each={nodes()}>
              {(node, index) => (
                <Show when={node().id} keyed>
                  {(_id) => <TreeNode node={node()} indexPath={[index]} />}
                </Show>
              )}
            </Index>
          </TreeView.Tree>
        </TreeView.Root>
      </ConfigTreeContext.Provider>
      <datalist id={LIGHT_NAMES}>
        <For each={graphNodes()}>{(name) => <option value={name} />}</For>
      </datalist>
    </div>
  )
}

const sameSelection = (a: Selection | null, b: Selection | null) => a?.layer === b?.layer && a?.mask === b?.mask

/**
 * Scrolls a node's row into view, if it isn't already. A branch only puts up what's in it once it's open, so this waits
 * a few frames for the row to be there.
 */
function scrollToRow(tree: HTMLElement, id: string, frames = 5) {
  requestAnimationFrame(() => {
    const row = tree.querySelector(`:is([data-part="item"], [data-part="branch-control"])[data-value="${CSS.escape(id)}"]`)
    if (row) row.scrollIntoView({ block: 'nearest' })
    else if (frames > 1) scrollToRow(tree, id, frames - 1)
  })
}

/** A node's row, then its fields and the nodes under it while it's open */
function TreeNode(props: { node: ConfigNode; indexPath: number[] }) {
  const tree = useContext(ConfigTreeContext)!
  const isBranch = () => (props.node.children?.length ?? 0) > 0 || props.node.childrenCount !== undefined
  const drag = rowDrag(props.node, tree)

  return (
    <TreeView.NodeProvider node={props.node} indexPath={props.indexPath}>
      <NodeScope node={props.node}>
        <Show
          when={isBranch()}
          fallback={
            <TreeView.Item class="scene-tree-row" {...drag}>
              <NodeRow node={props.node} depth={props.indexPath.length} />
            </TreeView.Item>
          }
        >
          <TreeView.Branch class={props.node.kind === 'sound' ? layerColorClass('sound') : undefined}>
            <TreeView.BranchControl
              class="scene-tree-row"
              {...drag}
              {...selectThenToggle(() => tree.isSelected(props.node.id), () => tree.toggle(props.node.id))}
            >
              <TreeView.BranchIndicator>
                <icons.ChevronRight aria-hidden="true" />
              </TreeView.BranchIndicator>
              <NodeRow node={props.node} depth={props.indexPath.length} />
            </TreeView.BranchControl>
            <TreeView.BranchContent>
              <TreeView.BranchIndentGuide />
              <Show when={props.node.fields}>
                {/* working in a node's fields selects it, as clicking its row does */}
                <div class="scene-tree-fields" onPointerDown={() => tree.select(props.node.id)} onFocusIn={() => tree.select(props.node.id)}>
                  <NodeFields node={props.node} />
                </div>
              </Show>
              <Index each={props.node.children ?? []}>
                {(child, index) => (
                  <Show when={child().id} keyed>
                    {(_id) => <TreeNode node={child()} indexPath={[...props.indexPath, index]} />}
                  </Show>
                )}
              </Index>
            </TreeView.BranchContent>
          </TreeView.Branch>
        </Show>
      </NodeScope>
    </TreeView.NodeProvider>
  )
}

/** What a layer's nodes are measured against and can respond to, and the accent color of a sound's */
function NodeScope(props: { node: ConfigNode; children: JSX.Element }) {
  const surfaces = useContext(LayerSurfacesContext)!
  return (
    <Switch fallback={props.children}>
      <Match when={is(props.node, 'layer')}>
        {(layer) => (
          <SurfaceContext.Provider value={() => surfaces(layer().lens.get())}>
            <TraversalsContext.Provider value={() => traversalNames(layer().lens.get())}>{props.children}</TraversalsContext.Provider>
          </SurfaceContext.Provider>
        )}
      </Match>
      <Match when={props.node.kind === 'traversal'}>
        {/* visits are what's being set up here, so its fields don't respond to them */}
        <TraversalsContext.Provider value={() => []}>{props.children}</TraversalsContext.Provider>
      </Match>
      <Match when={props.node.kind === 'sound'}>
        <LayerColorContext.Provider value="sound">{props.children}</LayerColorContext.Provider>
      </Match>
    </Switch>
  )
}

/**
 * A node's icon and label, and the controls that fit beside them. A root or a layer says what it is in words instead of an
 * icon, and only the nodes right under a layer have one, as more than that is too busy.
 */
function NodeRow(props: { node: ConfigNode; depth: number }) {
  const scene = useContext(SceneContext)!
  // the scene's nodes are a level down, under its root
  const hasIcon = () => props.depth <= 3 && !['section', 'layer', 'sound', 'modifiers'].includes(props.node.kind)
  const label = <span class={hasIcon() ? 'scene-tree-label' : 'scene-tree-label scene-tree-label-wide'}>{props.node.label}</span>
  return (
    <>
      <Show when={hasIcon()}>
        <NodeIcon node={props.node} />
      </Show>
      <Switch fallback={label}>
        <Match when={is(props.node, 'section')}>
          {(section) => (
            <>
              {label}
              <Count of={section().children} />
              <Switch>
                <Match when={section().section === 'scene'}>
                  <AddLayer node={section()} />
                </Match>
                <Match when={section().section === 'values'}>
                  <AddValue node={section()} />
                </Match>
              </Switch>
            </>
          )}
        </Match>
        <Match when={is(props.node, 'variable')}>
          {(variable) => (
            <>
              {label}
              <Controls node={props.node} class="scene-tree-end scene-tree-source">
                <VariableSource name={variable().name} />
              </Controls>
            </>
          )}
        </Match>
        <Match when={is(props.node, 'value')}>
          {(value) => (
            <>
              {label}
              <span class="scene-tree-summary">{NAMED_VALUE_LABELS[namedValueKind(scene.get().values![value().name]!)]}</span>
              <Controls node={props.node} class="scene-tree-end">
                <button
                  type="button"
                  class="scene-icon-button"
                  aria-label={`Remove value ${value().name}`}
                  title="Remove"
                  onClick={() => scene.set(withoutValue(scene.get(), value().name))}
                >
                  <icons.X aria-hidden="true" />
                </button>
              </Controls>
            </>
          )}
        </Match>
        <Match when={is(props.node, 'layer') || is(props.node, 'sound')}>
          {(layer) => <LayerRow node={layer()} />}
        </Match>
        <Match when={is(props.node, 'place')}>
          {(place) => {
            const display = () => place().lens.get().display
            const current = () => display() ?? layerPlane(place().lens.get())!
            // one on a plane or display the machine doesn't have stays chosen, so it can be seen and changed
            const options = () => {
              const names = display() !== undefined ? displayNames(scene.get()) : planeNames()
              return [...new Set([...names, current()])].map((name) => ({ value: name, label: name }))
            }
            const choose = (name: string) => {
              if (display() !== undefined) field(place().lens, 'display').set(name)
              else field(place().lens, 'surface').set({ Area: name })
            }
            return (
              <>
                {label}
                <Controls node={props.node}>
                  <Dropdown label="Draws on" options={options()} value={current()} onChange={choose} />
                </Controls>
              </>
            )
          }}
        </Match>
        <Match when={is(props.node, 'lights')}>
          {(lights) => (
            <>
              {label}
              <Controls node={props.node}>
                <LayerLights lens={lights().lens} />
              </Controls>
            </>
          )}
        </Match>
        <Match when={is(props.node, 'z')}>
          {(z) => (
            <>
              {label}
              <Controls node={props.node}>
                <NumberBox class="scene-tree-number" aria-label="Z-index" step="1" value={z().lens.get().z ?? 0} onNumber={(value) => field(z().lens, 'z').set(Math.round(value) || undefined)} />
              </Controls>
            </>
          )}
        </Match>
        <Match when={is(props.node, 'region') || is(props.node, 'path')}>
          {(region) => (
            <>
              {label}
              <ShapeKindPicker node={region()} />
            </>
          )}
        </Match>
        <Match when={is(props.node, 'fill')}>
          {(fill) => (
            <>
              {label}
              <FillKindPicker node={fill()} />
            </>
          )}
        </Match>
        <Match when={is(props.node, 'stops')}>
          {(stops) => (
            <>
              {label}
              <Count of={stops().children} />
              <AddStop node={stops()} />
            </>
          )}
        </Match>
        <Match when={is(props.node, 'mask')}>
          {(mask) => (
            <>
              {label}
              <MaskButton node={mask()} />
            </>
          )}
        </Match>
        <Match when={is(props.node, 'modifiers')}>{(modifiers) => <AddModifier node={modifiers()} />}</Match>
        <Match when={is(props.node, 'fit')}>
          {(fit) => (
            <>
              {label}
              <FitPicker node={fit()} />
            </>
          )}
        </Match>
        <Match when={is(props.node, 'points')}>
          {(points) => (
            <>
              {label}
              <Count of={points().children} />
              <AddPoint node={points()} />
            </>
          )}
        </Match>
        <Match when={is(props.node, 'traversals')}>
          {(traversals) => (
            <>
              {label}
              <Count of={traversals().children} />
              <AddTraversal node={traversals()} />
            </>
          )}
        </Match>
        <Match when={is(props.node, 'node-ref')}>
          {(ref) => (
            <>
              {label}
              <NodeRefKindPicker node={ref()} />
            </>
          )}
        </Match>
        <Match when={is(props.node, 'prop')}>
          {(prop) => (
            <>
              {label}
              <PropSummary node={props.node} spec={prop()} />
              <Controls node={props.node} class="scene-tree-end scene-tree-source">
                <PropSource label={props.node.label} {...propFieldSpec(prop())} />
              </Controls>
              {/* where the remove button would be, so every source lines up */}
              <Show when={!props.node.place}>
                <span class="scene-tree-gap" />
              </Show>
            </>
          )}
        </Match>
      </Switch>
      <Show when={props.node.kind !== 'layer' && props.node.kind !== 'sound' && props.node.place}>
        {(place) => <RemoveButton node={props.node} place={place()} />}
      </Show>
    </>
  )
}

function NodeIcon(props: { node: ConfigNode }) {
  const icon = (): Icon => {
    const node = props.node
    if (node.kind === 'region' || node.kind === 'path') return SHAPE_ICONS[regionKind(node.lens, node.kind === 'region' ? node.layer : undefined)]
    if (node.kind === 'fill') return FILL_ICONS[fillKind(node.lens)]
    if (node.modifier) return MODIFIER_ICONS[node.modifier]
    if (node.kind === 'prop') return SOURCE_ICONS[propKind(node.lens.get())]
    if (node.kind === 'place' && node.lens.get().display !== undefined) return icons.Monitor
    return KIND_ICONS[node.kind] ?? icons.Dot
  }
  return <Dynamic component={icon()} class="scene-tree-icon" aria-hidden="true" />
}

/**
 * Controls in a row. A row takes a click or a key from inside it as choosing the node, so they're kept from reaching it,
 * and a press on one selects the node instead.
 */
function Controls(props: { node: ConfigNode; class?: string; children: JSX.Element }) {
  const tree = useContext(ConfigTreeContext)!
  return (
    <span
      class={props.class ? `scene-tree-controls ${props.class}` : 'scene-tree-controls'}
      onClick={stopPropagation}
      onKeyDown={stopPropagation}
      onPointerDown={() => tree.select(props.node.id)}
    >
      {props.children}
    </span>
  )
}

function stopPropagation(e: Event) {
  e.stopPropagation()
}

/** How many there are in a list, while there are any */
function Count(props: { of: ConfigNode[] | undefined }) {
  return (
    <Show when={(props.of?.length ?? 0) > 0}>
      <span class="scene-tree-summary">{props.of!.length}</span>
    </Show>
  )
}

/** A layer's or a sound layer's name, which a double click edits, and its buttons */
function LayerRow(props: { node: Extract<ConfigNode, { kind: 'layer' | 'sound' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const layer = () => props.node.lens as Lens<{ name?: string; hidden?: boolean }>
  const scene = useContext(SceneContext)!
  const hidden = () => layer().get().hidden === true
  const type = () => {
    if (props.node.kind === 'sound') return 'Sound'
    const layer = props.node.lens.get() as SceneLayer
    if (layer.display !== undefined) return displayNames(scene.get()).length > 1 ? `Display · ${layer.display}` : 'Display'
    return layerPlane(layer) === null ? 'Light sequence' : 'Light area'
  }
  const open = () => {
    tree.select(props.node.id)
    tree.toggle(props.node.id)
  }
  return (
    <Controls node={props.node} class="scene-tree-grow">
      <LayerName lens={field(layer(), 'name')} placeholder={props.node.unnamed} hidden={hidden()} onClick={open} />
      <span class="scene-tree-summary scene-layer-type" onClick={open}>
        {type()}
      </span>
      <Show when={props.node.kind === 'layer'}>
        <EyeButton label={props.node.label} hidden={hidden()} onChange={(next) => field(layer(), 'hidden').set(next || undefined)} />
      </Show>
      <RemoveButton node={props.node} place={props.node.place!} bare />
    </Controls>
  )
}

/**
 * A layer's name in its row. A click does what a click on the row does, and a double click edits it. It's kept apart from
 * the row, as clicks in the box while editing would otherwise open and close the layer.
 */
function LayerName(props: { lens: Lens<string | undefined>; placeholder: string; hidden: boolean; onClick: () => void }) {
  // what's being typed, as the name only changes once that's done
  const [draft, setDraft] = createSignal<string | null>(null)

  return (
    <Editable.Root
      class="scene-item-name"
      classList={{ 'scene-item-hidden': props.hidden }}
      activationMode="dblclick"
      placeholder={props.placeholder}
      value={draft() ?? props.lens.get() ?? ''}
      onValueChange={(details) => setDraft(details.value)}
      onValueRevert={() => setDraft(null)}
      onValueCommit={(details) => {
        props.lens.set(details.value.trim() || undefined)
        setDraft(null)
      }}
    >
      <Editable.Area>
        <Editable.Input aria-label="Layer name" spellcheck={false} />
        <Editable.Preview title={props.lens.get() ?? props.placeholder} onClick={props.onClick} />
      </Editable.Area>
    </Editable.Root>
  )
}

/**
 * Dragging a row in a list to elsewhere in it, dropped before or after the row it's let go on. A press in a field or on a
 * button doesn't start a drag, so text can still be selected and buttons pressed.
 */
function rowDrag(node: ConfigNode, tree: ConfigTree) {
  const [grabbable, setGrabbable] = createSignal(false)
  const [drop, setDrop] = createSignal<'before' | 'after' | null>(null)
  const place = node.place
  const isDragged = () => {
    const from = tree.dragging()
    return !!place && from?.listId === place.listId && from.index === place.index
  }
  const accepts = () => {
    const from = tree.dragging()
    return !!place && from?.listId === place.listId && from.index !== place.index
  }
  return {
    get draggable() {
      return grabbable()
    },
    get 'data-dragging'() {
      return isDragged() || undefined
    },
    get 'data-drop'() {
      return drop() ?? undefined
    },
    onPointerDown: (e: PointerEvent) => setGrabbable(!!place && !(e.target as HTMLElement).closest('input, textarea, button, [contenteditable]')),
    onDragStart: (e: DragEvent) => {
      if (!place) return
      e.stopPropagation()
      e.dataTransfer!.effectAllowed = 'move'
      e.dataTransfer!.setData('text/plain', node.id)
      tree.setDragging(place)
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
      const side = drop()
      setDrop(null)
      if (!from || !place || !side || !accepts()) return
      e.preventDefault()
      // where it goes once it's out of the list, which moves those after it up one
      const at = place.index + (side === 'after' ? 1 : 0)
      const to = from.index < at ? at - 1 : at
      if (to !== from.index) tree.move(from, to - from.index)
    },
    onDragEnd: () => {
      tree.setDragging(null)
      setGrabbable(false)
    },
  }
}

/** Takes a node out of its list. Moving it is by dragging its row. */
function RemoveButton(props: { node: ConfigNode; place: ListPlace; bare?: boolean }) {
  const tree = useContext(ConfigTreeContext)!
  const button = () => (
    <span class="scene-tree-actions">
      <button type="button" class="scene-icon-button" aria-label={`Remove ${props.node.label}`} title="Remove" disabled={props.place.canRemove === false} onClick={() => tree.remove(props.place)}>
        <icons.X aria-hidden="true" />
      </button>
    </span>
  )
  return (
    <Show when={!props.bare} fallback={button()}>
      <Controls node={props.node} class="scene-tree-end">{button()}</Controls>
    </Show>
  )
}

/** The fields a node has of its own, above the nodes under it */
function NodeFields(props: { node: ConfigNode }) {
  return (
    <Switch>
      <Match when={is(props.node, 'section') && (props.node as { section: string }).section === 'variables'}>
        <p class="scene-note">Set a prop to Variable to add one.</p>
      </Match>
      <Match when={is(props.node, 'section') && (props.node as { section: string }).section === 'values'}>
        <p class="scene-note">Add a value, then point a variable at it from its default or from the game.</p>
      </Match>
      <Match when={is(props.node, 'variable')}>{(variable) => <VariableFields name={variable().name} />}</Match>
      <Match when={is(props.node, 'value')}>{(value) => <ValueFields name={value().name} />}</Match>
      <Match when={is(props.node, 'sound')}>{(sound) => <SoundFields lens={sound().lens} />}</Match>
      <Match when={is(props.node, 'region')}>
        {(region) => (
          <Show when={region().layer?.get().graph} fallback={<TextFields lens={region().lens as Lens<Shape>} layer={region().layer} />}>
            <GraphFields layer={region().layer!} />
          </Show>
        )}
      </Match>
      <Match when={is(props.node, 'fill')}>{(fill) => <FillFields lens={fill().lens} />}</Match>
      <Match when={is(props.node, 'mask')}>{(mask) => <MaskFields lens={mask().lens as Lens<Mask>} />}</Match>
      <Match when={is(props.node, 'stroke')}>{(stroke) => <StrokeFields node={stroke()} />}</Match>
      <Match when={is(props.node, 'traversal')}>{(traversal) => <TraversalFields lens={traversal().lens} layer={traversal().layer} />}</Match>
      <Match when={is(props.node, 'node-ref')}>{(ref) => <NodeRefFields label={ref().label} lens={ref().lens} />}</Match>
      <Match when={is(props.node, 'prop')}>
        {(prop) => (
          <>
            <PropField bare label={prop().label} {...propFieldSpec(prop())} />
            <Show when={prop().anchorOf}>{(of) => <AnchorPresets lens={prop().lens as Lens<Prop<Vec2>>} shape={of().shape} index={of().index} />}</Show>
          </>
        )}
      </Match>
    </Switch>
  )
}

/**
 * Buttons that put a scale's anchor on a corner or the center of the shape's box, as the machine last drew it. They're
 * off while there's no drawing of the shape to go by, or the anchor can't take a value, such as while it's random.
 */
function AnchorPresets(props: { lens: Lens<Prop<Vec2>>; shape: Lens<Shape>; index: number }) {
  const scene = useContext(SceneContext)!
  const poses = useContext(ShapePosesContext)
  const presets = () => {
    const pose = poses.of(props.shape.get())
    return pose && anchorPresets(pose, props.index)
  }
  const write = () => writer(scene, poses.time())
  const usable = () => presets() !== null && write().can(props.lens.get() as Prop<SceneValue>, [0, 0])
  return (
    <div class="scene-anchor-presets" role="group" aria-label="Put the anchor on">
      <For each={presets() ?? ANCHOR_SPOTS.map((label) => ({ label, anchor: [0, 0] as Vec2 }))}>
        {({ label, anchor }) => (
          <button
            type="button"
            class="scene-button scene-anchor-preset"
            title={`Anchor on the ${label.toLowerCase()} of the shape's box`}
            aria-label={label}
            disabled={!usable()}
            onClick={() => write().to(props.lens, anchor)}
          >
            <span class="scene-anchor-glyph" data-spot={label.toLowerCase().replace(' ', '-')} />
          </button>
        )}
      </For>
    </div>
  )
}

/** The anchor buttons, in their order, for while there's no drawing of the shape to place them by */
const ANCHOR_SPOTS = ['Top left', 'Bottom left', 'Center', 'Top right', 'Bottom right']

const propFieldSpec = (spec: PropSpec) => ({
  variableName: spec.variableName,
  lens: spec.lens,
  fallback: spec.fallback,
  ranges: spec.ranges,
})

/**
 * What a fixed prop is, or the variable or traversal a prop reads from, as a glance at it in its row. A prop that changes
 * over time shows nothing, as its source at the end of the row already says so.
 */
function PropSummary(props: { node: ConfigNode; spec: PropSpec }) {
  const prop = () => props.spec.lens.get()
  const text = () => {
    const current = prop()
    if ('Variable' in current) return current.Variable
    if ('Visit' in current) return current.Visit.traversal
    if ('Fixed' in current) {
      const value = current.Fixed
      if (typeof value === 'boolean') return value ? 'Yes' : 'No'
      if (typeof value === 'number') return formatNumber(value)
      if (value.length === 2) return value.map(formatNumber).join(', ')
    }
    return null
  }
  const isColor = () => {
    const current = prop()
    return 'Fixed' in current && Array.isArray(current.Fixed) && current.Fixed.length === 4
  }
  return (
    <Show when={isColor()} fallback={<span class="scene-tree-summary">{text()}</span>}>
      <Controls node={props.node}>
        <ColorField class="scene-tree-color" label={props.node.label} lens={variant(props.spec.lens, 'Fixed') as Lens<Color>} />
      </Controls>
    </Show>
  )
}

const formatNumber = (value: number) => String(Math.round(value * 100) / 100)

/** A sound layer's sound. How loud it is goes under it, and when it starts and ends is set on the timeline. */
function SoundFields(props: { lens: Lens<SoundLayer> }) {
  const scene = useContext(SceneContext)!
  /** What it's heard at when it's tried out */
  const startingVolume = () => startingValue(soundVolume(props.lens).get(), scene.get(), 1)
  const length = () => contentSoundLength(props.lens.get().sound)
  const end = () => soundEnd(props.lens.get(), length())

  return (
    <>
      <FieldRow label="Sound">
        <Show when={contentSounds().length > 0 || props.lens.get().sound !== ''} fallback={<span class="scene-note">No sounds in the machine's content folder</span>}>
          <SoundPicker lens={field(props.lens, 'sound')} volume={startingVolume()} />
        </Show>
      </FieldRow>
      <Show when={length()}>
        {(known) => (
          <div class="scene-fit">
            <span class="scene-note">{describeSound(props.lens.get(), known())}</span>
            <FitTimelineButton length={end()!} />
          </div>
        )}
      </Show>
    </>
  )
}

function describeSound({ start, skip = 0, end }: SoundLayer, length: number) {
  const skipped = skip > 0 ? `, the first ${skip.toFixed(2)} s skipped` : ''
  const cut = end === undefined || end - start >= length - skip ? '' : `, cut off after ${(end - start).toFixed(2)} s`
  return `${length.toFixed(2)} s long${skipped}${cut}`
}

/** A button for making the scene's timeline `length` long, to fit a sound or an image to it, then doing `onFit` */
function FitTimelineButton(props: { length: number; onFit?: () => void }) {
  const scene = useContext(SceneContext)!
  const length = () => snap(props.length)
  const fits = () => scene.get().timing?.duration === length()
  return (
    <button
      type="button"
      class="scene-button"
      disabled={fits() || length() < TIME_STEP}
      title={fits() ? 'The timeline is already this long' : `Make the timeline ${length().toFixed(2)} s long`}
      onClick={() =>
        batch(() => {
          scene.set(withDuration(scene.get(), length()))
          props.onFit?.()
        })
      }
    >
      Fit timeline
    </button>
  )
}

/** Adds a mask, which cuts a region out of the layer, as with no region a mask with the layer outside it hides all of it. Or takes it away. */
function MaskButton(props: { node: Extract<ConfigNode, { kind: 'mask' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const surface = useContext(SurfaceContext)!
  const add = () =>
    batch(() => {
      props.node.lens.set({ fill: { base: { Solid: { Fixed: WHITE } } }, region: newShape('Circle', surface()), invert: true })
      tree.select(props.node.id)
      tree.expand(props.node.id)
    })
  return (
    <Controls node={props.node} class="scene-tree-end">
      <Show
        when={props.node.lens.get()}
        fallback={
          <button type="button" class="scene-icon-button" aria-label="Add mask" title="Add mask" onClick={add}>
            <icons.Plus aria-hidden="true" />
          </button>
        }
      >
        <button type="button" class="scene-icon-button" aria-label="Remove mask" title="Remove mask" onClick={() => props.node.lens.set(undefined)}>
          <icons.X aria-hidden="true" />
        </button>
      </Show>
    </Controls>
  )
}

function MaskFields(props: { lens: Lens<Mask> }) {
  return (
    <FieldRow label="Show layer">
      <Dropdown
        label="Show layer"
        options={MASK_SHOWS_OPTIONS}
        value={props.lens.get().invert ? 'Outside' : 'Inside'}
        onChange={(value) => field(props.lens, 'invert').set(value === 'Outside' || undefined)}
      />
    </FieldRow>
  )
}

/** Which side of a shape's edge a stroke is on, and what it's drawn with */
function StrokeFields(props: { node: Extract<ConfigNode, { kind: 'stroke' }> }) {
  const stroke = () => variant(props.node.lens, 'Stroke')
  return (
    <>
      <FieldRow label="Align">
        <Dropdown
          label="Align"
          options={STROKE_ALIGN_OPTIONS}
          value={stroke().get().align ?? 'Centered'}
          onChange={(align) => field(stroke(), 'align').set(align === 'Centered' ? undefined : align)}
        />
      </FieldRow>
      <FieldRow label="Draw with">
        <Dropdown
          label="Draw with"
          options={STROKE_DRAWS_OPTIONS}
          value={stroke().get().color ? 'Color' : 'Fill'}
          onChange={(draws) => field(stroke(), 'color').set(draws === 'Color' ? { Fixed: WHITE } : undefined)}
        />
      </FieldRow>
    </>
  )
}

/** Text's font, its words, and where they sit in its box, which can be made to cover what the layer is on */
function TextFields(props: { lens: Lens<Shape>; layer?: Lens<SceneLayer> }) {
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const text = () => variant(field(props.lens, 'base') as Lens<ShapeNode>, 'Text')
  const source = () => field(text(), 'text')
  const sourceKind = () => Object.keys(source().get())[0] as TextSourceKind
  // the words it shows now, carried over to a fixed text or a variable's default
  const shown = () => {
    const current = source().get()
    if ('Fixed' in current) return current.Fixed
    const variable = 'Variable' in current ? scene.get().variables?.[current.Variable] : undefined
    return variable && 'Text' in variable ? variable.Text : 'TEXT'
  }
  const changeSource = (kind: TextSourceKind) => {
    if (kind === sourceKind()) return
    if (kind === 'Fixed') return source().set({ Fixed: shown() })
    if (kind === 'Number') {
      // a variable, as a number shown as text is nearly always the game's, with a default that previews the format
      const name = freshVariableName(scene.get(), 'number')
      scene.set(withVariableValue(scene.get(), name, { Int: NUMBER_PREVIEW }))
      return source().set({ Number: { value: { Variable: name }, format: 'Grouped' } })
    }
    const name = freshVariableName(scene.get(), 'text')
    scene.set(withVariableValue(scene.get(), name, { Text: shown() }))
    source().set({ Variable: name })
  }
  // one that isn't in the folder stays chosen, so it can be seen and changed
  const fonts = () => [...new Set([...contentFonts(), text().get().font])].filter((name) => name !== '').map((name) => ({ value: name, label: fontLabel(name) }))
  // a fixed size moves to the nearest the new font is crisp at
  const changeFont = (font: string) => {
    const { size } = text().get()
    text().set({ ...text().get(), font, size: 'Fixed' in size ? { Fixed: crispTextSize(font, size.Fixed) } : size })
  }
  // the display or plane the layer is on
  const place = () => {
    const layer = props.layer?.get()
    return layer?.display ?? (layer && layerPlane(layer)) ?? 'display'
  }
  const fill = ([width, height]: Vec2) => {
    const box = { ...text().get(), width: { Fixed: width }, height: { Fixed: height } }
    props.lens.set({ ...props.lens.get(), base: { Text: box }, transform: [{ Translate: { Fixed: [width / 2, height / 2] } }] })
  }
  return (
    <>
      <FieldRow label="Font">
        <Show when={fonts().length > 0} fallback={<span class="scene-note">No fonts in the machine's content folder</span>}>
          <Dropdown label="Font" options={fonts()} value={text().get().font} onChange={changeFont} />
        </Show>
      </FieldRow>
      <Show when={text().get().font !== '' && !contentFonts().includes(text().get().font)}>
        <p class="scene-note scene-note-warn">There's no font by this name in the content folder, so this draws nothing.</p>
      </Show>
      <FieldRow label="Text from">
        <Dropdown label="Text from" options={TEXT_SOURCE_OPTIONS} value={sourceKind()} onChange={changeSource} />
      </FieldRow>
      <Switch>
        <Match when={sourceKind() === 'Fixed'}>
          <FieldRow label="Text">
            <textarea
              class="scene-variable-name scene-text-words"
              aria-label="Text"
              rows={2}
              spellcheck={false}
              value={shown()}
              onInput={(e) => source().set({ Fixed: e.currentTarget.value })}
            />
          </FieldRow>
        </Match>
        <Match when={sourceKind() === 'Variable'}>
          <TextVariableField lens={source() as Lens<Extract<TextSource, { Variable: string }>>} />
        </Match>
        <Match when={sourceKind() === 'Number'}>
          <NumberFormatField lens={field(variant(source(), 'Number'), 'format')} />
        </Match>
      </Switch>
      <FieldRow label="Align">
        <Dropdown label="Align" options={TEXT_ALIGN_OPTIONS} value={text().get().align} onChange={(align) => field(text(), 'align').set(align)} />
      </FieldRow>
      <FieldRow label="Vertical align">
        <Dropdown
          label="Vertical align"
          options={VERTICAL_ALIGN_OPTIONS}
          value={text().get().vertical_align}
          onChange={(align) => field(text(), 'vertical_align').set(align)}
        />
      </FieldRow>
      <FieldRow label="Too wide">
        <Dropdown
          label="Too wide"
          options={TEXT_OVERFLOW_OPTIONS}
          value={text().get().overflow ?? 'Wrap'}
          // left out of the file while it wraps, as it does by default
          onChange={(overflow) => field(text(), 'overflow').set(overflow === 'Wrap' ? undefined : overflow)}
        />
      </FieldRow>
      <Show when={surface()}>
        {(size) => (
          <button
            type="button"
            class="scene-button scene-fields-button"
            title={`Sizes the text's box to the ${place()} and moves it over it, in place of its transforms`}
            onClick={() => fill(size())}
          >
            Fill {place()}
          </button>
        )}
      </Show>
    </>
  )
}

/** The variable a text reads its words from. A name nothing has yet becomes a variable, with the words it showed. */
function TextVariableField(props: { lens: Lens<Extract<TextSource, { Variable: string }>> }) {
  const scene = useContext(SceneContext)!
  const name = () => props.lens.get().Variable
  const variable = () => scene.get().variables?.[name()]
  const rename = (next: string) => {
    if (next === '' || next === name()) return
    const current = variable()
    if (scene.get().variables?.[next] === undefined) scene.set(withVariableValue(scene.get(), next, current && 'Text' in current ? current : { Text: '' }))
    props.lens.set({ Variable: next })
  }
  const readable = () => {
    const current = variable()
    return current !== undefined && ('Text' in current || 'Int' in current || 'Float' in current)
  }
  return (
    <>
      <FieldRow label="Variable">
        <input
          class="scene-variable-name"
          aria-label="Text variable name"
          spellcheck={false}
          value={name()}
          onChange={(e) => {
            rename(e.currentTarget.value.trim())
            e.currentTarget.value = name()
          }}
        />
      </FieldRow>
      <Show when={!readable()}>
        <p class="scene-note scene-note-warn">No text or number variable has that name, so this draws nothing.</p>
      </Show>
    </>
  )
}

/** How a number is written out, and for an abbreviated one, how many characters it fits in */
function NumberFormatField(props: { lens: Lens<NumberFormat> }) {
  const kind = (): NumberFormatKind => {
    const format = props.lens.get()
    return typeof format === 'string' ? format : 'Abbreviated'
  }
  const length = () => {
    const format = props.lens.get()
    return typeof format === 'string' ? ABBREVIATED_LENGTH : format.Abbreviated.length
  }
  return (
    <>
      <FieldRow label="Format">
        <Dropdown
          label="Format"
          options={NUMBER_FORMAT_OPTIONS}
          value={kind()}
          onChange={(next) => props.lens.set(next === 'Abbreviated' ? { Abbreviated: { length: length() } } : next)}
        />
      </FieldRow>
      <Show when={kind() === 'Abbreviated'}>
        <FieldRow label="Characters">
          <NumberBox
            class="scene-tree-number"
            aria-label="Characters"
            min="1"
            step="1"
            value={length()}
            onNumber={(value) => props.lens.set({ Abbreviated: { length: Math.max(1, Math.round(value)) } })}
          />
        </FieldRow>
      </Show>
    </>
  )
}

/** A graph built from a layer's lights, as its region. Its traversals, which props can respond to, are under it. */
function GraphFields(props: { layer: Lens<SceneLayer> }) {
  return (
    <>
      <p class="scene-note">
        Only lights in the graph light up on this layer. To show them as a traversal reaches them, set the layer's opacity to Visit. On a plane, lights
        more than half a unit above or below it, such as on ramps, aren't in the graph.
      </p>
      <Show when={props.layer.get().region}>
        <p class="scene-note scene-note-warn">This layer also has a region, set in its file, which still limits where it shows.</p>
        <button type="button" class="scene-button scene-fields-button" onClick={() => field(props.layer, 'region').set(undefined)}>
          Remove region
        </button>
      </Show>
    </>
  )
}

const LIGHT_NAMES = 'scene-light-names'

/**
 * A traversal from the first light in the graph out to all the others, over the rest of the cycle.
 * Before the graph has been drawn there are no lights to name, so it starts from whichever is nearest the middle.
 */
function newTraversal(name: string, nodes: string[], surface: Surface): Traversal {
  const [width, height] = surface ?? [1, 0]
  const from: NodeRef = nodes.length > 0 ? { Name: nodes[0] } : { Nearest: { Fixed: [width / 2, height / 2] } }
  return { name, from, algorithm: { BreadthFirst: { wander: { Fixed: 0 } } }, speed: { Duration: null }, start: 0 }
}

function AddTraversal(props: { node: Extract<ConfigNode, { kind: 'traversals' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const graphNodes = useGraphNodes()
  const add = () =>
    batch(() => {
      scene.set(withTiming(scene.get()))
      const list = props.node.lens.get()
      props.node.lens.set([...list, newTraversal(freshTraversalName(props.node.layer.get(), 'walk'), graphNodes(), surface())])
      tree.expand(props.node.id, `${props.node.id}/${list.length}`)
    })
  return <AddButton node={props.node} label="Add traversal" onClick={add} />
}

/** A traversal's name and how it goes. Where it goes from and to, and the props of how it goes, are under it. */
function TraversalFields(props: { lens: Lens<Traversal>; layer: Lens<SceneLayer> }) {
  const graphNodes = useGraphNodes()
  const surface = useContext(SurfaceContext)!
  const traversal = () => props.lens.get()
  const algorithm = () => field(props.lens, 'algorithm')
  const kind = () => algorithmKind(traversal().algorithm)
  const target = () => traversalTarget(algorithm())
  const wander = () =>
    field(variant(algorithm() as unknown as Lens<Record<Wandering, { wander: Prop<number> }>>, kind() as Wandering), 'wander')
  const speed = () => field(props.lens, 'speed')
  const speedKind = (): SpeedKind => ('Rate' in traversal().speed ? 'Rate' : 'Duration')
  const setSpeedKind = (next: SpeedKind) => {
    if (next === speedKind()) return
    speed().set(next === 'Rate' ? { Rate: { Fixed: RATE.rest! } } : ({ Duration: null } as Speed))
  }
  const used = createMemo(() => usedTraversals(props.layer.get()).has(traversal().name))

  // its responses and fits follow it to its new name
  const rename = (to: string) => {
    const from = traversal().name
    if (to === '' || to === from || traversalNames(props.layer.get()).includes(to)) return
    props.layer.set(withTraversalRenamed(props.layer.get(), from, to))
  }

  // a traversal going somewhere keeps going there as its algorithm changes, and keeps its wander
  const setKind = (next: AlgorithmKind) => {
    if (next === kind()) return
    // or to the far end of the graph, so a new one has somewhere to go
    const to = target()?.get() ?? { Name: graphNodes().at(-1) ?? '' }
    const keptWander: Prop<number> = WANDERING.includes(kind()) ? wander().get() : { Fixed: WANDER.rest! }
    if (next === 'ShortestPath') algorithm().set({ ShortestPath: { to, wander: keptWander } })
    else if (next === 'BreadthFirst') algorithm().set({ BreadthFirst: { wander: keptWander } })
    else if (next === 'Dijkstra') algorithm().set({ Dijkstra: { wander: keptWander } })
    else if (next === 'FanOut') algorithm().set({ FanOut: { to, angle: { Fixed: FAN_ANGLE.rest! }, count: { Fixed: FAN_COUNT.rest! }, wander: keptWander } })
    else if (next === 'Growth') {
      const growing = { reach: { Fixed: reachRange(surface()).rest! }, density: { Fixed: GROWTH_DENSITY.rest! }, branching: { Fixed: BRANCHING.rest! } }
      algorithm().set({ Growth: { to, ...growing, wander: keptWander } })
    } else if (next === 'AStar') algorithm().set({ AStar: { to } })
    else algorithm().set(next as TraversalAlgorithm)
  }

  return (
    <>
      <FieldRow label="Name">
        <input
          class="scene-variable-name"
          aria-label="Traversal name"
          spellcheck={false}
          value={traversal().name}
          onChange={(e) => {
            rename(e.currentTarget.value.trim())
            e.currentTarget.value = traversal().name
          }}
        />
      </FieldRow>
      <FieldRow label="Algorithm">
        <Dropdown label="Algorithm" options={ALGORITHM_OPTIONS} value={kind()} onChange={setKind} />
      </FieldRow>
      <Show when={!target()}>
        <p class="scene-note">Goes on to every light in the graph. The targeted algorithms go to one light.</p>
      </Show>
      <Show when={kind() === 'FanOut'}>
        <p class="scene-note">Also goes to random lights along an arc around From, through To, spread across it.</p>
      </Show>
      <Show when={kind() === 'Growth'}>
        <p class="scene-note">
          Sends offshoots from the path to as many random lights within Reach of it as Density. Branching is the chance an offshoot sends out one of its
          own.
        </p>
      </Show>
      <FieldRow label="Speed">
        <Dropdown label="Speed" options={SPEED_OPTIONS} value={speedKind()} onChange={setSpeedKind} />
      </FieldRow>
      <p class="scene-note">
        {speedKind() === 'Duration'
          ? 'Its start and how long it takes are set by its bar on the timeline.'
          : 'Its start is set by its bar on the timeline.'}
      </p>
      <Show when={!used()}>
        <p class="scene-note scene-note-warn">
          Nothing responds to {traversal().name}, so it doesn't change how the layer looks. Set an opacity, color or other value to Visit.
        </p>
      </Show>
    </>
  )
}

/** The lights in the selected layer's graph, by name, which are the only ones a traversal can start from or go to */
function useGraphNodes() {
  const picking = useContext(NodePickingContext)
  return () => picking?.nodes() ?? []
}

/** Whether a node of a graph is a light by name, or whichever is nearest a position, which is a prop under it */
function NodeRefKindPicker(props: { node: Extract<ConfigNode, { kind: 'node-ref' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const surface = useContext(SurfaceContext)!
  const graphNodes = useGraphNodes()
  const kind = () => ('Name' in props.node.lens.get() ? 'Name' : 'Nearest')
  const setKind = (next: 'Name' | 'Nearest') => {
    if (next === kind()) return
    const [width, height] = surface() ?? [1, 0]
    batch(() => {
      props.node.lens.set(next === 'Name' ? { Name: graphNodes()[0] ?? '' } : { Nearest: { Fixed: [width / 2, height / 2] } })
      tree.expand(props.node.id)
    })
  }
  return (
    <Controls node={props.node}>
      <Dropdown label={`${props.node.label} node`} options={NODE_OPTIONS} value={kind()} onChange={setKind} />
    </Controls>
  )
}

/** A light of the graph by name. It can be chosen by clicking it in the preview, and is pointed out there while this is hovered. */
function NodeRefFields(props: { label: string; lens: Lens<NodeRef> }) {
  const picking = useContext(NodePickingContext)
  const graphNodes = useGraphNodes()
  const name = () => (props.lens.get() as { Name: string }).Name

  // this field's own choosing, as another field's may be under way
  let mine: { choose: (name: string) => void } | null = null
  const choosing = () => picking !== undefined && picking.choosing[0]() !== null && picking.choosing[0]() === mine
  const toggleChoosing = () => {
    if (!picking) return
    const [, setChoosing] = picking.choosing
    if (choosing()) return setChoosing(null)
    mine = { choose: (chosen) => props.lens.set({ Name: chosen }) }
    setChoosing(mine)
  }
  onCleanup(() => {
    if (choosing()) picking!.choosing[1](null)
  })

  const point = (shown: boolean) => picking?.highlighted[1](shown ? name() : null)

  return (
    <div class="scene-field" onMouseEnter={() => point(true)} onMouseLeave={() => point(false)} onFocusIn={() => point(true)} onFocusOut={() => point(false)}>
      <div class="scene-node-pick">
        <input
          class="scene-variable-name"
          aria-label={`${props.label} light`}
          list={LIGHT_NAMES}
          spellcheck={false}
          value={name()}
          onChange={(e) => props.lens.set({ Name: e.currentTarget.value.trim() })}
        />
        <button
          type="button"
          class="scene-button"
          aria-pressed={choosing()}
          title={choosing() ? 'Stop choosing' : `Choose the ${props.label.toLowerCase()} light by clicking it in the preview`}
          onClick={toggleChoosing}
        >
          {choosing() ? 'Selecting…' : 'Select'}
        </button>
      </div>
      <Show when={graphNodes().length > 0 && !graphNodes().includes(name())}>
        <p class="scene-note scene-note-warn">This light isn't in the layer's graph, so the traversal reaches nothing from it.</p>
      </Show>
    </div>
  )
}

const fillKind = (lens: Lens<Fill>): FillKind => {
  const current = lens.get().base
  return 'Solid' in current ? 'Solid' : 'Gradient' in current ? current.Gradient.style : 'Image' in current ? 'Image' : 'Spritesheet'
}

/** What a fill is. A solid's color is under it, as is an animated image's frame. */
function FillKindPicker(props: { node: Extract<ConfigNode, { kind: 'fill' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const base = () => field(props.node.lens, 'base')
  const kind = () => fillKind(props.node.lens)

  const setKind = (next: FillKind) => {
    const current = base().get()
    const color = 'Solid' in current ? current.Solid : 'Gradient' in current ? current.Gradient.stops[0]?.color : undefined
    if (next === kind()) return
    batch(() => {
      if (next === 'Solid') {
        base().set({ Solid: color ?? { Fixed: WHITE } })
      } else if (next === 'Image') {
        const sheet = 'Spritesheet' in current ? current.Spritesheet : undefined
        base().set({ Image: { name: sheet?.name ?? '', size: sheet?.size } })
      } else if (next === 'Spritesheet') {
        const image = 'Image' in current ? current.Image : undefined
        base().set({ Spritesheet: { name: image?.name ?? '', rows: 1, columns: 1, size: image?.size } })
      } else if ('Gradient' in current) {
        base().set({ Gradient: { ...current.Gradient, style: next } })
      } else {
        const stops = [
          { at: 0, color: color ?? { Fixed: WHITE } },
          { at: 1, color: { Fixed: BLACK } },
        ]
        base().set({ Gradient: { stops, style: next } })
      }
      tree.expand(props.node.id)
    })
  }

  return (
    <Controls node={props.node}>
      <Dropdown label="Fill" options={FILL_OPTIONS} value={kind()} onChange={setKind} />
    </Controls>
  )
}

/** The image a fill shows. A gradient's stops are under it. */
function FillFields(props: { lens: Lens<Fill> }) {
  const base = field(props.lens, 'base')
  return (
    <Switch>
      <Match when={'Image' in base.get()}>
        <ImageEditor lens={variant(base, 'Image')} />
      </Match>
      <Match when={'Spritesheet' in base.get()}>
        <SpritesheetEditor lens={variant(base, 'Spritesheet')} />
      </Match>
    </Switch>
  )
}

function ImageEditor(props: { lens: Lens<ImageFill> }) {
  const scene = useContext(SceneContext)!
  const name = () => props.lens.get().name
  const info = () => contentImageInfo(name())
  const frame = () => props.lens.get().frame

  // whether a chosen image is animated is only known once the machine has decoded it
  let choosing: string | null = null

  // a frame's number means nothing in another image, but where a track or a variable leads is still wanted
  const choose = (next: string) => {
    if (next === name()) return
    const current = frame()
    choosing = next
    const size = props.lens.get().size
    props.lens.set(current && !('Fixed' in current) ? { name: next, frame: current, size } : { name: next, size })
  }

  // an animated image plays from the start, unless it's been given something else to do
  createEffect(
    on(info, (known) => {
      if (!known || name() !== choosing) return
      choosing = null
      if (known.frames <= 1 || frame() !== undefined) return
      const timed = withTiming(scene.get(), imageLength(known))
      scene.set(timed)
      field(props.lens, 'frame').set(framesTrack(known, timed.timing!))
    }),
  )

  const changes = () => (info()?.frames ?? 1) > 1 && frame() !== undefined && !('Fixed' in frame()!) && !('Variable' in frame()!)

  return (
    <>
      <ImagePicker lens={{ get: name, set: choose }} />
      <ImageSizeField lens={field(props.lens, 'size')} />
      <Show when={info()}>
        {(known) => (
          <div class="scene-fit">
            <span class="scene-note">{describeImage(known())}</span>
            <Show when={known().frames > 1 && imageLength(known())}>
              {/* a track of its frames goes back to playing it once at its own speed, which is what fitting is for */}
              <FitTimelineButton length={imageLength(known())!} onFit={() => frame() && 'Keyframes' in frame()! && field(props.lens, 'frame').set(framesTrack(known(), scene.get().timing!))} />
            </Show>
          </div>
        )}
      </Show>
      <Show when={changes() && info()}>
        {(known) => <p class="scene-note">A fraction of a frame is dropped, so going up to {known().frames} gives every frame the same time.</p>}
      </Show>
    </>
  )
}

/** The sheet's image, and how many rows and columns of sprites it's laid out in */
function SpritesheetEditor(props: { lens: Lens<SpritesheetFill> }) {
  const info = () => contentImageInfo(props.lens.get().name)
  const count = (key: 'rows' | 'columns') => (value: number) => field(props.lens, key).set(Math.max(1, Math.round(value) || 1))
  const sprite = () => {
    const known = info()
    const { rows, columns } = props.lens.get()
    return known && `Each sprite is ${+(known.width / columns).toFixed(2)} × ${+(known.height / rows).toFixed(2)}`
  }
  return (
    <>
      <ImagePicker lens={field(props.lens, 'name')} />
      <FieldRow label="Rows">
        <NumberBox class="scene-tree-number" aria-label="Rows" min="1" step="1" value={props.lens.get().rows} onNumber={count('rows')} />
      </FieldRow>
      <FieldRow label="Columns">
        <NumberBox class="scene-tree-number" aria-label="Columns" min="1" step="1" value={props.lens.get().columns} onNumber={count('columns')} />
      </FieldRow>
      <ImageSizeField lens={field(props.lens, 'size')} />
      <Show when={sprite()}>{(text) => <span class="scene-note">{text()}</span>}</Show>
    </>
  )
}

/** Whether an image is stretched over its region or drawn at its own size, which a `Translate` then pans across */
function ImageSizeField(props: { lens: Lens<ImageSize | undefined> }) {
  return (
    <FieldRow label="Size">
      <Dropdown
        label="Size"
        options={IMAGE_SIZE_OPTIONS}
        value={props.lens.get() ?? 'Stretch'}
        onChange={(size) => props.lens.set(size === 'Stretch' ? undefined : size)}
      />
    </FieldRow>
  )
}

function describeImage({ width, height, frames, duration }: ImageInfo) {
  const size = `${width} × ${height}`
  if (frames <= 1) return size
  const length = duration > 0 ? `, ${duration.toFixed(2)} s long` : ''
  return `${size}, ${frames} frames (0 to ${frames - 1})${length}`
}

/** Adds a stop at the end, in the color of the last one */
function AddStop(props: { node: Extract<ConfigNode, { kind: 'stops' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const add = () => {
    const list = props.node.lens.get()
    // its own copy, as a prop is told from another by which one it is
    const color = list.length > 0 ? structuredClone(list[list.length - 1].color) : { Fixed: WHITE }
    batch(() => {
      props.node.lens.set(spreadStops([...list, { at: 1, color }]))
      tree.expand(props.node.id)
    })
  }
  return <AddButton node={props.node} label="Add stop" onClick={add} />
}

/** Spaces stops evenly in their order, from 0 to 1 */
function spreadStops(stops: Stop[]): Stop[] {
  return stops.map((stop, i) => ({ ...stop, at: stops.length > 1 ? i / (stops.length - 1) : 0 }))
}

/** What a region or a path is. A graph is only for a layer's region, and only on a plane. */
function regionKind(lens: Lens<Shape | undefined>, layer?: Lens<SceneLayer>): RegionKind {
  if (layer?.get().graph) return 'Graph'
  const current = lens.get()
  return current ? (Object.keys(current.base)[0] as ShapeKind) : 'None'
}

/**
 * Picks what a mask's region, a layer's, or a sweep's path is, which can't be left out. On a light scene, a layer's region
 * can also be its graph. In the file the two are separate, and a layer could have both, but the graph already says which
 * lights it shows on. A region has its outline in the preview, which can be hidden.
 */
function ShapeKindPicker(props: { node: Extract<ConfigNode, { kind: 'region' | 'path' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const surface = useContext(SurfaceContext)!
  const scene = useContext(SceneContext)!
  const graphNodes = useGraphNodes()
  const visibility = useContext(RegionVisibilityContext)
  const path = () => props.node.kind === 'path'
  const layer = () => (props.node.kind === 'region' ? props.node.layer : undefined)
  const lens = () => props.node.lens as Lens<Shape | undefined>
  const base = () => field(lens() as Lens<Shape>, 'base') as Lens<ShapeNode>
  // a graph's nodes are where its lights are on a plane, so a sequence has none
  const graphs = () => layer() !== undefined && layerPlane(layer()!.get()) !== null
  const kind = () => regionKind(lens(), layer())
  const options = () => (path() ? PATH_OPTIONS : graphs() ? GRAPH_REGION_OPTIONS : SHAPE_OPTIONS)
  const of = () => selectionOf(props.node.id)

  const change = (next: RegionKind | TemplateKind) => {
    const owner = layer()
    if (next === 'Graph') {
      // only offered for a layer. A graph is there to be traversed, so it comes with a traversal.
      if (!owner) return
      scene.set(withTiming(scene.get()))
      const graph = { traversals: [newTraversal(freshTraversalName(owner.get(), 'walk'), graphNodes(), surface())] }
      return owner.set({ ...owner.get(), region: undefined, graph })
    }
    if (owner && kind() === 'Graph') {
      return owner.set({ ...owner.get(), graph: undefined, region: next === 'None' ? undefined : newShape(next, surface()) })
    }
    if (next === 'None') return lens().set(undefined)
    const current = lens().get()
    if (!current) return lens().set(newShape(next, surface()))
    // one shape turned into another takes up the same room
    const size = shapeSize(base().get(), scene.get())
    if (!isTemplate(next)) return base().set(sizedShape(next, size))
    // a template's copies are made before the shape is placed, in place of any it had
    const template = templateShape(next, Math.max(...size))
    const placing = (current.transform ?? []).filter((transform) => !COPY_KINDS.some((copy) => copy in transform))
    lens().set({ ...current, base: template.base, transform: [...template.transform, ...placing] })
  }

  return (
    <Controls node={props.node}>
      <Dropdown<RegionKind | TemplateKind>
        label={props.node.label}
        options={options()}
        value={kind()}
        onChange={(next) => {
          if (next === kind()) return
          batch(() => {
            change(next)
            if (next !== 'None') tree.expand(props.node.id)
          })
        }}
      />
      {/* what's selected has its region drawn over the preview */}
      <Show when={!path() && visibility && of() && kind() !== 'None'}>
        <EyeButton label="the region in the preview" hidden={!visibility!.shown(of()!)} onChange={(hidden) => visibility!.show(of()!, !hidden)} />
      </Show>
    </Controls>
  )
}

/** A new shape, placed so that it can be seen, at a size that fits. A template's transforms come before the one that places it. */
function newShape(kind: Exclude<ShapeKind, 'None'> | TemplateKind, surface: Surface): Shape {
  // a sequence runs from 0 to 1, with its points along the middle
  const [width, height] = surface ?? [1, 0]
  const across = newShapeSize(surface)
  const { base, transform } = isTemplate(kind) ? templateShape(kind, across) : { base: sizedShape(kind, [across, across]), transform: [] }
  return { base, transform: [...transform, { Translate: { Fixed: [width / 2, height / 2] } }] }
}

/** A template `across` wide, around the origin */
function templateShape(kind: TemplateKind, across: number): { base: ShapeNode; transform: Transform[] } {
  const length = across / 2
  // a star of one leg, pointing up from its base on the origin
  const triangle = (width: number): ShapeNode => ({ Star: { radius: { Fixed: length }, legs: { Fixed: 1 }, leg_width: { Fixed: width } } })
  if (kind === 'Triangle') return { base: triangle(TRIANGLE_WIDTH), transform: [] }
  if (kind === 'Pinwheel') {
    // its blades are triangles with their tips together
    return {
      base: triangle(PINWHEEL_BLADE_WIDTH),
      transform: [{ Revolve: { center: { Fixed: [0, -length] }, angle: { Fixed: 360 }, count: { Fixed: TEMPLATE_BLADES } } }],
    }
  }
  // each bar through the middle is two blades
  return {
    base: { Rect: { width: { Fixed: across }, height: { Fixed: across * PROPELLER_BLADE_WIDTH } } },
    transform: [{ Revolve: { center: { Fixed: [0, 0] }, angle: { Fixed: 180 }, count: { Fixed: TEMPLATE_BLADES / 2 } } }],
  }
}

/** A shape this wide and tall */
function sizedShape(kind: Exclude<ShapeKind, 'None'>, [width, height]: Vec2): ShapeNode {
  const reach = Math.max(width, height) / 2
  if (kind === 'Circle') return { Circle: { radius: { Fixed: reach } } }
  if (kind === 'Ellipse') return { Ellipse: { rx: { Fixed: width / 2 }, ry: { Fixed: height / 2 } } }
  if (kind === 'Rect') return { Rect: { width: { Fixed: width }, height: { Fixed: height } } }
  if (kind === 'Diamond') {
    return { Diamond: { width: { Fixed: width }, height: { Fixed: height } } }
  }
  if (kind === 'Star') {
    return { Star: { radius: { Fixed: reach }, legs: { Fixed: LEGS.rest! }, leg_width: { Fixed: LEG_WIDTH.rest! } } }
  }
  if (kind === 'Chevron') return { Chevron: { width: { Fixed: width }, height: { Fixed: height }, notch: { Fixed: NOTCH.rest! } } }
  if (kind === 'Text') {
    const font = contentFonts()[0] ?? ''
    const size = { Fixed: crispTextSize(font, Math.max(Math.round(height / 2), 1)) }
    return { Text: { text: { Fixed: 'TEXT' }, font, size, width: { Fixed: width }, height: { Fixed: height }, align: 'Center', vertical_align: 'Middle', kerning: { Fixed: 0 }, leading: { Fixed: 0 } } }
  }
  // a wave across it, level through each of its points
  const wave: Vec2[] = [
    [-width / 2, height / 2],
    [0, -height / 2],
    [width / 2, height / 2],
  ]
  if (kind === 'Polyline') return { Polyline: { points: wave.map((point) => ({ Fixed: point })) } }
  if (kind === 'Bezier') {
    return { Bezier: { points: wave.map((point) => ({ point: { Fixed: point }, handle: { Fixed: [width / 4, 0] } })) } }
  }
  const corners: Vec2[] = [
    [-width / 2, -height / 2],
    [width / 2, -height / 2],
    [width / 2, height / 2],
    [-width / 2, height / 2],
  ]
  return { Polygon: { points: corners.map((corner) => ({ Fixed: corner })) } }
}

/** How wide and tall a shape is in its own space */
function shapeSize(base: ShapeNode, scene: Scene): Vec2 {
  const value = (prop: Prop<number>) => startingValue(prop, scene, 0)
  if ('Circle' in base) return [value(base.Circle.radius) * 2, value(base.Circle.radius) * 2]
  if ('Ellipse' in base) return [value(base.Ellipse.rx) * 2, value(base.Ellipse.ry) * 2]
  if ('Rect' in base) return [value(base.Rect.width), value(base.Rect.height)]
  if ('Text' in base) return [value(base.Text.width), value(base.Text.height)]
  if ('Diamond' in base) return [value(base.Diamond.width), value(base.Diamond.height)]
  if ('Chevron' in base) return [value(base.Chevron.width), value(base.Chevron.height)]
  if ('Star' in base) return [value(base.Star.radius) * 2, value(base.Star.radius) * 2]
  const points = shapePoints(base).map((point) => startingValue(point, scene, [0, 0] as Vec2))
  const reach = (axis: 0 | 1) => Math.max(...points.map((p) => p[axis])) - Math.min(...points.map((p) => p[axis]))
  return [reach(0), reach(1)]
}

/**
 * Adds a point to a polygon halfway back to its first, so it keeps its outline. A line carries on from its end, and a
 * bezier's new point has the end's handle.
 */
function AddPoint(props: { node: Extract<ConfigNode, { kind: 'points' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const scene = useContext(SceneContext)!
  const at = (point: Prop<Vec2>) => startingValue(point, scene.get(), [0, 0] as Vec2)

  const added = (shape: PointsShape): unknown => {
    if (shape === 'Bezier') {
      const list = props.node.lens.get() as BezierPoint[]
      const last = list[list.length - 1]
      const point = pastEnd(at((list[list.length - 2] ?? last).point), at(last.point))
      return { point: { Fixed: point }, handle: { Fixed: at(last.handle) } }
    }
    const list = props.node.lens.get() as Prop<Vec2>[]
    const [first, last] = [at(list[0]), at(list[list.length - 1])]
    if (shape === 'Polyline') return { Fixed: pastEnd(at(list[list.length - 2] ?? list[0]), last) }
    return { Fixed: [(first[0] + last[0]) / 2, (first[1] + last[1]) / 2] }
  }

  const add = () =>
    batch(() => {
      const list = props.node.lens.get() as unknown[]
      ;(props.node.lens as Lens<unknown[]>).set([...list, added(props.node.shape)])
      tree.expand(props.node.id, `${props.node.id}/${list.length}`)
    })
  return <AddButton node={props.node} label="Add point" onClick={add} />
}

/** Half as far again along a line's last stretch */
function pastEnd(before: Vec2, end: Vec2): Vec2 {
  return [end[0] + (end[0] - before[0]) / 2, end[1] + (end[1] - before[1]) / 2]
}

/**
 * A transform that leaves things where they are, until it's changed. A fit starts on the layer's first traversal. A
 * mirror's line starts along the right of a shape `size` across, so its reflection is beside it, with its center at
 * the top corner, clear of the shape's move handle at the middle of that side. A revolve and a bisect start on its
 * middle.
 */
function newTransform(kind: TransformKind, traversals: string[], size: Vec2): Transform {
  if (kind === 'Translate') return { Translate: { Fixed: [0, 0] } }
  if (kind === 'Rotate') return { Rotate: { Fixed: 0 } }
  if (kind === 'Scale') return { Scale: { amount: { Fixed: 1 }, anchor: { Fixed: [0, 0] } } }
  if (kind === 'Mirror') return { Mirror: { center: { Fixed: [size[0] / 2, -size[1] / 2] }, rotation: { Fixed: 0 } } }
  if (kind === 'Revolve') return { Revolve: { center: { Fixed: [0, 0] }, angle: { Fixed: REVOLVE_ANGLE.rest! }, count: { Fixed: COPIES.rest! } } }
  if (kind === 'Bisect') return { Bisect: { center: { Fixed: [0, 0] }, rotation: { Fixed: 0 } } }
  if (kind === 'Shift') return { Shift: { amount: { Fixed: SHIFT.rest! }, rotation: { Fixed: 0 } } }
  if (kind === 'Bulge') return { Bulge: { Fixed: BULGE.rest! } }
  if (kind === 'Round') return { Round: { Fixed: 0 } }
  return { Fit: traversals[0] ?? '' }
}

/**
 * Adds a transform, an adjust, or a stroke, listed together, after those already there. A transform only for shapes
 * goes before a shape's first translate instead, so that it works in the shape's own space and moves with the shape.
 */
function AddModifier(props: { node: Extract<ConfigNode, { kind: 'modifiers' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const traversals = useContext(TraversalsContext)
  // a fit is only for a layer with traversals to fit to
  const transforms = () => [...TRANSFORM_OPTIONS, ...(props.node.shape ? SHAPE_TRANSFORM_OPTIONS : []), ...(traversals().length > 0 ? [FIT_OPTION] : [])]
  const groups = () => [transforms(), ...(props.node.adjusts ? [ADJUST_OPTIONS] : []), ...(props.node.shape ? [[STROKE_OPTION]] : [])]
  const owner = () => props.node.id.slice(0, props.node.id.lastIndexOf('/'))
  // a new modifier is selected and expanded, so its props show
  const show = (id: string) => {
    tree.select(id)
    tree.expand(id)
  }
  const add = (kind: TransformKind | AdjustKind | 'Stroke') =>
    batch(() => {
      if (kind === 'Stroke') {
        const strokes = orEmpty(field(props.node.shape!, 'adjust'))
        const list = strokes.get()
        strokes.set([...list, newStroke(surface())])
        show(`${owner()}/adjust/${list.length}`)
      } else if (kind in ADJUSTS) {
        const list = props.node.adjusts!.get()
        props.node.adjusts!.set([...list, newAdjust(kind as AdjustKind)])
        show(`${owner()}/adjust/${list.length}`)
      } else {
        const list = props.node.transforms.get()
        const shape = props.node.shape?.get()
        const size = shape ? shapeSize(shape.base, scene.get()) : ([0, 0] as Vec2)
        const placed = list.findIndex((transform) => 'Translate' in transform)
        // in the shape's own space, so it moves with the shape
        const inOwnSpace = COPY_KINDS.includes(kind as CopyKind) || RESHAPE_KINDS.includes(kind as ReshapeKind)
        const index = inOwnSpace && placed >= 0 ? placed : list.length
        props.node.transforms.set([...list.slice(0, index), newTransform(kind as TransformKind, traversals(), size), ...list.slice(index)])
        tree.insert(`${owner()}/transform`, index)
        show(`${owner()}/transform/${index}`)
      }
    })
  return (
    <Controls node={props.node}>
      <AddMenu label="Add a transform or an adjust" class="scene-add-modifier" groups={groups()} onPick={add}>
        <icons.Plus aria-hidden="true" />
        Modifier
      </AddMenu>
    </Controls>
  )
}

/** Which traversal a fit is to. It runs from where the traversal starts, at -0.5 across, to the light it reaches last, at 0.5. */
function FitPicker(props: { node: Extract<ConfigNode, { kind: 'fit' }> }) {
  const traversals = useContext(TraversalsContext)
  const fitted = () => (props.node.lens.get() as { Fit: string }).Fit
  // one the layer doesn't have anymore stays chosen, so it can be seen and changed
  const options = () => [...new Set([...traversals(), fitted()])].filter((name) => name !== '').map((name) => ({ value: name, label: name }))
  const missing = () => fitted() !== '' && !traversals().includes(fitted())
  return (
    <Controls node={props.node}>
      <Show when={options().length > 0} fallback={<span class="scene-tree-summary scene-note-warn">No traversals to fit to</span>}>
        <Dropdown label="Traversal to fit to" options={options()} value={fitted()} onChange={(name) => props.node.lens.set({ Fit: name })} />
      </Show>
      <Show when={missing()}>
        <span class="scene-tree-summary scene-note-warn" title="The layer has no traversal by this name, so this changes nothing.">
          Missing
        </span>
      </Show>
    </Controls>
  )
}

/** An add button for a list of several kinds of thing, which asks which kind. Groups are set apart by a line. */
function AddMenu<T extends string>(props: { label: string; class: string; groups: { value: T; label: string }[][]; onPick: (kind: T) => void; children: JSX.Element }) {
  return (
    <Menu.Root positioning={{ placement: 'bottom-start' }} onSelect={(details) => props.onPick(details.value as T)}>
      <Menu.Trigger class={props.class} aria-label={props.label} title={props.label}>
        {props.children}
      </Menu.Trigger>
      <ScenePortal>
        <Menu.Positioner>
          <Menu.Content class="scene-dropdown-list">
            <For each={props.groups}>
              {(group, index) => (
                <>
                  <Show when={index() > 0}>
                    <Menu.Separator />
                  </Show>
                  <For each={group}>
                    {(option) => (
                      <Menu.Item value={option.value}>
                        <Menu.ItemText>{option.label}</Menu.ItemText>
                      </Menu.Item>
                    )}
                  </For>
                </>
              )}
            </For>
          </Menu.Content>
        </Menu.Positioner>
      </ScenePortal>
    </Menu.Root>
  )
}

/** An add button in a list's row */
/** A layer added to the end of the scene, of a kind it can have */
function AddLayer(props: { node: ConfigNode }) {
  const tree = useContext(ConfigTreeContext)!
  return (
    <Controls node={props.node} class="scene-tree-end">
      <AddMenu label="Add layer" class="scene-icon-button" groups={[tree.layerOptions()]} onPick={tree.addLayer}>
        <icons.Plus aria-hidden="true" />
      </AddMenu>
    </Controls>
  )
}

/** A named value added to the scene, of the type picked */
function AddValue(props: { node: ConfigNode }) {
  const tree = useContext(ConfigTreeContext)!
  const scene = useContext(SceneContext)!
  const add = (kind: NamedValueKind) => {
    const before = new Set(Object.keys(scene.get().values ?? {}))
    const next = withNewValue(scene.get(), kind)
    const name = Object.keys(next.values ?? {}).find((value) => !before.has(value))!
    batch(() => {
      scene.set(next)
      tree.select(`values/${name}`)
      tree.expand('values', `values/${name}`)
    })
  }
  return (
    <Controls node={props.node} class="scene-tree-end">
      <AddMenu label="Add value" class="scene-icon-button" groups={[NAMED_VALUE_KINDS.map((kind) => ({ value: kind, label: NAMED_VALUE_LABELS[kind] }))]} onPick={add}>
        <icons.Plus aria-hidden="true" />
      </AddMenu>
    </Controls>
  )
}

function AddButton(props: { node: ConfigNode; label: string; onClick: () => void }) {
  return (
    <Controls node={props.node} class="scene-tree-end">
      <button type="button" class="scene-icon-button" aria-label={props.label} title={props.label} onClick={props.onClick}>
        <icons.Plus aria-hidden="true" />
      </button>
    </Controls>
  )
}
