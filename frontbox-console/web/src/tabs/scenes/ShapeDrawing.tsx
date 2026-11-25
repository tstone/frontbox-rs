import { createSignal, For, onCleanup, onMount, Show } from 'solid-js'
import { typingIn } from '../../lib/keys'
import type { PreviewSpace } from '../machine/PlayfieldView'
import { layerColorClass, type LayerColor } from './ScenePortal'
import type { Vec2 } from './sceneModel'

/** A box drawn over the preview, by its middle and size in the units of what it's on. `size` is `null` for a click. */
export type DrawnBox = { center: Vec2; size: Vec2 | null }

/**
 * Covers the preview while a shape is being drawn, taking presses there in place of the camera and the handles. A drag
 * draws a box from where it starts, held to `aspect` (width over height) when there is one or when shift is down, at
 * 1:1. `onDrawn` gets the box on release, and Escape calls `onCancel`. The box is in `color`.
 */
export default function ShapeDrawing(props: { space: PreviewSpace; aspect: number | null; color: LayerColor; onDrawn: (box: DrawnBox) => void; onCancel: () => void }) {
  // where the drag started, on screen and on what's being drawn on
  let start: { client: Vec2; point: Vec2 } | null = null
  const [corners, setCorners] = createSignal<[Vec2, Vec2] | null>(null)

  onMount(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      e.preventDefault()
      props.onCancel()
    }
    window.addEventListener('keydown', onKeyDown)
    onCleanup(() => window.removeEventListener('keydown', onKeyDown))
  })

  /** The far corner of the box out to `point`, held to its aspect */
  const farCorner = (from: Vec2, point: Vec2, shift: boolean): Vec2 => {
    const aspect = props.aspect ?? (shift ? 1 : null)
    let [width, height] = [Math.abs(point[0] - from[0]), Math.abs(point[1] - from[1])]
    if (aspect !== null) {
      // the larger of the two leads
      if (width / aspect > height) height = width / aspect
      else width = height * aspect
    }
    return [from[0] + Math.sign(point[0] - from[0] || 1) * width, from[1] + Math.sign(point[1] - from[1] || 1) * height]
  }

  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return
    const point = props.space.unproject(e.clientX, e.clientY)
    if (!point) return
    e.preventDefault()
    e.currentTarget instanceof Element && e.currentTarget.setPointerCapture(e.pointerId)
    start = { client: [e.clientX, e.clientY], point }
    setCorners([point, point])
  }

  const onPointerMove = (e: PointerEvent) => {
    const point = start && props.space.unproject(e.clientX, e.clientY)
    if (start && point) setCorners([start.point, farCorner(start.point, point, e.shiftKey)])
  }

  const onPointerUp = (e: PointerEvent) => {
    if (!start) return
    const from = start
    start = null
    setCorners(null)
    const point = props.space.unproject(e.clientX, e.clientY)
    if (Math.hypot(e.clientX - from.client[0], e.clientY - from.client[1]) < CLICK_SLOP || !point) {
      props.onDrawn({ center: from.point, size: null })
      return
    }
    const far = farCorner(from.point, point, e.shiftKey)
    props.onDrawn({
      center: [(from.point[0] + far[0]) / 2, (from.point[1] + far[1]) / 2],
      size: [Math.abs(far[0] - from.point[0]), Math.abs(far[1] - from.point[1])],
    })
  }

  /** The box's outline on screen, which on the machine is the box seen at an angle */
  const outline = () => {
    const box = corners()
    if (!box) return null
    const [[x0, y0], [x1, y1]] = box
    const onScreen = ([[x0, y0], [x1, y0], [x1, y1], [x0, y1]] as Vec2[]).map((corner) => props.space.project(corner))
    return onScreen.every((at) => at !== null) ? onScreen.map((at) => at!.join(',')).join(' ') : null
  }

  return (
    <div class={`scene-drawing ${layerColorClass(props.color) ?? ''}`} onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={onPointerUp} onPointerCancel={() => (start = null, setCorners(null))}>
      <Show when={outline()}>
        {(points) => (
          <svg aria-hidden="true">
            <polygon points={points()} />
          </svg>
        )}
      </Show>
    </div>
  )
}

/** The points of a polygon or line drawn over the preview, in order, in the units of what it's on */
export type DrawnPoints = { points: Vec2[] }

/**
 * Covers the preview while a polygon or a line is being drawn, a point at a time. Each click puts down a point, with a
 * dashed line from the last one out to the pointer. A polygon is finished by clicking its first point again, once there
 * are three, which closes it, and a line that's `open` by clicking its last point again (so a double-click ends it), once
 * there are two. Enter finishes either. `onDrawn` gets the points. Backspace takes back the last point. Escape or a
 * right-click calls `onCancel`, which leaves nothing drawn.
 */
export function PointsDrawing(props: { space: PreviewSpace; open: boolean; color: LayerColor; onDrawn: (drawn: DrawnPoints) => void; onCancel: () => void }) {
  const [points, setPoints] = createSignal<Vec2[]>([])
  // where the pointer is on what's being drawn on, while it's over it
  const [pointer, setPointer] = createSignal<Vec2 | null>(null)

  onMount(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        props.onCancel()
      } else if (e.key === 'Enter' && !typingIn(e.target) && enough()) {
        e.preventDefault()
        props.onDrawn({ points: points() })
      } else if (e.key === 'Backspace' && !typingIn(e.target)) {
        e.preventDefault()
        setPoints((list) => list.slice(0, -1))
      }
    }
    window.addEventListener('keydown', onKeyDown)
    onCleanup(() => window.removeEventListener('keydown', onKeyDown))
  })

  const onScreen = () => points().flatMap((point) => {
    const at = props.space.project(point)
    return at ? [at] : []
  })
  const enough = () => points().length >= (props.open ? 2 : 3)
  /** The point on screen a click on finishes it: a polygon's first, which closes it, or a line's last */
  const end = () => (props.open ? onScreen().at(-1) : onScreen()[0])
  /** Whether a point on screen is on the end, where a click finishes it */
  const closesAt = (at: Vec2 | null) => {
    const to = end()
    return enough() && at !== null && to !== undefined && Math.hypot(at[0] - to[0], at[1] - to[1]) <= CLOSE_RADIUS
  }
  const pointerOnScreen = () => {
    const at = pointer()
    return at && props.space.project(at)
  }
  const closing = () => closesAt(pointerOnScreen())

  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return
    e.preventDefault()
    const point = props.space.unproject(e.clientX, e.clientY)
    if (!point) return
    const at = props.space.project(point)
    if (closesAt(at)) {
      props.onDrawn({ points: points() })
      return
    }
    // a double-click on a polygon puts down one corner, not two in the same place
    const last = onScreen().at(-1)
    if (last && at && Math.hypot(at[0] - last[0], at[1] - last[1]) < CLICK_SLOP) return
    setPoints((list) => [...list, point])
  }

  /** The dashed line from the last point out to the pointer, which reaches a polygon's first corner exactly where a click would close it */
  const nextLine = () => {
    const last = onScreen().at(-1)
    if (closing() && props.open) return null
    const to = closing() ? onScreen()[0] : pointerOnScreen()
    return last && to ? { from: last, to } : null
  }

  const isEnd = (index: number) => index === (props.open ? points().length - 1 : 0)

  return (
    <div
      class={`scene-drawing ${layerColorClass(props.color) ?? ''}`}
      classList={{ 'scene-drawing-closing': closing() }}
      onPointerDown={onPointerDown}
      onPointerMove={(e) => setPointer(props.space.unproject(e.clientX, e.clientY))}
      onPointerLeave={() => setPointer(null)}
      onContextMenu={(e) => {
        e.preventDefault()
        props.onCancel()
      }}
    >
      <svg aria-hidden="true">
        <polyline class="scene-drawing-placed" points={onScreen().map((at) => at.join(',')).join(' ')} />
        <Show when={nextLine()}>{(line) => <line class="scene-drawing-next" x1={line().from[0]} y1={line().from[1]} x2={line().to[0]} y2={line().to[1]} />}</Show>
        <For each={onScreen()}>{(at, index) => <circle classList={{ 'scene-drawing-end': isEnd(index()) }} cx={at[0]} cy={at[1]} r={isEnd(index()) && closing() ? 6 : 3.5} />}</For>
      </svg>
    </div>
  )
}

/** How far the pointer can move between press and release and still count as a click, in pixels */
const CLICK_SLOP = 4

/** How near its end a click finishes a polygon or line, in pixels */
const CLOSE_RADIUS = 8
