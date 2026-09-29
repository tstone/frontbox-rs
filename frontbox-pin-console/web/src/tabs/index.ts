import type { Component } from 'solid-js'
import SystemsTab from './SystemsTab'
import LogTab from './LogTab'

export type TabDefinition = {
  /** Used in the URL hash, so keep it stable */
  id: string
  label: string
  component: Component
}

/** Order here is the order in the tab bar. The first tab is the default. */
export const tabs: TabDefinition[] = [
  { id: 'systems', label: 'Systems', component: SystemsTab },
  { id: 'log', label: 'Log', component: LogTab },
]
