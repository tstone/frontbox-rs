import { For, Match, Show, Switch, useContext } from 'solid-js'
import { FieldRow, LayerSurfacesContext, NumberBox, SceneContext, ValueField } from './SceneFields'
import { paramRanges, unwrapValue, withParam, withParamValue, type Lens, type PropValue, type SceneValue } from './sceneModel'

/** The defaults of a scene's parameters */
export default function ParametersPanel() {
  const scene = useContext(SceneContext)!
  const surfaces = useContext(LayerSurfacesContext)!
  const names = () => Object.keys(scene.get().params ?? {}).sort()
  const param = (name: string) => scene.get().params![name]

  const value = (name: string): Lens<SceneValue> => ({
    get: () => unwrapValue(param(name) as PropValue),
    set: (next) => scene.set(withParam(scene.get(), name, next)),
  })

  return (
    <section class="scene-params" aria-label="Parameters">
      <h3>Parameters</h3>
      <Show when={names().length > 0} fallback={<p class="scene-note">Set a prop to Param to add one.</p>}>
        <For each={names()}>
          {(name) => (
            <FieldRow label={name}>
              <Switch fallback={<ValueField label={name} lens={value(name)} ranges={paramRanges(scene.get(), name, surfaces)} />}>
                <Match when={'Text' in param(name) && (param(name) as { Text: string })}>
                  {(text) => (
                    <textarea
                      class="scene-param-name scene-text-words"
                      aria-label={name}
                      rows={1}
                      spellcheck={false}
                      value={text().Text}
                      onInput={(e) => scene.set(withParamValue(scene.get(), name, { Text: e.currentTarget.value }))}
                    />
                  )}
                </Match>
                <Match when={'Int' in param(name) && (param(name) as { Int: number })}>
                  {(int) => (
                    <NumberBox
                      class="scene-tree-number"
                      aria-label={name}
                      step="1"
                      value={int().Int}
                      onNumber={(next) => scene.set(withParamValue(scene.get(), name, { Int: Math.round(next) }))}
                    />
                  )}
                </Match>
              </Switch>
            </FieldRow>
          )}
        </For>
      </Show>
    </section>
  )
}
