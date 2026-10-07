import { Tabs } from '@ark-ui/solid/tabs'
import { createSignal, For, onCleanup } from 'solid-js'
import { Dynamic } from 'solid-js/web'
import { machine, startConsole } from './state/console'
import type { ConnectionStatus } from './state/socket'
import { tabs } from './tabs'
import './App.css'

const connectionLabels: Record<ConnectionStatus, string> = {
  connecting: 'Connecting',
  connected: 'Connected',
  disconnected: 'Disconnected',
}

function App() {
  onCleanup(startConsole())

  // the active tab lives in the URL hash so a reload lands on the same tab
  const tabFromHash = () => (tabs.find((t) => `#${t.id}` === location.hash) ?? tabs[0]).id
  const [active, setActive] = createSignal(tabFromHash())
  const onHashChange = () => setActive(tabFromHash())
  window.addEventListener('hashchange', onHashChange)
  onCleanup(() => window.removeEventListener('hashchange', onHashChange))

  return (
    <Tabs.Root
      class="app"
      value={active()}
      onValueChange={(details) => (location.hash = details.value)}
      // tabs mount on first visit and then stay mounted, so their UI state survives switching
      lazyMount
    >
      <header class="app-header">
        <h1>Frontbox Pin Console</h1>
        <Tabs.List>
          <For each={tabs}>{(tab) => <Tabs.Trigger value={tab.id}>{tab.label}</Tabs.Trigger>}</For>
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
    </Tabs.Root>
  )
}

export default App
