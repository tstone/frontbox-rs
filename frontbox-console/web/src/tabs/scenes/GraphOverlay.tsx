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
/** Points along a fan out's arc, enough that it reads as a curve */
const ARC_POINTS = 48

/**
 * The edges of the open layer's graph, drawn over the preview, with each node as a dot that names its LED. `highlighted`
 * is pointed out, and while `choosing` the nodes are drawn larger to click on. A traversal that starts from or goes to a
 * position has a handle there.
 */
export default function GraphOverlay(props: { space: PreviewSpace; graph: RenderedGraph | null; layer: number | null; highlighted: string | null; choosing: boolean }) {
  const scene = useContext(SceneContext)!
  const at = (node: number) => props.space.project(props.graph!.nodes[node])

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

  /** The node a traversal's `from` or `to` is, as the traversal finds it at the start */
  const nodeAt = (node: NodeRef): Vec2 | null => {
    const graph = props.graph!
    if ('Name' in node) return graph.nodes[graph.names.indexOf(node.Name)] ?? null
    const [x, y] = startingValue(node.Nearest, scene.get(), [0, 0])
    const distance = ([nx, ny]: Vec2) => (nx - x) ** 2 + (ny - y) ** 2
    return graph.nodes.reduce<Vec2 | null>((best, next) => (best && distance(best) <= distance(next) ? best : next), null)
  }

  // each fan out's arc, which its other paths end along
  const arcs = createMemo((): string[] => {
    const layer = props.layer
    if (layer === null || !props.graph) return []
    const traversals = scene.get().layers[layer]?.graph?.traversals ?? []
    return traversals.flatMap(({ from, algorithm }) => {
      if (typeof algorithm === 'string' || !('FanOut' in algorithm)) return []
      const { to, angle } = algorithm.FanOut
      const [center, end] = [nodeAt(from), nodeAt(to)]
      if (!center || !end) return []
      const spread = (startingValue(angle, scene.get(), 0) * Math.PI) / 180
      const [rx, ry] = [end[0] - center[0], end[1] - center[1]]
      const points = Array.from({ length: ARC_POINTS + 1 }, (_, index) => {
        const turn = (index / ARC_POINTS - 0.5) * spread
        const [cos, sin] = [Math.cos(turn), Math.sin(turn)]
        return props.space.project([center[0] + rx * cos - ry * sin, center[1] + rx * sin + ry * cos])
      })
      return points.every((point) => point !== null) ? [points.join(' ')] : []
    })
  })

  return (
    <Show when={props.graph}>
      {(graph) => (
        <div class="scene-handles">
          <svg class="scene-paths scene-graph" classList={{ 'scene-graph-choosing': props.choosing }} aria-hidden="true">
            <Index each={arcs()}>{(arc) => <polyline points={arc()} />}</Index>
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
