import { createEffect, createSignal, on, Show, untrack, useContext } from 'solid-js'
import { machine } from '../../state/console'
import type { LedQ } from '../../types/generated/LedQ'
import LedPicker, { canPick, EVERY_LED, ledPick, ledsLabel, pickedQuery, type LedPick } from './LedPicker'
import { Dropdown, FieldRow, NumberBox, SceneContext } from './SceneFields'
import { field, layerPlane, ledsCase, withLayerSurface, type Lens, type SceneLayer } from './sceneModel'

const TYPE_OPTIONS: { value: 'Sequence' | 'Area'; label: string }[] = [
  { value: 'Sequence', label: 'Sequence (1D)' },
  { value: 'Area', label: 'Area (2D)' },
]

const LEDS_NOTES = {
  None: "A layer has no LEDs yet. It previews on every LED, and this can't play in the game.",
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
    <Show when={scene.get().layers.length > 0}>
      <p class="scene-note">{LEDS_NOTES[ledsCase(scene.get())]}</p>
    </Show>
  )
}

/** Where a layer of an LED scene happens: on what, on which LEDs, and how it stacks up against everything else on them */
export default function LayerPlace(props: { lens: Lens<SceneLayer> }) {
  const scene = useContext(SceneContext)!
  const plane = () => layerPlane(props.lens.get())
  const type = () => (plane() === null ? 'Sequence' : 'Area')

  return (
    <Show when={scene.get().kind === 'Led'}>
      <section class="scene-group">
        <FieldRow label="Type">
          <Dropdown
            label="Type"
            options={TYPE_OPTIONS}
            value={type()}
            onChange={(value) => {
              if (value === type()) return
              props.lens.set(withLayerSurface(props.lens.get(), value === 'Area' ? { Area: planeNames()[0] ?? '' } : 'Sequence'))
            }}
          />
        </FieldRow>
        <Show when={plane() !== null}>
          <FieldRow label="Plane">
            <Dropdown
              label="Plane"
              options={planeNames().map((name) => ({ value: name, label: name }))}
              value={plane()!}
              onChange={(name) => field(props.lens, 'surface').set({ Area: name })}
            />
          </FieldRow>
        </Show>
        <FieldRow label="LEDs">
          <LayerLeds lens={field(props.lens, 'leds')} />
        </FieldRow>
        <FieldRow label="Z">
          <NumberBox aria-label="Z" step="1" value={props.lens.get().z ?? 0} onNumber={(z) => field(props.lens, 'z').set(Math.round(z) || undefined)} />
        </FieldRow>
      </section>
    </Show>
  )
}

/** Which LEDs a layer is on, if it's been said */
function LayerLeds(props: { lens: Lens<LedQ | undefined> }) {
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

  return (
    <Show
      when={props.lens.get()}
      fallback={
        <button type="button" class="scene-add" onClick={() => choose(EVERY_LED)}>
          Choose LEDs
        </button>
      }
    >
      {(leds) => (
        <div class="scene-layer-leds">
          <LedPicker pick={pick()} onChange={choose} />
          <span class="scene-note" title={canPick(leds()) ? undefined : 'Replaced if chosen here. Edit it in Raw to keep it.'}>
            {ledsLabel(leds())}
          </span>
          <button type="button" class="scene-layer-leds-clear" aria-label="Clear LEDs" onClick={() => props.lens.set(undefined)}>
            ×
          </button>
        </div>
      )}
    </Show>
  )
}
