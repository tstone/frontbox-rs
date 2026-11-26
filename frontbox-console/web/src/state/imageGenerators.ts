import { createSignal } from 'solid-js'
import type { ImageGeneratorInfo } from '../types/generated/ImageGeneratorInfo'

const [generators, setGenerators] = createSignal<ImageGeneratorInfo[]>([])

/** Every image generator the app has added, which a scene's fill can be made by, in order of name */
export const imageGenerators = generators

/** The image generator a scene names, if the app has it */
export const imageGenerator = (name: string) => generators().find((generator) => generator.name === name)

/** Asks the machine, whose app adds them as it starts */
export async function refreshImageGenerators() {
  try {
    const response = await fetch('/content/image-generators')
    if (!response.ok) throw new Error(response.statusText)
    setGenerators(await response.json())
  } catch {
    // a fill still shows the generator it names
  }
}
