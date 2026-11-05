import { createSignal } from 'solid-js'
import { createStore } from 'solid-js/store'

/** A position in the cabinet's coordinates, in inches */
export type Position = [number, number, number]

/**
 * Hardware positions moved in the 3D view, by hardware key (`switch:3`, `driver:1`, `led:<name>`). These only live in
 * the browser: they're for working out where things go, then copying the coordinates into the machine's code.
 */
const [moved, setMoved] = createStore<Record<string, Position | undefined>>({})
export const movedPositions = moved

/** The hardware currently following the cursor in the 3D view */
const [placing, setPlacing] = createSignal<string | null>(null)
export { placing }

export function startPlacing(key: string) {
  setPlacing(key)
}

export function cancelPlacing() {
  setPlacing(null)
}

export function place(key: string, position: Position) {
  setMoved(key, position)
  setPlacing(null)
}

export function resetPosition(key: string) {
  setMoved(key, undefined)
}

/** The hardware being pointed at in the hardware list, outlined in the 3D view so it's easy to find */
const [highlighted, setHighlighted] = createSignal<string | null>(null)
export { highlighted, setHighlighted }
