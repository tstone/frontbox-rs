import { createSignal } from 'solid-js'
import type { SceneSource } from '../types/generated/SceneSource'

const [names, setNames] = createSignal<string[]>([])
const [listFailed, setListFailed] = createSignal(false)

/** Every scene in the machine's content folder, by its path in the folder */
export const contentScenes = names
export const contentScenesFailed = listFailed

/** A scene's name is its path in the content folder, and the `.scene` needn't be typed */
export function sceneName(typed: string): string {
  const name = typed.trim()
  return name.toLowerCase().endsWith('.scene') ? name : `${name}.scene`
}

function sceneUrl(name: string): string {
  return `/content/scenes/${name.split('/').map(encodeURIComponent).join('/')}`
}

/** Scans the folder, which can change on the machine at any time. */
export async function refreshContentScenes() {
  try {
    const response = await fetch('/content/scenes')
    if (!response.ok) throw new Error(response.statusText)
    setNames(await response.json())
    setListFailed(false)
  } catch {
    setListFailed(true)
  }
}

/** The text of a response, which is what to show if it's a failure */
async function replyText(response: Response, failure: string): Promise<string> {
  const text = await response.text()
  if (!response.ok) throw new Error(text || failure)
  return text
}

/** A scene's file. Throws what to show if it can't be read. */
export async function readContentScene(name: string): Promise<string> {
  return replyText(await fetch(sceneUrl(name)), "The scene couldn't be read")
}

async function writeScene(method: string, name: string, source: SceneSource): Promise<string> {
  const headers = { 'Content-Type': 'application/json' }
  const response = await fetch(sceneUrl(name), { method, headers, body: JSON.stringify(source) })
  return replyText(response, "The scene couldn't be saved")
}

/** Adds a scene to the folder, giving back its file. Throws what to show if it can't, such as the name being taken. */
export function addContentScene(name: string, source: SceneSource): Promise<string> {
  return writeScene('POST', name, source)
}

/** Writes a scene over its file, giving back what was written. Throws what to show if it can't. */
export function saveContentScene(name: string, source: SceneSource): Promise<string> {
  return writeScene('PUT', name, source)
}

/** Removes a scene from the folder. Throws what to show if it can't. */
export async function deleteContentScene(name: string): Promise<void> {
  await replyText(await fetch(sceneUrl(name), { method: 'DELETE' }), "The scene couldn't be deleted")
}
