import { Editable } from '@ark-ui/solid/editable'
import { Menu } from '@ark-ui/solid/menu'
import { createTreeCollection, TreeView } from '@ark-ui/solid/tree-view'
import { batch, createContext, createEffect, createMemo, createSignal, For, Index, Match, on, onCleanup, onMount, Show, Switch, untrack, useContext, type JSX, type Signal } from 'solid-js'
import { Dynamic, Portal } from 'solid-js/web'
import * as icons from '../../components/icons'
import ScenePortal, { LayerColorContext, layerColorClass } from './ScenePortal'
import { contentImageInfo } from '../../state/contentImages'
import { contentSoundLength, contentSounds, refreshContentSounds } from '../../state/contentSounds'

import type { BezierPoint } from '../../types/generated/BezierPoint'
import type { Color } from '../../types/generated/Color'
import type { GradientStyle } from '../../types/generated/GradientStyle'
import type { FillNode } from '../../types/generated/FillNode'
import type { ImageInfo } from '../../types/generated/ImageInfo'
import type { Mask } from '../../types/generated/Mask'
import type { NodeRef } from '../../types/generated/NodeRef'
import type { Prop } from '../../types/generated/Prop'
import type { ShapeNode } from '../../types/generated/ShapeNode'
import type { SoundLayer } from '../../types/generated/SoundLayer'
import type { Speed } from '../../types/generated/Speed'
import type { Stop } from '../../types/generated/Stop'
import type { Transform } from '../../types/generated/Transform'
import type { Traversal } from '../../types/generated/Traversal'
import type { TraversalAlgorithm } from '../../types/generated/TraversalAlgorithm'
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
import { LayerLeds, planeNames } from './LayerPlace'
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
  SoundPicker,
  SurfaceContext,
  TraversalsContext,
  type PropKind,
} from './SceneFields'
import {
  ADJUSTS,
  BOW,
  BRANCHING,
  COPIES,
  COPY_KINDS,
  FAN_ANGLE,
  FAN_COUNT,
  field,
  framesTrack,
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
  NOTCH,
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
  WIDEST_AT,
  withDuration,
  withTiming,
  withTraversalRenamed,
  type AdjustKind,
  type CopyKind,
  type Lens,
  type Scene,
  type SceneLayer,
  type Selection,
  type Surface,
  type Vec2,
} from './sceneModel'

type FillKind = 'Solid' | GradientStyle | 'Image'
type ImageFill = Extract<FillNode, { Image: unknown }>['Image']
type ShapeKind = 'None' | 'Circle' | 'Ring' | 'Ellipse' | 'Rect' | 'Diamond' | 'Star' | 'Chevron' | 'Polygon' | 'Polyline' | 'Bezier'
/** A shape set up from the others, which is a shape of their kind once it's picked */
type TemplateKind = 'Pinwheel' | 'Propeller' | 'Triangle'
/** What a layer's region can be: a shape, or on an LED scene its graph */
type RegionKind = ShapeKind | 'Graph'
type Icon = typeof icons.Circle

/** How much of a new ring's reach is its hole */
const RING_HOLE = 0.75
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
  { value: 'Ring', label: 'Ring' },
  { value: 'Star', label: 'Star' },
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
 * path's own outline, so it has no templates.
 */
const PATH_OPTIONS: ShapeOption[] = [
  ...SHAPES,
  { value: 'Polyline', label: 'Polyline', group: 'Shapes' } as ShapeOption,
  { value: 'Bezier', label: 'Bezier', group: 'Shapes' } as ShapeOption,
].sort(byLabel)
const TRANSFORM_OPTIONS: { value: TransformKind; label: string }[] = (['Translate', 'Rotate', 'Scale'] as const).map((value) => ({ value, label: TRANSFORM_LABELS[value] }))
/** Only for a shape that's drawn, rather than a sweep's path */
const COPY_OPTIONS: { value: TransformKind; label: string }[] = COPY_KINDS.map((value) => ({ value, label: TRANSFORM_LABELS[value] }))
/** Only for a layer on an LED scene */
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
  { value: 'Name', label: 'LED' },
  { value: 'Nearest', label: 'Nearest to a position' },
]
const MASK_SHOWS_OPTIONS: { value: 'Inside' | 'Outside'; label: string }[] = [
  { value: 'Outside', label: 'Outside the mask' },
  { value: 'Inside', label: 'Inside the mask' },
]
const ADJUST_OPTIONS = (Object.keys(ADJUSTS) as AdjustKind[]).map((value) => ({ value, label: ADJUSTS[value].label }))
const SPEED_OPTIONS: { value: SpeedKind; label: string }[] = [
  { value: 'Duration', label: 'Finish in a time' },
  { value: 'Rate', label: 'Rate' },
]

const SHAPE_ICONS: Record<RegionKind, Icon> = {
  None: icons.Scan,
  Circle: icons.Circle,
  Ring: icons.CircleDot,
  Ellipse: icons.Egg,
  Rect: icons.Square,
  Polygon: icons.Pentagon,
  Diamond: icons.Diamond,
  Star: icons.Star,
  Chevron: icons.ChevronUp,
  Polyline: icons.Activity,
  Bezier: icons.Spline,
  Graph: icons.Network,
}
const FILL_ICONS: Record<FillKind, Icon> = { Solid: icons.PaintBucket, Linear: icons.Blend, Radial: icons.Blend, Conical: icons.Blend, Image: icons.ImageIcon }
/** A transform's or an adjust's icon says which kind it is, in place of where its value comes from */
const MODIFIER_ICONS: Record<TransformKind | AdjustKind, Icon> = {
  Translate: icons.Move,
  Rotate: icons.RotateCw,
  Scale: icons.Scaling,
  Fit: icons.Maximize2,
  Mirror: icons.SquareSplitHorizontal,
  Revolve: icons.RefreshCw,
  Divide: icons.Scissors,
  HueShift: icons.Palette,
  Opacity: icons.Droplet,
  Brightness: icons.Sun,
  Contrast: icons.Contrast,
}
/** A prop's icon says where its value comes from */
const SOURCE_ICONS: Record<PropKind, Icon> = {
  Fixed: icons.Pin,
  Param: icons.Variable,
  Keyframes: icons.ChartSpline,
  Random: icons.Dices,
  Modulated: icons.AudioWaveform,
  Sweep: icons.Orbit,
  Visit: icons.Footprints,
}
const KIND_ICONS: Partial<Record<NodeKind, Icon>> = {
  plane: icons.MapIcon,
  leds: icons.Lightbulb,
  z: icons.Layers2,
  mask: icons.SquareDashed,
  fit: icons.Maximize2,
  points: icons.Waypoints,
  'bezier-point': icons.Dot,
  traversals: icons.Route,
  traversal: icons.Footprints,
  'node-ref': icons.MapPin,
}

type SpeedKind = 'Rate' | 'Duration'
type NewLayer = 'Sequence' | 'Area' | 'Display' | 'Sound'

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
  move(place: ListPlace, by: number): void
  remove(place: ListPlace): void
  /** The place of the row being dragged, which can be dropped elsewhere in its list */
  dragging(): ListPlace | null
  setDragging(place: ListPlace | null): void
}

const ConfigTreeContext = createContext<ConfigTree>()

/**
 * The scene's layers and sound layers as a tree. Each layer's settings and props are nodes under it, and any of those
 * with settings of its own has them under it in turn. Selecting a node selects the layer, or the mask, it's in. Its
 * button for closing every node goes in `toolbar`.
 */
export default function ConfigPanel(props: { selection: Signal<Selection | null>; toolbar?: HTMLElement }) {
  const scene = useContext(SceneContext)!
  const surfaces = useContext(LayerSurfacesContext)!
  const [revealing] = useContext(RevealContext)!
  const graphNodes = useGraphNodes()
  const [open, setOpen] = props.selection
  onMount(refreshContentSounds)

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
  const [expanded, setExpanded] = createSignal<string[]>(initial ? ancestry(selectionNode(initial)) : [])

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
    dragging,
    setDragging,
  }

  // a layer opened from elsewhere, such as by a click in the preview, is selected and opened up in the tree
  createEffect(
    on(open, (current) => {
      if (sameSelection(selectionOf(untrack(selected)), current)) return
      if (current === null) return setSelected(null)
      const id = selectionNode(current)
      setSelected(id)
      expand(...ancestry(id))
    }),
  )

  // an LED layer from before layers started on every LED has none, which the game can't play, so it's given every LED
  createEffect(() => {
    const current = scene.get()
    if (current.kind === 'Dmd' || current.layers.every((layer) => layer.display !== undefined || layer.leds !== undefined)) return
    scene.set({ ...current, layers: current.layers.map((layer) => (layer.display === undefined && layer.leds === undefined ? { ...layer, leds: 'Every' } : layer)) })
  })

  // a prop's field only comes up once every node it's under is open
  createEffect(() => {
    const prop = revealing()
    if (prop === null) return
    const path = untrack(() => pathTo(nodes(), prop))
    if (path) expand(...path)
  })

  /** A DMD scene's layers are all on its display. An area needs a plane to cover. */
  const options = (): { value: NewLayer; label: string }[] => [
    ...(scene.get().kind === 'Dmd'
      ? [{ value: 'Display' as const, label: 'Display' }]
      : [
          { value: 'Sequence' as const, label: 'LED sequence' },
          ...(planeNames().length > 0 ? [{ value: 'Area' as const, label: 'LED area' }] : []),
        ]),
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
        expand(id)
      })
      return
    }
    const place = newLayerPlace(scene.get())
    const surface = kind === 'Area' ? { surface: { Area: planeNames()[0] } } : kind === 'Sequence' ? { surface: 'Sequence' as const } : {}
    const layers = scene.get().layers
    const id = selectionNode({ layer: layers.length, mask: false })
    batch(() => {
      scene.set({ ...scene.get(), layers: [...layers, { ...place, ...surface, fill: { base: { Solid: { Fixed: WHITE } } } }] })
      select(id)
      expand(id)
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
          class="scene-config-tree"
          collection={collection()}
          selectionMode="single"
          selectedValue={selected() === null ? [] : [selected()!]}
          onSelectionChange={(details) => select(details.selectedValue[0] ?? null)}
          expandedValue={expanded()}
          onExpandedChange={(details) => setExpanded(details.expandedValue)}
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
      <AddMenu label="Add layer" class="scene-add-layer" groups={[options()]} onPick={add}>
        <icons.Plus aria-hidden="true" />
        Layer
      </AddMenu>
      <datalist id={LED_NAMES}>
        <For each={graphNodes()}>{(name) => <option value={name} />}</For>
      </datalist>
    </div>
  )
}

const sameSelection = (a: Selection | null, b: Selection | null) => a?.layer === b?.layer && a?.mask === b?.mask

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
            <TreeView.BranchControl class="scene-tree-row" {...drag}>
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
 * A node's icon and label, and the controls that fit beside them. A layer says what it is in words instead of an icon, and
 * only the nodes right under a layer have one, as more than that is too busy.
 */
function NodeRow(props: { node: ConfigNode; depth: number }) {
  const hasIcon = () => props.depth <= 2 && props.node.kind !== 'layer' && props.node.kind !== 'sound' && props.node.kind !== 'modifiers'
  const label = <span class={hasIcon() ? 'scene-tree-label' : 'scene-tree-label scene-tree-label-wide'}>{props.node.label}</span>
  return (
    <>
      <Show when={hasIcon()}>
        <NodeIcon node={props.node} />
      </Show>
      <Switch fallback={label}>
        <Match when={is(props.node, 'layer') || is(props.node, 'sound')}>
          {(layer) => <LayerRow node={layer()} />}
        </Match>
        <Match when={is(props.node, 'plane')}>
          {(plane) => (
            <>
              {label}
              <Controls node={props.node}>
                <Dropdown
                  label="Plane"
                  options={planeNames().map((name) => ({ value: name, label: name }))}
                  value={layerPlane(plane().lens.get())!}
                  onChange={(name) => field(plane().lens, 'surface').set({ Area: name })}
                />
              </Controls>
            </>
          )}
        </Match>
        <Match when={is(props.node, 'leds')}>
          {(leds) => (
            <>
              {label}
              <Controls node={props.node}>
                <LayerLeds lens={leds().lens} />
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
    if (scene.get().kind === 'Dmd') return 'Display'
    return layerPlane(props.node.lens.get() as SceneLayer) === null ? 'LED Sequence' : 'LED Area'
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
      <Match when={is(props.node, 'sound')}>{(sound) => <SoundFields lens={sound().lens} />}</Match>
      <Match when={is(props.node, 'region')}>{(region) => <GraphFields layer={region().layer!} />}</Match>
      <Match when={is(props.node, 'fill')}>{(fill) => <FillFields lens={fill().lens} />}</Match>
      <Match when={is(props.node, 'mask')}>{(mask) => <MaskFields lens={mask().lens as Lens<Mask>} />}</Match>
      <Match when={is(props.node, 'traversal')}>{(traversal) => <TraversalFields lens={traversal().lens} layer={traversal().layer} />}</Match>
      <Match when={is(props.node, 'node-ref')}>{(ref) => <NodeRefFields label={ref().label} lens={ref().lens} />}</Match>
      <Match when={is(props.node, 'prop')}>{(prop) => <PropField bare label={prop().label} {...propFieldSpec(prop())} />}</Match>
    </Switch>
  )
}

const propFieldSpec = (spec: PropSpec) => ({
  paramName: spec.paramName,
  lens: spec.lens,
  fallback: spec.fallback,
  ranges: spec.ranges,
})

/**
 * What a fixed prop is, or the parameter or traversal a prop reads from, as a glance at it in its row. A prop that changes
 * over time shows nothing, as its source at the end of the row already says so.
 */
function PropSummary(props: { node: ConfigNode; spec: PropSpec }) {
  const prop = () => props.spec.lens.get()
  const text = () => {
    const current = prop()
    if ('Param' in current) return current.Param
    if ('Visit' in current) return current.Visit.traversal
    if ('Fixed' in current) {
      const value = current.Fixed
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

/** A graph built from a layer's LEDs, as its region. Its traversals, which props can respond to, are under it. */
function GraphFields(props: { layer: Lens<SceneLayer> }) {
  return (
    <>
      <p class="scene-note">
        Only LEDs in the graph light up on this layer. To show them as a traversal reaches them, set the layer's opacity to Visit. On a plane, LEDs
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

const LED_NAMES = 'scene-led-names'

/**
 * A traversal from the first LED in the graph out to all the others, over the rest of the cycle.
 * Before the graph has been drawn there are no LEDs to name, so it starts from whichever is nearest the middle.
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
          class="scene-param-name"
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
        <p class="scene-note">Goes on to every LED in the graph. The targeted algorithms go to one LED.</p>
      </Show>
      <Show when={kind() === 'FanOut'}>
        <p class="scene-note">Also goes to random LEDs along an arc around From, through To, spread across it.</p>
      </Show>
      <Show when={kind() === 'Growth'}>
        <p class="scene-note">
          Sends offshoots from the path to as many random LEDs within Reach of it as Density. Branching is the chance an offshoot sends out one of its
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

/** The LEDs in the selected layer's graph, by name, which are the only ones a traversal can start from or go to */
function useGraphNodes() {
  const picking = useContext(NodePickingContext)
  return () => picking?.nodes() ?? []
}

/** Whether a node of a graph is an LED by name, or whichever is nearest a position, which is a prop under it */
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

/** An LED of the graph by name. It can be chosen by clicking it in the preview, and is pointed out there while this is hovered. */
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
          class="scene-param-name"
          aria-label={`${props.label} LED`}
          list={LED_NAMES}
          spellcheck={false}
          value={name()}
          onChange={(e) => props.lens.set({ Name: e.currentTarget.value.trim() })}
        />
        <button
          type="button"
          class="scene-button"
          aria-pressed={choosing()}
          title={choosing() ? 'Stop choosing' : `Choose the ${props.label.toLowerCase()} LED by clicking it in the preview`}
          onClick={toggleChoosing}
        >
          {choosing() ? 'Selecting…' : 'Select'}
        </button>
      </div>
      <Show when={graphNodes().length > 0 && !graphNodes().includes(name())}>
        <p class="scene-note scene-note-warn">This LED isn't in the layer's graph, so the traversal reaches nothing from it.</p>
      </Show>
    </div>
  )
}

const fillKind = (lens: Lens<Fill>): FillKind => {
  const current = lens.get().base
  return 'Solid' in current ? 'Solid' : 'Gradient' in current ? current.Gradient.style : 'Image'
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
        base().set({ Image: { name: '' } })
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

  // a frame's number means nothing in another image, but where a track or a param leads is still wanted
  const choose = (next: string) => {
    if (next === name()) return
    const current = frame()
    choosing = next
    props.lens.set(current && !('Fixed' in current) ? { name: next, frame: current } : { name: next })
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

  const changes = () => (info()?.frames ?? 1) > 1 && frame() !== undefined && !('Fixed' in frame()!) && !('Param' in frame()!)

  return (
    <>
      <ImagePicker lens={{ get: name, set: choose }} />
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
 * Picks what a mask's region, a layer's, or a sweep's path is, which can't be left out. On an LED scene, a layer's region
 * can also be its graph. In the file the two are separate, and a layer could have both, but the graph already says which
 * LEDs it shows on. A region has its outline in the preview, which can be hidden.
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
  // a graph's nodes are where its LEDs are on a plane, so a sequence has none
  const graphs = () => layer() !== undefined && scene.get().kind === 'Led' && layerPlane(layer()!.get()) !== null
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
  const triangle = (width: number): ShapeNode => ({ Star: { radius: { Fixed: length }, legs: { Fixed: 1 }, leg_width: { Fixed: width }, bow: { Fixed: BOW.rest! } } })
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
  if (kind === 'Ring') return { Ring: { inner_radius: { Fixed: reach * RING_HOLE }, thickness: { Fixed: reach * (1 - RING_HOLE) } } }
  if (kind === 'Ellipse') return { Ellipse: { rx: { Fixed: width / 2 }, ry: { Fixed: height / 2 } } }
  if (kind === 'Rect') return { Rect: { width: { Fixed: width }, height: { Fixed: height } } }
  if (kind === 'Diamond') {
    return { Diamond: { width: { Fixed: width }, height: { Fixed: height }, widest_at: { Fixed: WIDEST_AT.rest! }, bow: { Fixed: BOW.rest! } } }
  }
  if (kind === 'Star') {
    return { Star: { radius: { Fixed: reach }, legs: { Fixed: LEGS.rest! }, leg_width: { Fixed: LEG_WIDTH.rest! }, bow: { Fixed: BOW.rest! } } }
  }
  if (kind === 'Chevron') return { Chevron: { width: { Fixed: width }, height: { Fixed: height }, notch: { Fixed: NOTCH.rest! } } }
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
  if ('Ring' in base) {
    const across = (value(base.Ring.inner_radius) + value(base.Ring.thickness)) * 2
    return [across, across]
  }
  if ('Ellipse' in base) return [value(base.Ellipse.rx) * 2, value(base.Ellipse.ry) * 2]
  if ('Rect' in base) return [value(base.Rect.width), value(base.Rect.height)]
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
 * mirror's line starts along the right of a shape `size` across, so its reflection is beside it, and a revolve and a
 * divide start on its middle.
 */
function newTransform(kind: TransformKind, traversals: string[], size: Vec2): Transform {
  if (kind === 'Translate') return { Translate: { Fixed: [0, 0] } }
  if (kind === 'Rotate') return { Rotate: { Fixed: 0 } }
  if (kind === 'Scale') return { Scale: { Fixed: 1 } }
  if (kind === 'Mirror') return { Mirror: { center: { Fixed: [size[0] / 2, 0] }, rotation: { Fixed: 0 } } }
  if (kind === 'Revolve') return { Revolve: { center: { Fixed: [0, 0] }, angle: { Fixed: REVOLVE_ANGLE.rest! }, count: { Fixed: COPIES.rest! } } }
  if (kind === 'Divide') return { Divide: { center: { Fixed: [0, 0] }, rotation: { Fixed: 0 } } }
  return { Fit: traversals[0] ?? '' }
}

/**
 * Adds a transform or an adjust, listed together, after those already there. A mirror, revolve, or divide goes before
 * a shape's first translate instead, so that its center is in the shape's own space and it moves with the shape.
 */
function AddModifier(props: { node: Extract<ConfigNode, { kind: 'modifiers' }> }) {
  const tree = useContext(ConfigTreeContext)!
  const scene = useContext(SceneContext)!
  const traversals = useContext(TraversalsContext)
  // a fit is only for a layer with traversals to fit to
  const transforms = () => [...TRANSFORM_OPTIONS, ...(props.node.shape ? COPY_OPTIONS : []), ...(traversals().length > 0 ? [FIT_OPTION] : [])]
  const groups = () => [transforms(), ...(props.node.adjusts ? [ADJUST_OPTIONS] : [])]
  const owner = () => props.node.id.slice(0, props.node.id.lastIndexOf('/'))
  const add = (kind: TransformKind | AdjustKind) =>
    batch(() => {
      if (kind in ADJUSTS) {
        const list = props.node.adjusts!.get()
        props.node.adjusts!.set([...list, newAdjust(kind as AdjustKind)])
        tree.select(`${owner()}/adjust/${list.length}`)
      } else {
        const list = props.node.transforms.get()
        const shape = props.node.shape?.get()
        const size = shape ? shapeSize(shape.base, scene.get()) : ([0, 0] as Vec2)
        const placed = list.findIndex((transform) => 'Translate' in transform)
        const index = COPY_KINDS.includes(kind as CopyKind) && placed >= 0 ? placed : list.length
        props.node.transforms.set([...list.slice(0, index), newTransform(kind as TransformKind, traversals(), size), ...list.slice(index)])
        tree.select(`${owner()}/transform/${index}`)
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

/** Which traversal a fit is to. It runs from where the traversal starts, at -0.5 across, to the LED it reaches last, at 0.5. */
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
function AddButton(props: { node: ConfigNode; label: string; onClick: () => void }) {
  return (
    <Controls node={props.node} class="scene-tree-end">
      <button type="button" class="scene-icon-button" aria-label={props.label} title={props.label} onClick={props.onClick}>
        <icons.Plus aria-hidden="true" />
      </button>
    </Controls>
  )
}
