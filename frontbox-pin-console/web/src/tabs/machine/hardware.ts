import type { ConsoleState } from '../../state/console'

export type HardwareKind = 'drivers' | 'motors' | 'switches' | 'leds'

export type HardwareKindInfo = {
  kind: HardwareKind
  label: string
  /** Shown when there's nothing of this kind */
  empty: string
}

/** Order here is the order of the filters and sections */
export const hardwareKinds: HardwareKindInfo[] = [
  { kind: 'drivers', label: 'Drivers', empty: 'No drivers configured' },
  { kind: 'motors', label: 'Motors', empty: 'Motors are not supported by Frontbox yet' },
  { kind: 'switches', label: 'Switches', empty: 'No switches configured' },
  { kind: 'leds', label: 'LEDs', empty: 'No LEDs configured' },
]

/** Switches and LEDs will be shown on the playfield, so the list starts without them */
export const defaultHardwareKinds: HardwareKind[] = ['drivers', 'motors']

export type HardwareRow = {
  key: string
  name: string
  address: string
  tags: string[]
  /**
   * Live state, when the console tracks one for this kind. These are accessors so that rows are
   * only rebuilt when the hardware changes, while state changes update just their own cell.
   */
  state: () => string | null
  /** Whether `state` means energized/closed, for highlighting */
  active: () => boolean
}

export function hardwareRows(machine: ConsoleState, kind: HardwareKind): HardwareRow[] {
  const hw = machine.hardware
  if (!hw) return []

  switch (kind) {
    case 'drivers':
      return Object.values(hw.drivers.by_id).map((driver) => {
        const state = () => machine.drivers[driver.id] ?? 'Off'
        return {
          key: `driver:${driver.id}`,
          name: driver.name,
          address: `${driver.assignment.board_idx}-${driver.assignment.pin}`,
          tags: driver.tags,
          state,
          active: () => state() !== 'Off',
        }
      })
    case 'switches':
      return Object.values(hw.switches.by_id).map((sw) => {
        const state = () =>
          machine.switches[sw.id] ?? (hw.switches.is_closed[sw.id] ? 'Closed' : 'Open')
        return {
          key: `switch:${sw.id}`,
          name: sw.name,
          address: `${sw.assignment.board_idx}-${sw.assignment.pin}`,
          tags: sw.tags,
          state,
          active: () => state() === 'Closed',
        }
      })
    case 'leds':
      return Object.values(hw.leds.by_name).map((led) => {
        const { exp, index } = led.address
        const board = exp.breakout === null ? `${exp.board_address}` : `${exp.board_address}.${exp.breakout}`
        return {
          key: `led:${led.name}`,
          name: led.name,
          address: `${board}-${exp.port}-${index}`,
          tags: led.tags,
          state: () => null,
          active: () => false,
        }
      })
    case 'motors':
      // not yet part of the hardware definition
      return []
  }
}
