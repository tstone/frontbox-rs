import { Popover } from '@ark-ui/solid/popover'
import { createSignal, For, onCleanup } from 'solid-js'
import * as icons from '../../components/icons'
import ScenePortal from './ScenePortal'
import type { ViewAngle } from '../machine/PlayfieldView'

/** How many milliseconds the pointer rests on the button for the angles to come up */
const OPEN_DELAY = 400

/** How many milliseconds the angles stay up once the pointer has left them */
const CLOSE_DELAY = 1000

/** The angles the camera can look at a plane from: tilted toward the player, then one from a front corner */
const ANGLES: { label: string; angle: ViewAngle }[] = [
  { label: 'Top', angle: { tilt: 0, turn: 0 } },
  { label: 'Top 30°', angle: { tilt: 30, turn: 0 } },
  { label: 'Top 60°', angle: { tilt: 60, turn: 0 } },
  { label: 'Three-quarter', angle: { tilt: 45, turn: 45 } },
]

const sameAngle = (a: ViewAngle, b: ViewAngle) => a.tilt === b.tilt && a.turn === b.turn

/**
 * A button that puts the camera back on the plane being edited, at the angle last picked. Resting the pointer on it
 * brings up the angles to pick from, which also put the camera back.
 */
export default function ViewAngleMenu(props: { angle: ViewAngle; onPick: (angle: ViewAngle) => void }) {
  const [open, setOpen] = createSignal(false)
  let timer: ReturnType<typeof setTimeout> | undefined

  const stay = () => clearTimeout(timer)
  const after = (delay: number, next: boolean) => {
    stay()
    timer = setTimeout(() => setOpen(next), delay)
  }
  const leave = () => after(CLOSE_DELAY, false)
  onCleanup(stay)

  const pick = (angle: ViewAngle) => {
    props.onPick(angle)
    stay()
    setOpen(false)
  }

  return (
    <Popover.Root
      lazyMount
      unmountOnExit
      autoFocus={false}
      restoreFocus={false}
      positioning={{ placement: 'bottom' }}
      open={open()}
      onOpenChange={(details) => {
        stay()
        if (!details.open) setOpen(false)
      }}
    >
      {/* a click resets the view rather than opening the angles, which come up on their own */}
      <Popover.Anchor>
        <button
          type="button"
          class="scene-icon-button scene-reset-view"
          aria-label="Reset view"
          onClick={() => pick(props.angle)}
          onPointerEnter={() => (open() ? stay() : after(OPEN_DELAY, true))}
          onPointerLeave={() => (open() ? leave() : stay())}
        >
          <icons.LocateFixed aria-hidden="true" />
        </button>
      </Popover.Anchor>
      <ScenePortal>
        <Popover.Positioner>
          <Popover.Content class="scene-view-angles" aria-label="View angle" onPointerEnter={stay} onPointerLeave={leave}>
            <For each={ANGLES}>
              {(angle) => (
                <button type="button" aria-pressed={sameAngle(angle.angle, props.angle)} onClick={() => pick(angle.angle)}>
                  {angle.label}
                </button>
              )}
            </For>
          </Popover.Content>
        </Popover.Positioner>
      </ScenePortal>
    </Popover.Root>
  )
}
