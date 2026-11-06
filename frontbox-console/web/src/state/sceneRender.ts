import { createSignal } from 'solid-js'
import type { RenderedClip } from '../types/generated/RenderedClip'
import type { RenderedScene } from '../types/generated/RenderedScene'
import type { SceneLeds } from '../types/generated/SceneLeds'
import type { SceneSize } from '../types/generated/SceneSize'
import type { SceneSource } from '../types/generated/SceneSource'
import { sendToMachine } from './console'

type SceneRenderReply = { rendered: RenderedScene } | { error: string }

export type SceneRender = SceneRenderReply & {
  /** What was asked for, if the reply could be matched to a request */
  source?: SceneSource
  /** Whether this answers the latest request, rather than one a newer edit has replaced */
  settled: boolean
}

/** How long to wait for a reply before asking again. A reply can be lost to a reconnect. */
const REPLY_TIMEOUT_MS = 1000

/** The same for a clip, which has a whole cycle to render */
const CLIP_TIMEOUT_MS = 30000

type Target = { source: SceneSource; size: SceneSize; leds: SceneLeds }

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
}

const [render, setRender] = createSignal<SceneRender | null>(null)

/** The machine's last reply; `null` until the first one */
export const sceneRender = render

let inFlight: { source: SceneSource; timeout: ReturnType<typeof setTimeout> } | null = null
let waiting: (Target & { time: number }) | null = null

/**
 * Asks the machine to render a scene. One request runs at a time and only the newest waits, so dragging a slider can't
 * queue up renders. Waits a tick so that the edits of one change go together.
 */
export function renderScene(source: SceneSource, size: SceneSize, leds: SceneLeds, time: number) {
  waiting = { source, size, leds, time }
  queueMicrotask(sendWaiting)
}

function sendWaiting() {
  if (inFlight || !waiting) return
  const { source, size, leds, time } = waiting
  waiting = null
  sendToMachine({ type: 'RenderScene', source, size, leds, time })
  inFlight = { source, timeout: setTimeout(replied, REPLY_TIMEOUT_MS) }
}

function replied() {
  if (inFlight) clearTimeout(inFlight.timeout)
  inFlight = null
  sendWaiting()
}

/** Called by the console for the machine's reply */
export function receiveSceneRender(reply: SceneRenderReply) {
  setRender({ ...reply, source: inFlight?.source, settled: inFlight !== null && waiting === null })
  replied()
}

const [clip, setClip] = createSignal<SceneClip | null>(null)
const [clipRendering, setClipRendering] = createSignal(false)
const [clipError, setClipError] = createSignal<string | null>(null)

/** The last clip that rendered. It may be of an earlier edit of the scene while `sceneClipRendering` is set. */
export const sceneClip = clip
export const sceneClipRendering = clipRendering
export const sceneClipError = clipError

let nextClipId = 0
let clipInFlight: { id: number; target: Target; timeout: ReturnType<typeof setTimeout> } | null = null
let clipWaiting: Target | null = null
let clipRendered: Target | null = null

const sameTarget = (a: Target, b: Target | null) =>
  b !== null &&
  a.source === b.source &&
  a.size.leds === b.size.leds &&
  a.size.width === b.size.width &&
  a.size.height === b.size.height &&
  a.leds.plane === b.leds.plane

/**
 * Asks the machine for a clip of a scene. Runs as `renderScene` does, and does nothing if that's the clip already
 * rendered or on its way.
 */
export function renderClip(source: SceneSource, size: SceneSize, leds: SceneLeds) {
  const target = { source, size, leds }
  if (sameTarget(target, clipWaiting ?? clipInFlight?.target ?? clipRendered)) return
  clipWaiting = target
  setClipRendering(true)
  sendClipWaiting()
}

function sendClipWaiting() {
  if (clipInFlight || !clipWaiting) return
  const target = clipWaiting
  clipWaiting = null
  const id = nextClipId++
  sendToMachine({ type: 'RenderClip', id, ...target })
  clipInFlight = { id, target, timeout: setTimeout(() => clipReplied(id), CLIP_TIMEOUT_MS) }
}

function clipReplied(id: number) {
  if (clipInFlight?.id !== id) return false
  clearTimeout(clipInFlight.timeout)
  clipInFlight = null
  setClipRendering(clipWaiting !== null)
  sendClipWaiting()
  return true
}

/** Called by the console for each binary message, which is always a clip */
export function receiveSceneClip(message: ArrayBuffer) {
  const headerLength = new DataView(message).getUint32(0, true)
  const header: RenderedClip = JSON.parse(new TextDecoder().decode(new Uint8Array(message, 4, headerLength)))
  const target = clipInFlight?.target ?? null
  if (!clipReplied(header.id)) return

  const { frames, width, height, leds } = header
  const pointsStart = 4 + headerLength
  const ledsStart = pointsStart + frames * width * height * 4
  clipRendered = target
  setClipError(null)
  setClip({
    frames,
    width,
    height,
    ledIndex: new Map(leds.map((name, index) => [name, index])),
    points: new Uint8ClampedArray(message, pointsStart, ledsStart - pointsStart),
    leds: new Uint8Array(message, ledsStart),
  })
}

export function receiveSceneClipFailed(id: number, error: string) {
  if (clipReplied(id) && clipWaiting === null) setClipError(error)
}
