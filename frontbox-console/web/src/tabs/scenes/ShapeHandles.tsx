import { createMemo, Index, Show, useContext } from 'solid-js'
import type { Layer } from '../../types/generated/Layer'
import type { Prop } from '../../types/generated/Prop'
import type { RenderedShape } from '../../types/generated/RenderedShape'
import type { Scene } from '../../types/generated/Scene'
import type { Transform } from '../../types/generated/Transform'
import type { PreviewSpace } from '../machine/PlayfieldView'
import { SceneContext } from './SceneFields'
import { field, item, orEmpty, paramFits, variant, withKey, withParam, type Lens, type SceneValue, type Vec2 } from './sceneModel'

type Shape = NonNullable<Layer['shape']>

/** A transform as the machine sends it: where it takes x's axis, y's axis, and the origin */
type Affine = RenderedShape['transforms'][number]

/** A grab handle: where it is, and what dragging it does */
type Handle = {
  label: string
  kind: 'move' | 'scale' | 'size' | 'point'
  cursor: string
  /** Where it sits in the scene */
  at: Vec2
  /** Set when it's held by something a drag can't change, such as a random value */
  locked: boolean
  /** Moves what it holds so the handle lands on a point in the scene. `proportional` keeps a scale's shape. */
  drag(to: Vec2, proportional: boolean): void
}

/** How the values a handle writes are rounded, so a drag doesn't leave long decimals in a scene */
const PRECISION = 1000

/** The closest a key has to be to the playhead to count as the key at that moment, in seconds */
const SAME_MOMENT = 0.001

/** The smallest scale a handle leaves, as a shape scaled to nothing has no handles to bring it back with */
const MIN_SCALE = 0.001

const IDENTITY: Affine = [1, 0, 0, 1, 0, 0]

/** Grab handles over the selected layer's shape, placed by the machine's account of where it is */
export default function ShapeHandles(props: { space: PreviewSpace; layer: number | null; pose: RenderedShape | null; time: number }) {
  const scene = useContext(SceneContext)!

  const handles = createMemo(() => {
    const { layer, pose } = props
    if (layer === null || !pose || !scene.get()?.layers[layer]?.shape) return []
    const shape = field(item(field(scene, 'layers'), layer), 'shape') as Lens<Shape>
    // an edit that's on its way to the machine can leave these out of step
    if ((shape.get().transform ?? []).length !== pose.transforms.length) return []
    return shapeHandles(shape, pose, writer(scene, props.time))
  })

  return (
    <div class="scene-handles">
      {/* by position, as the machine's every reply makes new handles and a held one has to stay put */}
      <Index each={handles()}>{(handle) => <Grip handle={handle()} space={props.space} />}</Index>
    </div>
  )
}

function Grip(props: { handle: Handle; space: PreviewSpace }) {
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
          classList={{ 'scene-handle-locked': props.handle.locked }}
          style={{ left: `${at()[0]}px`, top: `${at()[1]}px`, cursor: props.handle.locked ? 'not-allowed' : props.handle.cursor }}
          title={props.handle.locked ? `${props.handle.label} can't be dragged while it's random` : props.handle.label}
          onPointerDown={press}
        />
      )}
    </Show>
  )
}

/** The topmost layer with a shape at a point in the scene */
export function layerUnder(scene: Scene, outlines: Vec2[][], point: Vec2) {
  for (let index = scene.layers.length - 1; index >= 0; index--) {
    if (scene.layers[index].shape && encloses(outlines[index] ?? [], point)) return index
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
 * if there isn't one so that the rest of the track stays as it was. A random value has nothing to change.
 */
function writer(scene: Lens<Scene>, time: number): Write {
  const sameMoment = (at: number) => Math.abs(at - time) < SAME_MOMENT
  return {
    can(prop, like) {
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

function shapeHandles(shape: Lens<Shape>, pose: RenderedShape, write: Write): Handle[] {
  const toScene = chain(pose.transforms)
  const fromScene = invert(toScene)
  if (!fromScene) return []
  const toLocal = (at: Vec2) => apply(fromScene, at)
  const base = field(shape, 'base')
  const current = base.get()
  const list = orEmpty(field(shape, 'transform'))
  const [[left, top], [right, bottom]] = [pose.min, pose.max]

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

  const center: Vec2 = [(left + right) / 2, (top + bottom) / 2]
  const handles = [moveHandle(list, pose, write, center), scaleHandle(list, pose, write, [right, top])]
  if ('Ellipse' in current) {
    const { r1, r2 } = { r1: field(variant(base, 'Ellipse'), 'r1'), r2: field(variant(base, 'Ellipse'), 'r2') }
    handles.push(size('R1', r1, [0, top], down), size('R2', r2, [right, 0], across))
  } else if ('Rect' in current) {
    const rect = variant(base, 'Rect')
    handles.push(
      size('Width', field(rect, 'width'), [right, 0], (local) => across(local) * 2),
      size('Height', field(rect, 'height'), [0, top], (local) => down(local) * 2),
    )
  } else {
    const points = field(variant(base, 'Polygon'), 'points')
    pose.points.forEach((point, index) => {
      const prop = item(points, index)
      handles.push({
        label: `Point ${index + 1}`,
        kind: 'point',
        cursor: 'move',
        at: apply(toScene, point),
        locked: !write.can(prop.get(), [0, 0]),
        drag: (to) => write.to(prop, roundedPoint(toLocal(to))),
      })
    })
  }
  return handles
}

/** The shape's last `Translate`, or one added after its other transforms the first time this is dragged */
function moveHandle(list: Lens<Transform[]>, pose: RenderedShape, write: Write, center: Vec2): Handle {
  const found = list.get().findLastIndex((transform) => 'Translate' in transform)
  const index = found < 0 ? pose.transforms.length : found
  const before = chain(pose.transforms.slice(0, index))
  const undoAfter = invert(chain(pose.transforms.slice(index + 1)))!
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
