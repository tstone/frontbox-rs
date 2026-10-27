import { Checkbox } from '@ark-ui/solid/checkbox'
import { createMemo, createSignal, For, Match, Show, Switch } from 'solid-js'
import FilterList, { type FilterOption } from '../../components/FilterList'
import { shortName } from '../../lib/format'
import { describePower } from '../../lib/hardwareConfig'
import { configKind, formatConfigValue, isDefault, modeConfigKeys } from '../../lib/operatorConfig'
import { resetConfig, setConfig, setConfigInt, stepConfig } from '../../state/configControl'
import { driverMode, machine } from '../../state/console'
import type { ConfigView } from '../../types/generated/ConfigView'
import './ConfigTab.css'

type Source = 'hardware' | 'software'

const typeKey = (config: ConfigView) => shortName(config.type_name)
const changedKey = (config: ConfigView) => (isDefault(config) ? 'default' : 'changed')

export default function ConfigTab() {
  const [search, setSearch] = createSignal('')
  // Filters hold what's hidden, so values that show up later are visible by default
  const [hiddenTypes, setHiddenTypes] = createSignal<string[]>([])
  const [hiddenSources, setHiddenSources] = createSignal<string[]>([])
  const [hiddenChanged, setHiddenChanged] = createSignal<string[]>([])

  const toggle = (list: string[], value: string, visible: boolean) =>
    visible ? list.filter((v) => v !== value) : [...list, value]

  const configs = createMemo(() => Object.values(machine.configs).sort((a, b) => a.label.localeCompare(b.label)))

  /** Names of the drivers whose current mode uses each config key */
  const driversByKey = createMemo(() => {
    const byKey = new Map<string, string[]>()
    for (const driver of Object.values(machine.hardware?.drivers.by_id ?? {})) {
      const mode = driverMode(driver.id)
      if (!mode) continue
      for (const key of modeConfigKeys(mode)) byKey.set(key, [...(byKey.get(key) ?? []), driver.name])
    }
    return byKey
  })
  const sourceKey = (config: ConfigView): Source => (driversByKey().has(config.key) ? 'hardware' : 'software')

  const count = (key: (config: ConfigView) => string, labels: Record<string, string> = {}) => {
    const options = new Map<string, FilterOption>()
    for (const config of configs()) {
      const value = key(config)
      const option = options.get(value)
      if (option) option.count! += 1
      else options.set(value, { value, label: labels[value] ?? value, count: 1 })
    }
    return [...options.values()].sort((a, b) => a.label.localeCompare(b.label))
  }
  const typeOptions = createMemo(() => count(typeKey))
  const sourceOptions = createMemo(() => count(sourceKey, { hardware: 'Hardware', software: 'Software' }))
  const changedOptions = createMemo(() => count(changedKey, { changed: 'Changed', default: 'Default' }))

  const visible = createMemo(() => {
    const text = search().trim().toLowerCase()
    return configs().filter(
      (config) =>
        (text === '' || [config.label, config.key, config.desc].some((s) => s.toLowerCase().includes(text))) &&
        !hiddenTypes().includes(typeKey(config)) &&
        !hiddenSources().includes(sourceKey(config)) &&
        !hiddenChanged().includes(changedKey(config)),
    )
  })

  return (
    <div class="config-tab">
      <div class="config-list pane">
        <Show
          when={visible().length > 0}
          fallback={<p class="empty">{configs().length > 0 ? 'Everything is filtered out.' : 'No configs in use yet.'}</p>}
        >
          <ol>
            <For each={visible()}>
              {(config) => <ConfigRow config={config} drivers={driversByKey().get(config.key) ?? []} />}
            </For>
          </ol>
        </Show>
      </div>

      <aside class="config-sidebar pane">
        <input
          class="filter-input"
          type="search"
          placeholder="Filter by name"
          value={search()}
          onInput={(e) => setSearch(e.currentTarget.value)}
        />
        <p class="hint">Software configs are listed once something reads them.</p>
        <FilterList
          title="Value"
          options={changedOptions()}
          isChecked={(value) => !hiddenChanged().includes(value)}
          onChange={(value, checked) => setHiddenChanged((hidden) => toggle(hidden, value, checked))}
        />
        <FilterList
          title="Source"
          options={sourceOptions()}
          isChecked={(source) => !hiddenSources().includes(source)}
          onChange={(source, checked) => setHiddenSources((hidden) => toggle(hidden, source, checked))}
        />
        <FilterList
          title="Type"
          options={typeOptions()}
          isChecked={(type) => !hiddenTypes().includes(type)}
          onChange={(type, checked) => setHiddenTypes((hidden) => toggle(hidden, type, checked))}
        />
      </aside>
    </div>
  )
}

function ConfigRow(props: { config: ConfigView; drivers: string[] }) {
  const changed = () => !isDefault(props.config)

  return (
    <li class="config-row" classList={{ changed: changed() }}>
      <div class="about">
        <div class="title">
          <span class="label">{props.config.label}</span>
          <Show when={props.config.key !== props.config.label}>
            <span class="key mono">{props.config.key}</span>
          </Show>
          <Show when={changed()}>
            <span class="badge warn">Changed</span>
          </Show>
        </div>
        <Show when={props.config.desc}>
          <p class="desc">{props.config.desc}</p>
        </Show>
        <p class="meta">
          Default {formatConfigValue(props.config, props.config.default)}
          <Show when={props.drivers.length > 0}> · Used by {props.drivers.join(', ')}</Show>
        </p>
      </div>
      <div class="control">
        <ConfigControl config={props.config} />
        <button type="button" class="reset" disabled={!changed()} onClick={() => resetConfig(props.config)}>
          Reset
        </button>
      </div>
    </li>
  )
}

/**
 * Every control shows the machine's value, never one it hasn't confirmed: an edit is sent, and the control shows the
 * new value once the machine reports it back.
 */
function ConfigControl(props: { config: ConfigView }) {
  return (
    <Switch>
      <Match when={configKind(props.config) === 'bool'}>
        <BoolControl config={props.config} />
      </Match>
      <Match when={configKind(props.config) === 'choice'}>
        <ChoiceControl config={props.config} />
      </Match>
      <Match when={true}>
        <IntControl config={props.config} />
      </Match>
    </Switch>
  )
}

function BoolControl(props: { config: ConfigView }) {
  const on = () => 'Bool' in props.config.value && props.config.value.Bool
  return (
    <Checkbox.Root checked={on()} onCheckedChange={(details) => setConfig(props.config, { Bool: details.checked === true })}>
      <Checkbox.Control>
        <Checkbox.Indicator>✓</Checkbox.Indicator>
      </Checkbox.Control>
      <Checkbox.Label>{on() ? 'On' : 'Off'}</Checkbox.Label>
      <Checkbox.HiddenInput />
    </Checkbox.Root>
  )
}

function ChoiceControl(props: { config: ConfigView }) {
  const names = () => (typeof props.config.domain === 'object' && 'Choice' in props.config.domain ? props.config.domain.Choice : [])
  const selected = () => ('Choice' in props.config.value ? props.config.value.Choice : 0)
  return (
    <select
      value={selected()}
      onChange={(e) => {
        setConfig(props.config, { Choice: Number(e.currentTarget.value) })
        e.currentTarget.value = String(selected())
      }}
    >
      <For each={names()}>{(name, index) => <option value={index()}>{name}</option>}</For>
    </select>
  )
}

function IntControl(props: { config: ConfigView }) {
  const current = () => ('Int' in props.config.value ? props.config.value.Int : 0)
  const range = () =>
    typeof props.config.domain === 'object' && 'Int' in props.config.domain
      ? props.config.domain.Int
      : { min: 0, max: 0, step: 1 }
  const kind = () => configKind(props.config)

  return (
    <div class="stepper">
      <button type="button" aria-label="Decrease" disabled={current() <= range().min} onClick={() => stepConfig(props.config, -1)}>
        −
      </button>
      <input
        class="mono"
        type="number"
        min={range().min}
        max={range().max}
        step={range().step}
        value={current()}
        // fires on Enter or leaving the field, not on every keystroke
        onChange={(e) => {
          const typed = Number(e.currentTarget.value)
          if (Number.isFinite(typed)) setConfigInt(props.config, typed)
          e.currentTarget.value = String(current())
        }}
      />
      <button type="button" aria-label="Increase" disabled={current() >= range().max} onClick={() => stepConfig(props.config, 1)}>
        +
      </button>
      <Show when={kind() === 'duration' || kind() === 'power'}>
        <span class="unit">{kind() === 'duration' ? 'ms' : describePower(current())}</span>
      </Show>
    </div>
  )
}
