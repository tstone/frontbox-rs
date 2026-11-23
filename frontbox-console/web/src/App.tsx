import { Tabs } from '@ark-ui/solid/tabs'
import { createSignal, For, onCleanup } from 'solid-js'
import { Dynamic } from 'solid-js/web'
import { machine, startConsole } from './state/console'
import type { ConnectionStatus } from './state/socket'
import { headerTakenBy, setHeaderSlot } from './components/HeaderTakeover'
import DisconnectedOverlay from './components/DisconnectedOverlay'
import HistoryControls from './components/HistoryControls'
import { tabGroups, tabs } from './tabs'
import './App.css'

const connectionLabels: Record<ConnectionStatus, string> = {
  connecting: 'Connecting',
  connected: 'Connected',
  disconnected: 'Disconnected',
}

function App() {
  onCleanup(startConsole())

  // the active tab lives in the URL hash so a reload lands on the same tab. A tab can add to it after a slash, such as
  // the scene that's open, which coming back to the tab returns to.
  const tabFromHash = () => (tabs.find((t) => location.hash === `#${t.id}` || location.hash.startsWith(`#${t.id}/`)) ?? tabs[0]).id
  const [active, setActive] = createSignal(tabFromHash())
  const lastHash: Record<string, string> = {}
  const onHashChange = () => setActive(tabFromHash())
  window.addEventListener('hashchange', onHashChange)
  onCleanup(() => window.removeEventListener('hashchange', onHashChange))
  const choose = (tab: string) => {
    lastHash[active()] = location.hash
    location.hash = lastHash[tab] ?? tab
  }

  return (
    <Tabs.Root
      class="app"
      value={active()}
      onValueChange={(details) => choose(details.value)}
      // tabs mount on first visit and then stay mounted, so their UI state survives switching
      lazyMount
    >
      <header class="app-header" classList={{ 'app-header-taken': headerTakenBy(active()) }}>
        <h1>Frontbox</h1>
        {/* a tab's own controls, such as an open scene's, which stand in for the tabs while it's showing */}
        <div class="app-header-slot" ref={setHeaderSlot} />
        <Tabs.List>
          {/* grouped by what they show, with the history controls in with the live tabs whose history they clear */}
          <For each={tabGroups}>
            {(group) => (
              <div class="tab-group" data-group={group.group}>
                <For each={group.tabs}>{(tab) => <Tabs.Trigger value={tab.id}>{tab.label}</Tabs.Trigger>}</For>
                {group.group === 'live' && <HistoryControls />}
              </div>
            )}
          </For>
          <Tabs.Indicator />
        </Tabs.List>
        <span class="connection" data-status={machine.connection}>
          <span class="dot" />
          {connectionLabels[machine.connection]}
        </span>
      </header>
      <For each={tabs}>
        {(tab) => (
          <Tabs.Content value={tab.id}>
            <Dynamic component={tab.component} />
          </Tabs.Content>
        )}
      </For>
      <DisconnectedOverlay />
    </Tabs.Root>
  )
}

export default App
