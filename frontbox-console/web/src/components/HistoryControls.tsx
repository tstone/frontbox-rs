import { machine, sendToMachine } from '../state/console'
import { Download, Eraser } from './icons'

/**
 * Controls for the history the live tabs (Machine, Systems, Events) show: clearing it for a fresh look, clearing it
 * whenever a game starts, and exporting everything the console has. Sits in the tab bar with those tabs, to show
 * which tabs it affects.
 */
export default function HistoryControls() {
  return (
    <div class="history-controls">
      <button
        type="button"
        title="Clear the history in Machine, Systems, and Events: the event log, systems' activity, and despawned systems. What's current stays."
        onClick={() => sendToMachine({ type: 'ClearHistory' })}
      >
        <Eraser size={14} />
        Clear
      </button>
      <label title="Clear the history whenever a game starts">
        <input
          type="checkbox"
          checked={machine.clear_on_game_start}
          onChange={(event) => sendToMachine({ type: 'SetClearOnGameStart', clear: event.currentTarget.checked })}
        />
        At game start
      </label>
      <a href="/export" download="" title="Download everything the console has as JSON, to look over away from the machine">
        <Download size={14} />
        Export
      </a>
    </div>
  )
}
