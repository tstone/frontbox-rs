import { Dialog } from '@ark-ui/solid/dialog'
import { createEffect, createSignal, For, Show } from 'solid-js'
import { Portal } from 'solid-js/web'
import { machine } from '../../state/console'
import { addContentScene, contentScenes, contentScenesFailed, readContentScene, refreshContentScenes } from '../../state/contentScenes'
import SceneEditor from './SceneEditor'
import './ScenesTab.css'

const STARTING_SCENE = `(
  version: 10,
  viewport: Area,
  layers: [
    (fill: (base: Gradient(stops: [
      (at: 0.0, color: Fixed(255, 0, 0, 255)),
      (at: 1.0, color: Fixed(0, 0, 255, 255)),
    ]))),
  ],
)
`

const message = (err: unknown) => (err instanceof Error ? err.message : String(err))

/** A scene's name is its path in the content folder, and the `.ron` needn't be typed */
const sceneName = (typed: string) => {
  const name = typed.trim()
  return name.toLowerCase().endsWith('.ron') ? name : `${name}.ron`
}

/** The scenes in the machine's content folder, and the editor when one is open */
export default function ScenesTab() {
  const [open, setOpen] = createSignal<OpenScene | null>(null)

  return (
    <Show when={open()} keyed fallback={<SceneList onOpen={setOpen} />}>
      {(scene) => <SceneEditor name={scene.name} source={scene.source} onClose={() => setOpen(null)} />}
    </Show>
  )
}

/** A scene, with its file as it was opened */
type OpenScene = { name: string; source: string }

function SceneList(props: { onOpen: (scene: OpenScene) => void }) {
  const [error, setError] = createSignal<string | null>(null)

  // runs again on a reconnect, as the machine may not have been up the first time
  createEffect(() => {
    if (machine.connection === 'connected') refreshContentScenes()
  })

  const open = async (name: string) => {
    setError(null)
    try {
      props.onOpen({ name, source: await readContentScene(name) })
    } catch (err) {
      setError(message(err))
    }
  }

  return (
    <div class="scene-list">
      <header>
        <h2>Scenes</h2>
        <NewScene onAdded={props.onOpen} />
      </header>
      <Show when={error()}>{(text) => <p class="scene-note scene-note-warn">{text()}</p>}</Show>
      <Show when={contentScenesFailed()}>
        <p class="scene-note scene-note-warn">Couldn't read the machine's content folder</p>
      </Show>
      <Show when={!contentScenesFailed() && contentScenes().length === 0}>
        <p class="scene-note">No scenes in the machine's content folder</p>
      </Show>
      <ul>
        <For each={contentScenes()}>
          {(name) => (
            <li>
              <button type="button" title={name} onClick={() => open(name)}>
                {name}
              </button>
            </li>
          )}
        </For>
      </ul>
    </div>
  )
}

/** Asks for a name and adds a scene by it, so that what's being edited always has a file to save to */
function NewScene(props: { onAdded: (scene: OpenScene) => void }) {
  const [open, setOpen] = createSignal(false)
  const [typed, setTyped] = createSignal('')
  const [error, setError] = createSignal<string | null>(null)
  const [adding, setAdding] = createSignal(false)
  let input: HTMLInputElement | undefined

  const add = async (e: SubmitEvent) => {
    e.preventDefault()
    if (typed().trim() === '' || adding()) return
    setAdding(true)
    setError(null)
    const name = sceneName(typed())
    try {
      const source = await addContentScene(name, { Ron: STARTING_SCENE })
      setOpen(false)
      props.onAdded({ name, source })
    } catch (err) {
      setError(message(err))
    } finally {
      setAdding(false)
    }
  }

  return (
    <Dialog.Root
      lazyMount
      unmountOnExit
      open={open()}
      onOpenChange={(details) => {
        setOpen(details.open)
        setTyped('')
        setError(null)
      }}
      initialFocusEl={() => input ?? null}
    >
      <Dialog.Trigger class="scene-button">+ New</Dialog.Trigger>
      <Portal>
        <Dialog.Backdrop class="scene-dialog-backdrop" />
        <Dialog.Positioner class="scene-dialog-positioner">
          <Dialog.Content class="scene-dialog scene-dialog-small">
            <header>
              <Dialog.Title>New scene</Dialog.Title>
              <Dialog.CloseTrigger aria-label="Close">×</Dialog.CloseTrigger>
            </header>
            <form onSubmit={add}>
              <input
                ref={input}
                type="text"
                aria-label="Name"
                placeholder="attract, or led/attract to put it in a folder"
                spellcheck={false}
                value={typed()}
                onInput={(e) => setTyped(e.currentTarget.value)}
              />
              <Show when={error()}>{(text) => <p class="scene-note scene-note-warn">{text()}</p>}</Show>
              <footer>
                <button type="submit" class="scene-button" disabled={typed().trim() === '' || adding()}>
                  {adding() ? 'Creating…' : 'Create'}
                </button>
              </footer>
            </form>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  )
}
