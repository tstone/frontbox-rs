import { contentImageInfo } from '../../state/contentImages'
import type { BezierPoint } from '../../types/generated/BezierPoint'
import type { ColorAdjust } from '../../types/generated/ColorAdjust'
import type { GraphMask } from '../../types/generated/GraphMask'
import type { Layer } from '../../types/generated/Layer'
import type { LightQ } from '../../types/generated/LightQ'
import type { Modulated } from '../../types/generated/Modulated'
import type { NodeRef } from '../../types/generated/NodeRef'
import type { Prop } from '../../types/generated/Prop'
import type { ShapeAdjust } from '../../types/generated/ShapeAdjust'
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
  BULGE,
  cornerRange,
  transformProps,
  FAN_ANGLE,
  FAN_COUNT,
  field,
  frameRanges,
  GROWTH_DENSITY,
  item,
  layerLabel,
  layerVisible,
  orElse,
  orEmpty,
  pointRanges,
  RATE,
  reachRange,
  ROTATION,
  shapeProps,
  soundLabel,
  strokeProps,
  traversalTarget,
  translateRanges,
  UNIT,
  variant,
  WANDER,
  EVOLUTION,
  NOISE_SCALE,
  CELL_JITTER,
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
 * A node of the scene, a row in the tree on the timeline. Its id is its path through the scene, such as `layers/0/fill/transform/1`, so it stays put
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
  /** For a shape's first translate, which is where the shape is rather than a modifier added to it */
  placement?: boolean
  /** For a node with fields and nothing under it, which still opens to show them */
  childrenCount?: number
  children?: ConfigNode[]
}

/** What a node edits, which says how it's drawn */
export type NodeData =
  /** One of the tree's roots: the scene's layers, its variables, or its named values */
  | { kind: 'section'; section: Section }
  | { kind: 'variable'; name: string }
  | { kind: 'value'; name: string }
  /** `unnamed` is what a layer is called until it's given a name */
  | { kind: 'layer'; lens: Lens<SceneLayer>; unnamed: string }
  | { kind: 'sound'; lens: Lens<SoundLayer>; unnamed: string }
  | { kind: 'place'; lens: Lens<SceneLayer> }
  | { kind: 'lights'; lens: Lens<LightQ | undefined> }
  | { kind: 'z'; lens: Lens<SceneLayer> }
  /** With `layer`, a layer's region, which on a light scene can be its graph instead */
  | { kind: 'region'; lens: Lens<Shape | undefined>; layer?: Lens<SceneLayer> }
  /** The shape a sweep goes around */
  | { kind: 'path'; lens: Lens<Shape> }
  | { kind: 'fill'; lens: Lens<Fill> }
  | { kind: 'stops'; lens: Lens<Stop[]> }
  /** Whether a layer is a mask over the layers below it, and which side of it they show on */
  | { kind: 'mask'; lens: Lens<SceneLayer> }
  /** A fit transform, which picks its traversal in its row. Other transforms are props. */
  | { kind: 'fit'; lens: Lens<Transform> }
  /** A scale, mirror, revolve, bisect, shift, or warp, which has its props under it */
  | { kind: 'copy'; lens: Lens<Transform> }
  /**
   * The row for adding a transform, or an adjust where there's `adjusts`. With `shape`, it also offers a mirror, revolve,
   * bisect, or stroke.
   */
  | { kind: 'modifiers'; transforms: Lens<Transform[]>; adjusts?: Lens<ColorAdjust[]>; shape?: Lens<Shape> }
  /** A stroke along a shape's edge, which has its props under it */
  | { kind: 'stroke'; lens: Lens<ShapeAdjust> }
  | { kind: 'points'; shape: PointsShape; lens: Lens<Prop<Vec2>[]> | Lens<BezierPoint[]> }
  | { kind: 'bezier-point' }
  | { kind: 'traversals'; lens: Lens<Traversal[]>; layer: Lens<SceneLayer> }
  | { kind: 'traversal'; lens: Lens<Traversal>; layer: Lens<SceneLayer> }
  | { kind: 'node-ref'; lens: Lens<NodeRef> }
  | ({ kind: 'prop' } & PropSpec)

export type NodeKind = NodeData['kind']

/** A node of a kind, or `false` for one of another */
export function is<K extends NodeKind>(node: ConfigNode, kind: K) {
  return node.kind === kind ? (node as Extract<ConfigNode, { kind: K }>) : false
}

/** The tree's roots, by id */
export type Section = 'scene' | 'variables' | 'values'

/** Everything a prop's field needs */
export type PropSpec = {
  lens: Lens<Prop<SceneValue>>
  /** What to call a variable made from it */
  variableName: string
  /** For a prop with nothing to take a value from */
  fallback: SceneValue
  ranges: Ranges
  /** For a scale's anchor in a layer's region: the shape, and where the scale is in its transforms */
  anchorOf?: { shape: Lens<Shape>; index: number }
}

export type PointsShape = 'Polygon' | 'Polyline' | 'Bezier'

/** Where a node sits in a list, by the id of the list's node */
export type ListPlace = {
  list: Lens<unknown[]>
  listId: string
  index: number
  canRemove?: boolean
  /**
   * For a gradient's stops: moving one keeps the positions in their places, so its color is what moves, and taking one
   * out spaces the rest evenly again if they were
   */
  spread?: boolean
}

export type AlgorithmKind = 'BreadthFirst' | 'DepthFirst' | 'Dijkstra' | 'ShortestPath' | 'FanOut' | 'Growth' | 'AStar' | 'RandomWalk'
export type Wandering = 'BreadthFirst' | 'Dijkstra' | 'ShortestPath' | 'FanOut' | 'Growth'
export const WANDERING: AlgorithmKind[] = ['BreadthFirst', 'Dijkstra', 'ShortestPath', 'FanOut', 'Growth']
/** Algorithms whose rate is a distance a second, rather than a number of steps */
export const BY_DISTANCE: AlgorithmKind[] = ['Dijkstra', 'ShortestPath', 'FanOut', 'Growth']
export const algorithmKind = (algorithm: TraversalAlgorithm) => (typeof algorithm === 'string' ? algorithm : Object.keys(algorithm)[0]) as AlgorithmKind

export const TRANSFORM_LABELS = { Translate: 'Translate', Rotate: 'Rotate', Scale: 'Scale', Fit: 'Fit to traversal', Mirror: 'Mirror', Revolve: 'Revolve', Bisect: 'Bisect', Shift: 'Shift', Bulge: 'Bulge', Round: 'Round corners', Warp: 'Warp' }
export type TransformKind = keyof typeof TRANSFORM_LABELS
export const transformKind = (transform: Transform) => Object.keys(transform)[0] as TransformKind

/** Three roots: the scene, with its layers then its sound layers; its variables; and its named values */
export function configNodes(scene: Lens<Scene>, surfaces: LayerSurfaces): ConfigNode[] {
  const { variables = {}, values = {} } = scene.get()
  return [
    branch({ id: 'scene', label: 'Scene', kind: 'section', section: 'scene', value: { layers: scene.get().layers, sounds: scene.get().sounds }, children: layerNodes(scene, surfaces) }),
    branch({
      id: 'variables',
      label: 'Variables',
      kind: 'section',
      section: 'variables',
      value: variables,
      // a note on how to add one, while there are none
      fields: Object.keys(variables).length === 0,
      children: Object.keys(variables)
        .sort()
        .map((name) => branch({ id: `variables/${name}`, label: name, kind: 'variable', name, value: variables[name], fields: true })),
    }),
    branch({
      id: 'values',
      label: 'Values',
      kind: 'section',
      section: 'values',
      value: values,
      fields: Object.keys(values).length === 0,
      children: Object.keys(values).map((name) => branch({ id: `values/${name}`, label: name, kind: 'value', name, value: values[name], fields: true })),
    }),
  ]
}

/** The scene's layers, then its sound layers */
function layerNodes(scene: Lens<Scene>, surfaces: LayerSurfaces): ConfigNode[] {
  const layers = field(scene, 'layers')
  const sounds = field(scene, 'sounds')
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
        children: layerChildren(id, lens, surfaces(layer)),
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
      })
    }),
  ]
}

/** Which layer a node is in, which is what's selected in the preview while it is */
export function selectionOf(id: string | null): Selection | null {
  const match = id?.match(/^layers\/(\d+)/)
  return match ? { layer: Number(match[1]) } : null
}

export function selectionNode({ layer }: Selection) {
  return `layers/${layer}`
}

/** Which of a layer's region's transforms a node is, or is under. `null` for a node outside them. */
export function openTransform(id: string | null, of: Selection): number | null {
  const prefix = `${selectionNode(of)}/region/transform/`
  if (!id?.startsWith(prefix)) return null
  const index = Number(id.slice(prefix.length).split('/')[0])
  return Number.isInteger(index) ? index : null
}

/** The nodes that a node is under, from its root down, ending with itself. Layers and sounds are under the scene. */
export function ancestry(id: string): string[] {
  const parts = id.split('/')
  const root = parts[0] === 'layers' || parts[0] === 'sounds' ? 'scene' : parts[0]
  const under = parts.slice(1).map((_, i) => parts.slice(0, i + 2).join('/'))
  return [root, ...under.filter((node) => node !== root)]
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
function layerChildren(id: string, lens: Lens<SceneLayer>, surface: Surface): ConfigNode[] {
  const layer = lens.get()
  const light = layer.display === undefined
  const place: ConfigNode[] = [
    // a sequence is on its lights alone, so only an area's plane and a display layer's display can be picked
    ...(layer.surface !== 'Sequence' ? [{ id: `${id}/place`, label: 'Draws on', kind: 'place' as const, lens, value: layer.surface ?? layer.display }] : []),
    ...(light ? [{ id: `${id}/lights`, label: 'Lights', kind: 'lights' as const, lens: field(lens, 'lights'), value: layer.lights }] : []),
    { id: `${id}/z`, label: 'Z-index', kind: 'z', lens, value: layer.z },
  ]
  // a sequence's lights are already the ones it shows on, so it only has a region its file gave it, to be removed
  const regions = light && layer.surface === 'Sequence' && !layer.region ? [] : [regionNode(`${id}/region`, field(lens, 'region'), surface, lens)]
  const fill = fillNode(`${id}/fill`, field(lens, 'fill'), surface)
  const mask: ConfigNode = { id: `${id}/mask`, label: 'Mask', kind: 'mask', lens, value: layer.mask }
  const visible = propNode(`${id}/visible`, 'Visible', { lens: layerVisible(lens) as Lens<Prop<SceneValue>>, variableName: 'visible', fallback: true, ranges: both(UNIT) }, surface)
  return [visible, mask, ...place, ...regions, fill]
}

/** A layer's region, which can be its graph instead */
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
    // text has its words and font as fields
    fields: shape !== undefined && 'Text' in shape.base,
    children: shape ? shapeChildren(id, lens as Lens<Shape>, surface) : [],
  })
}

function pathNode(id: string, lens: Lens<Shape>, surface: Surface): ConfigNode {
  return branch({ id, label: 'Path', kind: 'path', lens, value: lens.get(), children: shapeChildren(id, lens, surface, false) })
}

/** A shape's sizes, or its points, then its transforms. A sweep follows only a path's own outline, so a path isn't offered a mirror, revolve, or bisect. */
function shapeChildren(id: string, lens: Lens<Shape>, surface: Surface, copies = true): ConfigNode[] {
  const base = field(lens, 'base') as Lens<ShapeNode>
  const current = base.get()
  const sizes: ConfigNode[] = shapeProps(base, surface).map(({ key, label, fallback, lens: propLens, ranges }) =>
    propNode(`${id}/${key}`, label, { lens: propLens, variableName: key, fallback, ranges }, surface),
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
  const position = (propLens: Lens<Prop<Vec2>>): PropSpec => ({ lens: propLens as Lens<Prop<SceneValue>>, variableName: 'point', fallback: [0, 0], ranges })
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
        propNode(`${pointId}/handle`, 'Handle', { ...position(field(point, 'handle')), variableName: 'handle' }, surface),
      ],
    })
  })
  return branch({ id, label: 'Points', kind: 'points', shape, lens, value: lens.get(), children })
}

/**
 * Transforms then adjusts, as rows beside the props of what they change, and a row for adding either. Translates,
 * rotates, bulges, rounds, and adjusts are props of their own, a fit picks its traversal in its row, and a scale,
 * mirror, revolve, bisect, shift, or warp has its props under it. With `shape`, the row for adding offers those too, and its
 * strokes are listed after its transforms, each with its props under it.
 */
function modifierNodes(id: string, transforms: Lens<Transform[]>, space: TransformSpace, surface: Surface, adjusts?: Lens<ColorAdjust[]>, shape?: Lens<Shape>): ConfigNode[] {
  const transformList = `${id}/transform`
  // a drawn shape is placed by its first translate, which the copies and reshapes go before
  const placement = shape ? transforms.get().findIndex((transform) => 'Translate' in transform) : -1
  const transformNodes = transforms.get().map((transform, index): ConfigNode => {
    const transformId = `${transformList}/${index}`
    const lens = item(transforms, index)
    const place = { list: transforms as Lens<unknown[]>, listId: transformList, index }
    const kind = transformKind(transform)
    if (kind === 'Fit') return { id: transformId, label: TRANSFORM_LABELS.Fit, kind: 'fit', lens, value: transform, place }
    if (kind === 'Mirror' || kind === 'Revolve' || kind === 'Bisect' || kind === 'Shift' || kind === 'Scale' || kind === 'Warp') {
      const children = transformProps(lens, surface, space).map(({ key, label, fallback, lens: propLens, ranges }) => {
        const anchorOf = key === 'anchor' && shape ? { shape, index } : undefined
        return propNode(`${transformId}/${key}`, label, { lens: propLens, variableName: `${kind.toLowerCase()}_${key}`, fallback, ranges, anchorOf }, surface)
      })
      // a warp's detail and pattern aren't props, so they're its fields
      return branch({ id: transformId, label: TRANSFORM_LABELS[kind], kind: 'copy', lens, value: transform, place, modifier: kind, fields: kind === 'Warp', children })
    }
    const spec: PropSpec =
      kind === 'Translate'
        ? { lens: variant(lens, 'Translate') as Lens<Prop<SceneValue>>, variableName: 'offset', fallback: [0, 0], ranges: translateRanges(surface, space) }
        : kind === 'Rotate'
          ? { lens: variant(lens, 'Rotate') as Lens<Prop<SceneValue>>, variableName: 'rotation', fallback: 0, ranges: both(ROTATION) }
          : kind === 'Bulge'
            ? { lens: variant(lens, 'Bulge') as Lens<Prop<SceneValue>>, variableName: 'bulge', fallback: BULGE.rest!, ranges: both(BULGE) }
            : { lens: variant(lens, 'Round') as Lens<Prop<SceneValue>>, variableName: 'corner_radius', fallback: 0, ranges: both(cornerRange(surface)) }
    // a translate is a position, so it can sweep around a path, which the prop has under it
    if (index === placement) return { ...propNode(transformId, 'Position', spec, surface), place, modifier: kind, placement: true }
    return { ...propNode(transformId, TRANSFORM_LABELS[kind], spec, surface), place, modifier: kind }
  })
  const adjustList = `${id}/adjust`
  const adjustNodes = (adjusts?.get() ?? []).map((adjust, index): ConfigNode => {
    const lens = item(adjusts!, index)
    const kind = adjustKind(adjust)
    const { label, variableName, range } = ADJUSTS[kind]
    const spec: PropSpec = { lens: adjustAmount(lens) as Lens<Prop<SceneValue>>, variableName, fallback: range.rest!, ranges: both(range) }
    return { ...propNode(`${adjustList}/${index}`, label, spec, surface), place: { list: adjusts as Lens<unknown[]>, listId: adjustList, index }, modifier: kind }
  })
  const strokes = shape && orEmpty(field(shape, 'adjust'))
  const strokeNodes = (strokes?.get() ?? []).map((stroke, index): ConfigNode => {
    const strokeId = `${adjustList}/${index}`
    const lens = item(strokes!, index)
    const children = strokeProps(lens, surface).map(({ key, label, fallback, lens: propLens, ranges }) =>
      propNode(`${strokeId}/${key}`, label, { lens: propLens, variableName: `stroke_${key}`, fallback, ranges }, surface),
    )
    const place = { list: strokes as Lens<unknown[]>, listId: adjustList, index }
    return branch({ id: strokeId, label: 'Stroke', kind: 'stroke', lens, value: stroke, place, fields: true, children })
  })
  const add: ConfigNode = { id: `${id}/modifiers`, label: 'Modifier', kind: 'modifiers', transforms, adjusts, shape, value: undefined }
  return [...transformNodes, ...adjustNodes, ...strokeNodes, add]
}

/**
 * A solid's color, a gradient's stops, a noise's scale, evolution, and stops, cells' scale, jitter, evolution, and stops, an animated image's frame, or a sprite
 * sheet's sprite, then its transforms and adjusts
 */
function fillNode(id: string, lens: Lens<Fill>, surface: Surface): ConfigNode {
  const base = field(lens, 'base')
  const current = base.get()
  const own: ConfigNode[] = []
  if ('Solid' in current) {
    own.push(propNode(`${id}/color`, 'Color', { lens: variant(base, 'Solid') as Lens<Prop<SceneValue>>, variableName: 'color', fallback: [255, 255, 255, 255], ranges: both(UNIT) }, surface))
  }
  if ('Gradient' in current) own.push(stopsNode(`${id}/stops`, field(variant(base, 'Gradient'), 'stops'), surface))
  if ('Noise' in current) {
    const noise = variant(base, 'Noise')
    own.push(
      propNode(`${id}/scale`, 'Scale', { lens: field(noise, 'scale') as Lens<Prop<SceneValue>>, variableName: 'noise_scale', fallback: 0.25, ranges: both(NOISE_SCALE) }, surface),
      propNode(`${id}/evolution`, 'Evolution', { lens: field(noise, 'evolution') as Lens<Prop<SceneValue>>, variableName: 'evolution', fallback: 0, ranges: both(EVOLUTION) }, surface),
      stopsNode(`${id}/stops`, field(noise, 'stops'), surface),
    )
  }
  if ('Cells' in current) {
    const cells = variant(base, 'Cells')
    own.push(
      propNode(`${id}/scale`, 'Scale', { lens: field(cells, 'scale') as Lens<Prop<SceneValue>>, variableName: 'cell_scale', fallback: 0.25, ranges: both(NOISE_SCALE) }, surface),
      propNode(`${id}/jitter`, 'Jitter', { lens: field(cells, 'jitter') as Lens<Prop<SceneValue>>, variableName: 'jitter', fallback: 1, ranges: both(CELL_JITTER) }, surface),
      propNode(`${id}/evolution`, 'Evolution', { lens: field(cells, 'evolution') as Lens<Prop<SceneValue>>, variableName: 'evolution', fallback: 0, ranges: both(EVOLUTION) }, surface),
      stopsNode(`${id}/stops`, field(cells, 'stops'), surface),
    )
  }
  if ('Image' in current) {
    const frames = contentImageInfo(current.Image.name)?.frames
    const frame = current.Image.frame
    // one that's already moving stays in reach, so it can be undone after picking an image that isn't animated
    if ((frames ?? 1) > 1 || (frame !== undefined && !('Fixed' in frame))) {
      const frameLens = orElse(field(variant(base, 'Image'), 'frame'), { Fixed: 0 })
      own.push(propNode(`${id}/frame`, 'Frame', { lens: frameLens as Lens<Prop<SceneValue>>, variableName: 'frame', fallback: 0, ranges: frameRanges(frames) }, surface))
    }
  }
  if ('Spritesheet' in current) {
    const { rows, columns } = current.Spritesheet
    const spriteLens = orElse(field(variant(base, 'Spritesheet'), 'sprite'), { Fixed: 0 })
    own.push(propNode(`${id}/sprite`, 'Sprite', { lens: spriteLens as Lens<Prop<SceneValue>>, variableName: 'sprite', fallback: 0, ranges: frameRanges(rows * columns) }, surface))
  }
  return branch({
    id,
    label: 'Fill',
    kind: 'fill',
    lens,
    value: lens.get(),
    fields: 'Image' in current || 'Spritesheet' in current || 'Noise' in current || 'Cells' in current,
    children: [...own, ...modifierNodes(id, orEmpty(field(lens, 'transform')), 'fill', surface, orEmpty(field(lens, 'adjust')))],
  })
}

/**
 * Each stop is its color, with its position under it, so the colors are all in sight at once. There's always one, as a
 * gradient of none would be nothing.
 */
function stopsNode(id: string, lens: Lens<Stop[]>, surface: Surface): ConfigNode {
  const list = lens as Lens<unknown[]>
  const children = lens.get().map((_, index) => {
    const stop = item(lens, index)
    const color = propNode(`${id}/${index}`, `Stop ${index + 1}`, { lens: field(stop, 'color') as Lens<Prop<SceneValue>>, variableName: 'stop_color', fallback: [255, 255, 255, 255], ranges: both(UNIT) }, surface)
    const position = propNode(`${id}/${index}/at`, 'Position', { lens: field(stop, 'at') as Lens<Prop<SceneValue>>, variableName: 'stop_position', fallback: 0, ranges: both(UNIT) }, surface)
    const place = { list, listId: id, index, canRemove: lens.get().length > 1, spread: true }
    return { ...color, place, children: [...(color.children ?? []), position] }
  })
  return branch({ id, label: 'Stops', kind: 'stops', lens, value: lens.get(), children })
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
    propNode(`${id}/${key}`, label, { lens: propLens as Lens<Prop<SceneValue>>, variableName: key, fallback, ranges }, surface)

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

/** A light is chosen by name in its fields, and a position is a prop under it */
function nodeRefNode(id: string, label: string, lens: Lens<NodeRef>, surface: Surface): ConfigNode {
  const current = lens.get()
  const children =
    'Nearest' in current
      ? [propNode(`${id}/position`, 'Position', { lens: variant(lens, 'Nearest') as Lens<Prop<SceneValue>>, variableName: 'position', fallback: [0, 0], ranges: translateRanges(surface, 'shape') }, surface)]
      : []
  return branch({ id, label, kind: 'node-ref', lens, value: current, fields: 'Name' in current, children })
}

/** A prop opens to its field. One that sweeps has its path and its position along it under it, and one in a wave its phase. */
function propNode(id: string, label: string, spec: PropSpec, surface: Surface): ConfigNode {
  const prop = spec.lens.get()
  const under = (name: 'position' | 'phase', label: string, lens: Lens<unknown>) =>
    propNode(`${id}/${name}`, label, { lens: lens as Lens<Prop<SceneValue>>, variableName: `${spec.variableName}_${name}`, fallback: 0, ranges: both(UNIT) }, surface)
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
