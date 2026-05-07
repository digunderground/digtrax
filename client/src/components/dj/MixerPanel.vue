<template>
<!-- Mixxx-style mixer panel.
       Layout: [Deck A]  [Center: 2 channel strips + master + crossfader]  [Deck B]
     Each Deck contains its own header / waveform / transport stack.
     The center column hosts vertical channel strips per deck (GAIN /
     HI / MID / LO / FLTR / VOL), plus master gain, zoom controls, and
     the crossfader. -->
<div class='dj-mixer-panel'>
    <DjDeck id='a' class='dj-mixer-deck' />

    <div class='dj-mixer-center'>
        <!-- Top row: zoom controls applied to both deck waveforms. -->
        <div class='dj-mixer-zoom'>
            <button class='dj-mixer-zoom-btn' @click='onZoomOut' title='Zoom out'>
                <q-icon name='mdi-magnify-minus-outline' size='13px' />
            </button>
            <span class='dj-mixer-zoom-label'>{{ djState.zoomSeconds }}s</span>
            <button class='dj-mixer-zoom-btn' @click='onZoomIn' title='Zoom in'>
                <q-icon name='mdi-magnify-plus-outline' size='13px' />
            </button>
        </div>

        <!-- Two channel strips side-by-side. Mixxx convention: deck A
             on the left, deck B on the right. -->
        <div class='dj-mixer-strips'>
            <DjChannelStrip id='a' />
            <DjChannelStrip id='b' />
        </div>

        <!-- Master gain (small horizontal slider above the crossfader). -->
        <div class='dj-mixer-master'>
            <q-icon name='mdi-tune-vertical' size='12px' class='q-mr-xs' />
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
import DjChannelStrip from './DjChannelStrip.vue';
import { djState, setCrossfader, setMasterGain, zoomIn, zoomOut } from '../../scripts/dj';

function onMaster(v: number | null) {
    if (typeof v === 'number') setMasterGain(v);
}
function onZoomIn() { zoomIn(); }
function onZoomOut() { zoomOut(); }
</script>

<style lang='scss' scoped>
.dj-mixer-panel {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    gap: 8px;
    align-items: stretch;
    padding: 8px 12px;
    background: var(--color-bg-elevated);
    border-top: 1px solid var(--color-border);
    min-height: 280px;
    box-sizing: border-box;
}
.dj-mixer-deck { min-width: 0; }

.dj-mixer-center {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 200px;
    flex-shrink: 0;
    padding: 4px 6px;
}

.dj-mixer-zoom {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
}
.dj-mixer-zoom-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 20px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xs, 3px);
    background: transparent;
    color: var(--color-fg-muted);
    cursor: pointer;
    transition: all var(--duration-fast, 120ms) ease;
}
.dj-mixer-zoom-btn:hover {
    color: var(--color-accent);
    border-color: var(--color-accent);
}
.dj-mixer-zoom-label {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    color: var(--color-fg-muted);
    min-width: 28px;
    text-align: center;
}

.dj-mixer-strips {
    display: flex;
    justify-content: center;
    gap: 6px;
    flex: 1;
}

.dj-mixer-master {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--color-fg-muted);
}
.dj-mixer-master-slider { flex: 1; }
.dj-mixer-master-label {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-fg-subtle);
}
</style>
