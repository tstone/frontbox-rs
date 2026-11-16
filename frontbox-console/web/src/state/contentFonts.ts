import { createSignal } from 'solid-js'
import type { FontInfo } from '../types/generated/FontInfo'

const [fonts, setFonts] = createSignal<FontInfo[]>([])

/** Every font a scene can use, by the name a scene uses for it, from the smallest capitals to the largest */
export const contentFonts = () => fonts().map(({ name }) => name)

/** For a pixel font, the size that's one scene unit to its pixels, which it's crisp at whole multiples of */
export const fontPixelSize = (name: string) => fonts().find((font) => font.name === name)?.pixel_size ?? undefined

/** A font's name, and for a pixel font how many pixels tall its capitals are, as "TinyPix (4)" */
export function fontLabel(name: string) {
  const capHeight = fonts().find((font) => font.name === name)?.cap_height
  return capHeight == null ? name : `${name} (${capHeight})`
}

/** Scans the folder, which can change on the machine at any time */
export async function refreshContentFonts() {
  try {
    const response = await fetch('/content/fonts')
    if (!response.ok) throw new Error(response.statusText)
    setFonts(sortedBySize(await response.json()))
  } catch {
    // a font field still shows the one it has, and can be typed into
  }
}

/** Pixel fonts from the smallest capitals to the largest, then the rest, each by name */
function sortedBySize(fonts: FontInfo[]) {
  const size = (font: FontInfo) => font.cap_height ?? Infinity
  return [...fonts].sort((a, b) => size(a) - size(b) || a.name.localeCompare(b.name))
}
