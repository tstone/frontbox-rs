import { Dialog } from '@ark-ui/solid/dialog'
import { Menu } from '@ark-ui/solid/menu'
import { createContext, createSignal, For, Show, type JSX } from 'solid-js'
import { Portal } from 'solid-js/web'
import { is, type ListPlace } from './configNodes'
import type { ConfigTree } from './configTree'
import { displayNames, planeNames } from './LayerPlace'
import ScenePortal, { LayerColorContext, layerColorClass, type LayerColor } from './ScenePortal'
import { canRandomize, randomizeNode } from './randomize'
import { layerPlane, type Lens, type Scene, type SceneLayer } from './sceneModel'

/**
 * What opens a layer's menu, and which layer's name is being edited in its row on the timeline. The menu is for a layer
 * or a sound layer, by its node's id.
 */
export type LayerMenuControl = {
  open(id: string, x: number, y: number): void
  renaming(): string | null
  setRenaming(id: string | null): void
  /** The menu and its dialog, which can go anywhere as they show over everything */
  View: () => JSX.Element
}

export const LayerMenuContext = createContext<LayerMenuControl>()

type MenuAction = 'edit' | 'rename' | 'duplicate' | 'duplicate-to' | 'randomize' | 'delete'

/** What the menu needs of the editor: the scene, and typing over a text layer's words in the preview */
export type LayerMenuEditor = {
  scene: () => Scene
  /** The scene to edit, for randomizing what's under a node */
  editing: Lens<Scene>
  color: (id: string) => LayerColor
  /** Whether the layer at an index is text whose words can be typed over */
  canEditText: (index: number) => boolean
  editText: (index: number) => void
}

/**
 * The menu of a layer or anything in it, opened at the pointer by a right-click on it. A layer's has Edit for text, then
 * Rename, Duplicate, Duplicate to another plane or display, and Delete. Anything with values under it, a layer
 * included, has Randomize, and something else in a list has Delete. It selects what it's for as it opens.
 */
export function createLayerMenu(tree: ConfigTree, editor: LayerMenuEditor): LayerMenuControl {
  const [shown, setShown] = createSignal<{ id: string; x: number; y: number } | null>(null)
  const [renaming, setRenaming] = createSignal<string | null>(null)
  // the layer being copied to another plane or display, while that's chosen
  const [copying, setCopying] = createSignal<{ id: string; place: ListPlace; layer: SceneLayer } | null>(null)

  const layerIndex = (id: string) => (/^layers\/\d+$/.test(id) ? Number(id.split('/')[1]) : null)

  const act = (action: MenuAction) => {
    const id = shown()?.id
    const node = id === undefined ? undefined : tree.find(id)
    setShown(null)
    if (node && action === 'randomize') return randomizeNode(node, editor.editing)
    if (!node?.place) return
    const index = layerIndex(node.id)
    if (action === 'edit' && index !== null) editor.editText(index)
    else if (action === 'rename') setRenaming(node.id)
    else if (action === 'delete') tree.remove(node.place)
    else if (action === 'duplicate-to') {
      const layer = is(node, 'layer')
      if (layer) setCopying({ id: node.id, place: node.place, layer: layer.lens.get() })
    }
    else if (action === 'duplicate') tree.duplicate(node.place, (copy) => copyNamed(copy as { name?: string }))
  }

  /** Whether it's a layer or a sound layer, which can be renamed and copied */
  const isLayer = (id: string) => {
    const node = tree.find(id)
    return node?.kind === 'layer' || node?.kind === 'sound'
  }
  const randomizable = (id: string) => {
    const node = tree.find(id)
    return !!node && canRandomize(node)
  }
  const removable = (id: string) => {
    const place = tree.find(id)?.place
    return !!place && place.canRemove !== false && !tree.find(id)?.placement
  }

  /** Whether a layer is on a plane or a display, rather than on every light in order, so there's somewhere else to copy it to */
  const placed = (id: string) => {
    const node = tree.find(id)
    const layer = node && is(node, 'layer')
    return !!layer && (layer.lens.get().display !== undefined || layerPlane(layer.lens.get()) !== null)
  }

  return {
    open(id, x, y) {
      const node = tree.find(id)
      if (!node || !(isLayer(id) || canRandomize(node) || node.place)) return
      tree.select(id)
      setShown({ id, x, y })
    },
    renaming,
    setRenaming,
    View: () => (
      <>
        <Show when={shown()} keyed>
          {(at) => (
            <LayerColorContext.Provider value={editor.color(at.id)}>
              <Menu.Root
                open
                // at the pointer, as there's no trigger for it to be beside
                positioning={{ placement: 'bottom-start', getAnchorRect: () => ({ x: at.x, y: at.y, width: 0, height: 0 }) }}
                onOpenChange={(details) => !details.open && setShown(null)}
                onSelect={(details) => act(details.value as MenuAction)}
              >
                <ScenePortal>
                  <Menu.Positioner>
                    <Menu.Content class={`scene-dropdown-list scene-layer-menu ${layerColorClass(editor.color(at.id)) ?? ''}`}>
                      <Show when={layerIndex(at.id) !== null && editor.canEditText(layerIndex(at.id)!)}>
                        <Menu.Item value="edit">
                          <Menu.ItemText>Edit</Menu.ItemText>
                        </Menu.Item>
                      </Show>
                      <Show when={isLayer(at.id)}>
                        <Menu.Item value="rename">
                          <Menu.ItemText>Rename</Menu.ItemText>
                        </Menu.Item>
                        <Menu.Item value="duplicate">
                          <Menu.ItemText>Duplicate</Menu.ItemText>
                        </Menu.Item>
                      </Show>
                      <Show when={placed(at.id)}>
                        <Menu.Item value="duplicate-to">
                          <Menu.ItemText>Duplicate to…</Menu.ItemText>
                        </Menu.Item>
                      </Show>
                      <Show when={randomizable(at.id)}>
                        <Menu.Item value="randomize">
                          <Menu.ItemText>Randomize</Menu.ItemText>
                        </Menu.Item>
                      </Show>
                      <Show when={removable(at.id)}>
                        <Menu.Item value="delete">
                          <Menu.ItemText>Delete</Menu.ItemText>
                          <kbd>Del</kbd>
                        </Menu.Item>
                      </Show>
                    </Menu.Content>
                  </Menu.Positioner>
                </ScenePortal>
              </Menu.Root>
            </LayerColorContext.Provider>
          )}
        </Show>
        <Show when={copying()} keyed>
          {(copy) => <DuplicateToDialog scene={editor.scene()} layer={copy.layer} onChoose={(to) => (setCopying(null), tree.duplicate(copy.place, (layer) => movedTo(layer as SceneLayer, to)))} onClose={() => setCopying(null)} />}
        </Show>
      </>
    ),
  }
}

/**
 * Asks which other plane or display to copy a layer to: a display's layer goes to another display, and a plane's to
 * another plane, as the two are measured differently.
 */
function DuplicateToDialog(props: { scene: Scene; layer: SceneLayer; onChoose: (to: string) => void; onClose: () => void }) {
  const onDisplay = () => props.layer.display !== undefined
  const current = () => props.layer.display ?? layerPlane(props.layer)
  const choices = () => (onDisplay() ? displayNames(props.scene) : planeNames()).filter((name) => name !== current())
  return (
    <Dialog.Root open onOpenChange={(details) => !details.open && props.onClose()}>
      <Portal>
        <Dialog.Backdrop class="scene-dialog-backdrop" />
        <Dialog.Positioner class="scene-dialog-positioner">
          <Dialog.Content class="scene-dialog scene-duplicate-dialog">
            <header>
              <Dialog.Title>Duplicate to</Dialog.Title>
              <Dialog.CloseTrigger aria-label="Close">×</Dialog.CloseTrigger>
            </header>
            <Show
              when={choices().length > 0}
              fallback={<p class="scene-note">{onDisplay() ? 'The machine has no other display to copy this to.' : 'The machine has no other plane to copy this to.'}</p>}
            >
              <p class="scene-note">A copy of {props.layer.name ?? 'this layer'} on another {onDisplay() ? 'display' : 'plane'}, at the same place and size.</p>
              <div class="scene-surface-list" role="list">
                <For each={choices()}>
                  {(name) => (
                    <button type="button" class="scene-button" onClick={() => props.onChoose(name)}>
                      {name}
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  )
}

/** A copy of a layer moved to another display, or another plane, by name */
function movedTo(layer: SceneLayer, to: string): SceneLayer {
  return layer.display !== undefined ? { ...layer, display: to } : { ...layer, surface: { Area: to } }
}

/** A copy of a named layer is called "<name> copy", and an unnamed one stays unnamed */
const copyNamed = <T extends { name?: string }>(copy: T): T => (copy.name ? { ...copy, name: `${copy.name} copy` } : copy)
