import { For, type JSX, Show } from 'solid-js'
import { describeDriverMode, describeSwitchConfig, hexAddress } from '../../lib/hardwareConfig'
import { machine } from '../../state/console'
import type { Driver } from '../../types/generated/Driver'
import type { LED } from '../../types/generated/LED'
import type { PlaneView } from '../../types/generated/PlaneView'
import type { ResolvedExpansionBoard } from '../../types/generated/ResolvedExpansionBoard'
import type { ResolvedIoBoard } from '../../types/generated/ResolvedIoBoard'
import type { Switch } from '../../types/generated/Switch'
import type { HardwareRef } from './hardware'

/** The expanded view of a hardware row */
export default function HardwareDetail(props: { item: HardwareRef }) {
  // a row's hardware never changes, so this only needs to pick once
  const item = props.item
  switch (item.kind) {
    case 'driver':
      return <DriverDetail driver={item.driver} />
    case 'switch':
      return <SwitchDetail switch={item.switch} />
    case 'led':
      return <LedDetail led={item.led} />
    case 'io-board':
      return <IoBoardDetail board={item.board} index={item.index} />
    case 'exp-board':
      return <ExpBoardDetail board={item.board} />
    case 'plane':
      return <PlaneDetail plane={item.plane} />
  }
}

function DriverDetail(props: { driver: Driver }) {
  const config = () => machine.hardware?.drivers.configs[props.driver.id]
  return (
    <Fields>
      <Field label="Name">{props.driver.name}</Field>
      <IoAssignment assignment={props.driver.assignment} />
      <Show when={config()} fallback={<Field label="Mode">Not configured</Field>}>
        {(mode) => {
          const described = describeDriverMode(mode())
          return (
            <>
              <Field label="Mode">{described.mode}</Field>
              <For each={described.fields}>{(field) => <Field label={field.label}>{field.value}</Field>}</For>
            </>
          )
        }}
      </Show>
    </Fields>
  )
}

function SwitchDetail(props: { switch: Switch }) {
  const config = () => machine.hardware?.switches.configs[props.switch.id]
  return (
    <Fields>
      <Field label="Name">{props.switch.name}</Field>
      <IoAssignment assignment={props.switch.assignment} />
      <Show when={config()} fallback={<Field label="Config">Default</Field>}>
        {(config) => <For each={describeSwitchConfig(config())}>{(field) => <Field label={field.label}>{field.value}</Field>}</For>}
      </Show>
    </Fields>
  )
}

function LedDetail(props: { led: LED }) {
  const exp = props.led.address.exp
  // an LED's breakout comes from its port (port / 4), not the board's own breakout, so match on address
  const board = () => machine.hardware?.exp_network.find((b) => b.address === exp.board_address)
  return (
    <Fields>
      <Field label="Name">{props.led.name}</Field>
      <Field label="Expansion board">
        {board()?.model ?? 'Unknown'} ({hexAddress(exp.board_address)})
      </Field>
      <Field label="Breakout">{exp.breakout ?? 'None'}</Field>
      {/* ports are 0-based in the address but numbered from 1 on the PCB and in `wire_led_port` */}
      <Field label="Port">{exp.port + 1}</Field>
    </Fields>
  )
}

function IoBoardDetail(props: { board: ResolvedIoBoard; index: number }) {
  const onBoard = <T extends { assignment: { board_idx: number; pin: number } }>(items: T[]) =>
    items.filter((item) => item.assignment.board_idx === props.index).sort((a, b) => a.assignment.pin - b.assignment.pin)
  const switches = () => onBoard(Object.values(machine.hardware?.switches.by_id ?? {}))
  const drivers = () => onBoard(Object.values(machine.hardware?.drivers.by_id ?? {}))

  return (
    <Fields>
      <Field label="Description">{props.board.description}</Field>
      <Field label="Node ID">{props.board.node_id}</Field>
      <Field label="Board revision">{props.board.board_revision}</Field>
      <Field label="Firmware">{props.board.firmware_version}</Field>
      <Field label="Switches">
        {switches().length} of {props.board.switch_count} connected
        <PinList count={props.board.switch_count} items={switches()} />
      </Field>
      <Field label="Drivers">
        {drivers().length} of {props.board.driver_count} connected
        <PinList count={props.board.driver_count} items={drivers()} />
      </Field>
    </Fields>
  )
}

function PlaneDetail(props: { plane: PlaneView }) {
  return (
    <Fields>
      <Field label="Name">{props.plane.name}</Field>
      <Field label="Size">
        {props.plane.extent[0]} × {props.plane.extent[1]} in
      </Field>
    </Fields>
  )
}

function ExpBoardDetail(props: { board: ResolvedExpansionBoard }) {
  return (
    <Fields>
      <Field label="Model">{props.board.model}</Field>
      <Field label="Address">{hexAddress(props.board.address)}</Field>
      <Show when={props.board.breakout !== null}>
        <Field label="Breakout">{props.board.breakout}</Field>
      </Show>
      <Field label="LED ports">{props.board.led_ports.length}</Field>
    </Fields>
  )
}

/** Board and pin, spelled out */
function IoAssignment(props: { assignment: { board_idx: number; pin: number } }) {
  const board = () => machine.hardware?.io_network[props.assignment.board_idx]
  return (
    <>
      <Field label="Board">
        {board()?.description ?? 'Unknown'} (board {props.assignment.board_idx})
      </Field>
      <Field label="Pin">{props.assignment.pin}</Field>
    </>
  )
}

/** Every pin on the board, so free pins are easy to spot */
function PinList(props: { count: number; items: { name: string; assignment: { pin: number } }[] }) {
  const pins = () => {
    const byPin = new Map(props.items.map((item) => [item.assignment.pin, item.name]))
    return Array.from({ length: props.count }, (_, pin) => ({ pin, name: byPin.get(pin) }))
  }
  return (
    <ul class="pin-list">
      <For each={pins()}>
        {(entry) => (
          <li classList={{ unassigned: entry.name === undefined }}>
            <span class="pin mono">{entry.pin}</span>
            {entry.name ?? '[not assigned]'}
          </li>
        )}
      </For>
    </ul>
  )
}

function Fields(props: { children: JSX.Element }) {
  return <dl class="hardware-detail">{props.children}</dl>
}

function Field(props: { label: string; children: JSX.Element }) {
  return (
    <>
      <dt>{props.label}</dt>
      <dd>{props.children}</dd>
    </>
  )
}
