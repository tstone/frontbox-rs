import { For, Match, Switch } from 'solid-js'
import './JsonView.css'

type Json = null | boolean | number | string | Json[] | { [key: string]: Json }

/** Arrays of at most this many plain values show on one line */
const INLINE_ARRAY_MAX = 8

/**
 * A JSON value laid out in full, with nothing to expand or collapse: each key on its own line, and nested objects and
 * arrays indented under theirs. For values small enough to read at a glance, such as a system's status. Large or
 * deeply nested ones are better in a `JsonTreeView`.
 */
export default function JsonView(props: { data: unknown }) {
  const data = () => props.data as Json
  return <div class="json-view">{opens(data()) ? <Entries value={data() as Branch} /> : <Value value={data()} />}</div>
}

type Branch = Json[] | { [key: string]: Json }

function Entries(props: { value: Branch }) {
  const entries = () =>
    Array.isArray(props.value) ? props.value.map((v, i) => [String(i), v] as const) : Object.entries(props.value)
  return (
    <For each={entries()}>
      {([key, value]) => (
        <div class="entry">
          <span classList={{ key: true, index: Array.isArray(props.value) }}>{key}</span>
          <Switch fallback={<Value value={value} />}>
            <Match when={opens(value)}>
              <div class="nested">
                <Entries value={value as Branch} />
              </div>
            </Match>
          </Switch>
        </div>
      )}
    </For>
  )
}

/** A value shown after its key on the same line */
function Value(props: { value: Json }) {
  const v = props.value
  if (v === null) return <span class="null">null</span>
  if (typeof v === 'string') return <span class="string">"{v}"</span>
  if (typeof v === 'number') return <span class="number">{v}</span>
  if (typeof v === 'boolean') return <span class="boolean">{String(v)}</span>
  if (Array.isArray(v)) {
    return (
      <span class="brace">
        [
        <For each={v}>
          {(item, i) => (
            <>
              {i() > 0 && ', '}
              <Value value={item} />
            </>
          )}
        </For>
        ]
      </span>
    )
  }
  return <span class="brace">{'{}'}</span>
}

/** Whether the value lists its contents on lines of their own, rather than after its key */
function opens(value: Json): boolean {
  if (value === null || typeof value !== 'object') return false
  if (Array.isArray(value)) {
    const plain = value.every((v) => v === null || typeof v !== 'object')
    return value.length > 0 && !(plain && value.length <= INLINE_ARRAY_MAX)
  }
  return Object.keys(value).length > 0
}
