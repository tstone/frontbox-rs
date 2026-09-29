import { createSignal, For, onCleanup } from 'solid-js'
import { Dynamic } from 'solid-js/web'
import { machine, startConsole } from './state/console'
import { tabs } from './tabs'
import './App.css'

function App() {
  onCleanup(startConsole())

  // the active tab lives in the URL hash so a reload lands on the same tab
  const tabFromHash = () => tabs.find((t) => `#${t.id}` === location.hash) ?? tabs[0]
  const [active, setActive] = createSignal(tabFromHash())
  const onHashChange = () => setActive(tabFromHash())
  window.addEventListener('hashchange', onHashChange)
  onCleanup(() => window.removeEventListener('hashchange', onHashChange))

  return (
    <div class="app">
      <header>
        <nav>
          <For each={tabs}>
            {(tab) => (
              <a href={`#${tab.id}`} classList={{ active: active() === tab }}>
                {tab.label}
              </a>
            )}
          </For>
        </nav>
        <span class={`connection ${machine.connection}`}>{machine.connection}</span>
      </header>
      <main>
        <Dynamic component={active().component} />
      </main>
    </div>
  )
}

export default App
