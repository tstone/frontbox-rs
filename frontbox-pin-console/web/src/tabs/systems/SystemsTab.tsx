import { Splitter } from '@ark-ui/solid/splitter'
import { createTreeCollection, TreeView } from '@ark-ui/solid/tree-view'
import { createMemo, createSignal, For, Show } from 'solid-js'
import { groupLabel, shortName } from '../../lib/format'
import { machine } from '../../state/console'
import type { System } from '../../types/generated/System'
import type { SystemGroup } from '../../types/generated/SystemGroup'
import SystemDetail from './SystemDetail'
import './SystemsTab.css'

type GroupNode = { id: string; label: string; kind: 'group'; group: SystemGroup; children: SystemNode[] }
type SystemNode = { id: string; label: string; kind: 'system'; group: SystemGroup; system: System }
type Node = GroupNode | SystemNode

export default function SystemsTab() {
  const groups = createMemo<GroupNode[]>(() =>
    machine.groups.map((group) => ({
      id: `group:${group.key}`,
      label: groupLabel(group.key),
      kind: 'group',
      group,
      children: group.systems.map((system) => ({
        id: `system:${system.id}`,
        label: shortName(system.name),
        kind: 'system',
        group,
        system,
      })),
    })),
  )

  const collection = createMemo(() =>
    createTreeCollection<Node | { id: string; label: string; children: GroupNode[] }>({
      nodeToValue: (node) => node.id,
      nodeToString: (node) => node.label,
      rootNode: { id: 'root', label: '', children: groups() },
    }),
  )

  // Track what's collapsed rather than what's expanded, so groups spawned later start open
  const [collapsed, setCollapsed] = createSignal<string[]>([])
  const groupIds = () => groups().map((g) => g.id)
  const expanded = () => groupIds().filter((id) => !collapsed().includes(id))

  const [selected, setSelected] = createSignal<string | null>(null)
  const selectedNode = () =>
    groups()
      .flatMap((group): Node[] => [group, ...group.children])
      .find((node) => node.id === selected())

  return (
    <Splitter.Root class="systems-tab" panels={[{ id: 'tree', minSize: 15 }, { id: 'detail' }]} defaultSize={[30, 70]}>
      <Splitter.Panel id="tree" class="tree-panel pane">
        <Show when={groups().length > 0} fallback={<p class="empty">No systems yet.</p>}>
          <TreeView.Root
            collection={collection()}
            selectionMode="single"
            selectedValue={selected() ? [selected()!] : []}
            onSelectionChange={(details) => setSelected(details.selectedValue[0] ?? null)}
            expandedValue={expanded()}
            onExpandedChange={(details) =>
              setCollapsed(groupIds().filter((id) => !details.expandedValue.includes(id)))
            }
          >
            <TreeView.Tree>
              <For each={groups()}>{(group, index) => <GroupBranch node={group} indexPath={[index()]} />}</For>
            </TreeView.Tree>
          </TreeView.Root>
        </Show>
      </Splitter.Panel>
      <Splitter.ResizeTrigger id="tree:detail" aria-label="Resize system tree" />
      <Splitter.Panel id="detail" class="detail-panel pane">
        {/* keyed, so the detail re-renders when the selection moves to a different node */}
        <Show when={selectedNode()} keyed fallback={<p class="empty">Select a system to see more about it.</p>}>
          {(node) =>
            node.kind === 'system' ? <SystemDetail system={node.system} group={node.group} /> : <GroupDetail group={node.group} />
          }
        </Show>
      </Splitter.Panel>
    </Splitter.Root>
  )
}

function GroupBranch(props: { node: GroupNode; indexPath: number[] }) {
  return (
    <TreeView.NodeProvider node={props.node} indexPath={props.indexPath}>
      <TreeView.Branch classList={{ inactive: !props.node.group.active }}>
        <TreeView.BranchControl>
          <TreeView.BranchIndicator>›</TreeView.BranchIndicator>
          <TreeView.BranchText>{props.node.label}</TreeView.BranchText>
          <Show when={!props.node.group.active}>
            <span class="inactive-tag">inactive</span>
          </Show>
          <span class="count">{props.node.children.length}</span>
        </TreeView.BranchControl>
        <TreeView.BranchContent>
          <TreeView.BranchIndentGuide />
          <For each={props.node.children}>
            {(child, index) => (
              <TreeView.NodeProvider node={child} indexPath={[...props.indexPath, index()]}>
                <TreeView.Item classList={{ inactive: !child.system.active }} title={child.system.name}>
                  <TreeView.ItemText>{child.label}</TreeView.ItemText>
                  <Show when={!child.system.active}>
                    <span class="inactive-tag">inactive</span>
                  </Show>
                </TreeView.Item>
              </TreeView.NodeProvider>
            )}
          </For>
        </TreeView.BranchContent>
      </TreeView.Branch>
    </TreeView.NodeProvider>
  )
}

function GroupDetail(props: { group: SystemGroup }) {
  return (
    <article class="detail">
      <h2>
        {groupLabel(props.group.key)}
        <span class={`badge ${props.group.active ? 'ok' : 'warn'}`}>{props.group.active ? 'Active' : 'Inactive'}</span>
      </h2>
      <dl>
        <dt>Kind</dt>
        <dd>System group</dd>
        <dt>Key</dt>
        <dd class="mono">{props.group.key}</dd>
        <dt>Systems</dt>
        <dd>{props.group.systems.length}</dd>
      </dl>
    </article>
  )
}
