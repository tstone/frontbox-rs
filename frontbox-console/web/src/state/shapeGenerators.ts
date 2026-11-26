import { createSignal } from 'solid-js'
import type { ShapeGeneratorInfo } from '../types/generated/ShapeGeneratorInfo'

const [generators, setGenerators] = createSignal<ShapeGeneratorInfo[]>([])

/** Every shape generator the app has added, which a scene's shape can be made by, in order of name */
export const shapeGenerators = generators

/** The shape generator a scene names, if the app has it */
export const shapeGenerator = (name: string) => generators().find((generator) => generator.name === name)

/** Asks the machine, whose app adds them as it starts */
export async function refreshShapeGenerators() {
  try {
    const response = await fetch('/content/shape-generators')
    if (!response.ok) throw new Error(response.statusText)
    setGenerators(await response.json())
  } catch {
    // a shape still shows the generator it names
  }
}
