import { For, Show, useContext } from 'solid-js'
import { FieldRow, SceneContext, ValueField } from './SceneFields'
import { paramRanges, unwrapValue, withParam, type Lens, type SceneValue } from './sceneModel'

/** The defaults of a scene's parameters */
export default function ParametersPanel() {
  const scene = useContext(SceneContext)!
  const names = () => Object.keys(scene.get().params ?? {}).sort()

  const value = (name: string): Lens<SceneValue> => ({
    get: () => unwrapValue(scene.get().params![name]),
    set: (next) => scene.set(withParam(scene.get(), name, next)),
  })

  return (
    <section class="scene-params" aria-label="Parameters">
      <h3>Parameters</h3>
      <Show when={names().length > 0} fallback={<p class="scene-note">Set a prop to Param to add one.</p>}>
        <For each={names()}>
          {(name) => (
            <FieldRow label={name}>
              <ValueField label={name} lens={value(name)} ranges={paramRanges(scene.get(), name)} />
            </FieldRow>
          )}
        </For>
      </Show>
    </section>
  )
}
