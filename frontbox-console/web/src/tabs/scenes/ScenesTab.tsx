import { createTreeCollection, TreeView } from '@ark-ui/solid/tree-view'
import { createEffect, createMemo, createSignal, For, Show } from 'solid-js'
import { machine } from '../../state/console'
import { addContentScene, contentScenes, contentScenesFailed, deleteContentScene, readContentScene, refreshContentScenes } from '../../state/contentScenes'
import ConfirmDialog from './ConfirmDialog'
import SceneEditor from './SceneEditor'
import SceneNameDialog, { SceneNameButton } from './SceneNameDialog'
import './ScenesTab.css'

const STARTING_SCENE = `(
  version: 11,
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

// Tracks what's collapsed rather than what's expanded, so new folders start open. Outlives the list, which is gone while a scene is open.
const [collapsed, setCollapsed] = createSignal<string[]>([])

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
  const [renaming, setRenaming] = createSignal<string | null>(null)
  const [deleting, setDeleting] = createSignal<string | null>(null)

  const tree = createMemo(() => sceneTree(contentScenes()))
  const collection = createMemo(() =>
    createTreeCollection<SceneNode>({
      nodeToValue: (node) => node.id,
      nodeToString: (node) => node.label,
      rootNode: { id: '/', label: '', children: tree() },
    }),
  )
  const folders = () => tree().flatMap(folderIds)

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

  /** Copies the file as it's written, comments and all. The list is scanned again either way, as a failed delete leaves both. */
  const rename = async (name: string) => {
    const old = renaming()
    if (old === null || name === old) return
    try {
      await addContentScene(name, { Ron: await readContentScene(old) })
      await deleteContentScene(old)
    } finally {
      await refreshContentScenes()
    }
  }

  const remove = async () => {
    const name = deleting()
    setDeleting(null)
    if (name === null) return
    setError(null)
    try {
      await deleteContentScene(name)
    } catch (err) {
      setError(message(err))
    }
    await refreshContentScenes()
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
      <Show when={contentScenes().length > 0}>
        {/* Nothing stays selected, so that choosing a scene is what opens it, by mouse or by keyboard, every time */}
        <TreeView.Root
          class="scene-tree"
          collection={collection()}
          selectionMode="single"
          selectedValue={[]}
          onSelectionChange={(details) => {
            const name = details.selectedValue[0]
            if (name !== undefined && contentScenes().includes(name)) open(name)
          }}
          expandedValue={folders().filter((id) => !collapsed().includes(id))}
          onExpandedChange={(details) => setCollapsed(folders().filter((id) => !details.expandedValue.includes(id)))}
        >
          <TreeView.Tree>
            <For each={tree()}>{(node, index) => <SceneTreeNode node={node} indexPath={[index()]} onRename={setRenaming} onDelete={setDeleting} />}</For>
          </TreeView.Tree>
        </TreeView.Root>
      </Show>
      {/* These stay outside of the tree, as events from a portal still bubble through it and would reach the row */}
      <SceneNameDialog
        open={renaming() !== null}
        title="Rename scene"
        action="Rename"
        acting="Renaming…"
        name={renaming() ?? ''}
        onClose={() => setRenaming(null)}
        onName={rename}
      />
      <ConfirmDialog
        open={deleting() !== null}
        title="Delete this scene?"
        description={`${deleting()} will be removed from the machine's content folder. This can't be undone.`}
        cancel="Cancel"
        action="Delete"
        onCancel={() => setDeleting(null)}
        onConfirm={remove}
      />
    </div>
  )
}

/** A folder and what's in it, or a scene, whose buttons show while it's hovered */
function SceneTreeNode(props: { node: SceneNode; indexPath: number[]; onRename: (name: string) => void; onDelete: (name: string) => void }) {
  return (
    <TreeView.NodeProvider node={props.node} indexPath={props.indexPath}>
      <Show
        when={props.node.children}
        fallback={
          <TreeView.Item title={props.node.id}>
            <TreeView.ItemText>{props.node.label}</TreeView.ItemText>
            {/* A row takes any click or key from inside it as choosing the scene */}
            <span class="scene-tree-actions" onClick={stopPropagation} onKeyDown={stopPropagation}>
              <button type="button" class="scene-button" title="Rename" aria-label={`Rename ${props.node.id}`} onClick={() => props.onRename(props.node.id)}>
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M11 2.5 13.5 5l-8 8H3v-2.5zM9.25 4.25l2.5 2.5" />
                </svg>
              </button>
              <button type="button" class="scene-button" title="Delete" aria-label={`Delete ${props.node.id}`} onClick={() => props.onDelete(props.node.id)}>
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M2.5 4.5h11M6 4.5v-2h4v2M4 4.5l.75 9h6.5l.75-9M6.75 7v4M9.25 7v4" />
                </svg>
              </button>
            </span>
          </TreeView.Item>
        }
      >
        {(children) => (
          <TreeView.Branch>
            <TreeView.BranchControl>
              <TreeView.BranchIndicator>›</TreeView.BranchIndicator>
              <TreeView.BranchText>{props.node.label}</TreeView.BranchText>
            </TreeView.BranchControl>
            <TreeView.BranchContent>
              <TreeView.BranchIndentGuide />
              <For each={children()}>
                {(child, index) => <SceneTreeNode node={child} indexPath={[...props.indexPath, index()]} onRename={props.onRename} onDelete={props.onDelete} />}
              </For>
            </TreeView.BranchContent>
          </TreeView.Branch>
        )}
      </Show>
    </TreeView.NodeProvider>
  )
}

/** Asks for a name and adds a scene by it, so that what's being edited always has a file to save to */
function NewScene(props: { onAdded: (scene: OpenScene) => void }) {
  const add = async (name: string) => {
    props.onAdded({ name, source: await addContentScene(name, { Ron: STARTING_SCENE }) })
  }

  return <SceneNameButton label="+ New" title="New scene" action="Create" acting="Creating…" onName={add} />
}

/** A folder has children, and a scene has none. A scene's id is its name, and a folder's is its path. */
type SceneNode = { id: string; label: string; children?: SceneNode[] }

/** The folders a scene's path goes through become nodes, each listing its folders before its scenes */
function sceneTree(names: string[]): SceneNode[] {
  const root: SceneNode[] = []
  for (const name of names) {
    const parts = name.split('/')
    const file = parts.pop() ?? name
    let level = root
    parts.forEach((part, depth) => {
      const id = parts.slice(0, depth + 1).join('/')
      let folder = level.find((node) => node.id === id)
      if (!folder) {
        folder = { id, label: part, children: [] }
        level.push(folder)
      }
      level = folder.children ?? []
    })
    level.push({ id: name, label: file })
  }
  return sortNodes(root)
}

function sortNodes(nodes: SceneNode[]): SceneNode[] {
  nodes.forEach((node) => node.children && sortNodes(node.children))
  return nodes.sort((a, b) => Number(!!b.children) - Number(!!a.children) || a.label.localeCompare(b.label))
}

function folderIds(node: SceneNode): string[] {
  return node.children ? [node.id, ...node.children.flatMap(folderIds)] : []
}

function stopPropagation(e: Event) {
  e.stopPropagation()
}
