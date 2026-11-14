import { createSignal } from 'solid-js'
import { createStore } from 'solid-js/store'
import { sendToMachine } from './console'

const [names, setNames] = createSignal<string[]>([])
const [listFailed, setListFailed] = createSignal(false)
/** Seconds by sound name, or `null` for one that couldn't be read */
const [lengths, setLengths] = createStore<Record<string, number | null>>({})
const asked = new Set<string>()

/** Every sound in the machine's content folder, by the name a scene uses for it */
export const contentSounds = names
export const contentSoundsFailed = listFailed

/** Scans the folder, which can change on the machine at any time */
export async function refreshContentSounds() {
  try {
    const response = await fetch('/content/sounds')
    if (!response.ok) throw new Error(response.statusText)
    setNames(await response.json())
    setListFailed(false)
    // a sound may have been replaced, so its length is read again when it's next asked for
    asked.clear()
  } catch {
    setListFailed(true)
  }
}

/** How many seconds a sound in the content folder lasts, which is `undefined` until it's been read */
export function contentSoundLength(sound: string): number | null | undefined {
  if (!asked.has(sound)) {
    asked.add(sound)
    // only the metadata, rather than decoding the whole sound
    const audio = new Audio()
    audio.preload = 'metadata'
    audio.onloadedmetadata = () => setLengths(sound, Number.isFinite(audio.duration) ? audio.duration : null)
    audio.onerror = () => setLengths(sound, null)
    audio.src = `/content/sounds/${sound.split('/').map(encodeURIComponent).join('/')}`
  }
  return lengths[sound]
}

/**
 * Plays a sound from the content folder on the machine, at a volume where 1 is as recorded, from `skip` seconds in, and
 * cut off after `length` seconds if given
 */
export function playContentSound(sound: string, volume: number, skip = 0, length: number | null = null) {
  sendToMachine({ type: 'PlaySound', sound, volume, skip, length })
}
