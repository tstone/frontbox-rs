import { onMount } from 'solid-js'
import type { PreviewSpace } from '../machine/PlayfieldView'
import { layerColorClass, type LayerColor } from './ScenePortal'
import type { Scene, SceneLayer, Vec2 } from './sceneModel'

/**
 * The words of a text layer that can be typed over in the preview: fixed text, or a variable's default. `null` for a
 * layer that isn't text, or whose text is a number.
 */
export function editableText(scene: Scene, layer: SceneLayer | undefined): string | null {
  const base = layer?.region?.base
  if (!base || !('Text' in base)) return null
  const source = base.Text.text
  if ('Fixed' in source) return source.Fixed
  if ('Variable' in source) {
    const value = scene.variables?.[source.Variable]
    return value && 'Text' in value ? value.Text : ''
  }
  return null
}

/** The scene with the words of the text layer at `index` changed, which are its own or its variable's default */
export function withText(scene: Scene, index: number, words: string): Scene {
  const layer = scene.layers[index]
  const base = layer?.region?.base
  if (!base || !('Text' in base)) return scene
  const source = base.Text.text
  if ('Variable' in source) return { ...scene, variables: { ...scene.variables, [source.Variable]: { Text: words } } }
  if (!('Fixed' in source)) return scene
  const region = { ...layer.region!, base: { Text: { ...base.Text, text: { Fixed: words } } } }
  return { ...scene, layers: scene.layers.map((each, at) => (at === index ? { ...each, region } : each)) }
}

/**
 * A field over a text layer's box in the preview, for typing its words there. What's typed goes to the layer as it's
 * typed. Enter or leaving the field keeps it, and Escape puts back what was there. Shift+Enter starts a new line.
 */
export default function TextEditor(props: { space: PreviewSpace; outlines: Vec2[][]; color: LayerColor; text: string; onInput: (words: string) => void; onDone: (keep: boolean) => void }) {
  let field!: HTMLTextAreaElement
  let done = false
  const finish = (keep: boolean) => {
    if (done) return
    done = true
    props.onDone(keep)
  }

  onMount(() => {
    field.focus()
    field.select()
  })

  // around the box as it's seen, which on a plane is at an angle
  const box = () => {
    const corners = props.outlines.flat().flatMap((point) => {
      const at = props.space.project(point)
      return at ? [at] : []
    })
    if (corners.length === 0) return null
    const [xs, ys] = [corners.map(([x]) => x), corners.map(([, y]) => y)]
    return { left: Math.min(...xs), top: Math.min(...ys), width: Math.max(...xs) - Math.min(...xs), height: Math.max(...ys) - Math.min(...ys) }
  }

  return (
    <textarea
      ref={field}
      class={`scene-text-editor ${layerColorClass(props.color) ?? ''}`}
      aria-label="Text"
      spellcheck={false}
      rows={1}
      value={props.text}
      style={
        box()
          ? { left: `${box()!.left + box()!.width / 2}px`, top: `${box()!.top + box()!.height / 2}px`, width: `${Math.max(box()!.width, MIN_WIDTH)}px` }
          : undefined
      }
      onPointerDown={(e) => e.stopPropagation()}
      onInput={(e) => props.onInput(e.currentTarget.value)}
      onKeyDown={(e) => {
        e.stopPropagation()
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault()
          finish(true)
        } else if (e.key === 'Escape') {
          e.preventDefault()
          finish(false)
        }
      }}
      onBlur={() => finish(true)}
    />
  )
}

/** The least the field is across, in pixels, so a small box still has room to type in. It's a line tall, across the middle of the box. */
const MIN_WIDTH = 120
