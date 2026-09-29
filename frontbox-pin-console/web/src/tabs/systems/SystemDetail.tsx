import type { System } from '../../types/generated/System'
import type { SystemGroup } from '../../types/generated/SystemGroup'
import { groupLabel, shortName } from '../../lib/format'

/** Everything the console knows about one system. This is the place to grow system details. */
export default function SystemDetail(props: { system: System; group: SystemGroup }) {
  return (
    <article class="detail">
      <h2>
        {shortName(props.system.name)}
        <span class={`badge ${props.system.active ? 'ok' : 'warn'}`}>
          {props.system.active ? 'Active' : 'Inactive'}
        </span>
      </h2>
      <dl>
        <dt>Full name</dt>
        <dd class="mono">{props.system.name}</dd>
        <dt>ID</dt>
        <dd class="mono">{props.system.id}</dd>
        <dt>Group</dt>
        <dd>{groupLabel(props.group.key)}</dd>
      </dl>
    </article>
  )
}
