import { createMemo, Index, Show, useContext } from 'solid-js'
import type { Layer } from '../../types/generated/Layer'
import type { Prop } from '../../types/generated/Prop'
import type { RenderedShape } from '../../types/generated/RenderedShape'
import type { Transform } from '../../types/generated/Transform'
import type { PreviewSpace } from '../machine/PlayfieldView'
import { SceneContext } from './SceneFields'
import { COPY_KINDS, field, isHidden, isOpen, item, orEmpty, paramFits, selectedShape, selectedShapeLens, shapePoints, variant, withKey, withParam, type CopyKind, type Lens, type Scene, type SceneValue, type Selection, type Vec2 } from './sceneModel'

type Shape = NonNullable<Layer['region']>

/** A transform as the machine sends it: where it takes x's axis, y's axis, and the origin */
type Affine = RenderedShape['transforms'][number]

/** A grab handle: where it is, and what dragging it does */
export type Handle = {
  label: string
  kind: 'move' | 'scale' | 'size' | 'point' | 'bezier' | 'center' | 'shift'
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
}

/** The lines a shape's points snap to, in the scene, and a corner of one of its cells to tell how far apart they're drawn */
type Grid = { lines: Vec2[][]; cell: [Vec2, Vec2, Vec2] }

/** What's drawn over a shape: its handles, the paths it goes along as lines in the scene, and the grids its points snap to */
type Overlay = { handles: Handle[]; paths: Vec2[][]; grids: Grid[] }

/** How the values a handle writes are rounded, so a drag doesn't leave long decimals in a scene */
const PRECISION = 1000

/** The closest a key has to be to the playhead to count as the key at that moment, in seconds */
const SAME_MOMENT = 0.001

/** The smallest scale a handle leaves, as a shape scaled to nothing has no handles to bring it back with */
const MIN_SCALE = 0.001

/** How many cells a grid goes past a shape's bounds on each side, so there's somewhere to drag its points out to */
const GRID_MARGIN = 4

/** The most lines a grid is drawn with. One finer than this still snaps, but is left undrawn. */
const MAX_GRID_LINES = 400

/** The fewest pixels apart a grid's lines are drawn, as any closer they'd hide the scene under them */
const MIN_GRID_SPACING = 6

const IDENTITY: Affine = [1, 0, 0, 1, 0, 0]

const NOTHING: Overlay = { handles: [], paths: [], grids: [] }

/**
 * Grab handles over the selected shape and the paths it follows, placed by the machine's account of where they are.
 * With a `grid`, a shape's points and how far it's moved snap to a grid of that size in its own space, which is drawn
 * over a shape made of points.
 */
export default function ShapeHandles(props: { space: PreviewSpace; selection: Selection | null; pose: RenderedShape | null; time: number; grid: number | null }) {
  const scene = useContext(SceneContext)!

  const overlay = createMemo((): Overlay => {
    const { selection, pose, grid } = props
    if (selection === null || !pose || !selectedShape(scene.get(), selection)) return NOTHING
    const shape = selectedShapeLens(scene, selection)
    if (outOfStep(shape, pose)) return NOTHING
    const write = writer(scene, props.time)
    const { handles, paths, grids } = pathOverlay(shape, pose, write, IDENTITY, grid)
    // a path's on top, as its shape is held to it and has nothing there to grab
    return { handles: [...shapeHandles(shape, pose, write, grid), ...handles], paths, grids: [...shapeGrid(pose, grid, IDENTITY), ...grids] }
  })

  const onScreen = (line: Vec2[]) => line.flatMap((point) => props.space.project(point) ?? []).join(' ')

  /** Whether a grid's lines are far enough apart on screen to draw */
  const spread = ({ cell: [corner, ...sides] }: Grid) => {
    const at = props.space.project(corner)
    return at !== null && sides.every((side) => {
      const to = props.space.project(side)
      return to !== null && Math.hypot(to[0] - at[0], to[1] - at[1]) >= MIN_GRID_SPACING
    })
  }

  return (
    <div class="scene-handles">
      <svg class="scene-paths" aria-hidden="true">
        <Index each={overlay().grids.filter(spread)}>
          {(grid) => <Index each={grid().lines}>{(line) => <polyline class="scene-grid" points={onScreen(line())} />}</Index>}
        </Index>
        <Index each={overlay().paths}>{(line) => <polyline points={onScreen(line())} />}</Index>
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
    if (e.button !== 0 || props.handle.locked) return
    e.preventDefault()
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
        />
      )}
    </Show>
  )
}

/**
 * The topmost layer with a region at a point on what's on show, which only some of a scene's layers may be on. Each
 * layer's outline is a loop for each copy of its shape.
 */
export function layerUnder(scene: Scene, outlines: Vec2[][][], point: Vec2, shown: (layer: Scene['layers'][number]) => boolean) {
  for (let index = scene.layers.length - 1; index >= 0; index--) {
    const { region, hidden } = scene.layers[index]
    if (region && !hidden && shown(scene.layers[index]) && (outlines[index] ?? []).some((loop) => encloses(loop, point))) return index
  }
  return null
}

function encloses(loop: Vec2[], [x, y]: Vec2) {
  let inside = false
  for (let i = 0, j = loop.length - 1; i < loop.length; j = i++) {
    const [[ax, ay], [bx, by]] = [loop[i], loop[j]]
    if (ay > y !== by > y && x < ax + ((bx - ax) * (y - ay)) / (by - ay)) inside = !inside
  }
  return inside
}

type Write = {
  /** Whether a prop can take a value like this from a drag */
  can(prop: Prop<SceneValue>, like: SceneValue): boolean
  to<T extends SceneValue>(lens: Lens<Prop<T>>, value: T): void
}

/**
 * How a drag changes a prop: the value itself, a parameter's default, or a track's key at the playhead, which is added
 * if there isn't one so that the rest of the track stays as it was. A random value or a sweep has nothing to change, and
 * nor has a hidden track, as what it shows isn't what's at the playhead.
 */
function writer(scene: Lens<Scene>, time: number): Write {
  const sameMoment = (at: number) => Math.abs(at - time) < SAME_MOMENT
  return {
    can(prop, like) {
      if (isHidden(prop)) return false
      if ('Fixed' in prop) return true
      if ('Keyframes' in prop) return prop.Keyframes.keys.length > 0
      return 'Param' in prop && paramFits(scene.get().params?.[prop.Param], like)
    },
    to(lens, value) {
      const prop = lens.get() as Prop<SceneValue>
      if ('Fixed' in prop) lens.set({ Fixed: value })
      else if ('Param' in prop) scene.set(withParam(scene.get(), prop.Param, value))
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
    scaleHandle(list, pose, write, [right, top]),
    ...centerHandles(list, pose, write),
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
    // level with its side corner, which is the farthest right of its outline unless its sides bow
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
function pathOverlay(shape: Lens<Shape>, pose: RenderedShape, write: Write, view: Affine, grid: number | null): Overlay {
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
    return [
      {
        handles: [...seenThrough(shapeHandles(path, pathPose, write, grid), pathView), ...further.handles],
        paths: [traced(path.get().base, pathPose).map((point) => apply(pathView, point)), ...further.paths],
        grids: [...shapeGrid(pathPose, grid, pathView), ...further.grids],
      },
    ]
  })
  return {
    handles: parts.flatMap(({ handles }) => handles),
    paths: parts.flatMap(({ paths }) => paths),
    grids: parts.flatMap(({ grids }) => grids),
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

/** What moves a shape's center after the transform at `index`. A turn stays on the center, so it's no part of that. */
function movesAfter(transforms: Transform[], pose: RenderedShape, index: number) {
  return pose.transforms.slice(index + 1).filter((affine, i) => !turnsInPlace(transforms[index + 1 + i], affine))
}

/**
 * The shape's last `Translate`, or one added after its other transforms the first time this is dragged. It's held by
 * the shape's center, which a mirror or revolve moves to the middle of what it draws. With a `grid`, the translate
 * snaps so the top left of the shape's bounds is on it, which keeps a box's edges on pixels even where its center is
 * between two.
 */
function moveHandle(list: Lens<Transform[]>, pose: RenderedShape, write: Write, grid: number | null): Handle {
  const found = list.get().findLastIndex((transform) => 'Translate' in transform)
  const index = found < 0 ? pose.transforms.length : found
  const undoAfter = invert(chain(movesAfter(list.get(), pose, index)))!
  // where the top left is before the translate moves it
  const corner = apply(chain(pose.transforms.slice(0, index)), pose.min)
  const prop = () => {
    if (found < 0 && list.get().length === pose.transforms.length) list.set([...list.get(), { Translate: { Fixed: [0, 0] } }])
    return variant(item(list, index) as Lens<Extract<Transform, { Translate: unknown }>>, 'Translate')
  }

  return {
    label: 'Move',
    kind: 'move',
    cursor: 'move',
    at: pose.pivots[pose.transforms.length],
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

/** The center of each mirror, revolve, and divide, which is in the space the transforms before it put the shape in */
function centerHandles(list: Lens<Transform[]>, pose: RenderedShape, write: Write): Handle[] {
  return list.get().flatMap((transform, index) => {
    const kind = COPY_KINDS.find((kind) => kind in transform)
    const center = pose.centers[index]
    if (!kind || !center) return []
    const copy = variant(item(list, index) as unknown as Lens<Record<CopyKind, { center: Prop<Vec2> }>>, kind)
    const prop = field(copy, 'center')
    // the transform itself leaves the shape where it is, so it's only those after it that place its center
    const toScene = chain(pose.transforms.slice(index))
    const fromScene = invert(toScene)
    if (!fromScene) return []
    const handle: Handle = {
      label: `${kind} center`,
      kind: 'center',
      cursor: 'move',
      at: apply(toScene, center),
      locked: !write.can(prop.get(), [0, 0]),
      drag: (to) => write.to(prop, roundedPoint(apply(fromScene, to))),
    }
    return [handle]
  })
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

/** The shape's last `Scale`, or one added before its other transforms the first time this is dragged */
function scaleHandle(list: Lens<Transform[]>, pose: RenderedShape, write: Write, corner: Vec2): Handle {
  const found = list.get().findLastIndex((transform) => 'Scale' in transform)
  const index = Math.max(found, 0)
  const before = chain(pose.transforms.slice(0, index))
  const undoAfter = invert(chain(pose.transforms.slice(found < 0 ? 0 : index + 1)))!
  const prop = () => {
    if (found < 0 && list.get().length === pose.transforms.length) list.set([{ Scale: { Fixed: 1 } }, ...list.get()])
    return variant(item(list, index) as Lens<Extract<Transform, { Scale: unknown }>>, 'Scale')
  }

  return {
    label: 'Scale',
    kind: 'scale',
    cursor: 'nesw-resize',
    at: apply(chain(pose.transforms), corner),
    locked: found >= 0 && !write.can(prop().get(), 0),
    drag(to) {
      const target = apply(undoAfter, to)
      const reached = apply(before, corner)
      // the scale that puts the corner nearest the pointer, as both axes scale together
      const length = reached[0] ** 2 + reached[1] ** 2
      if (length === 0) return
      const scale = (target[0] * reached[0] + target[1] * reached[1]) / length
      write.to(prop(), rounded(Math.max(scale, MIN_SCALE)))
    },
  }
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
