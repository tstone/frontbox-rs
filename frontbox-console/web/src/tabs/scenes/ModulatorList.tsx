import { createSignal, Index, Show, useContext } from 'solid-js'
import * as icons from '../../components/icons'
import type { Modulator } from '../../types/generated/Modulator'
import ConfirmDialog from './ConfirmDialog'
import { ModulatorFields, SceneContext } from './SceneFields'
import { field, modulatorUseCount, withModulatorRenamed, withNewModulator, withoutModulator, type Lens } from './sceneModel'

/** Where a modulator is on the page, for opening the Modulators tab at it */
export const modulatorElementId = (name: string) => `scene-modulator-${name}`

/**
 * The scene's modulators, each a wave's shape and rate that any number of modulated props move by, and a button to add
 * one. Each prop sets its own range and phase, so props sharing one can move differently.
 */
export default function ModulatorList() {
  const scene = useContext(SceneContext)!
  const names = () => Object.keys(scene.get().modulators ?? {})
  // a modulator props still move by is only removed once that's confirmed
  const [removing, setRemoving] = createSignal<{ name: string; uses: number } | null>(null)
  const remove = (name: string) => {
    const uses = modulatorUseCount(scene.get(), name)
    if (uses === 0) scene.set(withoutModulator(scene.get(), name))
    else setRemoving({ name, uses })
  }

  return (
    <section class="scene-sidebar-group" aria-label="Modulators">
      <Show when={names().length === 0}>
        <p class="scene-note">Add a modulator, then set a prop to Modulated to move it by one. Props can share a modulator, each with its own range and phase.</p>
      </Show>
      {/* by position, with each remade once a different one is there, so an edit doesn't tear down the field it was made in */}
      <Index each={names()}>
        {(name) => (
          <Show when={name()} keyed>
            {(name) => <ModulatorEntry name={name} onRemove={() => remove(name)} />}
          </Show>
        )}
      </Index>
      <button type="button" class="scene-add-modifier" onClick={() => scene.set(withNewModulator(scene.get())[0])}>
        <icons.Plus aria-hidden="true" />
        Modulator
      </button>
      <ConfirmDialog
        open={removing() !== null}
        title={`Remove ${removing()?.name}?`}
        description={`${removing()?.uses === 1 ? 'A prop moves' : `${removing()?.uses} props move`} by it, and will read as zero until given another modulator.`}
        cancel="Keep it"
        action="Remove"
        onCancel={() => setRemoving(null)}
        onConfirm={() => {
          scene.set(withoutModulator(scene.get(), removing()!.name))
          setRemoving(null)
        }}
      />
    </section>
  )
}

/** One modulator: its name, how many props move by it, and its shape and rate */
function ModulatorEntry(props: { name: string; onRemove: () => void }) {
  const scene = useContext(SceneContext)!
  const modulator = field(field(scene, 'modulators') as Lens<Record<string, Modulator>>, props.name)
  const uses = () => modulatorUseCount(scene.get(), props.name)

  return (
    <div class="scene-properties-node" id={modulatorElementId(props.name)}>
      <div class="scene-prop-header scene-value-header">
        <input
          class="scene-variable-name"
          aria-label="Modulator name"
          spellcheck={false}
          value={props.name}
          onChange={(e) => {
            scene.set(withModulatorRenamed(scene.get(), props.name, e.currentTarget.value.trim()))
            e.currentTarget.value = props.name
          }}
        />
        <span class="scene-field-label">{uses() === 1 ? '1 prop' : `${uses()} props`}</span>
        <button type="button" class="scene-icon-button" aria-label={`Remove modulator ${props.name}`} title="Remove" onClick={props.onRemove}>
          <icons.X aria-hidden="true" />
        </button>
      </div>
      <ModulatorFields label={props.name} lens={modulator} />
    </div>
  )
}
