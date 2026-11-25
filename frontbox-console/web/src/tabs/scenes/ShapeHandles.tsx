import { createMemo, createSignal, Index, Show, useContext } from 'solid-js'
import type { Layer } from '../../types/generated/Layer'
import type { Prop } from '../../types/generated/Prop'
import type { RenderedShape } from '../../types/generated/RenderedShape'
import type { Transform } from '../../types/generated/Transform'
import type { PreviewSpace } from '../machine/PlayfieldView'
import { layerColorClass, type LayerColor } from './ScenePortal'
import { SceneContext } from './SceneFields'
import { field, isHidden, isOpen, item, orEmpty, variableReads, selectedShape, selectedShapeLens, shapePoints, startingValue, variant, withKey, withVariable, type Lens, type Scene, type SceneValue, type Selection, type Surface, type Vec2 } from './sceneModel'

type Shape = NonNullable<Layer['region']>

/** A transform as the machine sends it: where it takes x's axis, y's axis, and the origin */
type Affine = RenderedShape['transforms'][number]

/** A grab handle: where it is, and what dragging it does */
export type Handle = {
  label: string
  kind: 'move' | 'group' | 'size' | 'point' | 'bezier' | 'center' | 'turn' | 'shift' | 'anchor' | 'corner' | 'radius'
  cursor: string
  /** Where it sits in the scene */
  at: Vec2
  /** Set when it's held by something a drag can't change, such as a random value, a sweep, or a hidden track */
  locked: boolean
  /** Moves what it holds so the handle lands on a point in the scene, with `shift` set while shift is held */
  drag(to: Vec2, shift: boolean): void
  /** Set when it's on a path, which has a color of its own to tell it from the shape going along it */
  onPath?: boolean
  /** Where a line to it comes from in the scene, for one that belongs to a point */
  arm?: Vec2
  /** What a click on it that doesn't move does, such as switching what the corner handle does */
  click?: () => void
  /** A mark drawn in it, saying what it does */
  glyph?: string
  /** The index of the transform it changes, if it changes one that's there */
  transform?: number
}

/** What the handle on a shape's top right corner does when dragged: turn it, scale it, or round its corners */
type CornerMode = 'rotate' | 'scale' | 'round'

const CORNER_MODES: CornerMode[] = ['rotate', 'scale', 'round']

const CORNER_GLYPHS: Record<CornerMode, string> = { rotate: '↻', scale: '⤢', round: '◜' }

/** The lines a shape's points snap to, in the scene, and a corner of one of its cells to tell how far apart they're drawn */
type Grid = { lines: Vec2[][]; cell: [Vec2, Vec2, Vec2] }

/**
 * What's drawn over a shape: its handles, the paths it goes along, the lines it's mirrored or cut along, and the box
 * around it and its copies as lines in the scene, and the grids its points snap to
 */
type Overlay = { handles: Handle[]; paths: Vec2[][]; lines: Vec2[][]; boxes: Vec2[][]; grids: Grid[] }

/** How the values a handle writes are rounded, so a drag doesn't leave long decimals in a scene */
const PRECISION = 1000

/** The closest a key has to be to the playhead to count as the key at that moment, in seconds */
const SAME_MOMENT = 0.001

/** How many cells a grid goes past a shape's bounds on each side, so there's somewhere to drag its points out to */
const GRID_MARGIN = 4

/** The most lines a grid is drawn with. One finer than this still snaps, but is left undrawn. */
const MAX_GRID_LINES = 400

/** How far a mirror's or bisect's line goes past the shape at each end, as a share of how far it is across it */
const LINE_MARGIN = 0.15

/** The degrees a line's turned in at a time while shift is held */
const TURN_STEP = 15

/** How far a press on a handle can move, in pixels, and still be a click on it rather than a drag */
const CLICK_SLOP = 3

const IDENTITY: Affine = [1, 0, 0, 1, 0, 0]

const NOTHING: Overlay = { handles: [], paths: [], lines: [], boxes: [], grids: [] }

/**
 * Grab handles over the selected shape and the paths it follows, placed by the machine's account of where they are.
 * `outlines` has a loop for each copy of the shape, in the scene. With a `grid`, a shape's points and how far it's
 * moved snap to a grid of that size in its own space, which is drawn over a shape made of points. A scale's anchor only
 * has a handle while that scale is `openTransform`, the one being edited, as it's often over the shape's center. While
 * a transform is open, only its own handles show, and none for the shape's base, such as its size or points.
 */
export default function ShapeHandles(props: { space: PreviewSpace; selection: Selection | null; pose: RenderedShape | null; outlines: Vec2[][]; time: number; grid: number | null; openTransform: number | null }) {
  const scene = useContext(SceneContext)!
  // what the corner handle does, which a click on it moves on to the next of
  const [cornerMode, setCornerMode] = createSignal<CornerMode>('rotate')
  const nextMode = () => setCornerMode(CORNER_MODES[(CORNER_MODES.indexOf(cornerMode()) + 1) % CORNER_MODES.length])

  const overlay = createMemo((): Overlay => {
    const { selection, pose, grid } = props
    if (selection === null || !pose || !selectedShape(scene.get(), selection)) return NOTHING
    const shape = selectedShapeLens(scene, selection)
    if (outOfStep(shape, pose)) return NOTHING
    const write = writer(scene, props.time)
    const { handles, paths, grids, owners: pathOwners } = pathOverlay(shape, pose, write, IDENTITY, grid)
    const lines = lineOverlay(shape, pose, write)
    const group = groupOverlay(orEmpty(field(shape, 'transform')), pose, write, props.outlines, grid)
    // a sequence's shapes are along a line, which has no area to turn, scale, or round
    const layer = scene.get().layers[selection.layer]
    const flat = layer?.display === undefined && layer?.surface === 'Sequence'
    const open = props.openTransform
    const corner = flat || isOpen(shape.get().base) ? [] : cornerHandles(shape, pose, write, cornerMode(), nextMode, scene.get(), open)
    // a line's over the shape's own, as its center can sit where the shape's moved from. A path's on top of both, as
    // its shape is held to it and has nothing there to grab.
    const all = [...(group ? [group.handle] : []), ...shapeHandles(shape, pose, write, grid), ...anchorHandles(orEmpty(field(shape, 'transform')), pose, write, open), ...lines.handles, ...corner, ...handles]
    if (open === null) return { handles: all, paths, lines: lines.lines, boxes: group ? [group.box] : [], grids: [...shapeGrid(pose, grid, IDENTITY), ...grids] }
    return {
      handles: all.filter(({ transform }) => transform === open),
      paths: paths.filter((_, index) => pathOwners[index] === open),
      lines: lines.lines.filter((_, index) => lines.owners[index] === open),
      boxes: [],
      grids: [],
    }
  })

  const onScreen = (line: Vec2[]) => line.flatMap((point) => props.space.project(point) ?? []).join(' ')

  // the grid itself isn't drawn for now, as only shapes made of points had one, which made it look broken elsewhere;
  // snapping to it works on every shape all the same
  return (
    <div class="scene-handles">
      <svg class="scene-paths" aria-hidden="true">
        <Index each={overlay().boxes}>{(box) => <polyline class="scene-group" points={onScreen(box())} />}</Index>
        <Index each={overlay().paths}>{(line) => <polyline points={onScreen(line())} />}</Index>
        <Index each={overlay().lines}>{(line) => <polyline class="scene-line" points={onScreen(line())} />}</Index>
        <Index each={overlay().handles.filter(({ arm }) => arm)}>
          {(handle) => <polyline class="scene-arm" points={onScreen([handle().arm!, handle().at])} />}
        </Index>
      </svg>
      {/* by position, as the machine's every reply makes new handles and a held one has to stay put */}
      <Index each={overlay().handles}>{(handle) => <Grip handle={handle()} space={props.space} />}</Index>
    </div>
  )
}

/** Whether an edit that's on its way to the machine has left a shape and its pose out of step */
function outOfStep(shape: Lens<Shape>, pose: RenderedShape) {
  const { base, transform } = shape.get()
  const handles = 'Bezier' in base ? base.Bezier.points.length : 0
  return (
    (transform ?? []).length !== pose.transforms.length ||
    shapePoints(base).length !== pose.points.length ||
    handles !== pose.handles.length
  )
}

export function Grip(props: { handle: Handle; space: PreviewSpace }) {
  const spot = () => props.space.project(props.handle.at)

  const press = (e: PointerEvent) => {
    if (e.button !== 0 || (props.handle.locked && !props.handle.click)) return
    e.preventDefault()
    const start = { x: e.clientX, y: e.clientY }
    let moved = false
    // the handle as it was pressed, so that each move works from the same place and not from a reply partway through
    const { handle, space } = props
    const pressed = space.unproject(e.clientX, e.clientY)
    if (!pressed) return
    const grabbed: Vec2 = [handle.at[0] - pressed[0], handle.at[1] - pressed[1]]

    let last: Vec2 | null = null
    const dragTo = (point: Vec2, shift: boolean) => {
      last = point
      handle.drag([point[0] + grabbed[0], point[1] + grabbed[1]], shift)
    }
    const move = (e: PointerEvent) => {
      moved ||= Math.hypot(e.clientX - start.x, e.clientY - start.y) >= CLICK_SLOP
      if (!moved || handle.locked) return
      const point = space.unproject(e.clientX, e.clientY)
      if (point) dragTo(point, e.shiftKey)
    }
    // pressing or letting go of shift drags again where the pointer is, so the change shows without moving it
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Shift' && last) dragTo(last, e.shiftKey)
    }
    // ahead of the preview, which would take the end of a drag over it for a click
    const release = (e: PointerEvent) => {
      e.stopPropagation()
      if (!moved) handle.click?.()
      document.body.style.cursor = ''
      window.removeEventListener('pointermove', move)
      window.removeEventListener('keydown', key)
      window.removeEventListener('keyup', key)
      window.removeEventListener('pointerup', release, { capture: true })
    }
    document.body.style.cursor = handle.cursor
    window.addEventListener('pointermove', move)
    window.addEventListener('keydown', key)
    window.addEventListener('keyup', key)
    window.addEventListener('pointerup', release, { capture: true })
  }

  return (
    <Show when={spot()}>
      {(at) => (
        <div
          class={`scene-handle scene-handle-${props.handle.kind}`}
          classList={{ 'scene-handle-locked': props.handle.locked, 'scene-handle-path': props.handle.onPath }}
          style={{ left: `${at()[0]}px`, top: `${at()[1]}px`, cursor: props.handle.locked ? 'not-allowed' : props.handle.cursor }}
          title={props.handle.locked ? `${props.handle.label} can't be dragged while it's random, on a path, or hidden` : props.handle.label}
          onPointerDown={press}
        >
          {props.handle.glyph}
        </div>
      )}
    </Show>
  )
}

/**
 * The outline of a layer the pointer is over, which says a click there selects it, in the color of what it's on. A
 * `selected` one is dashed, for a selected layer that has no handles to show it is.
 */
export function HoverOutline(props: { space: PreviewSpace; outlines: Vec2[][]; color: LayerColor; selected?: boolean }) {
  const onScreen = (loop: Vec2[]) => loop.flatMap((point) => props.space.project(point)?.join(',') ?? []).join(' ')
  return (
    <svg class={`scene-hover-outline ${layerColorClass(props.color) ?? ''}`} classList={{ 'scene-selected-outline': props.selected }} aria-hidden="true">
      <Index each={props.outlines}>{(loop) => <polygon points={onScreen(loop())} />}</Index>
    </svg>
  )
}

/**
 * The topmost layer with a region at a point on what's on show, which only some of a scene's layers may be on. Each
 * layer's outline is a loop for each copy of its shape. Where there's none, it's the topmost layer that covers all of
 * what it's on, such as a background image, if the point is on that.
 */
export function layerUnder(scene: Scene, outlines: Vec2[][][], point: Vec2, shown: (layer: Scene['layers'][number]) => boolean, surface: (layer: Scene['layers'][number]) => Surface) {
  const visible = (index: number) => !scene.layers[index].hidden && shown(scene.layers[index])
  for (let index = scene.layers.length - 1; index >= 0; index--) {
    if (scene.layers[index].region && visible(index) && (outlines[index] ?? []).some((loop) => encloses(loop, point))) return index
  }
  for (let index = scene.layers.length - 1; index >= 0; index--) {
    const outline = fullOutline(scene.layers[index], surface(scene.layers[index]))
    if (outline && visible(index) && encloses(outline, point)) return index
  }
  return null
}

/**
 * The edge of what a layer that covers all of what it's on is on, as it has no shape of its own to outline. `null`
 * for a layer with a region or a graph, and for one on a sequence, which has no area.
 */
export function fullOutline(layer: Scene['layers'][number], surface: Surface): Vec2[] | null {
  if (layer.region || layer.graph || !surface || surface[1] <= 0) return null
  const [width, height] = surface
  return [
    [0, 0],
    [width, 0],
    [width, height],
    [0, height],
  ]
}

function encloses(loop: Vec2[], [x, y]: Vec2) {
  let inside = false
  for (let i = 0, j = loop.length - 1; i < loop.length; j = i++) {
    const [[ax, ay], [bx, by]] = [loop[i], loop[j]]
    if (ay > y !== by > y && x < ax + ((bx - ax) * (y - ay)) / (by - ay)) inside = !inside
  }
  return inside
}

export type Write = {
  /** Whether a prop can take a value like this from a drag */
  can(prop: Prop<SceneValue>, like: SceneValue): boolean
  to<T extends SceneValue>(lens: Lens<Prop<T>>, value: T): void
}

/**
 * How a drag changes a prop: the value itself, a variable's default, or a track's key at the playhead, which is added
 * if there isn't one so that the rest of the track stays as it was. A random value or a sweep has nothing to change, and
 * nor has a hidden track, as what it shows isn't what's at the playhead.
 */
export function writer(scene: Lens<Scene>, time: number): Write {
  const sameMoment = (at: number) => Math.abs(at - time) < SAME_MOMENT
  return {
    can(prop, like) {
      if (isHidden(prop)) return false
      if ('Fixed' in prop) return true
      if ('Keyframes' in prop) return prop.Keyframes.keys.length > 0
      return 'Variable' in prop && variableReads(scene.get(), prop.Variable, like)
    },
    to(lens, value) {
      const prop = lens.get() as Prop<SceneValue>
      if ('Fixed' in prop) lens.set({ Fixed: value })
      else if ('Variable' in prop) scene.set(withVariable(scene.get(), prop.Variable, value))
      else if ('Keyframes' in prop) {
        const track = prop.Keyframes.keys.some((key) => sameMoment(key.at)) ? prop.Keyframes : withKey(prop.Keyframes, time)
        const keys = track.keys.map((key) => (sameMoment(key.at) ? { ...key, value } : key))
        ;(lens as Lens<Prop<SceneValue>>).set({ Keyframes: { ...track, keys } })
      }
    },
  }
}

function shapeHandles(shape: Lens<Shape>, pose: RenderedShape, write: Write, grid: number | null): Handle[] {
  const toScene = chain(pose.transforms)
  const fromScene = invert(toScene)
  if (!fromScene) return []
  const toLocal = (at: Vec2) => apply(fromScene, at)
  /** Where a point dragged to a place in the scene lands in the shape's own space, which is on the grid if there is one */
  const toPoint = (at: Vec2): Vec2 => {
    const [x, y] = toLocal(at)
    return grid ? [Math.round(x / grid) * grid, Math.round(y / grid) * grid] : [x, y]
  }
  const base = field(shape, 'base')
  const current = base.get()
  const list = orEmpty(field(shape, 'transform'))
  const [top, right] = [pose.min[1], pose.max[0]]

  /** A handle on one of the sizes of a shape, which grows from its center */
  const size = (label: string, prop: Lens<Prop<number>>, at: Vec2, grows: (local: Vec2) => number): Handle => ({
    label,
    kind: 'size',
    cursor: at[0] === 0 ? 'ns-resize' : 'ew-resize',
    at: apply(toScene, at),
    locked: !write.can(prop.get(), 0),
    drag: (to) => write.to(prop, rounded(grows(toLocal(to)))),
  })
  const across = (local: Vec2) => Math.abs(local[0])
  const down = (local: Vec2) => Math.abs(local[1])

  /**
   * A handle on each side of one of a shape's sizes, along x or y, where the size is `part` of the distance between
   * them. Dragging one moves that side and leaves the other where it is, moving the shape by half as much. With shift
   * held, or if the shape can't be moved, both sides move, mirrored about the center. With a `grid`, the side dragged
   * snaps to it, counted from the scene's origin, so a side lands on a pixel wherever the shape's center is.
   */
  const sides = (name: string, prop: Lens<Prop<number>>, axis: 0 | 1, part: number): Handle[] => {
    const shift = moveHandle(list, pose, write, null)
    const along = (length: number): Vec2 => (axis === 0 ? [length, 0] : [0, length])
    const origin = toLocal([0, 0])[axis]
    const snap = (at: number) => (grid ? origin + Math.round((at - origin) / grid) * grid : at)
    const edges = [pose.min[axis], pose.max[axis]]
    const ends = axis === 0 ? ['left', 'right'] : ['top', 'bottom']
    // once a drag has moved the shape, it's put back when shift is held, as a mirrored drag works from where it was
    let moved = false
    return edges.map((edge, i) => ({
      label: `${name} from the ${ends[i]}. Hold shift to size both sides.`,
      kind: 'size',
      cursor: axis === 0 ? 'ew-resize' : 'ns-resize',
      at: apply(toScene, along(edge)),
      locked: !write.can(prop.get(), 0),
      drag(to, mirrored) {
        const reached = snap(toLocal(to)[axis])
        const other = mirrored || shift.locked ? -reached : edges[1 - i]
        write.to(prop, rounded(Math.abs(reached - other) * part))
        const center = (reached + other) / 2
        if (center === 0 && !moved) return
        moved = true
        const [from, by] = [apply(toScene, [0, 0]), apply(toScene, along(center))]
        shift.drag([shift.at[0] + by[0] - from[0], shift.at[1] + by[1] - from[1]], false)
      },
    }))
  }

  const pointHandle = (index: number, prop: Lens<Prop<Vec2>>): Handle => ({
    label: `Point ${index + 1}`,
    kind: 'point',
    cursor: 'move',
    at: apply(toScene, pose.points[index]),
    locked: !write.can(prop.get(), [0, 0]),
    drag: (to) => write.to(prop, roundedPoint(toPoint(to))),
  })

  /** One of the two ends of a bezier's handle, either side of its point, which move as one. The end held is what snaps. */
  const curveHandle = (index: number, prop: Lens<Prop<Vec2>>, side: 1 | -1): Handle => {
    const [x, y] = pose.points[index]
    const [offsetX, offsetY] = pose.handles[index]
    return {
      label: `Point ${index + 1} handle`,
      kind: 'bezier',
      cursor: 'move',
      at: apply(toScene, [x + offsetX * side, y + offsetY * side]),
      arm: apply(toScene, [x, y]),
      locked: !write.can(prop.get(), [0, 0]),
      drag(to) {
        const [toX, toY] = toPoint(to)
        write.to(prop, roundedPoint([(toX - x) * side, (toY - y) * side]))
      },
    }
  }

  /** Width and height of a shape that's drawn in a box, from the corner at the right of its base and from its top */
  const boxed = (shaped: Lens<{ width: Prop<number>; height: Prop<number> }>, widest: number) => [
    size('Width', field(shaped, 'width'), [right, widest], (local) => across(local) * 2),
    size('Height', field(shaped, 'height'), [0, top], (local) => down(local) * 2),
  ]
  const bottom = pose.max[1]

  const handles = [
    moveHandle(list, pose, write, grid),
    ...revolveHandles(list, pose, write),
    ...shiftHandles(list, pose, write),
  ]
  if ('Circle' in current) {
    handles.push(size('Radius', field(variant(base, 'Circle'), 'radius'), [right, 0], across))
  } else if ('Ellipse' in current) {
    const ellipse = variant(base, 'Ellipse')
    handles.push(...sides('RX', field(ellipse, 'rx'), 0, 0.5), ...sides('RY', field(ellipse, 'ry'), 1, 0.5))
  } else if ('Rect' in current || 'Text' in current) {
    const rect = 'Rect' in current ? variant(base, 'Rect') : variant(base, 'Text')
    handles.push(...sides('Width', field(rect, 'width'), 0, 1), ...sides('Height', field(rect, 'height'), 1, 1))
  } else if ('Chevron' in current) {
    handles.push(...boxed(variant(base, 'Chevron'), bottom))
  } else if ('Diamond' in current) {
    // level with its side corner, which is the farthest right of its outline unless a transform reshapes it
    const widest = pose.outline.map(toLocal).reduce((far, point) => (point[0] > far[0] ? point : far), [-Infinity, 0])
    handles.push(...boxed(variant(base, 'Diamond'), widest[1]))
  } else if ('Star' in current) {
    // the first leg points up
    handles.push(size('Radius', field(variant(base, 'Star'), 'radius'), [0, top], down))
  } else if ('Polygon' in current || 'Polyline' in current) {
    const points = 'Polygon' in current ? field(variant(base, 'Polygon'), 'points') : field(variant(base, 'Polyline'), 'points')
    pose.points.forEach((_, index) => handles.push(pointHandle(index, item(points, index))))
  } else if ('Bezier' in current) {
    const points = field(variant(base, 'Bezier'), 'points')
    const point = (index: number) => item(points, index)
    pose.points.forEach((_, index) => handles.push(pointHandle(index, field(point(index), 'point'))))
    // over the points, so a handle pulled back onto its point can still be pulled off it. The ends only have the one side.
    pose.points.forEach((_, index) => {
      const prop = field(point(index), 'handle')
      if (index < pose.points.length - 1) handles.push(curveHandle(index, prop, 1))
      if (index > 0) handles.push(curveHandle(index, prop, -1))
    })
  }
  return handles
}

/**
 * The paths of a shape's transforms, and any on those in turn. `view` takes where the shape is to the scene being shown,
 * and each path is carried along from the values of its transform to where those put the shape's center.
 */
function pathOverlay(shape: Lens<Shape>, pose: RenderedShape, write: Write, view: Affine, grid: number | null): Overlay & { owners: number[] } {
  const list = orEmpty(field(shape, 'transform'))
  const parts = pose.paths.flatMap((pathPose, index) => {
    const transform = list.get()[index]
    if (!pathPose || !transform || !('Translate' in transform) || !('Sweep' in transform.Translate)) return []
    const offset = variant(item(list, index) as Lens<Extract<Transform, { Translate: unknown }>>, 'Translate')
    const path = field(variant(offset, 'Sweep'), 'path')
    if (outOfStep(path, pathPose)) return []

    const [x, y] = pose.pivots[index]
    const pathView = chain([[1, 0, 0, 1, x, y], ...movesAfter(list.get(), pose, index), view])
    const further = pathOverlay(path, pathPose, write, pathView, grid)
    const paths = [traced(path.get().base, pathPose).map((point) => apply(pathView, point)), ...further.paths]
    return [
      {
        handles: [...seenThrough(shapeHandles(path, pathPose, write, grid), pathView), ...further.handles].map((handle) => ({ ...handle, transform: index })),
        paths,
        grids: [...shapeGrid(pathPose, grid, pathView), ...further.grids],
        owners: paths.map(() => index),
      },
    ]
  })
  return {
    handles: parts.flatMap(({ handles }) => handles),
    paths: parts.flatMap(({ paths }) => paths),
    lines: [],
    boxes: [],
    grids: parts.flatMap(({ grids }) => grids),
    owners: parts.flatMap(({ owners }) => owners),
  }
}

/**
 * The grid a shape's points snap to, in its own space, over its bounds and a few cells past them, and taken through
 * `view` to the scene being shown. None for a shape with no points, or one too fine to draw.
 */
function shapeGrid(pose: RenderedShape, grid: number | null, view: Affine): Grid[] {
  if (!grid || pose.points.length === 0) return []
  const toScene = chain([...pose.transforms, view])
  const [fromX, fromY] = pose.min.map((value) => Math.floor(value / grid) - GRID_MARGIN)
  const [toX, toY] = pose.max.map((value) => Math.ceil(value / grid) + GRID_MARGIN)
  if (toX - fromX + (toY - fromY) + 2 > MAX_GRID_LINES) return []
  const steps = (from: number, to: number) => Array.from({ length: to - from + 1 }, (_, i) => (from + i) * grid)
  // a point at each crossing, so a line that runs out of sight still shows the part that's in it
  const [across, down] = [steps(fromX, toX), steps(fromY, toY)]
  const lines = [
    ...across.map((x) => down.map((y): Vec2 => [x, y])),
    ...down.map((y) => across.map((x): Vec2 => [x, y])),
  ].map((line) => line.map((point) => apply(toScene, point)))
  const cell: Grid['cell'] = [apply(toScene, [0, 0]), apply(toScene, [grid, 0]), apply(toScene, [0, grid])]
  return [{ lines, cell }]
}

/** The line that goes along a shape's edge, back to where it started if it's the kind that does */
function traced(base: Shape['base'], { outline }: RenderedShape) {
  return isOpen(base) ? outline : [...outline, ...outline.slice(0, 1)]
}

/** A path's handles as they are in the scene being shown */
function seenThrough(handles: Handle[], view: Affine): Handle[] {
  const back = invert(view)
  if (!back) return []
  return handles.map((handle) => ({
    ...handle,
    label: `Path ${handle.label.toLowerCase()}`,
    onPath: true,
    at: apply(view, handle.at),
    arm: handle.arm && apply(view, handle.arm),
    drag: (to, shift) => handle.drag(apply(back, to), shift),
  }))
}

/**
 * What moves a shape's own center after the transform at `index`. A turn or a scale goes around a point that moves with
 * the center, so it's no part of that, until a mirror or revolve has moved the center to the middle of what it draws,
 * which the turns and scales after it go around.
 */
function movesAfter(transforms: Transform[], pose: RenderedShape, index: number) {
  let copied = false
  return pose.transforms.slice(index + 1).filter((affine, i) => {
    const transform = transforms[index + 1 + i]
    copied ||= copies(transform)
    return copied || !('Scale' in transform || turnsInPlace(transform, affine))
  })
}

/** Whether a transform draws copies of the shape */
const copies = (transform: Transform) => 'Mirror' in transform || 'Revolve' in transform

/**
 * The `Translate` that moves the shape, or one added the first time this is dragged. That's its last one, or one added
 * after its other transforms, except with a mirror or revolve, where it's the last one before the first of those, or
 * one added just before it, so the shape moves on its own and its copies follow. It's held by the shape's center.
 * With a `grid`, the translate snaps so the top left of the shape's bounds is on it, which keeps a box's edges on pixels
 * even where its center is between two.
 */
function moveHandle(list: Lens<Transform[]>, pose: RenderedShape, write: Write, grid: number | null): Handle {
  const first = list.get().findIndex(copies)
  const limit = first < 0 ? pose.transforms.length : first
  const found = list.get().findLastIndex((transform, i) => i < limit && 'Translate' in transform)
  const index = found < 0 ? limit : found
  const undoAfter = invert(chain(movesAfter(list.get(), pose, index)))!
  // where the top left is before the translate moves it
  const corner = apply(chain(pose.transforms.slice(0, index)), pose.min)
  const prop = () => {
    if (found < 0 && list.get().length === pose.transforms.length) {
      list.set([...list.get().slice(0, limit), { Translate: { Fixed: [0, 0] } }, ...list.get().slice(limit)])
    }
    return variant(item(list, index) as Lens<Extract<Transform, { Translate: unknown }>>, 'Translate')
  }
  // on the shape itself while a mirror or revolve after the translate draws copies of it, as that's what it moves, and
  // otherwise on its center as the transforms leave it
  const at = first >= 0 ? apply(chain(pose.transforms), pose.pivots[0]) : pose.pivots[pose.transforms.length]

  return {
    label: 'Move',
    kind: 'move',
    cursor: 'move',
    at,
    transform: found < 0 ? undefined : index,
    locked: found >= 0 && !write.can(prop().get(), [0, 0]),
    drag(to) {
      const [x, y] = apply(undoAfter, to)
      const [fromX, fromY] = pose.pivots[index]
      const moved = [x - fromX, y - fromY].map((value, axis) =>
        grid ? Math.round((corner[axis] + value) / grid) * grid - corner[axis] : value,
      ) as Vec2
      write.to(prop(), roundedPoint(moved))
    },
  }
}

/**
 * For a shape with a mirror or revolve, a box around it and all its copies, with a handle on its top left corner that
 * moves the lot. That's the shape's last `Translate` after its last mirror or revolve, or one added after its other
 * transforms the first time this is dragged. With a `grid`, it moves in whole cells, so what's on the grid stays on it.
 */
function groupOverlay(list: Lens<Transform[]>, pose: RenderedShape, write: Write, outlines: Vec2[][], grid: number | null): { handle: Handle; box: Vec2[] } | null {
  const last = list.get().findLastIndex(copies)
  const points = outlines.flat()
  if (last < 0 || points.length === 0) return null
  const found = list.get().findLastIndex((transform, i) => i > last && 'Translate' in transform)
  const index = found < 0 ? pose.transforms.length : found
  // how a change to the translate moves what's drawn, which a turn around the shape's center turns along with it
  const [a, b, c, d] = chain(movesAfter(list.get(), pose, index))
  const undo = invert([a, b, c, d, 0, 0])
  if (!undo) return null
  const was: Vec2 = found < 0 ? [0, 0] : [pose.transforms[index][4], pose.transforms[index][5]]
  const prop = () => {
    if (found < 0 && list.get().length === pose.transforms.length) list.set([...list.get(), { Translate: { Fixed: [0, 0] } }])
    return variant(item(list, index) as Lens<Extract<Transform, { Translate: unknown }>>, 'Translate')
  }

  const [xs, ys] = [points.map(([x]) => x), points.map(([, y]) => y)]
  const [left, top, right, bottom] = [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)]
  const at: Vec2 = [left, top]
  const handle: Handle = {
    label: 'Move everything, with all its copies',
    kind: 'group',
    cursor: 'move',
    at,
    transform: found < 0 ? undefined : index,
    locked: found >= 0 && !write.can(prop().get(), [0, 0]),
    drag(to) {
      const by = apply(undo, [to[0] - at[0], to[1] - at[1]]).map((value) => (grid ? Math.round(value / grid) * grid : value))
      write.to(prop(), roundedPoint([was[0] + by[0], was[1] + by[1]]))
    },
  }
  return { handle, box: [at, [right, top], [right, bottom], [left, bottom], at] }
}

/**
 * The anchor of each rotate or scale, the point it turns around or keeps still. It's held as an offset from the shape's
 * center going into the transform, in the space the transforms before it put the shape in. Dragged near one of the
 * spots `anchorPresets` gives, it snaps onto it.
 */
function anchorHandles(list: Lens<Transform[]>, pose: RenderedShape, write: Write, open: number | null): Handle[] {
  return list.get().flatMap((transform, index) => {
    const anchor = pose.anchors[index]
    const kind = 'Rotate' in transform ? 'Rotate' : 'Scale' in transform ? 'Scale' : null
    if (index !== open || !kind || !anchor) return []
    const lens = item(list, index)
    const prop =
      kind === 'Rotate'
        ? field(variant(lens as Lens<Extract<Transform, { Rotate: unknown }>>, 'Rotate'), 'anchor')
        : field(variant(lens as Lens<Extract<Transform, { Scale: unknown }>>, 'Scale'), 'anchor')
    // the transform keeps its anchor where it is, so it's only the transforms after it that place it
    const toScene = chain(pose.transforms.slice(index + 1))
    const fromScene = invert(toScene)
    if (!fromScene) return []
    const [x, y] = pose.pivots[index]
    const spots = anchorPresets(pose, index) ?? []
    const handle: Handle = {
      label: kind === 'Rotate' ? 'Rotate anchor, the point the shape turns around. Drag to move it.' : 'Scale anchor, the point the scale keeps still. Drag to move it.',
      kind: 'anchor',
      cursor: 'move',
      transform: index,
      at: apply(toScene, anchor),
      locked: !write.can(prop.get(), [0, 0]),
      drag: (to) => {
        const [ax, ay] = apply(fromScene, to)
        write.to(prop, snapToSpot(roundedPoint([ax - x, ay - y]), spots, pose))
      },
    }
    return [handle]
  })
}

/** How near a dragged anchor has to come to a spot to snap onto it, as a share of the shape's box's longer side */
const SPOT_SNAP = 0.08

/** `anchor`, or the spot it's within `SPOT_SNAP` of */
function snapToSpot(anchor: Vec2, spots: AnchorPreset[], pose: RenderedShape): Vec2 {
  const reach = SPOT_SNAP * Math.max(pose.max[0] - pose.min[0], pose.max[1] - pose.min[1])
  const distance = (spot: AnchorPreset) => Math.hypot(spot.anchor[0] - anchor[0], spot.anchor[1] - anchor[1])
  const nearest = spots.reduce<AnchorPreset | null>((best, spot) => (best && distance(best) <= distance(spot) ? best : spot), null)
  return nearest && distance(nearest) <= reach ? nearest.anchor : anchor
}

/** Where a rotate's or scale's anchor can be put with a button, as offsets from the shape's center */
export type AnchorPreset = { label: string; anchor: Vec2 }

/**
 * The corners, the middles of the sides, and the center of the shape's box going into the rotate or scale at `index`,
 * as anchors for it, row by row from the top left: from the shape's center, in the space the transforms before it put
 * the shape in. Up is toward smaller y. The box is around the shape itself, so it's left out of what a mirror or revolve
 * before the transform draws.
 */
export function anchorPresets(pose: RenderedShape, index: number): AnchorPreset[] | null {
  const pivot = pose.pivots[index]
  if (!pivot) return null
  const before = chain(pose.transforms.slice(0, index))
  const [[minX, minY], [maxX, maxY]] = [pose.min, pose.max]
  const corners = [[minX, minY], [maxX, minY], [maxX, maxY], [minX, maxY]].map((corner) => apply(before, corner as Vec2))
  const [xs, ys] = [corners.map(([x]) => x), corners.map(([, y]) => y)]
  const [left, right, top, bottom] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)]
  const across = [left, (left + right) / 2, right]
  const down = [top, (top + bottom) / 2, bottom]
  return ANCHOR_SPOTS.map((label, spot) => {
    const [x, y] = [across[spot % 3], down[Math.floor(spot / 3)]]
    // the center's the shape's own, which a box turned by the transforms before isn't always around
    return { label, anchor: label === 'Center' ? [0, 0] : roundedPoint([x - pivot[0], y - pivot[1]]) }
  })
}

/** The spots an anchor can be put on, row by row from the top left */
export const ANCHOR_SPOTS = ['Top left', 'Top', 'Top right', 'Left', 'Center', 'Right', 'Bottom left', 'Bottom', 'Bottom right']

/** The center of each revolve, which is in the space the transforms before it put the shape in */
function revolveHandles(list: Lens<Transform[]>, pose: RenderedShape, write: Write): Handle[] {
  return list.get().flatMap((transform, index) => {
    const center = pose.centers[index]
    if (!('Revolve' in transform) || !center) return []
    const prop = field(variant(item(list, index) as Lens<Extract<Transform, { Revolve: unknown }>>, 'Revolve'), 'center')
    // the revolve itself leaves the shape where it is, so it's only the transforms after it that place its center
    const toScene = chain(pose.transforms.slice(index))
    const fromScene = invert(toScene)
    if (!fromScene) return []
    const handle: Handle = {
      label: 'Revolve center',
      kind: 'center',
      transform: index,
      cursor: 'move',
      at: apply(toScene, center),
      locked: !write.can(prop.get(), [0, 0]),
      drag: (to) => write.to(prop, roundedPoint(apply(fromScene, to))),
    }
    return [handle]
  })
}

/**
 * The line of each mirror and bisect, across the shape and its reflection and a little past them, with a handle on its
 * center that moves it and one at its far end that turns it around the center. The center and line are in the space
 * the transforms before it put the shape in.
 */
function lineOverlay(shape: Lens<Shape>, pose: RenderedShape, write: Write): { handles: Handle[]; lines: Vec2[][]; owners: number[] } {
  const list = orEmpty(field(shape, 'transform'))
  const parts = list.get().flatMap((transform, index) => {
    const [center, along] = [pose.centers[index], pose.lines[index]]
    if (!('Mirror' in transform || 'Bisect' in transform) || !center || !along) return []
    const kind = 'Mirror' in transform ? 'Mirror' : 'Bisect'
    const line = variant(item(list, index) as Lens<Record<typeof kind, Extract<Transform, { Mirror: unknown }>['Mirror']>>, kind)
    const [centerProp, rotation] = [field(line, 'center'), field(line, 'rotation')]
    // the transform itself leaves the shape where it is, so it's only those after it that place its line
    const toScene = chain(pose.transforms.slice(index))
    const fromScene = invert(toScene)
    if (!fromScene) return []

    // how far the shape reaches along the line either way from its center, which its reflection does too
    const reach = pose.outline.map((point) => {
      const [x, y] = apply(fromScene, point)
      return (x - center[0]) * along[0] + (y - center[1]) * along[1]
    })
    const [back, forward] = [Math.min(0, ...reach), Math.max(0, ...reach)]
    const margin = (forward - back) * LINE_MARGIN
    const at = (t: number) => apply(toScene, [center[0] + along[0] * t, center[1] + along[1] * t])
    const end = at(forward + margin)
    const name = kind.toLowerCase()
    // the rotation the line has now, which a turn stays within half a turn of
    const now = (Math.atan2(along[1], along[0]) * 180) / Math.PI - 90

    const handles: Handle[] = [
      {
        label: `${kind} line. Drag to move it.`,
        kind: 'center',
        transform: index,
        cursor: 'move',
        at: apply(toScene, center),
        locked: !write.can(centerProp.get(), [0, 0]),
        drag: (to) => write.to(centerProp, roundedPoint(apply(fromScene, to))),
      },
      {
        label: `Turn the ${name} line. Hold shift to turn in steps of ${TURN_STEP}°.`,
        kind: 'turn',
        transform: index,
        cursor: 'grab',
        at: end,
        locked: !write.can(rotation.get(), 0),
        drag(to, stepped) {
          const [x, y] = apply(fromScene, to)
          const pointed = (Math.atan2(y - center[1], x - center[0]) * 180) / Math.PI - 90
          const turned = now + ((((pointed - now) % 360) + 540) % 360) - 180
          write.to(rotation, rounded(stepped ? Math.round(turned / TURN_STEP) * TURN_STEP : turned))
        },
      },
    ]
    return [{ handles, line: [at(back - margin), end], index }]
  })
  return { handles: parts.flatMap(({ handles }) => handles), lines: parts.map(({ line }) => line), owners: parts.map(({ index }) => index) }
}

/**
 * The middle of each shift, where it's moved to, with a line back to the shape's center. It drags along the shift's
 * line, as far as the end of the shape either way.
 */
function shiftHandles(list: Lens<Transform[]>, pose: RenderedShape, write: Write): Handle[] {
  return list.get().flatMap((transform, index) => {
    const line = pose.shifts[index]
    if (!('Shift' in transform) || !line) return []
    const prop = field(variant(item(list, index) as Lens<Extract<Transform, { Shift: unknown }>>, 'Shift'), 'amount')
    // the shift itself leaves the shape where it is, so it's only those after it that place its line
    const toScene = chain(pose.transforms.slice(index))
    const fromScene = invert(toScene)
    if (!fromScene) return []
    const [x, y] = pose.pivots[index]
    const fromCenter = ([toX, toY]: Vec2): Vec2 => [toX - x, toY - y]
    const [back, forward] = [fromCenter(line.back), fromCenter(line.forward)]
    const handle: Handle = {
      label: 'Shift',
      kind: 'shift',
      transform: index,
      cursor: 'move',
      at: apply(toScene, line.middle),
      arm: apply(toScene, [x, y]),
      locked: !write.can(prop.get(), 0),
      drag(to) {
        const at = fromCenter(apply(fromScene, to))
        // how much of the way it is to an end, by how far it is along the line toward that end
        const share = ([endX, endY]: Vec2) => {
          const length = endX ** 2 + endY ** 2
          return length > 0 ? (at[0] * endX + at[1] * endY) / length : 0
        }
        const amount = share(forward) > 0 ? share(forward) : -share(back)
        write.to(prop, rounded(Math.max(-1, Math.min(1, amount))))
      },
    }
    return [handle]
  })
}

/**
 * The handle on the top right corner of a shape's box, which a click switches between turning the shape, scaling it,
 * and rounding its corners. Dragging it to turn or scale changes the shape's first `Rotate` or `Scale`, or adds one
 * the first time, ahead of any mirror or revolve so the shape changes on its own and its copies follow. Rounding has a
 * handle of its own, in from the corner by the radius, on the shape's first `Round`, or one added ahead of its other
 * transforms so the radius is in the shape's own space.
 */
function cornerHandles(shape: Lens<Shape>, pose: RenderedShape, write: Write, chosen: CornerMode, next: () => void, scene: Scene, open: number | null): Handle[] {
  const list = orEmpty(field(shape, 'transform'))
  // an open rotate, scale, or round has the corner handle for itself, which stays at what it does
  const openKind = open === null ? undefined : (['Rotate', 'Scale', 'Round'] as const).find((kind) => kind in (list.get()[open] ?? {}))
  const mode: CornerMode = openKind ? (openKind.toLowerCase() as CornerMode) : chosen
  const toScene = chain(pose.transforms)
  const fromScene = invert(toScene)
  if (!fromScene) return []
  const cornerLocal: Vec2 = [pose.max[0], pose.min[1]]
  const corner = apply(toScene, cornerLocal)
  const first = list.get().findIndex(copies)
  const limit = first < 0 ? pose.transforms.length : first
  const center = first >= 0 ? apply(toScene, pose.pivots[0]) : pose.pivots[pose.transforms.length]
  const nextName = CORNER_MODES[(CORNER_MODES.indexOf(mode) + 1) % CORNER_MODES.length]

  /** The first transform of a kind, or one put in at `at` the first time it's dragged, and the index it's at */
  const found = (kind: 'Rotate' | 'Scale' | 'Round', at: number, made: Transform) => {
    const index = openKind === kind ? open! : list.get().findIndex((transform) => kind in transform)
    const lens = () => {
      if (index < 0 && list.get().length === pose.transforms.length) list.set([...list.get().slice(0, at), made, ...list.get().slice(at)])
      return item(list, index < 0 ? at : index)
    }
    return { index, lens }
  }

  /** The index of the transform the handle changes, which is the open one while there is one */
  let changes: number | undefined
  const handle = (label: string, cursor: string, prop: (() => Prop<SceneValue>) | null, drag: Handle['drag']): Handle => ({
    label: openKind ? label : `${label} Click to ${nextName === 'round' ? 'round its corners' : nextName} instead.`,
    kind: 'corner',
    cursor,
    at: corner,
    locked: prop !== null && !write.can(prop(), 0),
    drag,
    click: openKind ? undefined : next,
    glyph: CORNER_GLYPHS[mode],
    transform: changes,
  })

  if (mode === 'rotate') {
    const { index, lens } = found('Rotate', limit, { Rotate: { degrees: { Fixed: 0 }, anchor: { Fixed: [0, 0] } } })
    changes = index < 0 ? undefined : index
    const prop = () => field(variant(lens() as Lens<Extract<Transform, { Rotate: unknown }>>, 'Rotate'), 'degrees')
    // the turn it has now, as the machine drew it, which a drag adds to
    const [a, b] = index < 0 ? [1, 0] : pose.transforms[index]
    const was = (Math.atan2(b, a) * 180) / Math.PI
    // it turns around its anchor, which stays where it is, so only the transforms after it place it
    const anchor = index < 0 ? undefined : pose.anchors[index]
    const around = anchor ? apply(chain(pose.transforms.slice(index + 1)), anchor) : center
    const angleAround = ([x, y]: Vec2) => (Math.atan2(y - around[1], x - around[0]) * 180) / Math.PI
    const from = angleAround(corner)
    return [
      handle('Turn the shape. Hold shift to turn in steps of 15°.', 'grab', index < 0 ? null : () => prop().get(), (to, stepped) => {
        const turned = was + angleAround(to) - from
        write.to(prop(), rounded(stepped ? Math.round(turned / TURN_STEP) * TURN_STEP : turned))
      }),
    ]
  }

  if (mode === 'scale') {
    const { index, lens } = found('Scale', limit, { Scale: { amount: { Fixed: 1 }, anchor: { Fixed: [0, 0] } } })
    changes = index < 0 ? undefined : index
    const amount = () => field(variant(lens() as Lens<Extract<Transform, { Scale: unknown }>>, 'Scale'), 'amount')
    const [a, b] = index < 0 ? [1, 0] : pose.transforms[index]
    const was = Math.hypot(a, b)
    const reach = Math.hypot(corner[0] - center[0], corner[1] - center[1])
    return [
      handle('Scale the shape.', 'nesw-resize', index < 0 ? null : () => amount().get(), (to) => {
        if (reach === 0) return
        const scaled = (was * Math.hypot(to[0] - center[0], to[1] - center[1])) / reach
        write.to(amount(), rounded(Math.max(scaled, 0.01)))
      }),
    ]
  }

  const { index, lens } = found('Round', 0, { Round: { Fixed: 0 } })
  changes = index < 0 ? undefined : index
  const radiusProp = () => variant(lens() as Lens<Extract<Transform, { Round: unknown }>>, 'Round')
  const radius = index < 0 ? 0 : (startingValue(radiusProp().get(), scene, 0) as number)
  // the radius is in the space the transforms before the round put the shape in
  const before = index < 0 ? IDENTITY : chain(pose.transforms.slice(0, index))
  const after = index < 0 ? toScene : chain(pose.transforms.slice(index))
  const back = invert(after)
  if (!back) return [handle('Round the shape’s corners with the handle in from this one.', 'pointer', null, () => {})]
  const [cornerX, cornerY] = apply(before, cornerLocal)
  const largest = Math.min(pose.max[0] - pose.min[0], pose.max[1] - pose.min[1]) / 2
  return [
    handle('Round the shape’s corners with the handle in from this one.', 'pointer', null, () => {}),
    {
      label: 'Corner radius. Drag in from the corner to round the corners more.',
      kind: 'radius',
      transform: changes,
      cursor: 'nesw-resize',
      at: apply(after, [cornerX - radius, cornerY + radius]),
      locked: index >= 0 && !write.can(radiusProp().get(), 0),
      drag(to) {
        const [x, y] = apply(back, to)
        const inward = (cornerX - x + (y - cornerY)) / 2
        write.to(radiusProp(), rounded(Math.max(0, Math.min(largest, inward))))
      },
    },
  ]
}

/** Whether a transform is a `Rotate` that turns the shape, and not one that runs it along a `Sequence` */
function turnsInPlace(transform: Transform, [a, b]: Affine) {
  return 'Rotate' in transform && (a !== 1 || b !== 0)
}

const rounded = (value: number) => Math.round(value * PRECISION) / PRECISION

const roundedPoint = ([x, y]: Vec2): Vec2 => [rounded(x), rounded(y)]

function apply([a, b, c, d, e, f]: Affine, [x, y]: Vec2): Vec2 {
  return [a * x + c * y + e, b * x + d * y + f]
}

/** Each transform in turn */
function chain(transforms: Affine[]) {
  return transforms.reduce((done, [a, b, c, d, e, f]): Affine => {
    const [x, y] = apply([a, b, c, d, e, f], [done[4], done[5]])
    const axis = (i: number): Vec2 => [a * done[i] + c * done[i + 1], b * done[i] + d * done[i + 1]]
    return [...axis(0), ...axis(2), x, y]
  }, IDENTITY)
}

function invert([a, b, c, d, e, f]: Affine): Affine | null {
  const determinant = a * d - b * c
  if (Math.abs(determinant) < Number.EPSILON) return null
  const [ia, ib, ic, id] = [d / determinant, -b / determinant, -c / determinant, a / determinant]
  return [ia, ib, ic, id, -(ia * e + ic * f), -(ib * e + id * f)]
}
