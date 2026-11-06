import { createSignal } from 'solid-js'
import type { RenderedScene } from '../types/generated/RenderedScene'
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

const [render, setRender] = createSignal<SceneRender | null>(null)

/** The machine's last reply; `null` until the first one */
export const sceneRender = render

let inFlight: { source: SceneSource; timeout: ReturnType<typeof setTimeout> } | null = null
let waiting: { source: SceneSource; size: SceneSize } | null = null

/**
 * Asks the machine to render a scene. One request runs at a time and only the newest waits, so dragging a slider can't
 * queue up renders. Waits a tick so that the edits of one change go together.
 */
export function renderScene(source: SceneSource, size: SceneSize) {
  waiting = { source, size }
  queueMicrotask(sendWaiting)
}

function sendWaiting() {
  if (inFlight || !waiting) return
  const { source, size } = waiting
  waiting = null
  sendToMachine({ type: 'RenderScene', source, size })
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
