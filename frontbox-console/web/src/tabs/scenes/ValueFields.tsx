import { Match, Show, Switch, useContext } from 'solid-js'
import * as icons from '../../components/icons'
import type { NamedValue } from '../../types/generated/NamedValue'
import type { Prop } from '../../types/generated/Prop'
import { NumberBox, PropField, PropSource, SceneContext } from './SceneFields'
import { both, field, UNREFERENCED, variant, withoutValue, withValueRenamed, type Lens, type NamedValueKind, type SceneValue } from './sceneModel'

/** What a prop of each kind of named value starts at with nothing to go on */
const FALLBACKS: Record<DrawnKind, SceneValue> = { Color: [0, 0, 0, 0], Float: 0, Vec2: [0, 0], Bool: true }

export const NAMED_VALUE_LABELS: Record<NamedValueKind, string> = { Color: 'Color', Float: 'Number', Vec2: 'Position', Int: 'Whole number', Text: 'Text', Bool: 'True or false' }

export const namedValueKind = (value: NamedValue) => Object.keys(value)[0] as NamedValueKind

/** A kind of named value that's a prop, which can be keyframed or moved in a wave */
type DrawnKind = 'Color' | 'Float' | 'Vec2' | 'Bool'

/**
 * One of a scene's named values: its name and type side by side, then its value. A variable points at it, from its
 * default or from the game.
 */
export function ValueFields(props: { name: string }) {
  const scene = useContext(SceneContext)!
  const value = field(field(scene, 'values') as Lens<Record<string, NamedValue>>, props.name)
  const kind = () => namedValueKind(value.get())
  const drawn = () => (kind() === 'Int' || kind() === 'Text' ? null : (kind() as DrawnKind))
  const spec = (of: DrawnKind) => ({
    label: NAMED_VALUE_LABELS[of],
    variableName: props.name,
    lens: variant(value as Lens<Record<string, Prop<SceneValue>>>, of) as Lens<Prop<SceneValue>>,
    fallback: FALLBACKS[of],
    ranges: both(UNREFERENCED),
  })

  return (
    <>
      <div class="scene-prop-header scene-value-header">
        <Show when={drawn()}>{(of) => <PropSource {...spec(of())} />}</Show>
        <input
          class="scene-variable-name"
          aria-label="Value name"
          spellcheck={false}
          value={props.name}
          onChange={(e) => {
            scene.set(withValueRenamed(scene.get(), props.name, e.currentTarget.value.trim()))
            e.currentTarget.value = props.name
          }}
        />
        <span class="scene-field-label">{NAMED_VALUE_LABELS[kind()]}</span>
        <button type="button" class="scene-icon-button" aria-label={`Remove value ${props.name}`} title="Remove" onClick={() => scene.set(withoutValue(scene.get(), props.name))}>
          <icons.X aria-hidden="true" />
        </button>
      </div>
      <Switch>
        <Match when={kind() === 'Int'}>
          <NumberBox
            class="scene-tree-number"
            aria-label={props.name}
            step="1"
            value={(value.get() as { Int: number }).Int}
            onNumber={(next) => value.set({ Int: Math.round(next) })}
          />
        </Match>
        <Match when={kind() === 'Text'}>
          <textarea
            class="scene-variable-name scene-text-words"
            aria-label={props.name}
            rows={1}
            spellcheck={false}
            value={(value.get() as { Text: string }).Text}
            onInput={(e) => value.set({ Text: e.currentTarget.value })}
          />
        </Match>
        <Match when={drawn()}>{(of) => <PropField bare {...spec(of())} />}</Match>
      </Switch>
    </>
  )
}
