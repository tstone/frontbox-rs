import { Dialog } from '@ark-ui/solid/dialog'
import { createEffect, createSignal, For, Show } from 'solid-js'
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
  return (
    <>
      <div class="scene-image-chosen">
        <Show when={props.lens.get()} fallback={<span class="scene-note">No image chosen</span>}>
          <img src={contentImageUrl(props.lens.get())} alt="" />
          <span title={props.lens.get()}>{props.lens.get()}</span>
        </Show>
        <button type="button" class="scene-button" onClick={() => setOpen(true)}>
          Choose
        </button>
      </div>
      <ImageDialog
        open={open()}
        chosen={props.lens.get()}
        onChoose={(name) => {
          props.lens.set(name)
          setOpen(false)
        }}
        onClose={() => setOpen(false)}
      />
    </>
  )
}

/** Lists the images in the machine's content folder to choose one, and adds one to it from a file */
export function ImageDialog(props: { open: boolean; chosen: string; onChoose: (name: string) => void; onClose: () => void }) {
  const [error, setError] = createSignal<string | null>(null)
  const [adding, setAdding] = createSignal(false)
  let list: HTMLDivElement | undefined

  // the folder can have changed since it was last read
  createEffect(() => {
    if (!props.open) return
    setError(null)
    refreshContentImages()
  })

  const add = async (input: HTMLInputElement) => {
    const file = input.files?.[0]
    input.value = ''
    if (!file) return
    setAdding(true)
    setError(null)
    try {
      props.onChoose(await addContentImage(file))
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
      open={props.open}
      onOpenChange={(details) => !details.open && props.onClose()}
      // opens at the chosen image, which could be a long way down. Taking focus doesn't scroll to it here.
      initialFocusEl={() => {
        const selected = list?.querySelector<HTMLElement>('[aria-selected="true"]') ?? null
        selected?.scrollIntoView({ block: 'center' })
        return selected
      }}
    >
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
                  <button type="button" class="scene-image" role="option" title={name} aria-selected={props.chosen === name} onClick={() => props.onChoose(name)}>
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
