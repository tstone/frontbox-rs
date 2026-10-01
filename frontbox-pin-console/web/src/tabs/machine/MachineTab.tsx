import { Splitter } from '@ark-ui/solid/splitter'
import HardwareList from './HardwareList'
import PlayfieldView from './PlayfieldView'
import './MachineTab.css'

export default function MachineTab() {
  return (
    <Splitter.Root class="machine-tab" panels={[{ id: 'playfield' }, { id: 'hardware' }]} defaultSize={[70, 30]}>
      <Splitter.Panel id="playfield" class="playfield">
        <PlayfieldView />
      </Splitter.Panel>
      <Splitter.ResizeTrigger id="playfield:hardware" aria-label="Resize playfield" />
      <Splitter.Panel id="hardware">
        <HardwareList />
      </Splitter.Panel>
    </Splitter.Root>
  )
}
