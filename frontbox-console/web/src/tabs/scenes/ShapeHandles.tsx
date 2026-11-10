import { createMemo, Index, Show, useContext } from 'solid-js'
import type { Layer } from '../../types/generated/Layer'
import type { Prop } from '../../types/generated/Prop'
import type { RenderedShape } from '../../types/generated/RenderedShape'
import type { Transform } from '../../types/generated/Transform'
import type { PreviewSpace } from '../machine/PlayfieldView'
import { SceneContext } from './SceneFields'
import { field, isHidden, isOpen, item, orEmpty, paramFits, selectedShape, selectedShapeLens, shapePoints, variant, withKey, withParam, type Lens, type Scene, type SceneValue, type Selection, type Vec2 } from './sceneModel'

type Shape = NonNullable<Layer['region']>

/** A transform as the machine sends it: where it takes x's axis, y's axis, and the origin */
type Affine = RenderedShape['transforms'][number]

/** A grab handle: where it is, and what dragging it does */
export type Handle = {
  label: string
  kind: 'move' | 'scale' | 'size' | 'point' | 'bezier'
  cursor: string
  /** Where it sits in the scene */
  at: Vec2
  /** Set when it's held by something a drag can't change, such as a random value, a sweep, or a hidden track */
  locked: boolean
  /** Moves what it holds so the handle lands on a point in the scene. `proportional` keeps a scale's shape. */
  drag(to: Vec2, proportional: boolean): void
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

/** How many points a ring's hole is drawn with */
const HOLE_POINTS = 64

const IDENTITY: Affine = [1, 0, 0, 1, 0, 0]

const NOTHING: Overlay = { handles: [], paths: [], grids: [] }

/**
 * Grab handles over the selected shape and the paths it follows, placed by the machine's account of where they are.
 * With a `grid`, a shape's points snap to a grid of that size in its own space, which is drawn over it.
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

    const move = (e: PointerEvent) => {
      const point = space.unproject(e.clientX, e.clientY)
      if (point) handle.drag([point[0] + grabbed[0], point[1] + grabbed[1]], e.shiftKey)
    }
    // ahead of the preview, which would take the end of a drag over it for a click
    const release = (e: PointerEvent) => {
      e.stopPropagation()
      document.body.style.cursor = ''
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', release, { capture: true })
    }
    document.body.style.cursor = handle.cursor
    window.addEventListener('pointermove', move)
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

/** The topmost layer with a region at a point on what's on show, which only some of a scene's layers may be on */
export function layerUnder(scene: Scene, outlines: Vec2[][], point: Vec2, shown: (layer: Scene['layers'][number]) => boolean) {
  for (let index = scene.layers.length - 1; index >= 0; index--) {
    const { region, hidden } = scene.layers[index]
    if (region && !hidden && shown(scene.layers[index]) && encloses(outlines[index] ?? [], point)) return index
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
      if ('Track' in prop) return prop.Track.keys.length > 0
      return 'Param' in prop && paramFits(scene.get().params?.[prop.Param], like)
    },
    to(lens, value) {
      const prop = lens.get() as Prop<SceneValue>
      if ('Fixed' in prop) lens.set({ Fixed: value })
      else if ('Param' in prop) scene.set(withParam(scene.get(), prop.Param, value))
      else if ('Track' in prop) {
        const track = prop.Track.keys.some((key) => sameMoment(key.at)) ? prop.Track : withKey(prop.Track, time)
        const keys = track.keys.map((key) => (sameMoment(key.at) ? { ...key, value } : key))
        ;(lens as Lens<Prop<SceneValue>>).set({ Track: { ...track, keys } })
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

  const center = middle(pose)
  const handles = [moveHandle(list, pose, write, center), scaleHandle(list, pose, write, [right, top])]
  if ('Circle' in current) {
    handles.push(size('Radius', field(variant(base, 'Circle'), 'radius'), [right, 0], across))
  } else if ('Ring' in current) {
    const ring = variant(base, 'Ring')
    handles.push(
      size('Inner radius', field(ring, 'inner_radius'), [pose.hole, 0], across),
      size('Thickness', field(ring, 'thickness'), [right, 0], (local) => Math.max(across(local) - pose.hole, 0)),
    )
  } else if ('Ellipse' in current) {
    const { rx, ry } = { rx: field(variant(base, 'Ellipse'), 'rx'), ry: field(variant(base, 'Ellipse'), 'ry') }
    handles.push(size('RX', rx, [right, 0], across), size('RY', ry, [0, top], down))
  } else if ('Rect' in current) {
    const rect = variant(base, 'Rect')
    handles.push(
      size('Width', field(rect, 'width'), [right, 0], (local) => across(local) * 2),
      size('Height', field(rect, 'height'), [0, top], (local) => down(local) * 2),
    )
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

    const [x, y] = apply(chain(pose.transforms.slice(0, index)), middle(pose))
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

/** A ring's hole as a closed loop in the scene, or none for a shape without one */
export function holeOutline(pose: RenderedShape): Vec2[] {
  if (pose.hole <= 0) return []
  const toScene = chain(pose.transforms)
  return Array.from({ length: HOLE_POINTS }, (_, index) => {
    const angle = (index / HOLE_POINTS) * 2 * Math.PI
    return apply(toScene, [Math.cos(angle) * pose.hole, Math.sin(angle) * pose.hole])
  })
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
    drag: (to, proportional) => handle.drag(apply(back, to), proportional),
  }))
}

/** The middle of a shape's bounds, in its own space */
function middle({ min, max }: RenderedShape): Vec2 {
  return [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2]
}

/** What moves a shape's center after the transform at `index`. A turn stays on the center, so it's no part of that. */
function movesAfter(transforms: Transform[], pose: RenderedShape, index: number) {
  return pose.transforms.slice(index + 1).filter((affine, i) => !turnsInPlace(transforms[index + 1 + i], affine))
}

/** The shape's last `Translate`, or one added after its other transforms the first time this is dragged */
function moveHandle(list: Lens<Transform[]>, pose: RenderedShape, write: Write, center: Vec2): Handle {
  const found = list.get().findLastIndex((transform) => 'Translate' in transform)
  const index = found < 0 ? pose.transforms.length : found
  const before = chain(pose.transforms.slice(0, index))
  const undoAfter = invert(chain(movesAfter(list.get(), pose, index)))!
  const prop = () => {
    if (found < 0 && list.get().length === pose.transforms.length) list.set([...list.get(), { Translate: { Fixed: [0, 0] } }])
    return variant(item(list, index) as Lens<Extract<Transform, { Translate: unknown }>>, 'Translate')
  }

  return {
    label: 'Move',
    kind: 'move',
    cursor: 'move',
    at: apply(chain(pose.transforms), center),
    locked: found >= 0 && !write.can(prop().get(), [0, 0]),
    drag(to) {
      const [x, y] = apply(undoAfter, to)
      const [fromX, fromY] = apply(before, center)
      write.to(prop(), roundedPoint([x - fromX, y - fromY]))
    },
  }
}

/** The shape's last `Scale`, or one added before its other transforms the first time this is dragged */
function scaleHandle(list: Lens<Transform[]>, pose: RenderedShape, write: Write, corner: Vec2): Handle {
  const found = list.get().findLastIndex((transform) => 'Scale' in transform)
  const index = Math.max(found, 0)
  const before = chain(pose.transforms.slice(0, index))
  const undoAfter = invert(chain(pose.transforms.slice(found < 0 ? 0 : index + 1)))!
  const start: Vec2 = found < 0 ? [1, 1] : [pose.transforms[index][0], pose.transforms[index][3]]
  const prop = () => {
    if (found < 0 && list.get().length === pose.transforms.length) list.set([{ Scale: { Fixed: [1, 1] } }, ...list.get()])
    return variant(item(list, index) as Lens<Extract<Transform, { Scale: unknown }>>, 'Scale')
  }

  return {
    label: 'Scale',
    kind: 'scale',
    cursor: 'nesw-resize',
    at: apply(chain(pose.transforms), corner),
    locked: found >= 0 && !write.can(prop().get(), [0, 0]),
    drag(to, proportional) {
      const target = apply(undoAfter, to)
      const reached = apply(before, corner)
      // how much each axis has to grow by, which stays put on an axis the corner sits on the center of
      const grow = [0, 1].map((axis) => (Math.abs(reached[axis]) > 0 ? target[axis] / (reached[axis] * start[axis]) : 1))
      const by = proportional ? (grow[0] + grow[1]) / 2 : null
      const scale = start.map((value, axis) => value * Math.max(by ?? grow[axis], MIN_SCALE / Math.abs(value || 1))) as Vec2
      write.to(prop(), roundedPoint(scale))
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
