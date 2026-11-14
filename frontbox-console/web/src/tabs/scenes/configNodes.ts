import { contentImageInfo } from '../../state/contentImages'
import type { BezierPoint } from '../../types/generated/BezierPoint'
import type { ColorAdjust } from '../../types/generated/ColorAdjust'
import type { GraphMask } from '../../types/generated/GraphMask'
import type { Layer } from '../../types/generated/Layer'
import type { LedQ } from '../../types/generated/LedQ'
import type { Mask } from '../../types/generated/Mask'
import type { Modulated } from '../../types/generated/Modulated'
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
  ADJUSTS,
  adjustAmount,
  adjustKind,
  both,
  BRANCHING,
  copyProps,
  FAN_ANGLE,
  FAN_COUNT,
  field,
  frameRanges,
  GROWTH_DENSITY,
  item,
  layerLabel,
  layerPlane,
  orElse,
  orEmpty,
  pointRanges,
  RATE,
  reachRange,
  ROTATION,
  scaleRanges,
  shapeProps,
  soundLabel,
  soundVolume,
  traversalTarget,
  translateRanges,
  UNIT,
  variant,
  VOLUME,
  WANDER,
  type LayerSurfaces,
  type AdjustKind,
  type Lens,
  type Ranges,
  type Scene,
  type SceneLayer,
  type SceneValue,
  type Selection,
  type Surface,
  type TransformSpace,
  type Vec2,
} from './sceneModel'

export type Fill = Layer['fill']
export type Shape = NonNullable<Layer['region']>

/**
 * A row in the config tree. Its id is its path through the scene, such as `layers/0/fill/transform/1`, so it stays put
 * as what's in it changes, and starts with the ids of the nodes it's under.
 */
export type ConfigNode = NodeData & {
  id: string
  label: string
  /** What it covers in the scene, for finding the node a prop is in */
  value: unknown
  /** Whether it has fields of its own, shown above the nodes under it */
  fields?: boolean
  /** Its place in a list, which its row has buttons for moving it around in and taking it out of */
  place?: ListPlace
  /** For a prop that's a transform or an adjust, which kind it is */
  modifier?: TransformKind | AdjustKind
  /** For a node with fields and nothing under it, which still opens to show them */
  childrenCount?: number
  children?: ConfigNode[]
}

/** What a node edits, which says how it's drawn */
export type NodeData =
  /** `unnamed` is what a layer is called until it's given a name */
  | { kind: 'layer'; lens: Lens<SceneLayer>; unnamed: string }
  | { kind: 'sound'; lens: Lens<SoundLayer>; unnamed: string }
  | { kind: 'plane'; lens: Lens<SceneLayer> }
  | { kind: 'leds'; lens: Lens<LedQ | undefined> }
  | { kind: 'z'; lens: Lens<SceneLayer> }
  /** With `layer`, a layer's region, which on an LED scene can be its graph instead */
  | { kind: 'region'; lens: Lens<Shape | undefined>; layer?: Lens<SceneLayer> }
  /** The shape a sweep goes around */
  | { kind: 'path'; lens: Lens<Shape> }
  | { kind: 'fill'; lens: Lens<Fill> }
  | { kind: 'stops'; lens: Lens<Stop[]> }
  | { kind: 'mask'; lens: Lens<Mask | undefined> }
  /** A fit transform, which picks its traversal in its row. Other transforms are props. */
  | { kind: 'fit'; lens: Lens<Transform> }
  /** A mirror, revolve, or divide, which has its props under it */
  | { kind: 'copy'; lens: Lens<Transform> }
  /** The row for adding a transform, or an adjust where there's `adjusts`. With `shape`, it also offers a mirror, revolve, or divide. */
  | { kind: 'modifiers'; transforms: Lens<Transform[]>; adjusts?: Lens<ColorAdjust[]>; shape?: Lens<Shape> }
  | { kind: 'points'; shape: PointsShape; lens: Lens<Prop<Vec2>[]> | Lens<BezierPoint[]> }
  | { kind: 'bezier-point' }
  | { kind: 'traversals'; lens: Lens<Traversal[]>; layer: Lens<SceneLayer> }
  | { kind: 'traversal'; lens: Lens<Traversal>; layer: Lens<SceneLayer> }
  | { kind: 'node-ref'; lens: Lens<NodeRef> }
  | ({ kind: 'prop' } & PropSpec)

export type NodeKind = NodeData['kind']

/** Everything a prop's field needs */
export type PropSpec = {
  lens: Lens<Prop<SceneValue>>
  /** What to call a parameter made from it */
  paramName: string
  /** For a prop with nothing to take a value from */
  fallback: SceneValue
  ranges: Ranges
}

export type PointsShape = 'Polygon' | 'Polyline' | 'Bezier'

/** Where a node sits in a list, by the id of the list's node */
export type ListPlace = {
  list: Lens<unknown[]>
  listId: string
  index: number
  canRemove?: boolean
  /** For a gradient's stops, whose `at` comes of their order: they're spaced evenly again after any change */
  spread?: boolean
}

export type AlgorithmKind = 'BreadthFirst' | 'DepthFirst' | 'Dijkstra' | 'ShortestPath' | 'FanOut' | 'Growth' | 'AStar' | 'RandomWalk'
export type Wandering = 'BreadthFirst' | 'Dijkstra' | 'ShortestPath' | 'FanOut' | 'Growth'
export const WANDERING: AlgorithmKind[] = ['BreadthFirst', 'Dijkstra', 'ShortestPath', 'FanOut', 'Growth']
/** Algorithms whose rate is a distance a second, rather than a number of steps */
export const BY_DISTANCE: AlgorithmKind[] = ['Dijkstra', 'ShortestPath', 'FanOut', 'Growth']
export const algorithmKind = (algorithm: TraversalAlgorithm) => (typeof algorithm === 'string' ? algorithm : Object.keys(algorithm)[0]) as AlgorithmKind

export const TRANSFORM_LABELS = { Translate: 'Translate', Rotate: 'Rotate', Scale: 'Scale', Fit: 'Fit to traversal', Mirror: 'Mirror', Revolve: 'Revolve', Divide: 'Divide' }
export type TransformKind = keyof typeof TRANSFORM_LABELS
export const transformKind = (transform: Transform) => Object.keys(transform)[0] as TransformKind

/** The scene's layers, then its sound layers */
export function configNodes(scene: Lens<Scene>, surfaces: LayerSurfaces): ConfigNode[] {
  const layers = field(scene, 'layers')
  const sounds = field(scene, 'sounds')
  const led = scene.get().kind === 'Led'
  return [
    ...layers.get().map((layer, index) => {
      const id = `layers/${index}`
      const lens = item(layers, index)
      return branch({
        id,
        label: layerLabel(layer, index),
        kind: 'layer',
        unnamed: layerLabel({ ...layer, name: undefined }, index),
        lens,
        value: layer,
        place: { list: layers, listId: 'layers', index },
        children: layerChildren(id, lens, led, surfaces(layer)),
      })
    }),
    ...sounds.get().map((sound, index) => {
      const id = `sounds/${index}`
      const lens = item(sounds, index)
      return branch({
        id,
        label: soundLabel(sound, index),
        kind: 'sound',
        unnamed: soundLabel({ ...sound, name: undefined }, index),
        lens,
        value: sound,
        fields: true,
        place: { list: sounds, listId: 'sounds', index },
        children: [propNode(`${id}/volume`, 'Volume', { lens: soundVolume(lens), paramName: 'volume', fallback: 1, ranges: both(VOLUME) }, null)],
      })
    }),
  ]
}

/** Which layer, or its mask, a node is in, which is what's selected in the preview while it is */
export function selectionOf(id: string | null): Selection | null {
  const match = id?.match(/^layers\/(\d+)(\/mask(?:\/|$))?/)
  return match ? { layer: Number(match[1]), mask: match[2] !== undefined } : null
}

export function selectionNode({ layer, mask }: Selection) {
  return `layers/${layer}${mask ? '/mask' : ''}`
}

/** The nodes that a node is under, from the top down, ending with itself */
export function ancestry(id: string): string[] {
  const parts = id.split('/')
  return parts.slice(1).map((_, i) => parts.slice(0, i + 2).join('/'))
}

/**
 * Ids, with those of a list's items and what's under them following the items to where `to` puts them, or dropped
 * where it gives `null`
 */
export function remapped(ids: string[], listId: string, to: (index: number) => number | null): string[] {
  const prefix = `${listId}/`
  return ids.flatMap((id) => {
    if (!id.startsWith(prefix)) return [id]
    const [index, ...rest] = id.slice(prefix.length).split('/')
    const next = to(Number(index))
    return next === null ? [] : [[`${prefix}${next}`, ...rest].join('/')]
  })
}

/** The ids of the nodes down to the deepest one with `target` in what it covers */
export function pathTo(nodes: ConfigNode[], target: unknown): string[] | null {
  for (const node of nodes) {
    if (!holds(node.value, target)) continue
    return [node.id, ...(pathTo(node.children ?? [], target) ?? [])]
  }
  return null
}

function holds(value: unknown, target: unknown): boolean {
  if (value === target) return true
  if (value === null || typeof value !== 'object') return false
  return Object.values(value).some((inner) => holds(inner, target))
}

/** A node with nodes under it, or fields to open up to, can be opened */
function branch(node: ConfigNode): ConfigNode {
  return node.fields && (node.children?.length ?? 0) === 0 ? { ...node, childrenCount: 0 } : node
}

/** Where a layer is, then what it draws and where */
function layerChildren(id: string, lens: Lens<SceneLayer>, led: boolean, surface: Surface): ConfigNode[] {
  const layer = lens.get()
  const place: ConfigNode[] = led
    ? [
        ...(layerPlane(layer) !== null ? [{ id: `${id}/plane`, label: 'Plane', kind: 'plane' as const, lens, value: layer.surface }] : []),
        { id: `${id}/leds`, label: 'LEDs', kind: 'leds', lens: field(lens, 'leds'), value: layer.leds },
        { id: `${id}/z`, label: 'Z-index', kind: 'z', lens, value: layer.z },
      ]
    : []
  // a sequence's LEDs are already the ones it shows on, so it only has a region its file gave it, to be removed
  const regions = led && layer.surface === 'Sequence' && !layer.region ? [] : [regionNode(`${id}/region`, field(lens, 'region'), surface, lens)]
  const fill = fillNode(`${id}/fill`, field(lens, 'fill'), surface)
  const mask = maskNode(`${id}/mask`, field(lens, 'mask'), surface)
  // on an LED layer the mask comes first, as it's over everything else the layer is
  return led ? [mask, ...place, ...regions, fill] : [...regions, fill, mask]
}

/** A mask's region, or a layer's, which can be its graph instead */
function regionNode(id: string, lens: Lens<Shape | undefined>, surface: Surface, layer?: Lens<SceneLayer>): ConfigNode {
  const graph = layer?.get().graph
  if (layer && graph) {
    const traversals = field(field(layer, 'graph') as Lens<GraphMask>, 'traversals')
    return branch({
      id,
      label: 'Region',
      kind: 'region',
      lens,
      layer,
      value: { region: lens.get(), graph },
      fields: true,
      children: [traversalsNode(`${id}/traversals`, traversals, layer, surface)],
    })
  }
  const shape = lens.get()
  return branch({
    id,
    label: 'Region',
    kind: 'region',
    lens,
    layer,
    value: shape,
    children: shape ? shapeChildren(id, lens as Lens<Shape>, surface) : [],
  })
}

function pathNode(id: string, lens: Lens<Shape>, surface: Surface): ConfigNode {
  return branch({ id, label: 'Path', kind: 'path', lens, value: lens.get(), children: shapeChildren(id, lens, surface, false) })
}

/** A shape's sizes, or its points, then its transforms. A sweep follows only a path's own outline, so a path isn't offered a mirror, revolve, or divide. */
function shapeChildren(id: string, lens: Lens<Shape>, surface: Surface, copies = true): ConfigNode[] {
  const base = field(lens, 'base') as Lens<ShapeNode>
  const current = base.get()
  const sizes: ConfigNode[] = shapeProps(base, surface).map(({ key, label, fallback, lens: propLens, ranges }) =>
    propNode(`${id}/${key}`, label, { lens: propLens, paramName: key, fallback, ranges }, surface),
  )
  if ('Polygon' in current) sizes.push(pointsNode(`${id}/points`, 'Polygon', field(variant(base, 'Polygon'), 'points'), surface))
  if ('Polyline' in current) sizes.push(pointsNode(`${id}/points`, 'Polyline', field(variant(base, 'Polyline'), 'points'), surface))
  if ('Bezier' in current) sizes.push(pointsNode(`${id}/points`, 'Bezier', field(variant(base, 'Bezier'), 'points'), surface))

  return [...sizes, ...modifierNodes(id, orEmpty(field(lens, 'transform')), 'shape', surface, undefined, copies ? lens : undefined)]
}

/** A polygon has to have three points, and a line two */
function pointsNode(id: string, shape: PointsShape, lens: Lens<Prop<Vec2>[]> | Lens<BezierPoint[]>, surface: Surface): ConfigNode {
  const least = shape === 'Polygon' ? 3 : 2
  const list = lens as Lens<unknown[]>
  const ranges = pointRanges(surface)
  const position = (propLens: Lens<Prop<Vec2>>): PropSpec => ({ lens: propLens as Lens<Prop<SceneValue>>, paramName: 'point', fallback: [0, 0], ranges })
  const children = list.get().map((_, index): ConfigNode => {
    const place = { list, listId: id, index, canRemove: list.get().length > least }
    const pointId = `${id}/${index}`
    if (shape !== 'Bezier') return { ...propNode(pointId, `Point ${index + 1}`, position(item(lens as Lens<Prop<Vec2>[]>, index)), surface), place }
    const point = item(lens as Lens<BezierPoint[]>, index)
    return branch({
      id: pointId,
      label: `Point ${index + 1}`,
      kind: 'bezier-point',
      value: point.get(),
      place,
      children: [
        propNode(`${pointId}/point`, 'Position', position(field(point, 'point')), surface),
        propNode(`${pointId}/handle`, 'Handle', { ...position(field(point, 'handle')), paramName: 'handle' }, surface),
      ],
    })
  })
  return branch({ id, label: 'Points', kind: 'points', shape, lens, value: lens.get(), children })
}

/**
 * Transforms then adjusts, as rows beside the props of what they change, and a row for adding either. Translates,
 * rotates, scales, and adjusts are props of their own, a fit picks its traversal in its row, and a mirror, revolve, or
 * divide has its props under it. With `shape`, the row for adding offers those too.
 */
function modifierNodes(id: string, transforms: Lens<Transform[]>, space: TransformSpace, surface: Surface, adjusts?: Lens<ColorAdjust[]>, shape?: Lens<Shape>): ConfigNode[] {
  const transformList = `${id}/transform`
  const transformNodes = transforms.get().map((transform, index): ConfigNode => {
    const transformId = `${transformList}/${index}`
    const lens = item(transforms, index)
    const place = { list: transforms as Lens<unknown[]>, listId: transformList, index }
    const kind = transformKind(transform)
    if (kind === 'Fit') return { id: transformId, label: TRANSFORM_LABELS.Fit, kind: 'fit', lens, value: transform, place }
    if (kind === 'Mirror' || kind === 'Revolve' || kind === 'Divide') {
      const children = copyProps(lens, surface).map(({ key, label, fallback, lens: propLens, ranges }) =>
        propNode(`${transformId}/${key}`, label, { lens: propLens, paramName: `${kind.toLowerCase()}_${key}`, fallback, ranges }, surface),
      )
      return branch({ id: transformId, label: TRANSFORM_LABELS[kind], kind: 'copy', lens, value: transform, place, modifier: kind, children })
    }
    const spec: PropSpec =
      kind === 'Translate'
        ? { lens: variant(lens, 'Translate') as Lens<Prop<SceneValue>>, paramName: 'offset', fallback: [0, 0], ranges: translateRanges(surface, space) }
        : kind === 'Rotate'
          ? { lens: variant(lens, 'Rotate') as Lens<Prop<SceneValue>>, paramName: 'rotation', fallback: 0, ranges: both(ROTATION) }
          : { lens: variant(lens, 'Scale') as Lens<Prop<SceneValue>>, paramName: 'scale', fallback: 1, ranges: scaleRanges() }
    // a translate is a position, so it can sweep around a path, which the prop has under it
    return { ...propNode(transformId, TRANSFORM_LABELS[kind], spec, surface), place, modifier: kind }
  })
  const adjustList = `${id}/adjust`
  const adjustNodes = (adjusts?.get() ?? []).map((adjust, index): ConfigNode => {
    const lens = item(adjusts!, index)
    const kind = adjustKind(adjust)
    const { label, paramName, range } = ADJUSTS[kind]
    const spec: PropSpec = { lens: adjustAmount(lens) as Lens<Prop<SceneValue>>, paramName, fallback: range.rest!, ranges: both(range) }
    return { ...propNode(`${adjustList}/${index}`, label, spec, surface), place: { list: adjusts as Lens<unknown[]>, listId: adjustList, index }, modifier: kind }
  })
  const add: ConfigNode = { id: `${id}/modifiers`, label: 'Modifier', kind: 'modifiers', transforms, adjusts, shape, value: undefined }
  return [...transformNodes, ...adjustNodes, add]
}

/** A solid's color, a gradient's stops, or an animated image's frame, then its transforms and adjusts */
function fillNode(id: string, lens: Lens<Fill>, surface: Surface): ConfigNode {
  const base = field(lens, 'base')
  const current = base.get()
  const own: ConfigNode[] = []
  if ('Solid' in current) {
    own.push(propNode(`${id}/color`, 'Color', { lens: variant(base, 'Solid') as Lens<Prop<SceneValue>>, paramName: 'color', fallback: [255, 255, 255, 255], ranges: both(UNIT) }, surface))
  }
  if ('Gradient' in current) own.push(stopsNode(`${id}/stops`, field(variant(base, 'Gradient'), 'stops'), surface))
  if ('Image' in current) {
    const frames = contentImageInfo(current.Image.name)?.frames
    const frame = current.Image.frame
    // one that's already moving stays in reach, so it can be undone after picking an image that isn't animated
    if ((frames ?? 1) > 1 || (frame !== undefined && !('Fixed' in frame))) {
      const frameLens = orElse(field(variant(base, 'Image'), 'frame'), { Fixed: 0 })
      own.push(propNode(`${id}/frame`, 'Frame', { lens: frameLens as Lens<Prop<SceneValue>>, paramName: 'frame', fallback: 0, ranges: frameRanges(frames) }, surface))
    }
  }
  return branch({
    id,
    label: 'Fill',
    kind: 'fill',
    lens,
    value: lens.get(),
    fields: 'Image' in current,
    children: [...own, ...modifierNodes(id, orEmpty(field(lens, 'transform')), 'fill', surface, orEmpty(field(lens, 'adjust')))],
  })
}

/** Each stop is its color, spaced evenly by its order. There's always one, as a gradient of none would be nothing. */
function stopsNode(id: string, lens: Lens<Stop[]>, surface: Surface): ConfigNode {
  const list = lens as Lens<unknown[]>
  const children = lens.get().map((_, index) => {
    const color = field(item(lens, index), 'color') as Lens<Prop<SceneValue>>
    const place = { list, listId: id, index, canRemove: lens.get().length > 1, spread: true }
    return { ...propNode(`${id}/${index}`, `Stop ${index + 1}`, { lens: color, paramName: 'stop_color', fallback: [255, 255, 255, 255], ranges: both(UNIT) }, surface), place }
  })
  return branch({ id, label: 'Stops', kind: 'stops', lens, value: lens.get(), children })
}

/** With no mask there's nothing under it, and its row has a button to add one */
function maskNode(id: string, lens: Lens<Mask | undefined>, surface: Surface): ConfigNode {
  const mask = lens.get()
  if (!mask) return { id, label: 'Mask', kind: 'mask', lens, value: undefined }
  const present = lens as Lens<Mask>
  return branch({
    id,
    label: 'Mask',
    kind: 'mask',
    lens,
    value: mask,
    fields: true,
    children: [regionNode(`${id}/region`, field(present, 'region'), surface), fillNode(`${id}/fill`, field(present, 'fill'), surface)],
  })
}

function traversalsNode(id: string, lens: Lens<Traversal[]>, layer: Lens<SceneLayer>, surface: Surface): ConfigNode {
  const children = lens.get().map((_, index) => traversalNode(`${id}/${index}`, item(lens, index), layer, surface, { list: lens as Lens<unknown[]>, listId: id, index }))
  return branch({ id, label: 'Traversals', kind: 'traversals', lens, layer, value: lens.get(), children })
}

/** Where a traversal goes from and to, then the props of its algorithm and speed */
function traversalNode(id: string, lens: Lens<Traversal>, layer: Lens<SceneLayer>, surface: Surface, place: ListPlace): ConfigNode {
  const traversal = lens.get()
  const algorithm = field(lens, 'algorithm')
  const kind = algorithmKind(traversal.algorithm)
  const target = traversalTarget(algorithm)
  const prop = (key: string, label: string, fallback: number, ranges: Ranges, propLens: Lens<Prop<number>>) =>
    propNode(`${id}/${key}`, label, { lens: propLens as Lens<Prop<SceneValue>>, paramName: key, fallback, ranges }, surface)

  const children: ConfigNode[] = [nodeRefNode(`${id}/from`, 'From', field(lens, 'from'), surface)]
  if (target) children.push(nodeRefNode(`${id}/to`, 'To', target, surface))
  if (kind === 'FanOut') {
    const fan = variant(algorithm as Lens<Extract<TraversalAlgorithm, { FanOut: unknown }>>, 'FanOut')
    children.push(prop('angle', 'Arc angle', FAN_ANGLE.rest!, both(FAN_ANGLE), field(fan, 'angle')), prop('count', 'Additional paths', FAN_COUNT.rest!, both(FAN_COUNT), field(fan, 'count')))
  }
  if (kind === 'Growth') {
    const growth = variant(algorithm as Lens<Extract<TraversalAlgorithm, { Growth: unknown }>>, 'Growth')
    const reach = reachRange(surface)
    children.push(
      prop('reach', 'Reach', reach.rest!, both(reach), field(growth, 'reach')),
      prop('density', 'Density', GROWTH_DENSITY.rest!, both(GROWTH_DENSITY), field(growth, 'density')),
      prop('branching', 'Branching', BRANCHING.rest!, both(BRANCHING), field(growth, 'branching')),
    )
  }
  if (WANDERING.includes(kind)) {
    const wandering = algorithm as unknown as Lens<Record<Wandering, { wander: Prop<number> }>>
    children.push(prop('wander', 'Wander', WANDER.rest!, both(WANDER), field(variant(wandering, kind as Wandering), 'wander')))
  }
  if ('Rate' in traversal.speed) {
    const label = BY_DISTANCE.includes(kind) ? 'Units a second' : 'Steps a second'
    children.push(prop('rate', label, RATE.rest!, both(RATE), variant(field(lens, 'speed') as Lens<Extract<Speed, { Rate: unknown }>>, 'Rate')))
  }
  return branch({ id, label: traversal.name, kind: 'traversal', lens, layer, value: traversal, fields: true, place, children })
}

/** An LED is chosen by name in its fields, and a position is a prop under it */
function nodeRefNode(id: string, label: string, lens: Lens<NodeRef>, surface: Surface): ConfigNode {
  const current = lens.get()
  const children =
    'Nearest' in current
      ? [propNode(`${id}/position`, 'Position', { lens: variant(lens, 'Nearest') as Lens<Prop<SceneValue>>, paramName: 'position', fallback: [0, 0], ranges: translateRanges(surface, 'shape') }, surface)]
      : []
  return branch({ id, label, kind: 'node-ref', lens, value: current, fields: 'Name' in current, children })
}

/** A prop opens to its field. One that sweeps has its path and its position along it under it, and one in a wave its phase. */
function propNode(id: string, label: string, spec: PropSpec, surface: Surface): ConfigNode {
  const prop = spec.lens.get()
  const under = (name: 'position' | 'phase', label: string, lens: Lens<unknown>) =>
    propNode(`${id}/${name}`, label, { lens: lens as Lens<Prop<SceneValue>>, paramName: `${spec.paramName}_${name}`, fallback: 0, ranges: both(UNIT) }, surface)
  const children =
    'Sweep' in prop
      ? [pathNode(`${id}/path`, field(variant(spec.lens, 'Sweep'), 'path'), surface), under('position', 'Position', field(variant(spec.lens, 'Sweep'), 'position'))]
      : 'Modulated' in prop
        ? [under('phase', 'Phase', field(variant(spec.lens, 'Modulated') as Lens<Modulated>, 'phase'))]
        : []
  // keyframes are edited on the timeline and a fixed color by its swatch in the row, so there's nothing to open
  const fixedColor = 'Fixed' in prop && Array.isArray(prop.Fixed) && prop.Fixed.length === 4
  return branch({ id, label, kind: 'prop', ...spec, value: prop, fields: !('Keyframes' in prop) && !fixedColor, children })
}
