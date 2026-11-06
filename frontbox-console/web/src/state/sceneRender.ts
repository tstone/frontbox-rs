import { createSignal } from 'solid-js'
import type { RenderedScene } from '../types/generated/RenderedScene'
import type { SceneTarget } from '../types/generated/SceneTarget'
import { sendToMachine } from './console'

export type SceneRender = { rendered: RenderedScene } | { error: string }

const [render, setRender] = createSignal<SceneRender | null>(null)

/** The machine's last reply; `null` until the first one */
export const sceneRender = render

/** Asks the machine to render a scene from its RON source */
export function renderScene(source: string, target: SceneTarget) {
  sendToMachine({ type: 'RenderScene', source, target })
}

/** Called by the console for the machine's reply */
export function receiveSceneRender(reply: SceneRender) {
  setRender(reply)
}
