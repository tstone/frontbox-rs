import { batch, Index, Show, useContext } from 'solid-js'
import LedPicker, { EVERY_LED, ledsLabel, pickedQuery, type LedPick } from './LedPicker'
import { SceneContext } from './SceneFields'
import { withTargetAdded, withTargetRemoved, withTargetRenamed } from './sceneModel'

/**
 * The LEDs to preview a scene on. A scene file never names hardware, so these are kept in the browser: `only` for a
 * scene with no targets, and otherwise a pick for each target by name.
 */
export type LedPicks = { only: LedPick; targets: Record<string, LedPick> }

const NO_PICKS: LedPicks = { only: EVERY_LED, targets: {} }

const storageKey = (file: string) => `frontbox.scenes.ledPicks.${file}`

export function loadLedPicks(file: string): LedPicks {
  try {
    return { ...NO_PICKS, ...JSON.parse(localStorage.getItem(storageKey(file)) ?? '{}') }
  } catch {
    return NO_PICKS
  }
}

export function storeLedPicks(file: string, picks: LedPicks) {
  try {
    localStorage.setItem(storageKey(file), JSON.stringify(picks))
  } catch {
    // only a convenience, and they're still there until the editor closes
  }
}

/** What a scene's targets are previewed on, and with more than one, their names */
export default function TargetsPanel(props: { picks: LedPicks; onChange: (picks: LedPicks) => void }) {
  const scene = useContext(SceneContext)!
  const targets = () => scene.get().targets ?? []
  const picked = (name: string) => props.picks.targets[name] ?? EVERY_LED

  const pick = (name: string, chosen: LedPick) =>
    props.onChange({ ...props.picks, targets: { ...props.picks.targets, [name]: chosen } })

  /** The one pick there's been so far carries on as the first target's */
  const add = () => {
    const first = targets().length === 0
    const next = withTargetAdded(scene.get())
    batch(() => {
      if (first) props.onChange({ ...props.picks, targets: { [next.targets![0]]: props.picks.only } })
      scene.set(next)
    })
  }

  /** The last target left stops being one, and keeps its pick */
  const remove = (name: string) => {
    const next = withTargetRemoved(scene.get(), name)
    const { [name]: _, ...rest } = props.picks.targets
    const left = targets().filter((target) => target !== name)
    batch(() => {
      props.onChange(left.length === 1 ? { only: picked(left[0]), targets: {} } : { ...props.picks, targets: rest })
      scene.set(next)
    })
  }

  /** A name has to stay its own, so anything else puts the input back */
  const rename = (name: string, input: HTMLInputElement) => {
    const chosen = input.value.trim()
    if (chosen === '' || targets().includes(chosen)) {
      input.value = name
      return
    }
    const { [name]: pick, ...rest } = props.picks.targets
    batch(() => {
      props.onChange({ ...props.picks, targets: pick ? { ...rest, [chosen]: pick } : rest })
      scene.set(withTargetRenamed(scene.get(), name, chosen))
    })
  }

  return (
    <>
      <Show
        when={targets().length > 0}
        fallback={
          <div class="scene-target">
            <LedPicker pick={props.picks.only} onChange={(only) => props.onChange({ ...props.picks, only })} />
            <span class="scene-note">{ledsLabel(pickedQuery(props.picks.only))}</span>
          </div>
        }
      >
        <Index each={targets()}>
          {(name) => (
            <div class="scene-target">
              <input
                class="scene-target-name"
                aria-label="Target name"
                spellcheck={false}
                value={name()}
                onChange={(e) => rename(name(), e.currentTarget)}
              />
              <LedPicker pick={picked(name())} onChange={(chosen) => pick(name(), chosen)} />
              <span class="scene-note">{ledsLabel(pickedQuery(picked(name())))}</span>
              <button type="button" class="scene-target-remove" aria-label={`Remove ${name()}`} onClick={() => remove(name())}>
                ×
              </button>
            </div>
          )}
        </Index>
      </Show>
      <button type="button" class="scene-add" onClick={add}>
        Add target
      </button>
    </>
  )
}
