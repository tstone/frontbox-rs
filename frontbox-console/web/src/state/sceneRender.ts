import { createSignal, untrack } from 'solid-js'
import type { ClipSound } from '../types/generated/ClipSound'
import type { RenderedClip } from '../types/generated/RenderedClip'
import type { RenderedScene } from '../types/generated/RenderedScene'
import type { SceneSize } from '../types/generated/SceneSize'
import type { SceneSource } from '../types/generated/SceneSource'
import { sendToMachine } from './console'

type SceneRenderReply = { rendered: RenderedScene } | { error: string }

export type SceneRender = SceneRenderReply & {
  /** What was asked for */
  source: SceneSource
  /** Whether this answers the latest request, rather than one a newer edit has replaced */
  settled: boolean
}

/** How long to wait for a reply before asking again. A reply can be lost to a reconnect. */
const REPLY_TIMEOUT_MS = 1000

/** The same for a clip, which has a whole cycle to render */
const CLIP_TIMEOUT_MS = 30000

type Target = { source: SceneSource; size: SceneSize }

/** A cycle of a scene, rendered ahead so it can be played without asking the machine for each frame */
export type SceneClip = {
  frames: number
  width: number
  height: number
  /** Where each LED's colors are in a frame */
  ledIndex: Map<string, number>
  /** Each frame's points, as a canvas takes them */
  points: Uint8ClampedArray<ArrayBuffer>
  /** Each frame's LEDs, as red, green, and blue */
  leds: Uint8Array
  /** The clips the cycle starts, in the order they start */
  sounds: ClipSound[]
}

const [render, setRender] = createSignal<SceneRender | null>(null)

/** The machine's last reply; `null` until the first one */
export const sceneRender = render

let inFlight: { timeout: ReturnType<typeof setTimeout> } | null = null
let waiting: (Target & { time: number; cycle: number | null }) | null = null
// what the next reply answers, which outlives `inFlight` as a slow render is still worth having. `null` once forgotten.
let asked: SceneSource | null = null

/**
 * Asks the machine to render a scene. One request runs at a time and only the newest waits, so dragging a slider can't
 * queue up renders. Waits a tick so that the edits of one change go together.
 */
export function renderScene(source: SceneSource, size: SceneSize, time: number, cycle: number | null) {
  waiting = { source, size, time, cycle }
  queueMicrotask(sendWaiting)
}

function sendWaiting() {
  if (inFlight || !waiting) return
  const { source, size, time, cycle } = waiting
  waiting = null
  sendToMachine({ type: 'RenderScene', source, size, time, cycle })
  asked = source
  inFlight = { timeout: setTimeout(replied, REPLY_TIMEOUT_MS) }
}

function replied() {
  if (inFlight) clearTimeout(inFlight.timeout)
  inFlight = null
  sendWaiting()
}

/** Called by the console for the machine's reply */
export function receiveSceneRender(reply: SceneRenderReply) {
  if (asked) setRender({ ...reply, source: asked, settled: waiting === null })
  replied()
}

/** Lets go of everything to do with the scene that's been rendering, including replies still on their way */
export function forgetSceneRender() {
  asked = null
  waiting = null
  setRender(null)
  clipInFlight = null
  clipsWanted = null
  clipsRendered = null
  setClips(new Map())
  setClipRendering(false)
  setClipError(null)
}

const [clips, setClips] = createSignal<ReadonlyMap<number, SceneClip>>(new Map())
const [clipRendering, setClipRendering] = createSignal(false)
const [clipError, setClipError] = createSignal<string | null>(null)

/** The clip of a cycle, if it's rendered. It may be of an earlier edit of the scene while `sceneClipRendering` is set. */
export const sceneClip = (cycle: number) => clips().get(cycle) ?? null
/** Whether the first of the cycles asked for is still to come, as the ones after it are only rendered ahead */
export const sceneClipRendering = clipRendering
export const sceneClipError = clipError

let nextClipId = 0
let clipInFlight: { id: number; target: Target; cycle: number; timeout: ReturnType<typeof setTimeout> } | null = null
let clipsWanted: (Target & { cycles: number[] }) | null = null
// what `clips` are of
let clipsRendered: Target | null = null

const sameTarget = (a: Target, b: Target | null) =>
  b !== null &&
  a.source === b.source &&
  a.size.leds === b.size.leds &&
  a.size.width === b.size.width &&
  a.size.height === b.size.height

/**
 * Has the machine render these cycles of a scene, in order, and lets go of any others. Runs as `renderScene` does, one
 * at a time with only the newest scene waiting, and does nothing for a clip that's already rendered.
 */
export function renderClips(source: SceneSource, size: SceneSize, cycles: number[]) {
  clipsWanted = { source, size, cycles }
  // called from effects, which shouldn't run again for what this does to the clips
  untrack(() => {
    const kept = [...clips()].filter(([cycle]) => cycles.includes(cycle))
    if (sameTarget(clipsWanted!, clipsRendered) && kept.length < clips().size) setClips(new Map(kept))
    sendMissingClip()
  })
}

function sendMissingClip() {
  const wanted = clipsWanted
  const rendered = wanted !== null && sameTarget(wanted, clipsRendered)
  const cycle = wanted?.cycles.find((cycle) => !rendered || !clips().has(cycle))
  setClipRendering(cycle !== undefined && cycle === wanted?.cycles[0])
  if (clipInFlight || !wanted || cycle === undefined) return

  const { source, size } = wanted
  const target = { source, size }
  const id = nextClipId++
  sendToMachine({ type: 'RenderClip', id, ...target, cycle })
  clipInFlight = { id, target, cycle, timeout: setTimeout(() => clipReplied(id), CLIP_TIMEOUT_MS) }
}

function clipReplied(id: number) {
  if (clipInFlight?.id !== id) return
  clearTimeout(clipInFlight.timeout)
  clipInFlight = null
  sendMissingClip()
}

/** Called by the console for each clip: a binary message without its first byte */
export function receiveSceneClip(message: ArrayBuffer) {
  const headerLength = new DataView(message).getUint32(0, true)
  const header: RenderedClip = JSON.parse(new TextDecoder().decode(new Uint8Array(message, 4, headerLength)))
  if (clipInFlight?.id !== header.id) return
  const { target, cycle } = clipInFlight

  const { frames, width, height, leds, sounds } = header
  const pointsStart = 4 + headerLength
  const ledsStart = pointsStart + frames * width * height * 4
  const clip = {
    frames,
    width,
    height,
    ledIndex: new Map(leds.map((name, index) => [name, index])),
    points: new Uint8ClampedArray(message, pointsStart, ledsStart - pointsStart),
    leds: new Uint8Array(message, ledsStart),
    sounds,
  }
  setClips((rendered) => new Map(sameTarget(target, clipsRendered) ? [...rendered, [cycle, clip]] : [[cycle, clip]]))
  clipsRendered = target
  setClipError(null)
  clipReplied(header.id)
}

export function receiveSceneClipFailed(id: number, error: string) {
  if (clipInFlight?.id !== id) return
  // not asked for again, as it would only fail again
  if (clipsWanted && sameTarget(clipsWanted, clipInFlight.target)) {
    clipsWanted = null
    setClipError(error)
  }
  clipReplied(id)
}
