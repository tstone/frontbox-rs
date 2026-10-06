import { Tooltip } from '@ark-ui/solid/tooltip'
import type { JSX } from 'solid-js'
import { Portal } from 'solid-js/web'
import './InfoTip.css'

export default function Tip(props: { text: string; children: JSX.Element }) {
  return (
    <Tooltip.Root openDelay={300} closeDelay={50} positioning={{ placement: 'top' }}>
      <Tooltip.Trigger
        asChild={(triggerProps) => (
          <span class="tip-trigger" {...triggerProps()}>
            {props.children}
          </span>
        )}
      />
      <Portal>
        <Tooltip.Positioner>
          <Tooltip.Content class="info-tip-content">{props.text}</Tooltip.Content>
        </Tooltip.Positioner>
      </Portal>
    </Tooltip.Root>
  )
}
