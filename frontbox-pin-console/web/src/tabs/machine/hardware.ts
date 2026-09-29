import { triggerSwitches } from '../../lib/drivers'
import { ledName } from '../../lib/format'
import { hexAddress } from '../../lib/hardwareConfig'
import { type ConsoleState, driverState } from '../../state/console'
import type { Driver } from '../../types/generated/Driver'
import type { Hardware } from '../../types/generated/Hardware'
import type { LED } from '../../types/generated/LED'
import type { ResolvedExpansionBoard } from '../../types/generated/ResolvedExpansionBoard'
import type { ResolvedIoBoard } from '../../types/generated/ResolvedIoBoard'
import type { Switch } from '../../types/generated/Switch'

export type HardwareKind =
  | 'manual-drivers'
  | 'automatic-drivers'
  | 'motors'
  | 'switches'
  | 'leds'
  | 'io-boards'
  | 'exp-boards'

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
  { kind: 'manual-drivers', label: 'Manual Drivers', empty: 'No manual drivers configured' },
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
  { kind: 'io-boards', label: 'I/O Boards', empty: 'No I/O boards configured' },
  { kind: 'exp-boards', label: 'Expansion Boards', empty: 'No expansion boards configured' },
]

/** Switches and LEDs will be shown on the playfield, so the list starts without them */
export const defaultHardwareKinds: HardwareKind[] = ['manual-drivers', 'automatic-drivers', 'motors']

/** The hardware a row describes, for its expanded details */
export type HardwareRef =
  | { kind: 'driver'; driver: Driver }
  | { kind: 'switch'; switch: Switch }
  | { kind: 'led'; led: LED }
  | { kind: 'io-board'; board: ResolvedIoBoard; index: number }
  | { kind: 'exp-board'; board: ResolvedExpansionBoard }

export type HardwareRow = {
  key: string
  name: string
  address: string
  /** Spelled-out version of `address`, shown on hover */
  addressTip?: string
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
  ref: HardwareRef
}

export function hardwareRows(machine: ConsoleState, kind: HardwareKind): HardwareRow[] {
  const hw = machine.hardware
  if (!hw) return []

  switch (kind) {
    case 'manual-drivers':
    case 'automatic-drivers': {
      const automatic = kind === 'automatic-drivers'
      return Object.values(hw.drivers.by_id)
        .sort((a, b) => a.id - b.id)
        .map((driver) => ({ driver, triggers: triggerSwitches(hw.drivers.configs[driver.id]) }))
        .filter(({ triggers }) => triggers.length > 0 === automatic)
        .map(({ driver, triggers }) => driverRow(hw, driver, triggers, automatic))
    }
    case 'switches':
      return Object.values(hw.switches.by_id)
        .sort((a, b) => a.id - b.id)
        .map((sw) => {
          const state = () =>
            machine.switches[sw.id] ?? (hw.switches.is_closed[sw.id] ? 'Closed' : 'Open')
          return {
            key: `switch:${sw.id}`,
            name: sw.name,
            ...ioAddress(hw, sw.assignment),
            tags: sw.tags,
            detail: null,
            state,
            active: () => state() === 'Closed',
            ref: { kind: 'switch', switch: sw },
          }
        })
    case 'leds':
      return Object.values(hw.leds.by_name)
        .sort((a, b) => ledOrder(a) - ledOrder(b))
        .map((led) => {
          const { exp, index } = led.address
          return {
            key: `led:${led.name}`,
            name: ledName(led.name),
            // PCB port numbering, matching `wire_led_port`
            address: `${hexAddress(exp.board_address)}-${exp.port + 1}-${index}`,
            tags: led.tags,
            detail: null,
            state: () => null,
            active: () => false,
            ref: { kind: 'led', led },
          }
        })
    case 'io-boards':
      return hw.io_network.map((board, index) => ({
        key: `io-board:${index}`,
        // the description is the meaningful name ("Cabinet IO"); `name` is only "Board N"
        name: board.description,
        address: `node ${board.node_id}`,
        tags: [],
        detail: null,
        state: () => null,
        active: () => false,
        ref: { kind: 'io-board', board, index },
      }))
    case 'exp-boards':
      return hw.exp_network.map((board) => ({
        key: `exp-board:${board.address}:${board.breakout ?? ''}`,
        name: board.model,
        address: board.breakout === null ? hexAddress(board.address) : `${hexAddress(board.address)}.${board.breakout}`,
        tags: [],
        detail: null,
        state: () => null,
        active: () => false,
        ref: { kind: 'exp-board', board },
      }))
    case 'motors':
      // not yet part of the hardware definition
      return []
  }
}

/** Compact `board-pin` address, plus the spelled-out version */
function ioAddress(hw: Hardware, assignment: { board_idx: number; pin: number }) {
  const board = hw.io_network[assignment.board_idx]
  return {
    address: `${assignment.board_idx}-${assignment.pin}`,
    addressTip: `${board?.description ?? `Board ${assignment.board_idx}`} - pin ${assignment.pin}`,
  }
}

function driverRow(hw: Hardware, driver: Driver, triggers: string[], automatic: boolean): HardwareRow {
  // automatic drivers usually fire without software knowing, so only show a state when it did
  const state = automatic
    ? () => (driverState(driver.id) === 'Off' ? null : driverState(driver.id))
    : () => driverState(driver.id)
  return {
    key: `driver:${driver.id}`,
    name: driver.name,
    ...ioAddress(hw, driver.assignment),
    tags: driver.tags,
    detail: automatic ? `on ${triggers.join(', ')}` : null,
    state,
    active: () => driverState(driver.id) !== 'Off',
    ref: { kind: 'driver', driver },
  }
}

/** Sort key for LEDs: board, then port, then position on the port */
function ledOrder(led: LED): number {
  const { exp, index } = led.address
  return (exp.board_address * 256 + exp.port) * 65536 + index
}
