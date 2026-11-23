import { createSignal, onCleanup, Show, type JSX } from 'solid-js'
import { Portal } from 'solid-js/web'

const [takenBy, setTakenBy] = createSignal<string[]>([])
const [slot, setSlot] = createSignal<HTMLElement>()

/** Whether a tab has put its own controls in the app's header, in place of the tabs */
export const headerTakenBy = (tab: string) => takenBy().includes(tab)

/** Where in the app's header the controls go */
export const setHeaderSlot = setSlot

/**
 * Puts its children in the app's header in place of the tabs, for as long as it's up. The app shows them only while
 * the tab is the one showing.
 */
export function HeaderTakeover(props: { tab: string; children: JSX.Element }) {
  const tab = props.tab
  setTakenBy((tabs) => [...tabs, tab])
  onCleanup(() =>
    setTakenBy((tabs) => {
      const index = tabs.indexOf(tab)
      return index === -1 ? tabs : [...tabs.slice(0, index), ...tabs.slice(index + 1)]
    }),
  )
  return (
    <Show when={slot()}>
      {(mount) => (
        <Portal mount={mount()} ref={(wrapper: HTMLDivElement) => (wrapper.className = 'app-header-controls')}>
          {props.children}
        </Portal>
      )}
    </Show>
  )
}
