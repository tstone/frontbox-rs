import { Match, Show, Switch, useContext } from 'solid-js'
import { Dropdown, LayerSurfacesContext, NumberBox, SceneContext, ValueField } from './SceneFields'
import { ownValue, variableRanges, unwrapValue, withVariable, withVariableValue, type Lens, type PropValue, type SceneValue } from './sceneModel'

/** Where a variable's default is set: here, as its own value, or by pointing at one of the scene's named values */
const OWN_VALUE = ''

/** Whether a variable's default is its own value, or which of the scene's named values it points at */
export function VariableSource(props: { name: string }) {
  const scene = useContext(SceneContext)!
  const source = () => {
    const current = scene.get().variables![props.name]!
    return 'Value' in current ? current.Value : OWN_VALUE
  }
  const sources = () => [{ value: OWN_VALUE, label: '<dynamic>' }, ...Object.keys(scene.get().values ?? {}).map((value) => ({ value, label: value }))]
  const choose = (next: string) => {
    if (next === source()) return
    if (next !== OWN_VALUE) return scene.set(withVariableValue(scene.get(), props.name, { Value: next }))
    const named = scene.get().values?.[source()]
    scene.set(withVariableValue(scene.get(), props.name, named ? ownValue(scene.get(), named) : { Float: 0 }))
  }
  return (
    <Show when={sources().length > 1 || source() !== OWN_VALUE}>
      <Dropdown compact label={`${props.name} default`} options={sources()} value={source()} onChange={choose} />
    </Show>
  )
}

/** A variable's default, where it's its own value. One pointing at a named value is set by that value. */
export function VariableFields(props: { name: string }) {
  const scene = useContext(SceneContext)!
  const surfaces = useContext(LayerSurfacesContext)!
  const variable = () => scene.get().variables![props.name]!

  const value: Lens<SceneValue> = {
    get: () => unwrapValue(variable() as PropValue),
    set: (next) => scene.set(withVariable(scene.get(), props.name, next)),
  }

  return (
    <Switch fallback={<ValueField label={props.name} lens={value} ranges={variableRanges(scene.get(), props.name, surfaces)} />}>
      <Match when={'Text' in variable() && (variable() as { Text: string })}>
        {(text) => (
          <textarea
            class="scene-variable-name scene-text-words"
            aria-label={props.name}
            rows={1}
            spellcheck={false}
            value={text().Text}
            onInput={(e) => scene.set(withVariableValue(scene.get(), props.name, { Text: e.currentTarget.value }))}
          />
        )}
      </Match>
      <Match when={'Value' in variable() && scene.get().values?.[(variable() as { Value: string }).Value] === undefined}>
        <p class="scene-note scene-note-warn">The scene has no value by that name, so this reads as zero.</p>
      </Match>
      <Match when={'Value' in variable()}>
        <p class="scene-note">Set by the value it points at.</p>
      </Match>
      <Match when={'Int' in variable() && (variable() as { Int: number })}>
        {(int) => (
          <NumberBox
            class="scene-tree-number"
            aria-label={props.name}
            step="1"
            value={int().Int}
            onNumber={(next) => scene.set(withVariableValue(scene.get(), props.name, { Int: Math.round(next) }))}
          />
        )}
      </Match>
    </Switch>
  )
}
