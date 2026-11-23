import { Dialog } from '@ark-ui/solid/dialog'
import { For } from 'solid-js'
import { Portal } from 'solid-js/web'
import * as icons from '../../components/icons'

/** Ctrl, or Command on a Mac */
export const MOD = /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘' : 'Ctrl'

/** The scene editor's keyboard shortcuts, by what they're for */
const SHORTCUTS: { title: string; items: { keys: string[]; does: string }[] }[] = [
  {
    title: 'Playing',
    items: [
      { keys: ['Space'], does: 'Play or stop the scene' },
      { keys: ['K'], does: 'Add a key at the playhead to the selected property' },
    ],
  },
  {
    title: 'Editing',
    items: [
      { keys: [MOD, 'S'], does: 'Save' },
      { keys: [MOD, 'Z'], does: 'Undo' },
      { keys: [MOD, 'Shift', 'Z'], does: 'Redo, as does ' + MOD + '+Y' },
      { keys: ['Delete'], does: 'Remove what’s selected' },
      { keys: ['Esc'], does: 'Select nothing' },
      { keys: ['Double-click a slider’s knob'], does: 'Put it back to its default' },
    ],
  },
  {
    title: 'Drawing',
    items: [
      { keys: ['Shift'], does: 'While drawing a box, keep it square' },
      { keys: ['Backspace'], does: 'Take back a polygon’s last corner' },
      { keys: ['Esc'], does: 'Stop drawing, which a right-click does too' },
      { keys: ['Shift'], does: 'While dragging a side, size both sides' },
      { keys: ['Shift'], does: 'While turning, turn in steps of 15°' },
    ],
  },
  {
    title: 'Text',
    items: [
      { keys: ['Enter'], does: 'Keep the text' },
      { keys: ['Shift', 'Enter'], does: 'Start a new line' },
      { keys: ['Esc'], does: 'Put back what was there' },
    ],
  },
  {
    title: 'Layers',
    items: [
      { keys: ['↑', '↓'], does: 'Go between rows' },
      { keys: ['→', '←'], does: 'Open or close a row' },
    ],
  },
]

/** A mouse action, such as a right-click, written out rather than shown as a key */
const isMouse = (key: string) => /click|drag/i.test(key)

/** A button in the header that opens a list of the editor's shortcuts */
export default function ShortcutsButton() {
  return (
    <Dialog.Root lazyMount unmountOnExit>
      <Dialog.Trigger class="scene-icon-button scene-shortcuts-button" aria-label="Keyboard shortcuts" title="Keyboard shortcuts">
        <icons.Info aria-hidden="true" />
      </Dialog.Trigger>
      <Portal>
        <Dialog.Backdrop class="scene-dialog-backdrop" />
        <Dialog.Positioner class="scene-dialog-positioner">
          <Dialog.Content class="scene-dialog scene-shortcuts-dialog">
            <header>
              <Dialog.Title>Keyboard shortcuts</Dialog.Title>
              <Dialog.CloseTrigger aria-label="Close">×</Dialog.CloseTrigger>
            </header>
            <div class="scene-shortcuts">
              <For each={SHORTCUTS}>
                {(group) => (
                  <section>
                    <h4>{group.title}</h4>
                    <dl>
                      <For each={group.items}>
                        {(item) => (
                          <>
                            <dt>
                              <For each={item.keys}>{(key) => (isMouse(key) ? <span class="scene-shortcut-mouse">{key}</span> : <kbd>{key}</kbd>)}</For>
                            </dt>
                            <dd>{item.does}</dd>
                          </>
                        )}
                      </For>
                    </dl>
                  </section>
                )}
              </For>
            </div>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  )
}
