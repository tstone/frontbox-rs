import { Dialog } from '@ark-ui/solid/dialog'
import { createEffect, createMemo, createSignal, For, Show } from 'solid-js'
import { Portal } from 'solid-js/web'
import { contentSounds, contentSoundsFailed, playContentSound, refreshContentSounds } from '../../state/contentSounds'
import type { Lens } from './sceneModel'

/**
 * The chosen sound, with a button to try it out on the machine, and a way to choose from the machine's content folder by
 * name. Each sound in the list can be tried out too, at `volume`.
 */
export function SoundPicker(props: { lens: Lens<string>; volume: number }) {
  const [open, setOpen] = createSignal(false)
  return (
    <>
      <div class="scene-sound">
        <Show when={props.lens.get()} fallback={<span class="scene-note">No sound chosen</span>}>
          <span class="scene-sound-chosen" title={props.lens.get()}>
            {props.lens.get()}
          </span>
          <button type="button" class="scene-button" aria-label={`Play ${props.lens.get()} on the machine`} onClick={() => playContentSound(props.lens.get(), props.volume)}>
            ▶
          </button>
        </Show>
        <button type="button" class="scene-button" onClick={() => setOpen(true)}>
          Choose
        </button>
      </div>
      <SoundDialog
        open={open()}
        chosen={props.lens.get()}
        volume={props.volume}
        onChoose={(name) => {
          props.lens.set(name)
          setOpen(false)
        }}
        onClose={() => setOpen(false)}
      />
    </>
  )
}

/** Lists the sounds in the machine's content folder to choose one, filtered by name, each of which can be tried out at `volume` */
export function SoundDialog(props: { open: boolean; chosen: string; volume: number; onChoose: (name: string) => void; onClose: () => void }) {
  const [filter, setFilter] = createSignal('')
  let list: HTMLDivElement | undefined
  let search: HTMLInputElement | undefined

  // the folder can have changed since it was last read
  createEffect(() => {
    if (!props.open) return
    setFilter('')
    refreshContentSounds()
  })

  // one that's gone from the folder is still listed, as the scene still names it
  const names = () => (props.chosen === '' || contentSounds().includes(props.chosen) ? contentSounds() : [props.chosen, ...contentSounds()])
  // every word typed has to be somewhere in the sound's path, in any order
  const matching = createMemo(() => {
    const words = filter().trim().toLowerCase().split(/\s+/).filter(Boolean)
    return names().filter((name) => words.every((word) => name.toLowerCase().includes(word)))
  })

  const play = (name: string) => playContentSound(name, props.volume)

  return (
    <Dialog.Root
      lazyMount
      unmountOnExit
      open={props.open}
      onOpenChange={(details) => !details.open && props.onClose()}
      // typing goes straight to the search, and the list starts at the chosen sound, which could be a long way down
      initialFocusEl={() => {
        list?.querySelector<HTMLElement>('[aria-selected="true"]')?.scrollIntoView({ block: 'center' })
        return search ?? null
      }}
    >
      <Portal>
        <Dialog.Backdrop class="scene-dialog-backdrop" />
        <Dialog.Positioner class="scene-dialog-positioner">
          <Dialog.Content class="scene-dialog scene-dialog-sounds">
            <header>
              <Dialog.Title>Choose a sound</Dialog.Title>
              <Dialog.CloseTrigger aria-label="Close">×</Dialog.CloseTrigger>
            </header>
            <input
              ref={search}
              class="filter-input"
              type="search"
              placeholder="Filter by name"
              aria-label="Filter sounds by name"
              autocomplete="off"
              spellcheck={false}
              value={filter()}
              onInput={(e) => setFilter(e.currentTarget.value)}
              // Enter takes the only match, so a sound can be chosen without leaving the keyboard
              onKeyDown={(e) => {
                if (e.key === 'Enter' && matching().length === 1) props.onChoose(matching()[0])
              }}
            />
            <Show when={contentSoundsFailed()}>
              <p class="scene-note scene-note-warn">Couldn't read the machine's content folder</p>
            </Show>
            <Show when={!contentSoundsFailed() && names().length === 0}>
              <p class="scene-note">No sounds in the machine's content folder</p>
            </Show>
            <Show when={names().length > 0 && matching().length === 0}>
              <p class="scene-note">Nothing matches "{filter().trim()}"</p>
            </Show>
            <div class="scene-sound-list" role="listbox" aria-label="Sound" ref={list}>
              <For each={matching()}>
                {(name) => (
                  <div class="scene-sound-option" aria-selected={props.chosen === name}>
                    <button type="button" class="scene-button" aria-label={`Play ${name} on the machine`} onClick={() => play(name)}>
                      ▶
                    </button>
                    <button type="button" class="scene-sound-name" role="option" title={name} aria-selected={props.chosen === name} onClick={() => props.onChoose(name)}>
                      {name}
                    </button>
                  </div>
                )}
              </For>
            </div>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  )
}
