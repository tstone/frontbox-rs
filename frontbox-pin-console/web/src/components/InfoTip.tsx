import { Tooltip } from '@ark-ui/solid/tooltip'
import { Portal } from 'solid-js/web'
import './InfoTip.css'

/** A small "i" that explains something on hover or focus */
export default function InfoTip(props: { text: string }) {
  return (
    <Tooltip.Root openDelay={150} closeDelay={50} positioning={{ placement: 'top' }}>
      <Tooltip.Trigger class="info-tip" aria-label={props.text}>
        i
      </Tooltip.Trigger>
      {/* portaled so the content doesn't inherit styles from wherever the tip sits */}
      <Portal>
        <Tooltip.Positioner>
          <Tooltip.Content class="info-tip-content">{props.text}</Tooltip.Content>
        </Tooltip.Positioner>
      </Portal>
    </Tooltip.Root>
  )
}
