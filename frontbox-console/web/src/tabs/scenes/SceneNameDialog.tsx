import { Dialog } from '@ark-ui/solid/dialog'
import { createEffect, createSignal, on, Show, splitProps, type JSX } from 'solid-js'
import { Portal } from 'solid-js/web'
import { sceneName } from '../../state/contentScenes'

/** Asks for a name for a scene. It stays open with the error if `onName` throws, such as the name being taken. */
export default function SceneNameDialog(props: NameProps & { open: boolean; onClose: () => void }) {
  const [typed, setTyped] = createSignal('')
  const [error, setError] = createSignal<string | null>(null)
  const [acting, setActing] = createSignal(false)
  let input: HTMLInputElement | undefined

  createEffect(
    on(
      () => props.open,
      (open) => {
        setTyped(open ? (props.name ?? '') : '')
        setError(null)
      },
    ),
  )

  const submit = async (e: SubmitEvent) => {
    e.preventDefault()
    if (typed().trim() === '' || acting()) return
    setActing(true)
    setError(null)
    try {
      await props.onName(sceneName(typed()))
      props.onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setActing(false)
    }
  }

  return (
    <Dialog.Root lazyMount unmountOnExit open={props.open} onOpenChange={(details) => !details.open && props.onClose()} initialFocusEl={() => input ?? null}>
      <Portal>
        <Dialog.Backdrop class="scene-dialog-backdrop" />
        <Dialog.Positioner class="scene-dialog-positioner">
          <Dialog.Content class="scene-dialog scene-dialog-small">
            <header>
              <Dialog.Title>{props.title}</Dialog.Title>
              <Dialog.CloseTrigger aria-label="Close">×</Dialog.CloseTrigger>
            </header>
            <form onSubmit={submit}>
              {props.children}
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
                <button type="submit" class="scene-button" disabled={typed().trim() === '' || acting()}>
                  {acting() ? props.acting : props.action}
                </button>
              </footer>
            </form>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  )
}

/** A button that opens a `SceneNameDialog` */
export function SceneNameButton(props: NameProps & { label: string; disabled?: boolean }) {
  const [own, dialog] = splitProps(props, ['label', 'disabled'])
  const [open, setOpen] = createSignal(false)

  return (
    <>
      <button type="button" class="scene-button" aria-haspopup="dialog" disabled={own.disabled} onClick={() => setOpen(true)}>
        {own.label}
      </button>
      <SceneNameDialog {...dialog} open={open()} onClose={() => setOpen(false)} />
    </>
  )
}

type NameProps = {
  title: string
  action: string
  acting: string
  /** What the name starts out as */
  name?: string
  onName: (name: string) => Promise<void>
  /** Anything else to ask, above the name */
  children?: JSX.Element
}
