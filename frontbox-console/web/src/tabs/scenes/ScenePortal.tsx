import { createContext, useContext, type JSX } from 'solid-js'
import { Portal } from 'solid-js/web'
import type { SceneLayer } from './sceneModel'

/**
 * What the layer the fields below are for draws on, as it gives them their accent color: the lights, a display, or
 * the speakers. `undefined` is for what isn't in a layer, which keeps the usual accent.
 */
export type LayerColor = 'lights' | 'display' | 'sound' | undefined

export const LayerColorContext = createContext<LayerColor>()

/** The class that gives what's in it the accent color of `color`'s layers */
export const layerColorClass = (color: LayerColor) => (color ? `scene-color-${color}` : undefined)

/** The color of a drawn layer, by whether it's on a display */
export const layerColor = (layer: SceneLayer): LayerColor => (layer.display === undefined ? 'lights' : 'display')

/**
 * A `Portal` whose content keeps the accent color of the layer it's for, as on the page it's outside that layer and
 * wouldn't otherwise get it
 */
export default function ScenePortal(props: { children: JSX.Element }) {
  const color = useContext(LayerColorContext)
  return (
    <Portal>
      <div class={layerColorClass(color)} style={{ display: 'contents' }}>
        {props.children}
      </div>
    </Portal>
  )
}
