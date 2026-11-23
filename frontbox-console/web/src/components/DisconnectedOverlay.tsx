import { createEffect, createSignal, onCleanup, Show } from 'solid-js'
import { machine } from '../state/console'

/**
 * Covers the console while it can't reach the machine, so that what doesn't respond is plainly why. It comes up once
 * the connection has been out for a moment, so that one quickly made again, as when the page loads, doesn't flash it.
 * It goes as soon as the connection is back.
 */
export default function DisconnectedOverlay() {
  const [shown, setShown] = createSignal(false)
  createEffect(() => {
    if (machine.connection === 'connected') return setShown(false)
    const timer = setTimeout(() => setShown(true), SHOW_AFTER_MS)
    onCleanup(() => clearTimeout(timer))
  })

  return (
    <Show when={shown()}>
      <div class="disconnected" role="alertdialog" aria-modal="true" aria-labelledby="disconnected-title" aria-describedby="disconnected-text">
        <div class="disconnected-card">
          <span class="disconnected-dot" aria-hidden="true" />
          <h2 id="disconnected-title">Disconnected</h2>
          <p id="disconnected-text">The console can't reach the machine, so nothing here will respond. It's trying to reconnect.</p>
        </div>
      </div>
    </Show>
  )
}

/** How long the connection has to be out before the overlay comes up */
const SHOW_AFTER_MS = 1000
