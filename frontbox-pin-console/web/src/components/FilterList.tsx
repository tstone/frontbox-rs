import { Checkbox } from '@ark-ui/solid/checkbox'
import { For, Show } from 'solid-js'
import './FilterList.css'

export type FilterOption = {
  value: string
  label: string
  count?: number
  /** CSS color for a leading dot */
  color?: string
}

type Props = {
  title: string
  options: FilterOption[]
  isChecked: (value: string) => boolean
  onChange: (value: string, checked: boolean) => void
}

/** A titled list of checkboxes, with shortcuts to check or clear all of them */
export default function FilterList(props: Props) {
  const setAll = (checked: boolean) => props.options.forEach((o) => props.onChange(o.value, checked))

  return (
    <section class="filter-list">
      <header>
        <h3>{props.title}</h3>
        <button type="button" onClick={() => setAll(true)}>All</button>
        <button type="button" onClick={() => setAll(false)}>None</button>
      </header>
      <For each={props.options} fallback={<p class="empty">Nothing yet</p>}>
        {(option) => (
          <Checkbox.Root
            checked={props.isChecked(option.value)}
            onCheckedChange={(details) => props.onChange(option.value, details.checked === true)}
          >
            <Checkbox.Control>
              <Checkbox.Indicator>✓</Checkbox.Indicator>
            </Checkbox.Control>
            <Checkbox.Label>
              <Show when={option.color}>
                <span class="dot" style={{ background: option.color }} />
              </Show>
              <span class="label">{option.label}</span>
              <Show when={option.count !== undefined}>
                <span class="count">{option.count}</span>
              </Show>
            </Checkbox.Label>
            <Checkbox.HiddenInput />
          </Checkbox.Root>
        )}
      </For>
    </section>
  )
}
