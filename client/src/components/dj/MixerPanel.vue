<template>
<!-- Top-level layout for the DJ mixer view. Phase 1 ships a 3-column
     grid:  [Deck A]  [Center: crossfader + master gain]  [Deck B]
     Phase 4 expands the center column with EQ/filter strips per deck.
     Phase 5 adds the dual scrolling-waveform header above this layout. -->
<div class='dj-mixer-panel'>
    <DjDeck id='a' class='dj-mixer-deck' />
    <div class='dj-mixer-center'>
        <div class='dj-mixer-master'>
            <q-icon name='mdi-tune-vertical' size='14px' class='q-mr-xs' />
            <q-slider
                :model-value='djState.masterGain'
                @update:model-value='onMaster'
                :min='0' :max='2' :step='0.01'
                class='dj-mixer-master-slider'
            />
            <span class='dj-mixer-master-label'>MAIN</span>
        </div>
        <Crossfader
            :model-value='djState.crossfader'
            @update:model-value='setCrossfader'
        />
    </div>
    <DjDeck id='b' class='dj-mixer-deck' />
</div>
</template>

<script lang='ts' setup>
import DjDeck from './DjDeck.vue';
import Crossfader from './Crossfader.vue';
import { djState, setCrossfader, setMasterGain } from '../../scripts/dj';

function onMaster(v: number | null) {
    if (typeof v === 'number') setMasterGain(v);
}
</script>

<style lang='scss' scoped>
.dj-mixer-panel {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    gap: 12px;
    align-items: stretch;
    padding: 10px 14px;
    background: var(--color-bg-elevated);
    border-top: 1px solid var(--color-border);
    min-height: 240px;
    box-sizing: border-box;
}
.dj-mixer-deck { min-width: 0; }
.dj-mixer-center {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 10px;
    width: 280px;
    padding: 6px 12px;
    flex-shrink: 0;
}
.dj-mixer-master {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--color-fg-muted);
}
.dj-mixer-master-slider { flex: 1; }
.dj-mixer-master-label {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-fg-subtle);
}
</style>
