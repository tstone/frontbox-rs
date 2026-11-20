import { Splitter } from '@ark-ui/solid/splitter'
import { createTreeCollection, TreeView } from '@ark-ui/solid/tree-view'
import { createMemo, createSignal, For, Show } from 'solid-js'
import { shortName } from '../../lib/format'
import { machine } from '../../state/console'
import type { System } from '../../types/generated/System'
import SystemDetail from './SystemDetail'
import './SystemsTab.css'

type SystemNode = { id: string; label: string; system: System; children: SystemNode[] }

export default function SystemsTab() {
  // the system tree, from each system's parent: top-level systems first, each followed by its children
  const roots = createMemo<SystemNode[]>(() => {
    const children = new Map<number | null, System[]>()
    for (const system of machine.systems) {
      const siblings = children.get(system.parent) ?? []
      siblings.push(system)
      children.set(system.parent, siblings)
    }
    const node = (system: System): SystemNode => ({
      id: `system:${system.id}`,
      label: shortName(system.name),
      system,
      children: (children.get(system.id) ?? []).map(node),
    })
    return (children.get(null) ?? []).map(node)
  })

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
    createTreeCollection<SystemNode | { id: string; label: string; children: SystemNode[] }>({
      nodeToValue: (node) => node.id,
      nodeToString: (node) => node.label,
      rootNode: { id: 'root', label: '', children: roots() },
    }),
  )

  // Track what's collapsed rather than what's expanded, so systems spawned later start open
  const [collapsed, setCollapsed] = createSignal<string[]>([])
  const branchIds = () => allNodes().filter((node) => node.children.length > 0).map((node) => node.id)
  const expanded = () => branchIds().filter((id) => !collapsed().includes(id))

  const [selected, setSelected] = createSignal<string | null>(null)
  const selectedNode = () => allNodes().find((node) => node.id === selected())
  const parentOf = (system: System) => machine.systems.find((s) => s.id === system.parent)

  return (
    <Splitter.Root class="systems-tab" panels={[{ id: 'tree', minSize: 15 }, { id: 'detail' }]} defaultSize={[30, 70]}>
      {/* Ark sets overflow: hidden inline on panels, so each scrolls an inner element instead */}
      <Splitter.Panel id="tree">
        <div class="tree-panel pane">
          <Show when={roots().length > 0} fallback={<p class="empty">No systems yet.</p>}>
            <TreeView.Root
              collection={collection()}
              selectionMode="single"
              selectedValue={selected() ? [selected()!] : []}
              onSelectionChange={(details) => setSelected(details.selectedValue[0] ?? null)}
              expandedValue={expanded()}
              onExpandedChange={(details) =>
                setCollapsed(branchIds().filter((id) => !details.expandedValue.includes(id)))
              }
            >
              <TreeView.Tree>
                <For each={roots()}>{(node, index) => <SystemTreeNode node={node} indexPath={[index()]} />}</For>
              </TreeView.Tree>
            </TreeView.Root>
          </Show>
        </div>
      </Splitter.Panel>
      <Splitter.ResizeTrigger id="tree:detail" aria-label="Resize system tree" />
      <Splitter.Panel id="detail">
        <div class="detail-panel pane">
          {/* keyed, so the detail re-renders when the selection moves to a different node */}
          <Show when={selectedNode()} keyed fallback={<MachineStates />}>
            {(node) => (
              <SystemDetail system={node.system} parent={parentOf(node.system)} children={node.children.length} />
            )}
          </Show>
        </div>
      </Splitter.Panel>
    </Splitter.Root>
  )
}

/** A system, as a branch when it has children */
function SystemTreeNode(props: { node: SystemNode; indexPath: number[] }) {
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
          <TreeView.Item classList={{ inactive: !props.node.system.active }} title={props.node.system.name}>
            <TreeView.ItemText>{props.node.label}</TreeView.ItemText>
            {tags()}
          </TreeView.Item>
        }
      >
        <TreeView.Branch classList={{ inactive: !props.node.system.active }}>
          <TreeView.BranchControl title={props.node.system.name}>
            <TreeView.BranchIndicator>›</TreeView.BranchIndicator>
            <TreeView.BranchText>{props.node.label}</TreeView.BranchText>
            {tags()}
            <span class="count">{props.node.children.length}</span>
          </TreeView.BranchControl>
          <TreeView.BranchContent>
            <TreeView.BranchIndentGuide />
            <For each={props.node.children}>
              {(child, index) => <SystemTreeNode node={child} indexPath={[...props.indexPath, index()]} />}
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

/** Shown with nothing selected: the states, which have one value for the whole machine */
function MachineStates() {
  return (
    <article class="detail">
      <p class="empty">Select a system to see more about it.</p>
      <h3>States</h3>
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
