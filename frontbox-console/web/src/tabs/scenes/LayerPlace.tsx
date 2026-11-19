import { createEffect, createSignal, on, Show, untrack, useContext } from 'solid-js'
import { machine } from '../../state/console'
import type { LightQ } from '../../types/generated/LightQ'
import LightPicker, { canPick, lightPick, lightsLabel, pickedQuery, type LightPick } from './LightPicker'
import { SceneContext } from './SceneFields'
import { layerPlace, lightsCase, type LayerPlace, type Lens, type Scene } from './sceneModel'

// a layer with no lights yet says so in its own row
const LIGHTS_NOTES = {
  None: null,
  Replaceable: 'Code can replace these lights.',
  Fixed: "Layers are on different lights, so code can't replace them.",
}

/** The names of the planes a layer can cover, as one with no extent has no size to give it */
export function planeNames() {
  return machine.planes.filter((plane) => plane.extent[0] > 0 && plane.extent[1] > 0).map((plane) => plane.name)
}

/** What a display layer is on when the machine has no displays and the scene names none, as the framework's scenes are */
const DEFAULT_DISPLAY = 'Dmd'

/** The displays a scene's layers can be on: the machine's, then any others the scene names */
export function displayNames(scene: Scene) {
  const named = scene.layers.flatMap((layer) => (layer.display === undefined ? [] : [layer.display]))
  const names = [...new Set([...machine.displays.map((display) => display.name), ...named])]
  return names.length > 0 ? names : [DEFAULT_DISPLAY]
}

/** The kinds of place a new layer can go on, which pick a plane or display for it that "Draws on" can change */
export type NewLayerPlace = 'Sequence' | 'Area' | 'Display'

export const NEW_LAYER_OPTIONS: { value: NewLayerPlace; label: string }[] = [
  { value: 'Sequence', label: 'Light sequence' },
  { value: 'Area', label: 'Light area' },
  { value: 'Display', label: 'Display' },
]

/** Where a new layer of a kind goes: the plane or display of the scene's last layer on one, or else the first there is */
export function newPlace(scene: Scene, kind: NewLayerPlace): LayerPlace {
  if (kind === 'Sequence') return 'Sequence'
  const places = scene.layers.map(layerPlace)
  const prefix = kind === 'Area' ? 'Area:' : 'Display:'
  const first = kind === 'Area' ? `Area:${planeNames()[0]}` : `Display:${displayNames(scene)[0]}`
  return places.findLast((place) => place.startsWith(prefix)) ?? (first as LayerPlace)
}

/** What code can do with a scene's lights, which comes of the lights its layers are on */
export function LightsNote() {
  const scene = useContext(SceneContext)!
  return (
    <Show when={scene.get().layers.some((layer) => layer.display === undefined) && LIGHTS_NOTES[lightsCase(scene.get())]}>
      {(note) => <p class="scene-note">{note()}</p>}
    </Show>
  )
}

/** Which lights a layer is on. New layers start on every light, and only a scene from before then has none. */
export function LayerLights(props: { lens: Lens<LightQ | undefined> }) {
  // what's been picked each way is kept here to come back to, as the file only has the way that's in use
  const [pick, setPick] = createSignal(lightPick(props.lens.get() ?? 'Every'))
  createEffect(
    on(
      () => props.lens.get(),
      (lights) => {
        const query = lights ?? 'Every'
        if (JSON.stringify(pickedQuery(untrack(pick))) !== JSON.stringify(query)) setPick(lightPick(query))
      },
    ),
  )
  const choose = (chosen: LightPick) => {
    setPick(chosen)
    props.lens.set(pickedQuery(chosen))
  }

  const label = () => {
    const lights = props.lens.get()
    return lights === undefined ? 'None' : lightsLabel(lights)
  }
  const title = () => {
    const lights = props.lens.get()
    return lights === undefined || canPick(lights) ? label() : 'Replaced if chosen here. Edit it in Raw to keep it.'
  }

  return (
    <div class="scene-layer-lights">
      <span class="scene-note" title={title()}>
        {label()}
      </span>
      <LightPicker pick={pick()} onChange={choose} />
    </div>
  )
}
