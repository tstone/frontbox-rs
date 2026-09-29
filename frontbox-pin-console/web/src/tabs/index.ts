import type { Component } from 'solid-js'
import MachineTab from './machine/MachineTab'
import SystemsTab from './systems/SystemsTab'
import LogTab from './log/LogTab'

export type TabDefinition = {
  /** Used in the URL hash, so keep it stable */
  id: string
  label: string
  component: Component
}

/** Order here is the order in the tab bar. The first tab is the default. */
export const tabs: TabDefinition[] = [
  { id: 'machine', label: 'Machine', component: MachineTab },
  { id: 'systems', label: 'Systems', component: SystemsTab },
  { id: 'log', label: 'Log', component: LogTab },
]
