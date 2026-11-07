import { Dialog } from '@ark-ui/solid/dialog'
import { createSignal, For, Show } from 'solid-js'
import { Portal } from 'solid-js/web'
import {
  addContentImage,
  contentImages,
  contentImagesFailed,
  contentImageUrl,
  refreshContentImages,
} from '../../state/contentImages'
import type { Lens } from './sceneModel'

/** The chosen image, and a way to choose from the machine's content folder or add to it */
export function ImagePicker(props: { lens: Lens<string> }) {
  const [open, setOpen] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)
  const [adding, setAdding] = createSignal(false)
  let list: HTMLDivElement | undefined

  const choose = (name: string) => {
    props.lens.set(name)
    setOpen(false)
  }

  const add = async (input: HTMLInputElement) => {
    const file = input.files?.[0]
    input.value = ''
    if (!file) return
    setAdding(true)
    setError(null)
    try {
      choose(await addContentImage(file))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
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
        setError(null)
        if (details.open) refreshContentImages()
      }}
      // opens at the chosen image, which could be a long way down. Taking focus doesn't scroll to it here.
      initialFocusEl={() => {
        const selected = list?.querySelector<HTMLElement>('[aria-selected="true"]') ?? null
        selected?.scrollIntoView({ block: 'center' })
        return selected
      }}
    >
      <div class="scene-image-chosen">
        <Show when={props.lens.get()} fallback={<span class="scene-note">No image chosen</span>}>
          <img src={contentImageUrl(props.lens.get())} alt="" />
          <span title={props.lens.get()}>{props.lens.get()}</span>
        </Show>
        <Dialog.Trigger class="scene-button">Choose</Dialog.Trigger>
      </div>
      <Portal>
        <Dialog.Backdrop class="scene-dialog-backdrop" />
        <Dialog.Positioner class="scene-dialog-positioner">
          <Dialog.Content class="scene-dialog">
            <header>
              <Dialog.Title>Choose an image</Dialog.Title>
              <Dialog.CloseTrigger aria-label="Close">×</Dialog.CloseTrigger>
            </header>
            <Show when={contentImagesFailed()}>
              <p class="scene-note scene-note-warn">Couldn't read the machine's content folder</p>
            </Show>
            <Show when={!contentImagesFailed() && contentImages().length === 0}>
              <p class="scene-note">No images in the machine's content folder</p>
            </Show>
            <div class="scene-image-list" role="listbox" aria-label="Image" ref={list}>
              <For each={contentImages()}>
                {(name) => (
                  <button
                    type="button"
                    class="scene-image"
                    role="option"
                    title={name}
                    aria-selected={props.lens.get() === name}
                    onClick={() => choose(name)}
                  >
                    <img src={contentImageUrl(name)} alt="" loading="lazy" />
                    <span>{name}</span>
                  </button>
                )}
              </For>
            </div>
            <footer>
              <Show when={error()}>
                <p class="scene-note scene-note-warn">{error()}</p>
              </Show>
              <label class="scene-button">
                {adding() ? 'Adding…' : 'Add image…'}
                <input type="file" accept="image/*" disabled={adding()} onChange={(e) => add(e.currentTarget)} />
              </label>
            </footer>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  )
}
