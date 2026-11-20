import type { Component } from 'solid-js'
import MachineTab from './machine/MachineTab'
import SystemsTab from './systems/SystemsTab'
import EventsTab from './events/EventsTab'
import ConfigTab from './config/ConfigTab'
import MetricsTab from './metrics/MetricsTab'
import ScenesTab from './scenes/ScenesTab'

/**
 * What a tab shows, which the tab bar groups them by:
 * - `live`: the machine as it is now, with the recent history that led to it. Clearing the history clears theirs.
 * - `settings`: values in use now, which can be changed but have no history.
 * - `history`: stored over time, outside the console.
 * - `authoring`: making content for the machine.
 */
export type TabGroup = 'live' | 'settings' | 'history' | 'authoring'

export type TabDefinition = {
  /** Used in the URL hash, so keep it stable */
  id: string
  label: string
  group: TabGroup
  component: Component
}

/** Order here is the order in the tab bar, and tabs in a group go together. The first tab is the default. */
export const tabs: TabDefinition[] = [
  { id: 'machine', label: 'Machine', group: 'live', component: MachineTab },
  { id: 'systems', label: 'Systems', group: 'live', component: SystemsTab },
  { id: 'events', label: 'Events', group: 'live', component: EventsTab },
  { id: 'config', label: 'Config', group: 'settings', component: ConfigTab },
  { id: 'metrics', label: 'Metrics', group: 'history', component: MetricsTab },
  { id: 'scenes', label: 'Scenes', group: 'authoring', component: ScenesTab },
]

/** The tabs in each group, in order */
export const tabGroups: { group: TabGroup; tabs: TabDefinition[] }[] = tabs.reduce<
  { group: TabGroup; tabs: TabDefinition[] }[]
>((groups, tab) => {
  const last = groups.at(-1)
  if (last?.group === tab.group) last.tabs.push(tab)
  else groups.push({ group: tab.group, tabs: [tab] })
  return groups
}, [])
