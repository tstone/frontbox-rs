import { createEffect, createSignal, on, Show, untrack, useContext } from 'solid-js'
import { machine } from '../../state/console'
import type { LedQ } from '../../types/generated/LedQ'
import LedPicker, { canPick, ledPick, ledsLabel, pickedQuery, type LedPick } from './LedPicker'
import { SceneContext } from './SceneFields'
import { ledsCase, type Lens } from './sceneModel'

// a layer with no LEDs yet says so in its own row
const LEDS_NOTES = {
  None: null,
  Replaceable: 'Code can replace these LEDs.',
  Fixed: "Layers are on different LEDs, so code can't replace them.",
}

/** The names of the planes a layer can cover, as one with no extent has no size to give it */
export function planeNames() {
  return machine.planes.filter((plane) => plane.extent[0] > 0 && plane.extent[1] > 0).map((plane) => plane.name)
}

/** What code can do with a scene's LEDs, which comes of the LEDs its layers are on */
export function LedsNote() {
  const scene = useContext(SceneContext)!
  return (
    <Show when={scene.get().layers.length > 0 && LEDS_NOTES[ledsCase(scene.get())]}>
      {(note) => <p class="scene-note">{note()}</p>}
    </Show>
  )
}

/** Which LEDs a layer is on. New layers start on every LED, and only a scene from before then has none. */
export function LayerLeds(props: { lens: Lens<LedQ | undefined> }) {
  // what's been picked each way is kept here to come back to, as the file only has the way that's in use
  const [pick, setPick] = createSignal(ledPick(props.lens.get() ?? 'Every'))
  createEffect(
    on(
      () => props.lens.get(),
      (leds) => {
        const query = leds ?? 'Every'
        if (JSON.stringify(pickedQuery(untrack(pick))) !== JSON.stringify(query)) setPick(ledPick(query))
      },
    ),
  )
  const choose = (chosen: LedPick) => {
    setPick(chosen)
    props.lens.set(pickedQuery(chosen))
  }

  const label = () => {
    const leds = props.lens.get()
    return leds === undefined ? 'None' : ledsLabel(leds)
  }
  const title = () => {
    const leds = props.lens.get()
    return leds === undefined || canPick(leds) ? label() : 'Replaced if chosen here. Edit it in Raw to keep it.'
  }

  return (
    <div class="scene-layer-leds">
      <span class="scene-note" title={title()}>
        {label()}
      </span>
      <LedPicker pick={pick()} onChange={choose} />
    </div>
  )
}
