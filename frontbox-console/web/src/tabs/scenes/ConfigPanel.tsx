import { Accordion } from '@ark-ui/solid/accordion'
import { Editable } from '@ark-ui/solid/editable'
import { Menu } from '@ark-ui/solid/menu'
import { Popover } from '@ark-ui/solid/popover'
import { SegmentGroup } from '@ark-ui/solid/segment-group'
import { createEffect, createMemo, createSignal, For, Index, Match, on, onCleanup, Show, Switch, useContext, type Signal } from 'solid-js'
import { Portal } from 'solid-js/web'
import { contentImageInfo } from '../../state/contentImages'

import type { BezierPoint } from '../../types/generated/BezierPoint'
import type { Color } from '../../types/generated/Color'
import type { ColorAdjust } from '../../types/generated/ColorAdjust'
import type { EdgeRule } from '../../types/generated/EdgeRule'
import type { FillNode } from '../../types/generated/FillNode'
import type { GradientStyle } from '../../types/generated/GradientStyle'
import type { GraphMask } from '../../types/generated/GraphMask'
import type { ImageInfo } from '../../types/generated/ImageInfo'
import type { Layer } from '../../types/generated/Layer'
import type { Mask } from '../../types/generated/Mask'
import type { NodeRef } from '../../types/generated/NodeRef'
import type { Prop } from '../../types/generated/Prop'
import type { ShapeNode } from '../../types/generated/ShapeNode'
import type { Stop } from '../../types/generated/Stop'
import type { Transform } from '../../types/generated/Transform'
import type { Traversal } from '../../types/generated/Traversal'
import type { TraversalAlgorithm } from '../../types/generated/TraversalAlgorithm'
import { ImagePicker } from './ImagePicker'
import LayerPlace from './LayerPlace'
import { Dropdown, EyeButton, FieldRow, LayerSurfacesContext, NodePickingContext, NumberField, PropField, RevealContext, SceneContext, SurfaceContext, TraversalsContext } from './SceneFields'
import {
  adjustAmount,
  adjustKind,
  ADJUSTS,
  BLADE_FRACTION,
  BLADES,
  both,
  field,
  frameRanges,
  framesTrack,
  freshTraversalName,
  imageLength,
  item,
  layerLabel,
  moved,
  newAdjust,
  newLayerPlace,
  newShapeSize,
  orElse,
  radius,
  RATE,
  orEmpty,
  pointRanges,
  removed,
  ROTATION,
  roundRange,
  scaleRanges,
  shapePoints,
  sizeRanges,
  startingValue,
  TIME_STEP,
  translateRanges,
  traversalNames,
  traversalTarget,
  UNIT,
  variant,
  type Scene,
  type SceneLayer,
  withTiming,
  usedTraversals,
  withTraversalRenamed,
  type AdjustKind,
  type Lens,
  type Range,
  type Selection,
  type Surface,
  type Vec2,
  type TransformSpace,
} from './sceneModel'

type Fill = Layer['fill']
type FillKind = 'Solid' | GradientStyle | 'Image'
type ImageFill = Extract<FillNode, { Image: unknown }>['Image']
type Shape = NonNullable<Layer['region']>
type ShapeKind = 'None' | 'Circle' | 'Ring' | 'Ellipse' | 'Rect' | 'Polygon' | 'Pinwheel' | 'Polyline' | 'Bezier'

/** How much of a new ring's reach is its hole */
const RING_HOLE = 0.75

const WHITE: Color = [255, 255, 255, 255]
const BLACK: Color = [0, 0, 0, 255]
const COLOR_RANGES = both(UNIT)

/** How far in pixels a press on a stop has to move before it's a drag and not a click */
const DRAG_START = 3

const FILL_OPTIONS: { value: FillKind; label: string }[] = [
  { value: 'Solid', label: 'Solid' },
  { value: 'Linear', label: 'Linear gradient' },
  { value: 'Radial', label: 'Radial gradient' },
  { value: 'Conical', label: 'Conical gradient' },
  { value: 'Image', label: 'Image' },
]
const SHAPE_OPTIONS: { value: ShapeKind; label: string }[] = [
  { value: 'None', label: 'All' },
  { value: 'Circle', label: 'Circle' },
  { value: 'Ring', label: 'Ring' },
  { value: 'Ellipse', label: 'Ellipse' },
  { value: 'Rect', label: 'Rectangle' },
  { value: 'Polygon', label: 'Polygon' },
  { value: 'Pinwheel', label: 'Pinwheel' },
]
/** A path has to be something, and can also be a line, which has no inside to draw as a shape */
const PATH_OPTIONS: { value: ShapeKind; label: string }[] = [
  ...SHAPE_OPTIONS.slice(1),
  { value: 'Polyline', label: 'Polyline' },
  { value: 'Bezier', label: 'Bezier' },
]
const TRANSFORM_OPTIONS: { value: TransformKind; label: string }[] = [
  { value: 'Translate', label: 'Translate' },
  { value: 'Rotate', label: 'Rotate' },
  { value: 'Scale', label: 'Scale' },
]
/** Only for a layer with traversals to fit to */
const FIT_OPTION: { value: TransformKind; label: string } = { value: 'Fit', label: 'Fit to traversal' }
const EDGE_RULE_OPTIONS: { value: EdgeRule; label: string }[] = [
  { value: 'RelativeNeighborhood', label: 'Sparse, branching' },
  { value: 'Gabriel', label: 'Dense, meshed' },
]
type AlgorithmKind = 'BreadthFirst' | 'DepthFirst' | 'Dijkstra' | 'ShortestPath' | 'AStar' | 'RandomWalk'
const ALGORITHM_OPTIONS: { value: AlgorithmKind; label: string }[] = [
  { value: 'BreadthFirst', label: 'Breadth first' },
  { value: 'Dijkstra', label: 'By distance (Dijkstra)' },
  { value: 'DepthFirst', label: 'Depth first' },
  { value: 'ShortestPath', label: 'Shortest path' },
  { value: 'AStar', label: 'A* search' },
  { value: 'RandomWalk', label: 'Random walk' },
]
/** Algorithms whose rate is a distance a second, rather than a number of steps */
const BY_DISTANCE: AlgorithmKind[] = ['Dijkstra', 'ShortestPath']
const algorithmKind = (algorithm: TraversalAlgorithm) => (typeof algorithm === 'string' ? algorithm : Object.keys(algorithm)[0]) as AlgorithmKind
const NODE_OPTIONS: { value: 'Name' | 'Nearest'; label: string }[] = [
  { value: 'Name', label: 'LED' },
  { value: 'Nearest', label: 'Nearest to a position' },
]
/** How much longer a shortest path's edges can be made at random, as a fraction of each */
const JITTER: Range = { min: 0, max: 2, step: 0.01, rest: 0 }
const EDITING = ['Layer', 'Mask'] as const
const MASK_SHOWS_OPTIONS: { value: 'Inside' | 'Outside'; label: string }[] = [
  { value: 'Outside', label: 'Outside the mask' },
  { value: 'Inside', label: 'Inside the mask' },
]
const ADJUST_OPTIONS = (Object.keys(ADJUSTS) as AdjustKind[]).map((value) => ({ value, label: ADJUSTS[value].label }))

type TransformKind = 'Translate' | 'Rotate' | 'Scale' | 'Fit'

/** A transform that leaves things where they are, until it's changed. A fit starts on the layer's first traversal. */
function newTransform(kind: TransformKind, traversals: string[]): Transform {
  if (kind === 'Translate') return { Translate: { Fixed: [0, 0] } }
  if (kind === 'Rotate') return { Rotate: { Fixed: 0 } }
  if (kind === 'Scale') return { Scale: { Fixed: [1, 1] } }
  return { Fit: traversals[0] ?? '' }
}

/** The scene's layers */
export default function ConfigPanel(props: { selection: Signal<Selection | null> }) {
  return (
    <div class="scene-config">
      <LayerList open={props.selection} />
    </div>
  )
}

/** The scene's layers, with one open at a time */
function LayerList(props: { open: Signal<Selection | null> }) {
  const scene = useContext(SceneContext)!
  const layers = field(scene, 'layers')
  // by position, so this follows the open layer as layers around it move and go
  const [open, setOpen] = props.open

  const add = () => {
    layers.set([...layers.get(), { ...newLayerPlace(scene.get()), fill: { base: { Solid: { Fixed: WHITE } } } }])
    setOpen({ layer: layers.get().length - 1, mask: false })
  }

  const move = (index: number, by: number) => {
    layers.set(moved(layers.get(), index, by))
    setOpen((current) => {
      if (current === null) return null
      const layer = current.layer === index ? index + by : current.layer === index + by ? index : current.layer
      return { ...current, layer }
    })
  }

  const remove = (index: number) => {
    layers.set(removed(layers.get(), index))
    setOpen((current) => {
      if (current === null || current.layer === index) return null
      return current.layer > index ? { ...current, layer: current.layer - 1 } : current
    })
  }

  const toggle = (index: number) => setOpen((current) => (current?.layer === index ? null : { layer: index, mask: false }))

  return (
    <>
      <Accordion.Root
        class="scene-layers"
        collapsible
        lazyMount
        unmountOnExit
        value={open() === null ? [] : [String(open()!.layer)]}
        onValueChange={(details) => {
          const layer = details.value.length > 0 ? Number(details.value[0]) : null
          setOpen((current) => (layer === null ? null : current?.layer === layer ? current : { layer, mask: false }))
        }}
      >
        <Index each={layers.get()}>
          {(layer, index) => (
            <Accordion.Item class="scene-item" value={String(index)}>
              <header>
                <Accordion.ItemTrigger aria-label={layerLabel(layer(), index)}>
                  <Accordion.ItemIndicator>›</Accordion.ItemIndicator>
                </Accordion.ItemTrigger>
                <LayerName
                  lens={field(item(layers, index), 'name')}
                  placeholder={`Layer ${index + 1}`}
                  hidden={layer().hidden === true}
                  onClick={() => toggle(index)}
                />
                <EyeButton
                  label={layerLabel(layer(), index)}
                  hidden={layer().hidden === true}
                  onChange={(hidden) => field(item(layers, index), 'hidden').set(hidden || undefined)}
                />
                <ItemButtons
                  title={layerLabel(layer(), index)}
                  list={layers}
                  index={index}
                  onMove={(by) => move(index, by)}
                  onRemove={() => remove(index)}
                />
              </header>
              <Accordion.ItemContent>
                <div class="scene-config">
                  <LayerEditor
                    lens={item(layers, index)}
                    mask={open()?.mask === true}
                    onMask={(mask) => setOpen({ layer: index, mask })}
                  />
                </div>
              </Accordion.ItemContent>
            </Accordion.Item>
          )}
        </Index>
      </Accordion.Root>
      <AddButton onClick={add}>Add layer</AddButton>
    </>
  )
}

/**
 * A layer's name in its header. A click does what a click on the header does, and a double click edits it. It can't go
 * in the trigger, as that's a button.
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

/** Measures what's in it against what the layer is on */
function LayerEditor(props: { lens: Lens<SceneLayer>; mask: boolean; onMask: (mask: boolean) => void }) {
  const surfaces = useContext(LayerSurfacesContext)!
  const scene = useContext(SceneContext)!
  return (
    <SurfaceContext.Provider value={() => surfaces(props.lens.get())}>
      <TraversalsContext.Provider value={() => traversalNames(props.lens.get())}>
      <SegmentGroup.Root
        class="scene-prop-kind"
        aria-label="Editing"
        value={props.mask ? 'Mask' : 'Layer'}
        onValueChange={(details) => props.onMask(details.value === 'Mask')}
      >
        <SegmentGroup.Indicator />
        <For each={EDITING}>
          {(option) => (
            <SegmentGroup.Item value={option}>
              <SegmentGroup.ItemText>{option}</SegmentGroup.ItemText>
              <SegmentGroup.ItemControl />
              <SegmentGroup.ItemHiddenInput />
            </SegmentGroup.Item>
          )}
        </For>
      </SegmentGroup.Root>
      <Show
        when={props.mask}
        fallback={
          <>
            <LayerPlace lens={props.lens} />
            <ShapeEditor lens={field(props.lens, 'region')} />
            <Show when={scene.get().kind === 'Led'}>
              <GraphEditor lens={field(props.lens, 'graph')} layer={props.lens} />
            </Show>
            <FillEditor lens={field(props.lens, 'fill')} />
          </>
        }
      >
        <MaskEditor lens={field(props.lens, 'mask')} />
      </Show>
      </TraversalsContext.Provider>
    </SurfaceContext.Provider>
  )
}

/** Where a layer shows. A new mask cuts a region out of it, as with no region a mask with the layer outside it hides all of it. */
function MaskEditor(props: { lens: Lens<Mask | undefined> }) {
  const surface = useContext(SurfaceContext)!
  const mask = props.lens as Lens<Mask>
  const add = () => props.lens.set({ fill: { base: { Solid: { Fixed: WHITE } } }, region: newShape('Circle', surface()), invert: true })

  return (
    <Show when={props.lens.get()} fallback={<AddButton onClick={add}>Add mask</AddButton>}>
      <section class="scene-group">
        <FieldRow label="Show layer">
          <Dropdown
            label="Show layer"
            options={MASK_SHOWS_OPTIONS}
            value={mask.get().invert ? 'Outside' : 'Inside'}
            onChange={(value) => field(mask, 'invert').set(value === 'Outside' || undefined)}
          />
        </FieldRow>
      </section>
      <ShapeEditor lens={field(mask, 'region')} />
      <FillEditor lens={field(mask, 'fill')} />
      <AddButton onClick={() => props.lens.set(undefined)}>Remove mask</AddButton>
    </Show>
  )
}

/** Limits a layer to a graph built from its LEDs, with traversals over it that props can respond to */
function GraphEditor(props: { lens: Lens<GraphMask | undefined>; layer: Lens<SceneLayer> }) {
  const graphNodes = useGraphNodes()
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const graph = props.lens as Lens<GraphMask>
  const edges = () => field(graph, 'edges')
  const traversals = () => field(graph, 'traversals')
  const add = () => props.lens.set({ edges: { rule: 'RelativeNeighborhood' }, traversals: [] })
  const used = createMemo(() => usedTraversals(props.layer.get()))

  const addTraversal = () => {
    scene.set(withTiming(scene.get()))
    const name = freshTraversalName(props.layer.get(), 'walk')
    // from the first LED in the graph, as one that isn't in it reaches nothing
    const from = { Name: graphNodes()[0] ?? '' }
    traversals().set([...traversals().get(), { name, from, algorithm: 'BreadthFirst', rate: { Fixed: RATE.rest! }, start: 0 }])
  }

  // its responses and fits follow it to its new name
  const rename = (from: string, to: string) => {
    if (to === '' || to === from || traversalNames(props.layer.get()).includes(to)) return
    props.layer.set(withTraversalRenamed(props.layer.get(), from, to))
  }

  return (
    <Show when={props.lens.get()} fallback={<AddButton onClick={add}>Add graph</AddButton>}>
      <section class="scene-group">
        <FieldRow label="Graph edges">
          <Dropdown label="Graph edges" options={EDGE_RULE_OPTIONS} value={graph.get().edges.rule} onChange={field(edges(), 'rule').set} />
        </FieldRow>
        <FieldRow label="Longest edge">
          <NumberField
            label="Longest edge"
            range={edgeRange(surface())}
            value={graph.get().edges.max_length ?? 0}
            onChange={(length) => field(edges(), 'max_length').set(length > 0 ? length : undefined)}
          />
        </FieldRow>
        <p class="scene-note">LEDs further apart than this aren't joined. 0 is no limit.</p>
        <p class="scene-note">
          Only LEDs in the graph light up on this layer. On a plane, LEDs more than half a unit above or below it, such as on ramps, aren't in the
          graph.
        </p>
        {/* visits are what's being set up here, so these fields don't respond to them */}
        <TraversalsContext.Provider value={() => []}>
          <Index each={traversals().get()}>
            {(traversal, index) => (
              <ListItem title={`Traversal ${traversal().name}`} heading={<span>Traversal</span>} list={traversals()} index={index}>
                <TraversalEditor lens={item(traversals(), index)} used={used().has(traversal().name)} onRename={(to) => rename(traversal().name, to)} />
              </ListItem>
            )}
          </Index>
        </TraversalsContext.Provider>
        <AddButton onClick={addTraversal}>Add traversal</AddButton>
        <AddButton onClick={() => props.lens.set(undefined)}>Remove graph</AddButton>
      </section>
      <datalist id={LED_NAMES}>
        <For each={graphNodes()}>{(name) => <option value={name} />}</For>
      </datalist>
    </Show>
  )
}

const LED_NAMES = 'scene-led-names'


/** The longest edge a graph's slider goes to: the width of what it's on, or for a sequence the whole of it */
function edgeRange(surface: Surface): Range {
  const max = surface ? Math.max(...surface) : 1
  return { min: 0, max, step: max / 200, rest: 0 }
}

function TraversalEditor(props: { lens: Lens<Traversal>; used: boolean; onRename: (name: string) => void }) {
  const graphNodes = useGraphNodes()
  const scene = useContext(SceneContext)!
  const traversal = () => props.lens.get()
  const algorithm = () => field(props.lens, 'algorithm')
  const kind = () => algorithmKind(traversal().algorithm)
  const target = () => traversalTarget(algorithm())

  // a traversal going somewhere keeps going there as its algorithm changes
  const setKind = (next: AlgorithmKind) => {
    if (next === kind()) return
    // or to the far end of the graph, so a new one has somewhere to go
    const to = target()?.get() ?? { Name: graphNodes().at(-1) ?? '' }
    if (next === 'ShortestPath') algorithm().set({ ShortestPath: { to, jitter: 0 } })
    else if (next === 'AStar') algorithm().set({ AStar: { to } })
    else algorithm().set(next)
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
            props.onRename(e.currentTarget.value.trim())
            e.currentTarget.value = traversal().name
          }}
        />
      </FieldRow>
      <NodeRefField label="From" lens={field(props.lens, 'from')} />
      <FieldRow label="Algorithm">
        <Dropdown label="Algorithm" options={ALGORITHM_OPTIONS} value={kind()} onChange={setKind} />
      </FieldRow>
      <Show when={target()} fallback={<p class="scene-note">Goes on to every LED in the graph. Shortest path and A* search go to one LED.</p>}>
        {(to) => <NodeRefField label="To" lens={to()} />}
      </Show>
      <Show when={kind() === 'ShortestPath'}>
        <FieldRow label="Jitter">
          <NumberField
            label="Jitter"
            range={JITTER}
            value={(traversal().algorithm as { ShortestPath: { jitter: number } }).ShortestPath.jitter}
            onChange={field(variant(algorithm() as Lens<Extract<TraversalAlgorithm, { ShortestPath: unknown }>>, 'ShortestPath'), 'jitter').set}
          />
        </FieldRow>
      </Show>
      <PropField label={BY_DISTANCE.includes(kind()) ? 'Units a second' : 'Steps a second'} paramName="rate" fallback={RATE.rest!} ranges={both(RATE)} lens={field(props.lens, 'rate')} />
      <FieldRow label="Start">
        <NumberField
          label="Start"
          range={{ min: 0, max: scene.get().timing?.duration ?? 1, step: TIME_STEP, rest: 0 }}
          value={traversal().start}
          onChange={field(props.lens, 'start').set}
        />
      </FieldRow>
      <Show when={!props.used}>
        <p class="scene-note scene-note-warn">
          Nothing responds to {traversal().name} yet, so it doesn't change how the layer looks. Set a color, opacity or other value to Visit to light
          LEDs as it reaches them.
        </p>
      </Show>
    </>
  )
}

/** The LEDs in the open layer's graph, by name, which are the only ones a traversal can start from or go to */
function useGraphNodes() {
  const picking = useContext(NodePickingContext)
  return () => picking?.nodes() ?? []
}

/**
 * A node of a graph: an LED by name, or whichever is nearest a position. An LED can be chosen by clicking it in the
 * preview, and is pointed out there while this is hovered.
 */
function NodeRefField(props: { label: string; lens: Lens<NodeRef> }) {
  const surface = useContext(SurfaceContext)!
  const picking = useContext(NodePickingContext)
  const graphNodes = useGraphNodes()
  const kind = () => ('Name' in props.lens.get() ? 'Name' : 'Nearest')
  const name = () => (props.lens.get() as { Name: string }).Name
  const setKind = (next: 'Name' | 'Nearest') => {
    if (next === kind()) return
    const [width, height] = surface() ?? [1, 0]
    props.lens.set(next === 'Name' ? { Name: graphNodes()[0] ?? '' } : { Nearest: { Fixed: [width / 2, height / 2] } })
  }

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

  const point = (shown: boolean) => picking?.highlighted[1](shown && kind() === 'Name' ? name() : null)

  return (
    <div onMouseEnter={() => point(true)} onMouseLeave={() => point(false)} onFocusIn={() => point(true)} onFocusOut={() => point(false)}>
      <FieldRow label={props.label}>
        <Dropdown label={`${props.label} node`} options={NODE_OPTIONS} value={kind()} onChange={setKind} />
      </FieldRow>
      <Show
        when={kind() === 'Name'}
        fallback={
          <PropField
            label={`${props.label} position`}
            paramName="position"
            fallback={[0, 0] as Vec2}
            ranges={translateRanges(surface(), 'shape')}
            lens={variant(props.lens, 'Nearest')}
          />
        }
      >
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
      </Show>
    </div>
  )
}

function FillEditor(props: { lens: Lens<Fill> }) {
  const base = field(props.lens, 'base')
  const kind = (): FillKind => {
    const current = base.get()
    return 'Solid' in current ? 'Solid' : 'Gradient' in current ? current.Gradient.style : 'Image'
  }

  const setKind = (next: FillKind) => {
    const current = base.get()
    const color = 'Solid' in current ? current.Solid : 'Gradient' in current ? current.Gradient.stops[0]?.color : undefined
    if (next === kind()) return
    if (next === 'Solid') {
      base.set({ Solid: color ?? { Fixed: WHITE } })
    } else if (next === 'Image') {
      base.set({ Image: { name: '' } })
    } else if ('Gradient' in current) {
      base.set({ Gradient: { ...current.Gradient, style: next } })
    } else {
      const stops = [
        { at: 0, color: color ?? { Fixed: WHITE } },
        { at: 1, color: { Fixed: BLACK } },
      ]
      base.set({ Gradient: { stops, style: next } })
    }
  }

  return (
    <section class="scene-group">
      <FieldRow label="Fill">
        <Dropdown label="Fill" options={FILL_OPTIONS} value={kind()} onChange={setKind} />
      </FieldRow>
      <Switch>
        <Match when={kind() === 'Solid'}>
          <PropField label="Color" paramName="color" fallback={WHITE} ranges={COLOR_RANGES} lens={variant(base, 'Solid')} />
        </Match>
        <Match when={'Gradient' in base.get()}>
          <GradientEditor lens={variant(base, 'Gradient')} />
        </Match>
        <Match when={kind() === 'Image'}>
          <ImageEditor lens={variant(base, 'Image')} />
        </Match>
      </Switch>
      <TransformList lens={orEmpty(field(props.lens, 'transform'))} space="fill" />
      <AdjustList lens={orEmpty(field(props.lens, 'adjust'))} />
    </section>
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

  // one that's already moving stays in reach, so it can be undone after picking an image that isn't animated
  const hasFrames = () => (info()?.frames ?? 1) > 1 || (frame() !== undefined && !('Fixed' in frame()!))
  const changes = () => hasFrames() && frame() !== undefined && !('Fixed' in frame()!) && !('Param' in frame()!)

  return (
    <>
      <ImagePicker lens={{ get: name, set: choose }} />
      <Show when={info()}>{(known) => <p class="scene-note">{describeImage(known())}</p>}</Show>
      <Show when={hasFrames()}>
        <PropField
          label="Frame"
          paramName="frame"
          fallback={0}
          ranges={frameRanges(info()?.frames)}
          lens={orElse(field(props.lens, 'frame'), { Fixed: 0 })}
        />
      </Show>
      <Show when={changes() && info()}>
        {(known) => (
          <p class="scene-note">
            A fraction of a frame is dropped, so going up to {known().frames} gives every frame the same time.
          </p>
        )}
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

function GradientEditor(props: { lens: Lens<{ stops: Stop[] }> }) {
  const scene = useContext(SceneContext)!
  const [revealing] = useContext(RevealContext)!
  const stops = field(props.lens, 'stops')
  // both by position, which is what a stop keeps as the others move around it
  const [open, setOpen] = createSignal<number | null>(null)
  const [dragging, setDragging] = createSignal<number | null>(null)
  let row!: HTMLDivElement
  let pressedAt: Vec2 | null = null
  // the press that a drag was ends in a click, which isn't one
  let dragged = false

  // the format needs stops in order of `at`, so each stays between its neighbors
  const stopRange = (index: number): Range => ({
    min: stops.get()[index - 1]?.at ?? UNIT.min,
    max: stops.get()[index + 1]?.at ?? UNIT.max,
    step: UNIT.step,
  })

  // the stops trade places and the positions stay, as the format needs them in order of `at`
  const moveStop = (index: number, by: number) => {
    const list = stops.get()
    stops.set(moved(list, index, by).map((stop, i) => ({ ...stop, at: list[i].at })))
  }

  // in the widest gap, in the color of the stop before it, so adding one doesn't change much
  const addStop = () => {
    const list = stops.get()
    let index = 1
    for (let i = 1; i < list.length; i++) {
      if (list[i].at - list[i - 1].at > list[index].at - list[index - 1].at) index = i
    }
    const [before, after] = [list[index - 1], list[index]]
    // its own copy, as a prop is told from another by which one it is
    const added: Stop = { at: ((before?.at ?? 0) + (after?.at ?? 1)) / 2, color: before ? structuredClone(before.color) : { Fixed: WHITE } }
    stops.set([...list.slice(0, index), added, ...list.slice(index)])
    setOpen(index)
  }

  const removeStop = (index: number) => {
    setOpen(null)
    stops.set(removed(stops.get(), index))
  }

  const press = (e: PointerEvent & { currentTarget: HTMLElement }) => {
    dragged = false
    if (e.button !== 0) return
    pressedAt = [e.clientX, e.clientY]
    e.currentTarget.setPointerCapture(e.pointerId)
  }

  const drag = (e: PointerEvent, index: number) => {
    if (pressedAt === null) return
    if (dragging() === null) {
      if (Math.hypot(e.clientX - pressedAt[0], e.clientY - pressedAt[1]) < DRAG_START) return
      dragged = true
      setOpen(null)
      setDragging(index)
    }
    const over = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-stop]')
    if (!over || !row.contains(over)) return
    const [from, to] = [dragging()!, Number(over.dataset.stop)]
    if (to === from) return
    moveStop(from, to - from)
    setDragging(to)
  }

  const release = () => {
    pressedAt = null
    setDragging(null)
  }

  // a stop's fields are in its popup, so finding one from the timeline opens it
  createEffect(() => {
    const index = stops.get().findIndex((stop) => stop.color === revealing())
    if (index < 0) return
    setOpen(index)
    row.querySelector(`[data-stop="${index}"]`)?.scrollIntoView({ block: 'center', behavior: 'smooth' })
  })

  return (
    <div class="scene-stops" ref={row}>
      <Index each={stops.get()}>
        {(stop, index) => (
          <Popover.Root
            lazyMount
            unmountOnExit
            autoFocus={false}
            positioning={{ placement: 'bottom-start' }}
            open={open() === index}
            onOpenChange={(details) => {
              if (details.open && dragged) return
              // another stop's can be opening as this one closes
              setOpen((current) => (details.open ? index : current === index ? null : current))
            }}
          >
            <Popover.Trigger
              class="scene-stop"
              classList={{ dragging: dragging() === index }}
              data-stop={index}
              aria-label={`Stop ${index + 1}`}
              style={{ background: cssColor(startingValue(stop().color, scene.get(), WHITE)) }}
              onPointerDown={press}
              onPointerMove={(e) => drag(e, index)}
              onPointerUp={release}
              onPointerCancel={release}
            />
            <Portal>
              <Popover.Positioner>
                <Popover.Content class="scene-config scene-stop-popup">
                  <FieldRow label="At">
                    <NumberField
                      label={`Stop ${index + 1} position`}
                      range={stopRange(index)}
                      value={stop().at}
                      onChange={field(item(stops, index), 'at').set}
                    />
                  </FieldRow>
                  <PropField
                    label="Color"
                    paramName="stop_color"
                    fallback={WHITE}
                    ranges={COLOR_RANGES}
                    lens={field(item(stops, index), 'color')}
                  />
                  <Show when={stops.get().length > 1}>
                    <button type="button" class="scene-add" onClick={() => removeStop(index)}>
                      Remove stop
                    </button>
                  </Show>
                </Popover.Content>
              </Popover.Positioner>
            </Portal>
          </Popover.Root>
        )}
      </Index>
      <button type="button" class="scene-add" aria-label="Add stop" title="Add stop" onClick={addStop}>
        +
      </button>
    </div>
  )
}

const cssColor = ([r, g, b, a]: Color) => `rgb(${r} ${g} ${b} / ${a / 255})`

/** A layer's region, or with `path` the shape a sweep goes around, which can't be left out */
export function ShapeEditor(props: { lens: Lens<Shape | undefined> } | { lens: Lens<Shape>; path: true }) {
  const path = 'path' in props
  const label = path ? 'Path' : 'Region'
  const surface = useContext(SurfaceContext)!
  const scene = useContext(SceneContext)!
  const lens = props.lens as Lens<Shape | undefined>
  const shape = props.lens as Lens<Shape>
  const base = field(shape, 'base') as Lens<ShapeNode>
  const kind = (): ShapeKind => {
    const current = lens.get()
    return current ? (Object.keys(current.base)[0] as ShapeKind) : 'None'
  }

  const setKind = (next: ShapeKind) => {
    if (next === kind()) return
    if (next === 'None') return lens.set(undefined)
    // one shape turned into another takes up the same room
    if (lens.get()) base.set(sizedShape(next, shapeSize(base.get(), scene.get(), surface())))
    else lens.set(newShape(next, surface()))
  }

  const ranges = () => sizeRanges(surface())

  return (
    <section class="scene-group">
      <FieldRow label={label}>
        <Dropdown label={label} options={path ? PATH_OPTIONS : SHAPE_OPTIONS} value={kind()} onChange={setKind} />
      </FieldRow>
      <Switch>
        <Match when={kind() === 'Circle'}>
          <PropField label="Radius" paramName="radius" fallback={0} ranges={both(roundRange(surface()))} lens={field(variant(base, 'Circle'), 'radius')} />
        </Match>
        <Match when={kind() === 'Ring'}>
          <PropField label="Inner radius" paramName="inner_radius" fallback={0} ranges={both(roundRange(surface()))} lens={field(variant(base, 'Ring'), 'inner_radius')} />
          <PropField label="Thickness" paramName="thickness" fallback={0} ranges={both(roundRange(surface()))} lens={field(variant(base, 'Ring'), 'thickness')} />
        </Match>
        <Match when={kind() === 'Ellipse'}>
          <PropField label="RX" paramName="rx" fallback={0} ranges={both(radius(ranges()[0]))} lens={field(variant(base, 'Ellipse'), 'rx')} />
          <PropField label="RY" paramName="ry" fallback={0} ranges={both(radius(ranges()[1]))} lens={field(variant(base, 'Ellipse'), 'ry')} />
        </Match>
        <Match when={kind() === 'Rect'}>
          <PropField label="Width" paramName="width" fallback={0} ranges={both(ranges()[0])} lens={field(variant(base, 'Rect'), 'width')} />
          <PropField label="Height" paramName="height" fallback={0} ranges={both(ranges()[1])} lens={field(variant(base, 'Rect'), 'height')} />
        </Match>
        <Match when={kind() === 'Pinwheel'}>
          <PropField label="Blades" paramName="blades" fallback={0} ranges={both(BLADES)} lens={field(variant(base, 'Pinwheel'), 'blades')} />
          <PropField label="Blade fraction" paramName="blade_fraction" fallback={BLADE_FRACTION.rest!} ranges={both(BLADE_FRACTION)} lens={field(variant(base, 'Pinwheel'), 'blade_fraction')} />
        </Match>
        <Match when={kind() === 'Polygon'}>
          <PolygonEditor lens={field(variant(base, 'Polygon'), 'points')} />
        </Match>
        <Match when={kind() === 'Polyline'}>
          <PolygonEditor open lens={field(variant(base, 'Polyline'), 'points')} />
        </Match>
        <Match when={kind() === 'Bezier'}>
          <BezierEditor lens={field(variant(base, 'Bezier'), 'points')} />
        </Match>
      </Switch>
      <Show when={kind() !== 'None'}>
        <TransformList lens={orEmpty(field(shape, 'transform'))} space="shape" />
      </Show>
    </section>
  )
}

/** A new shape, placed so that it can be seen, at a size that fits */
function newShape(kind: Exclude<ShapeKind, 'None'>, surface: Surface): Shape {
  // a sequence runs from 0 to 1, with its points along the middle
  const [width, height] = surface ?? [1, 0]
  const across = newShapeSize(surface)
  return {
    base: sizedShape(kind, [across, across]),
    transform: [{ Scale: { Fixed: [1, 1] } }, { Translate: { Fixed: [width / 2, height / 2] } }],
  }
}

/** A shape this wide and tall */
function sizedShape(kind: Exclude<ShapeKind, 'None'>, [width, height]: Vec2): ShapeNode {
  const reach = Math.max(width, height) / 2
  if (kind === 'Circle') return { Circle: { radius: { Fixed: reach } } }
  if (kind === 'Ring') return { Ring: { inner_radius: { Fixed: reach * RING_HOLE }, thickness: { Fixed: reach * (1 - RING_HOLE) } } }
  if (kind === 'Ellipse') return { Ellipse: { rx: { Fixed: width / 2 }, ry: { Fixed: height / 2 } } }
  if (kind === 'Rect') return { Rect: { width: { Fixed: width }, height: { Fixed: height } } }
  if (kind === 'Pinwheel') return { Pinwheel: { blades: { Fixed: BLADES.rest! }, blade_fraction: { Fixed: BLADE_FRACTION.rest! } } }
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

/** How wide and tall a shape is in its own space. A pinwheel runs past the edges, so what comes after it is a new size. */
function shapeSize(base: ShapeNode, scene: Scene, surface: Surface): Vec2 {
  const value = (prop: Prop<number>) => startingValue(prop, scene, 0)
  if ('Circle' in base) return [value(base.Circle.radius) * 2, value(base.Circle.radius) * 2]
  if ('Ring' in base) {
    const across = (value(base.Ring.inner_radius) + value(base.Ring.thickness)) * 2
    return [across, across]
  }
  if ('Ellipse' in base) return [value(base.Ellipse.rx) * 2, value(base.Ellipse.ry) * 2]
  if ('Rect' in base) return [value(base.Rect.width), value(base.Rect.height)]
  if ('Pinwheel' in base) return [newShapeSize(surface), newShapeSize(surface)]
  const points = shapePoints(base).map((point) => startingValue(point, scene, [0, 0] as Vec2))
  const reach = (axis: 0 | 1) => Math.max(...points.map((p) => p[axis])) - Math.min(...points.map((p) => p[axis]))
  return [reach(0), reach(1)]
}

/** The points of a polygon, or with `open` of a line that doesn't come back around */
function PolygonEditor(props: { lens: Lens<Prop<Vec2>[]>; open?: boolean }) {
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const points = props.lens

  // halfway back to the first point, so the polygon keeps its outline. A line carries on from its end.
  const addPoint = () => {
    const list = points.get()
    const at = (point: Prop<Vec2>) => startingValue(point, scene.get(), [0, 0])
    const [first, last] = [at(list[0]), at(list[list.length - 1])]
    if (props.open) return points.set([...list, { Fixed: pastEnd(at(list[list.length - 2] ?? list[0]), last) }])
    points.set([...list, { Fixed: [(first[0] + last[0]) / 2, (first[1] + last[1]) / 2] }])
  }

  return (
    <>
      <Index each={points.get()}>
        {(_, index) => (
          <ListItem title={`Point ${index + 1}`} list={points} index={index} canRemove={points.get().length > (props.open ? 2 : 3)}>
            <PropField
              label="Position"
              paramName="point"
              fallback={[0, 0] as Vec2}
              ranges={pointRanges(surface())}
              lens={item(points, index)}
            />
          </ListItem>
        )}
      </Index>
      <AddButton onClick={addPoint}>Add point</AddButton>
    </>
  )
}

function BezierEditor(props: { lens: Lens<BezierPoint[]> }) {
  const scene = useContext(SceneContext)!
  const surface = useContext(SurfaceContext)!
  const points = props.lens

  // carrying on from the end the same way, with the end's handle
  const addPoint = () => {
    const list = points.get()
    const at = ({ point }: BezierPoint) => startingValue(point, scene.get(), [0, 0])
    const last = list[list.length - 1]
    const point = pastEnd(at(list[list.length - 2] ?? last), at(last))
    points.set([...list, { point: { Fixed: point }, handle: { Fixed: startingValue(last.handle, scene.get(), [0, 0]) } }])
  }

  return (
    <>
      <Index each={points.get()}>
        {(_, index) => (
          <ListItem title={`Point ${index + 1}`} list={points} index={index} canRemove={points.get().length > 2}>
            <PropField
              label="Position"
              paramName="point"
              fallback={[0, 0] as Vec2}
              ranges={pointRanges(surface())}
              lens={field(item(points, index), 'point')}
            />
            <PropField
              label="Handle"
              paramName="handle"
              fallback={[0, 0] as Vec2}
              ranges={pointRanges(surface())}
              lens={field(item(points, index), 'handle')}
            />
          </ListItem>
        )}
      </Index>
      <AddButton onClick={addPoint}>Add point</AddButton>
    </>
  )
}

/** Half as far again along a line's last stretch */
function pastEnd(before: Vec2, end: Vec2): Vec2 {
  return [end[0] + (end[0] - before[0]) / 2, end[1] + (end[1] - before[1]) / 2]
}

function TransformList(props: { lens: Lens<Transform[]>; space: TransformSpace }) {
  const traversals = useContext(TraversalsContext)
  return (
    <>
      <Index each={props.lens.get()}>
        {(_, index) => (
          <ListItem title="Transform" heading={<TransformPicker lens={item(props.lens, index)} />} list={props.lens} index={index}>
            <TransformEditor lens={item(props.lens, index)} space={props.space} />
          </ListItem>
        )}
      </Index>
      <AddMenu options={transformOptions(traversals())} onPick={(kind) => props.lens.set([...props.lens.get(), newTransform(kind, traversals())])}>
        Add transform
      </AddMenu>
    </>
  )
}

const transformKind = (transform: Transform) => Object.keys(transform)[0] as TransformKind

/** What a transform can be, which is only a fit with traversals to fit to, or for one that already is */
const transformOptions = (traversals: string[], kind?: TransformKind) =>
  traversals.length > 0 || kind === 'Fit' ? [...TRANSFORM_OPTIONS, FIT_OPTION] : TRANSFORM_OPTIONS

/** Heads a transform, saying which kind it is and changing it */
function TransformPicker(props: { lens: Lens<Transform> }) {
  const traversals = useContext(TraversalsContext)
  const kind = () => transformKind(props.lens.get())
  return (
    <Dropdown
      label="Kind of transform"
      options={transformOptions(traversals(), kind())}
      value={kind()}
      onChange={(next) => {
        if (next !== kind()) props.lens.set(newTransform(next, traversals()))
      }}
    />
  )
}

function TransformEditor(props: { lens: Lens<Transform>; space: TransformSpace }) {
  const surface = useContext(SurfaceContext)!
  const traversals = useContext(TraversalsContext)
  const kind = () => transformKind(props.lens.get())
  const fitted = () => (props.lens.get() as { Fit: string }).Fit
  // one the layer doesn't have anymore stays chosen, so it can be seen and changed
  const fitOptions = () => [...new Set([...traversals(), fitted()])].filter((name) => name !== '').map((name) => ({ value: name, label: name }))
  return (
    <Switch>
      <Match when={kind() === 'Translate'}>
        <PropField
          label="Offset"
          paramName="offset"
          fallback={[0, 0] as Vec2}
          ranges={translateRanges(surface(), props.space)}
          lens={variant(props.lens, 'Translate')}
        />
      </Match>
      <Match when={kind() === 'Rotate'}>
        <PropField
          label="Degrees"
          paramName="rotation"
          fallback={0}
          ranges={both(ROTATION)}
          lens={variant(props.lens, 'Rotate')}
        />
      </Match>
      <Match when={kind() === 'Scale'}>
        <PropField
          label="Scale"
          paramName="scale"
          fallback={[1, 1] as Vec2}
          ranges={scaleRanges()}
          lens={variant(props.lens, 'Scale')}
          notPosition
        />
      </Match>
      <Match when={kind() === 'Fit'}>
        <FieldRow label="Traversal">
          <Show when={fitOptions().length > 0} fallback={<p class="scene-note scene-note-warn">The layer has no traversals to fit to.</p>}>
            <Dropdown label="Traversal to fit to" options={fitOptions()} value={fitted()} onChange={(name) => props.lens.set({ Fit: name })} />
          </Show>
        </FieldRow>
        <Show when={fitted() !== '' && !traversals().includes(fitted())}>
          <p class="scene-note scene-note-warn">The layer has no traversal by this name, so this changes nothing.</p>
        </Show>
        <p class="scene-note">
          Runs from where the traversal starts, at -0.5 across, to the LED it reaches last, at 0.5. Transforms above this one are measured that way.
        </p>
      </Match>
    </Switch>
  )
}

function AdjustList(props: { lens: Lens<ColorAdjust[]> }) {
  return (
    <>
      <Index each={props.lens.get()}>
        {(_, index) => (
          <ListItem title="Adjust" heading={<AdjustPicker lens={item(props.lens, index)} />} list={props.lens} index={index}>
            <AdjustEditor lens={item(props.lens, index)} />
          </ListItem>
        )}
      </Index>
      <AddMenu options={ADJUST_OPTIONS} onPick={(kind) => props.lens.set([...props.lens.get(), newAdjust(kind)])}>
        Add adjust
      </AddMenu>
    </>
  )
}

/** Heads an adjust, saying which kind it is and changing it */
function AdjustPicker(props: { lens: Lens<ColorAdjust> }) {
  const kind = () => adjustKind(props.lens.get())
  return (
    <Dropdown
      label="Kind of adjust"
      options={ADJUST_OPTIONS}
      value={kind()}
      onChange={(next) => {
        if (next !== kind()) props.lens.set(newAdjust(next))
      }}
    />
  )
}

function AdjustEditor(props: { lens: Lens<ColorAdjust> }) {
  const adjust = () => ADJUSTS[adjustKind(props.lens.get())]
  return (
    <PropField
      label={adjust().amount}
      paramName={adjust().paramName}
      fallback={adjust().range.rest!}
      ranges={both(adjust().range)}
      lens={adjustAmount(props.lens)}
    />
  )
}

type ItemButtonsProps<T> = {
  title: string
  list: Lens<T[]>
  index: number
  canRemove?: boolean
  /** For a list that can't just be rearranged */
  onMove?: (by: number) => void
  onRemove?: () => void
}

/** A thing in a list, with the buttons for moving and removing it. `heading` stands in for its title on screen. */
function ListItem<T>(props: ItemButtonsProps<T> & { heading?: any; children: any }) {
  return (
    <div class="scene-item">
      <header>
        <Show when={props.heading} fallback={<span>{props.title}</span>}>
          {props.heading}
        </Show>
        <ItemButtons {...props} />
      </header>
      {props.children}
    </div>
  )
}

function ItemButtons<T>(props: ItemButtonsProps<T>) {
  const move = (by: number) => {
    if (props.onMove) props.onMove(by)
    else props.list.set(moved(props.list.get(), props.index, by))
  }
  const remove = () => {
    if (props.onRemove) props.onRemove()
    else props.list.set(removed(props.list.get(), props.index))
  }
  return (
    <>
      <button
        type="button"
        aria-label={`Move ${props.title} up`}
        disabled={props.index === 0}
        onClick={() => move(-1)}
      >
        ↑
      </button>
      <button
        type="button"
        aria-label={`Move ${props.title} down`}
        disabled={props.index === props.list.get().length - 1}
        onClick={() => move(1)}
      >
        ↓
      </button>
      <button type="button" aria-label={`Remove ${props.title}`} disabled={props.canRemove === false} onClick={remove}>
        ×
      </button>
    </>
  )
}

/** An add button for a list of several kinds of thing, which asks which kind */
function AddMenu<T extends string>(props: { options: { value: T; label: string }[]; onPick: (kind: T) => void; children: any }) {
  return (
    <Menu.Root positioning={{ sameWidth: true }} onSelect={(details) => props.onPick(details.value as T)}>
      <Menu.Trigger class="scene-add">{props.children}</Menu.Trigger>
      <Portal>
        <Menu.Positioner>
          <Menu.Content class="scene-dropdown-list">
            <For each={props.options}>
              {(option) => (
                <Menu.Item value={option.value}>
                  <Menu.ItemText>{option.label}</Menu.ItemText>
                </Menu.Item>
              )}
            </For>
          </Menu.Content>
        </Menu.Positioner>
      </Portal>
    </Menu.Root>
  )
}

function AddButton(props: { onClick: () => void; children: any }) {
  return (
    <button type="button" class="scene-add" onClick={props.onClick}>
      {props.children}
    </button>
  )
}
