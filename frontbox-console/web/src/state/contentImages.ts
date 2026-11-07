import { createSignal } from 'solid-js'
import type { ImageInfo } from '../types/generated/ImageInfo'

const [names, setNames] = createSignal<string[]>([])
const [listFailed, setListFailed] = createSignal(false)
const [infos, setInfos] = createSignal<Record<string, ImageInfo>>({})
const asked = new Set<string>()

/** Every image in the machine's content folder, by the name a scene uses for it. */
export const contentImages = names
export const contentImagesFailed = listFailed

export function contentImageUrl(name: string): string {
  return `/content/images/${name.split('/').map(encodeURIComponent).join('/')}`
}

/** What the machine says about an image, once it has. The machine decodes it to find out, so it's only asked about images in use. */
export function contentImageInfo(name: string): ImageInfo | undefined {
  if (name !== '' && !asked.has(name)) fetchInfo(name)
  return infos()[name]
}

async function fetchInfo(name: string) {
  asked.add(name)
  try {
    const response = await fetch(`/content/image-info/${name.split('/').map(encodeURIComponent).join('/')}`)
    if (response.ok) {
      const info: ImageInfo = await response.json()
      setInfos((current) => ({ ...current, [name]: info }))
    }
  } catch {
    // the scene's render says what's wrong with an image, and this one can be left as what's known
  }
}

/** Scans the folder, which can change on the machine at any time. */
export async function refreshContentImages() {
  asked.clear()
  Object.keys(infos()).forEach(fetchInfo)
  try {
    const response = await fetch('/content/images')
    if (!response.ok) throw new Error(response.statusText)
    setNames(await response.json())
    setListFailed(false)
  } catch {
    setListFailed(true)
  }
}

/** Adds a file to the top of the folder, under its own name. Throws what to show if it can't. */
export async function addContentImage(file: File): Promise<string> {
  const response = await fetch(contentImageUrl(file.name), { method: 'PUT', body: file })
  if (!response.ok) throw new Error((await response.text()) || "The image couldn't be added")
  await refreshContentImages()
  return file.name
}
