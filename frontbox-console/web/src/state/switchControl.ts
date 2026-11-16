import { createSignal } from 'solid-js'
import { createStore } from 'solid-js/store'
import { machine, sendToMachine } from './console'

// Switches this browser has latched away from their initial state (shift-click, the menu, the hardware list), by id
const [latched, setLatched] = createStore<Record<number, boolean>>({})
// The switch held away from its initial state by the pointer right now, put back when the press ends
const [pressed, setPressed] = createSignal<number | null>(null)
export { latched, pressed }

const send = (switch_id: number, closed: boolean) => sendToMachine({ type: 'SetSwitch', switch_id, closed })

/** Live state as the machine last reported it, falling back to the state at boot */
export function isSwitchClosed(id: number): boolean {
  const state = machine.switches[id]
  return state ? state === 'Closed' : (machine.hardware?.switches.is_closed[id] ?? false)
}

/** Whether the switch is closed in its initial state, such as a coin door switch. Open for a switch with none. */
export function isInitiallyClosed(id: number): boolean {
  return machine.hardware?.switches.initial_states[id] === 'Closed'
}

/** Holds the switch away from its initial state until `release` */
export function pressSwitch(id: number) {
  release()
  setPressed(id)
  send(id, !isInitiallyClosed(id))
}

/** Ends a press. A switch that's also latched stays as it is. */
export function release() {
  const id = pressed()
  if (id === null) return
  setPressed(null)
  if (!latched[id]) send(id, isInitiallyClosed(id))
}

export function toggleLatch(id: number) {
  setSwitchClosed(id, latched[id] ? isInitiallyClosed(id) : !isInitiallyClosed(id))
}

/** Set a switch, latching it if that's away from its initial state */
export function setSwitchClosed(id: number, closed: boolean) {
  setLatched(id, closed !== isInitiallyClosed(id))
  send(id, closed)
}
