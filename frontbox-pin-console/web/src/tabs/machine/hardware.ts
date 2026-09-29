import { triggerSwitches } from '../../lib/drivers'
import { type ConsoleState, driverState } from '../../state/console'
import type { Driver } from '../../types/generated/Driver'

export type HardwareKind = 'drivers' | 'automatic-drivers' | 'motors' | 'switches' | 'leds'

export type HardwareKindInfo = {
  kind: HardwareKind
  label: string
  /** Shown when there's nothing of this kind */
  empty: string
  /** Explanation shown in a tooltip next to the section title */
  note?: string
}

/** Order here is the order of the filters and sections */
export const hardwareKinds: HardwareKindInfo[] = [
  { kind: 'drivers', label: 'Drivers', empty: 'No software-commanded drivers configured' },
  {
    kind: 'automatic-drivers',
    label: 'Automatic Drivers',
    empty: 'No automatic drivers configured',
    note:
      'Fired by the hardware when a switch changes, like flippers and slingshots. The console only ' +
      'sees drivers commanded by software, so these only show a state when software fires them.',
  },
  { kind: 'motors', label: 'Motors', empty: 'Motors are not supported by Frontbox yet' },
  { kind: 'switches', label: 'Switches', empty: 'No switches configured' },
  { kind: 'leds', label: 'LEDs', empty: 'No LEDs configured' },
]

/** Switches and LEDs will be shown on the playfield, so the list starts without them */
export const defaultHardwareKinds: HardwareKind[] = ['drivers', 'automatic-drivers', 'motors']

export type HardwareRow = {
  key: string
  name: string
  address: string
  tags: string[]
  /** Secondary information, like which switch fires an automatic driver */
  detail: string | null
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
    case 'automatic-drivers': {
      const automatic = kind === 'automatic-drivers'
      return Object.values(hw.drivers.by_id)
        .map((driver) => ({ driver, triggers: triggerSwitches(hw.drivers.configs[driver.id]) }))
        .filter(({ triggers }) => triggers.length > 0 === automatic)
        .map(({ driver, triggers }) => driverRow(driver, triggers, automatic))
    }
    case 'switches':
      return Object.values(hw.switches.by_id).map((sw) => {
        const state = () =>
          machine.switches[sw.id] ?? (hw.switches.is_closed[sw.id] ? 'Closed' : 'Open')
        return {
          key: `switch:${sw.id}`,
          name: sw.name,
          address: `${sw.assignment.board_idx}-${sw.assignment.pin}`,
          tags: sw.tags,
          detail: null,
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
          detail: null,
          state: () => null,
          active: () => false,
        }
      })
    case 'motors':
      // not yet part of the hardware definition
      return []
  }
}

function driverRow(driver: Driver, triggers: string[], automatic: boolean): HardwareRow {
  // automatic drivers usually fire without software knowing, so only show a state when it did
  const state = automatic
    ? () => (driverState(driver.id) === 'Off' ? null : driverState(driver.id))
    : () => driverState(driver.id)
  return {
    key: `driver:${driver.id}`,
    name: driver.name,
    address: `${driver.assignment.board_idx}-${driver.assignment.pin}`,
    tags: driver.tags,
    detail: automatic ? `on ${triggers.join(', ')}` : null,
    state,
    active: () => driverState(driver.id) !== 'Off',
  }
}
