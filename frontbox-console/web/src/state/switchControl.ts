import { createSignal } from 'solid-js'
import { createStore } from 'solid-js/store'
import { machine, sendToMachine } from './console'

// Switches this browser has latched closed (shift-click, the menu, the hardware list), by id
const [latched, setLatched] = createStore<Record<number, boolean>>({})
// The switch held closed by the pointer right now, released when the press ends
const [pressed, setPressed] = createSignal<number | null>(null)
export { latched, pressed }

const send = (switch_id: number, closed: boolean) => sendToMachine({ type: 'SetSwitch', switch_id, closed })

/** Live state as the machine last reported it, falling back to the state at boot */
export function isSwitchClosed(id: number): boolean {
  const state = machine.switches[id]
  return state ? state === 'Closed' : (machine.hardware?.switches.is_closed[id] ?? false)
}

export function pressSwitch(id: number) {
  release()
  setPressed(id)
  send(id, true)
}

/** Ends a press. A switch that's also latched stays closed. */
export function release() {
  const id = pressed()
  if (id === null) return
  setPressed(null)
  if (!latched[id]) send(id, false)
}

export function toggleLatch(id: number) {
  setSwitchClosed(id, !latched[id])
}

/** Latch a switch closed, or open it (whether it was latched or not) */
export function setSwitchClosed(id: number, closed: boolean) {
  setLatched(id, closed)
  send(id, closed)
}
