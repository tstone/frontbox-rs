import { Splitter } from '@ark-ui/solid/splitter'
import HardwareList from './HardwareList'
import './MachineTab.css'

export default function MachineTab() {
  return (
    <Splitter.Root class="machine-tab" panels={[{ id: 'playfield' }, { id: 'hardware' }]} defaultSize={[65, 35]}>
      <Splitter.Panel id="playfield" class="playfield">
        <p class="empty">Playfield</p>
      </Splitter.Panel>
      <Splitter.ResizeTrigger id="playfield:hardware" aria-label="Resize playfield" />
      <Splitter.Panel id="hardware">
        <HardwareList />
      </Splitter.Panel>
    </Splitter.Root>
  )
}
