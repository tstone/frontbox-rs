import { Splitter } from '@ark-ui/solid/splitter'
import { createTreeCollection, TreeView } from '@ark-ui/solid/tree-view'
import { createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js'
import { shortName } from '../../lib/format'
import { machine } from '../../state/console'
import type { System } from '../../types/generated/System'
import { Pause, Play } from '../../components/icons'
import { typingIn } from '../../lib/keys'
import { selectThenToggle } from '../../components/treeClick'
import SystemDetail from './SystemDetail'
import { pausedAt, togglePause } from './timelinePause'
import './SystemsTab.css'

type SystemNode = { id: string; label: string; system: System; children: SystemNode[] }
type StatesNode = { id: string; label: string }

/** The tree's first entry, which shows the states rather than a system */
const STATES_NODE: StatesNode = { id: 'states', label: '<States>' }

export default function SystemsTab() {
  // the system tree, from each system's parent: top-level systems first, each followed by its children
  // despawned systems are kept for a while, to look back at, but only shown when asked for
  const [showDespawned, setShowDespawned] = createSignal(false)
  // what's typed in the search box, lowercased: systems whose full name or description holds it are shown, with
  // their ancestors
  const [search, setSearch] = createSignal('')
  const searching = () => search() !== ''

  const roots = createMemo<SystemNode[]>(() => {
    const children = new Map<number | null, System[]>()
    for (const system of machine.systems) {
      if (system.despawned_ms !== null && !showDespawned()) continue
      const siblings = children.get(system.parent) ?? []
      siblings.push(system)
      children.set(system.parent, siblings)
    }
    const node = (system: System): SystemNode => ({
      id: `system:${system.id}`,
      label: system.group ? `Group: ${system.name}` : shortName(system.name),
      system,
      children: (children.get(system.id) ?? []).map(node),
    })
    const all = (children.get(null) ?? []).map(node)
    if (!searching()) return all

    // a system stays if it matches, or something under it does
    const keep = (n: SystemNode): SystemNode | null => {
      const kept = n.children.map(keep).filter((child): child is SystemNode => child !== null)
      const matches = [n.system.name, n.system.description ?? ''].some((text) => text.toLowerCase().includes(search()))
      return kept.length > 0 || matches ? { ...n, children: kept } : null
    }
    return all.map(keep).filter((n): n is SystemNode => n !== null)
  })
  const showStates = () => !searching() || STATES_NODE.label.toLowerCase().includes(search())

  const allNodes = () => {
    const nodes: SystemNode[] = []
    const visit = (node: SystemNode) => {
      nodes.push(node)
      node.children.forEach(visit)
    }
    roots().forEach(visit)
    return nodes
  }

  const collection = createMemo(() =>
    createTreeCollection<SystemNode | StatesNode | { id: string; label: string; children: (SystemNode | StatesNode)[] }>({
      nodeToValue: (node) => node.id,
      nodeToString: (node) => node.label,
      rootNode: { id: 'root', label: '', children: [...(showStates() ? [STATES_NODE] : []), ...roots()] },
    }),
  )

  // Track what's collapsed rather than what's expanded, so systems spawned later start open. Everything's open while
  // searching, so every match shows.
  const [collapsed, setCollapsed] = createSignal<string[]>([])
  const branchIds = () => allNodes().filter((node) => node.children.length > 0).map((node) => node.id)
  const expanded = () => (searching() ? branchIds() : branchIds().filter((id) => !collapsed().includes(id)))

  // the selection stays shown while a search hides it in the tree
  const [selected, setSelected] = createSignal<string | null>(null)
  const selectedSystem = () => machine.systems.find((s) => `system:${s.id}` === selected())
  const childCount = (system: System) =>
    machine.systems.filter((s) => s.parent === system.id && (showDespawned() || s.despawned_ms === null)).length
  const parentOf = (system: System) => machine.systems.find((s) => s.id === system.parent)

  // a branch opens or closes on a click once it's selected, or on its arrow (`selectThenToggle`)
  const branches: Branches = {
    isSelected: (id) => selected() === id,
    toggle: (id) => setCollapsed((ids) => (ids.includes(id) ? ids.filter((c) => c !== id) : [...ids, id])),
  }

  // space pauses and resumes, as in the scene editor, unless it's typing
  let root: HTMLDivElement | undefined
  onMount(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      // the tab stays mounted while another is showing
      if (!root?.checkVisibility()) return
      if (e.key !== ' ' || e.ctrlKey || e.metaKey || e.altKey || typingIn(e.target)) return
      // taken from a focused button or tree item too, which would otherwise take it
      e.preventDefault()
      if (!e.repeat) togglePause()
    }
    window.addEventListener('keydown', onKeyDown)
    onCleanup(() => window.removeEventListener('keydown', onKeyDown))
  })

  return (
    <Splitter.Root ref={root} class="systems-tab" panels={[{ id: 'tree', minSize: 15 }, { id: 'detail' }]} defaultSize={[20, 80]}>
      {/* Ark sets overflow: hidden inline on panels, so each scrolls an inner element instead */}
      <Splitter.Panel id="tree">
        <div class="tree-panel pane">
          <div class="tree-options">
            <button
              class="pause-timelines"
              classList={{ paused: pausedAt() !== null }}
              onClick={togglePause}
              title={pausedAt() === null ? 'Pause every timeline, to look at them as they are now (Space)' : 'Resume, catching up with what happened (Space)'}
            >
              {pausedAt() === null ? <Pause size={12} /> : <Play size={12} />}
              {pausedAt() === null ? 'Pause' : 'Paused'}
            </button>
            <label>
              <input type="checkbox" checked={showDespawned()} onChange={(e) => setShowDespawned(e.currentTarget.checked)} />
              Show despawned
            </label>
            <input
              class="search"
              type="search"
              placeholder="Search systems"
              value={search()}
              onInput={(e) => setSearch(e.currentTarget.value.trim().toLowerCase())}
              onKeyDown={(e) => e.key === 'Escape' && ((e.currentTarget.value = ''), setSearch(''))}
            />
          </div>
          <Show
            when={roots().length > 0 || showStates()}
            fallback={<p class="empty">{searching() ? 'No systems match.' : 'No systems yet.'}</p>}
          >
            <TreeView.Root
              collection={collection()}
              selectionMode="single"
              selectedValue={selected() ? [selected()!] : []}
              onSelectionChange={(details) => setSelected(details.selectedValue[0] ?? null)}
              expandOnClick={false}
              expandedValue={expanded()}
              onExpandedChange={(details) => {
                // what's opened or closed during a search isn't kept
                if (!searching()) setCollapsed(branchIds().filter((id) => !details.expandedValue.includes(id)))
              }}
            >
              <TreeView.Tree>
                <Show when={showStates()}>
                  <TreeView.NodeProvider node={STATES_NODE} indexPath={[0]}>
                    <TreeView.Item>
                      <span class="indicator-spacer" />
                      <TreeView.ItemText>{STATES_NODE.label}</TreeView.ItemText>
                    </TreeView.Item>
                  </TreeView.NodeProvider>
                </Show>
                <For each={roots()}>
                  {(node, index) => <SystemTreeNode node={node} indexPath={[index() + (showStates() ? 1 : 0)]} branches={branches} />}
                </For>
              </TreeView.Tree>
            </TreeView.Root>
          </Show>
        </div>
      </Splitter.Panel>
      <Splitter.ResizeTrigger id="tree:detail" aria-label="Resize system tree" />
      <Splitter.Panel id="detail">
        <div class="detail-panel pane">
          {/* keyed, so the detail re-renders when the selection moves to a different node */}
          <Show when={selected() === STATES_NODE.id} fallback={
            <Show when={selectedSystem()} keyed fallback={<p class="empty">Select a system to see more about it.</p>}>
              {(system) => <SystemDetail system={system} parent={parentOf(system)} children={childCount(system)} />}
            </Show>
          }>
            <MachineStates />
          </Show>
        </div>
      </Splitter.Panel>
    </Splitter.Root>
  )
}

/** A system, as a branch when it has children */
/** How a branch's row opens and closes: see `selectThenToggle` */
type Branches = { isSelected(id: string): boolean; toggle(id: string): void }

function SystemTreeNode(props: { node: SystemNode; indexPath: number[]; branches: Branches }) {
  const tags = () => (
    <>
      <For each={routinesHeldBy(props.node.system.id)}>
        {(routine) => (
          <span class="routine-tag" title={`Taking its step in ${routine.routine}`}>
            {shortName(routine.routine)}
          </span>
        )}
      </For>
      <Show when={!props.node.system.active}>
        <span class="inactive-tag">inactive</span>
      </Show>
    </>
  )

  return (
    <TreeView.NodeProvider node={props.node} indexPath={props.indexPath}>
      <Show
        when={props.node.children.length > 0}
        fallback={
          <TreeView.Item classList={{ inactive: !props.node.system.active, despawned: props.node.system.despawned_ms !== null }} title={hoverText(props.node.system)}>
            {/* in place of a branch's arrow, so names at the same depth line up */}
            <span class="indicator-spacer" />
            <Origin system={props.node.system} />
            <TreeView.ItemText>{props.node.label}</TreeView.ItemText>
            {tags()}
          </TreeView.Item>
        }
      >
        <TreeView.Branch classList={{ inactive: !props.node.system.active, despawned: props.node.system.despawned_ms !== null }}>
          <TreeView.BranchControl
            title={hoverText(props.node.system)}
            {...selectThenToggle(() => props.branches.isSelected(props.node.id), () => props.branches.toggle(props.node.id))}
          >
            <TreeView.BranchIndicator>›</TreeView.BranchIndicator>
            <Origin system={props.node.system} />
            <TreeView.BranchText>{props.node.label}</TreeView.BranchText>
            {tags()}
            <span class="count">{props.node.children.length}</span>
          </TreeView.BranchControl>
          <TreeView.BranchContent>
            <TreeView.BranchIndentGuide />
            <For each={props.node.children}>
              {(child, index) => <SystemTreeNode node={child} indexPath={[...props.indexPath, index()]} branches={props.branches} />}
            </For>
          </TreeView.BranchContent>
        </TreeView.Branch>
      </Show>
    </TreeView.NodeProvider>
  )
}

/** Routines whose current step belongs to the system */
function routinesHeldBy(id: number) {
  return machine.routines.filter((r) => r.current === id)
}

/** The states, which have one value for the whole machine */
function MachineStates() {
  return (
    <article class="detail">
      <h2>States</h2>
      <Show when={Object.keys(machine.states).length > 0} fallback={<p class="empty">No states set.</p>}>
        <dl>
          <For each={Object.entries(machine.states)}>
            {([name, value]) => (
              <>
                <dt title={name}>{shortName(name)}</dt>
                <dd class="mono">{JSON.stringify(value)}</dd>
              </>
            )}
          </For>
        </dl>
      </Show>
    </article>
  )
}

/** Whether the system is part of Frontbox, rather than the game: a `Group`, or a type from one of its crates */
export function isFrontbox(system: System): boolean {
  return system.group || system.name.startsWith('frontbox_')
}

/**
 * An icon for where the system comes from: a box for Frontbox, a gamepad for the game. Filled silhouettes in their own
 * colors, as outlines this small read the same.
 */
function Origin(props: { system: System }) {
  return (
    <span
      class="origin"
      classList={{ frontbox: isFrontbox(props.system), game: !isFrontbox(props.system) }}
      title={isFrontbox(props.system) ? 'Part of Frontbox' : 'Part of the game'}
    >
      <svg width="13" height="13" viewBox="0 0 16 16" aria-hidden="true">
        {isFrontbox(props.system) ? (
          <>
            <path d="M8 .8l6.6 3.6v7.2L8 15.2l-6.6-3.6V4.4z" fill="currentColor" />
            {/* the box's edges, cut out of it */}
            <path d="M1.6 4.5L8 8l6.4-3.5M8 8v7" fill="none" stroke="var(--surface)" stroke-width="1.3" />
          </>
        ) : (
          <path
            fill="currentColor"
            fill-rule="evenodd"
            d="M4.6 3.5h6.8a4.1 4.1 0 0 1 4.1 4.1v3.3a2 2 0 0 1-3.5 1.3L10.6 10.6H5.4L4 12.2a2 2 0 0 1-3.5-1.3V7.6a4.1 4.1 0 0 1 4.1-4.1z M3.9 5.6h1.2v1.3h1.3v1.2H5.1v1.3H3.9V8.1H2.6V6.9h1.3z M12.4 6.4a.85.85 0 1 1-1.7 0 .85.85 0 1 1 1.7 0z M13.9 8.4a.85.85 0 1 1-1.7 0 .85.85 0 1 1 1.7 0z"
          />
        )}
      </svg>
    </span>
  )
}

/** A tree node's hover text: its full name, and what it's for */
function hoverText(system: System): string {
  return system.description ? `${system.name}\n${system.description}` : system.name
}
