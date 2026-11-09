import { createMemo, Index, Show, useContext } from 'solid-js'
import type { NodeRef } from '../../types/generated/NodeRef'
import type { Prop } from '../../types/generated/Prop'
import type { RenderedGraph } from '../../types/generated/RenderedGraph'
import type { PreviewSpace } from '../machine/PlayfieldView'
import { SceneContext } from './SceneFields'
import { field, item, startingValue, traversalTarget, variant, type Lens, type Vec2 } from './sceneModel'
import { Grip, type Handle } from './ShapeHandles'

/** How the positions a handle writes are rounded, so a drag doesn't leave long decimals in a scene */
const PRECISION = 1000

/**
 * The edges of the open layer's graph, drawn over the preview, with each node as a dot that names its LED. `highlighted`
 * is pointed out, and while `choosing` the nodes are drawn larger to click on. A traversal that starts from or goes to a
 * position has a handle there.
 */
export default function GraphOverlay(props: { space: PreviewSpace; graph: RenderedGraph | null; layer: number | null; highlighted: string | null; choosing: boolean }) {
  const scene = useContext(SceneContext)!
  const at = (node: number) => props.space.project(props.graph!.nodes[node])

  const handles = createMemo((): Handle[] => {
    const layer = props.layer
    if (layer === null || !props.graph) return []
    const graph = field(item(field(scene, 'layers'), layer), 'graph')
    if (!graph.get()) return []
    const traversals = field(graph as Lens<NonNullable<ReturnType<typeof graph.get>>>, 'traversals')
    return traversals.get().flatMap(({ name }, index) => {
      const traversal = item(traversals, index)
      const target = traversalTarget(field(traversal, 'algorithm'))
      return [positionHandle(field(traversal, 'from'), `${name} start`), target && positionHandle(target, `${name} end`)].filter((handle) => handle !== null)
    })
  })

  /** A handle for a node chosen by position, which is `null` for one chosen by name */
  const positionHandle = (node: Lens<NodeRef>, label: string): Handle | null => {
    if (!('Nearest' in node.get())) return null
    const position = variant(node, 'Nearest') as Lens<Prop<Vec2>>
    return {
      label,
      kind: 'point',
      cursor: 'move',
      at: startingValue(position.get(), scene.get(), [0, 0]),
      // only a fixed position can be dragged, as anything else would only be put back
      locked: !('Fixed' in position.get()),
      drag: ([x, y]) => position.set({ Fixed: [Math.round(x * PRECISION) / PRECISION, Math.round(y * PRECISION) / PRECISION] }),
    }
  }

  return (
    <Show when={props.graph}>
      {(graph) => (
        <div class="scene-handles">
          <svg class="scene-paths scene-graph" classList={{ 'scene-graph-choosing': props.choosing }} aria-hidden="true">
            <Index each={graph().edges}>
              {(edge) => {
                const ends = () => [at(edge()[0]), at(edge()[1])]
                return (
                  <Show when={ends().every((end) => end !== null)}>
                    <line x1={ends()[0]![0]} y1={ends()[0]![1]} x2={ends()[1]![0]} y2={ends()[1]![1]} />
                  </Show>
                )
              }}
            </Index>
            <Index each={graph().nodes}>
              {(_, node) => (
                <Show when={at(node)}>
                  {(spot) => (
                    <circle
                      classList={{ 'scene-graph-highlighted': graph().names[node] === props.highlighted }}
                      cx={spot()[0]}
                      cy={spot()[1]}
                      r={graph().names[node] === props.highlighted ? 7 : props.choosing ? 4 : 2.5}
                    >
                      <title>{graph().names[node]}</title>
                    </circle>
                  )}
                </Show>
              )}
            </Index>
          </svg>
          <Index each={handles()}>{(handle) => <Grip handle={handle()} space={props.space} />}</Index>
        </div>
      )}
    </Show>
  )
}
