import { createEffect, createSignal, Index, on, Show, untrack, useContext } from 'solid-js'
import { machine } from '../../state/console'
import type { LedTarget } from '../../types/generated/LedTarget'
import LedPicker, { canPick, ledsLabel, ledPick, pickedQuery, type LedPick } from './LedPicker'
import { Dropdown, NumberBox, SceneContext } from './SceneFields'
import { sceneTargets, withTargetAdded, withTargetChanged, withTargetRemoved, withTargetRenamed } from './sceneModel'

/** The names of the planes a scene can cover, as one with no extent has no size to give it */
export function planeNames() {
  return machine.planes.filter((plane) => plane.extent[0] > 0 && plane.extent[1] > 0).map((plane) => plane.name)
}

/** What an LED scene plays on, which is one of three things: nothing yet, LEDs code can replace, or fixed, named targets */
export default function TargetsPanel() {
  const scene = useContext(SceneContext)!
  const targets = () => sceneTargets(scene.get())
  const add = () => scene.set(withTargetAdded(scene.get(), planeNames()[0]))

  return (
    <>
      <Show when={targets().length === 0}>
        <p class="scene-note">No LEDs yet. This previews on every LED, and can't play in the game.</p>
      </Show>
      <Index each={targets()}>{(target) => <TargetRow name={target().name} target={target().target} />}</Index>
      <Show when={targets().length === 1}>
        <p class="scene-note">Code can replace these LEDs.</p>
      </Show>
      <Show when={targets().length > 1}>
        <p class="scene-note">Each layer chooses its targets. Code can't replace these LEDs.</p>
      </Show>
      <button type="button" class="scene-add" onClick={add}>
        {targets().length === 0 ? 'Choose LEDs' : 'Add target'}
      </button>
    </>
  )
}

/** The LEDs of a target, the layer they're declared on, and for an `Area` the plane they're on. `name` is `null` on a single target. */
function TargetRow(props: { name: string | null; target: LedTarget }) {
  const scene = useContext(SceneContext)!
  const change = (target: LedTarget) => scene.set(withTargetChanged(scene.get(), props.name, target))

  // what's been picked each way is kept here to come back to, as the file only has the way that's in use
  const [pick, setPick] = createSignal(ledPick(props.target.leds))
  createEffect(
    on(
      () => props.target.leds,
      (leds) => {
        if (JSON.stringify(pickedQuery(untrack(pick))) !== JSON.stringify(leds)) setPick(ledPick(leds))
      },
    ),
  )
  const choose = (chosen: LedPick) => {
    setPick(chosen)
    change({ ...props.target, leds: pickedQuery(chosen) })
  }

  /** A name has to stay its own, so anything else puts the input back */
  const rename = (input: HTMLInputElement) => {
    const chosen = input.value.trim()
    const taken = sceneTargets(scene.get()).some((target) => target.name === chosen)
    if (chosen === '' || taken) {
      input.value = props.name ?? ''
      return
    }
    scene.set(withTargetRenamed(scene.get(), props.name!, chosen))
  }

  return (
    <div class="scene-target">
      <Show when={props.name !== null}>
        <input
          class="scene-target-name"
          aria-label="Target name"
          spellcheck={false}
          value={props.name ?? ''}
          onChange={(e) => rename(e.currentTarget)}
        />
      </Show>
      <LedPicker pick={pick()} onChange={choose} />
      <span class="scene-note" title={canPick(props.target.leds) ? undefined : 'Replaced if chosen here. Edit it in Raw to keep it.'}>
        {ledsLabel(props.target.leds)}
      </span>
      <label class="scene-target-z">
        z
        <NumberBox aria-label="Layer" step="1" value={props.target.z ?? 0} onNumber={(z) => change({ ...props.target, z: Math.round(z) })} />
      </label>
      <Show when={scene.get().viewport === 'Area'}>
        <Dropdown
          label="Plane"
          options={planeNames().map((plane) => ({ value: plane, label: plane }))}
          value={props.target.plane ?? ''}
          onChange={(plane) => change({ ...props.target, plane })}
        />
      </Show>
      <button
        type="button"
        class="scene-target-remove"
        aria-label={`Remove ${props.name ?? 'target'}`}
        onClick={() => scene.set(withTargetRemoved(scene.get(), props.name))}
      >
        ×
      </button>
    </div>
  )
}
