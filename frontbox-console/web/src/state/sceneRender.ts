import { createSignal } from 'solid-js'
import type { RenderedScene } from '../types/generated/RenderedScene'
import type { SceneSize } from '../types/generated/SceneSize'
import { sendToMachine } from './console'

export type SceneRender = { rendered: RenderedScene } | { error: string }

const [render, setRender] = createSignal<SceneRender | null>(null)

/** The machine's last reply; `null` until the first one */
export const sceneRender = render

/** Asks the machine to render a scene from its RON source */
export function renderScene(source: string, size: SceneSize) {
  sendToMachine({ type: 'RenderScene', source, size })
}

/** Called by the console for the machine's reply */
export function receiveSceneRender(reply: SceneRender) {
  setRender(reply)
}
