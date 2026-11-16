import { createContext, useContext, type JSX } from 'solid-js'
import { Portal } from 'solid-js/web'

/** Which kind of layer the fields below are for, as it gives them their accent color. `undefined` is the usual one. */
export type LayerColor = 'sound' | undefined

export const LayerColorContext = createContext<LayerColor>()

/** The class that gives what's in it the accent color of `color`'s layers */
export const layerColorClass = (color: LayerColor) => (color ? `scene-color-${color}` : undefined)

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
