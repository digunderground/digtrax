<template>
<!-- Per-deck vertical channel strip — Mixxx-style.
     GAIN → HI → MID → LO → FLTR → CHANNEL FADER, top to bottom.
     Each "knob" is actually a vertical slider for now; true rotary
     knobs are a v2 polish item. -->
<div class='dj-strip'>
    <DjKnob label='GAIN'  :model-value='state.gain'      @update:model-value='(v: any) => onSetGain(v)'    :min='0' :max='2' :default-value='1.0' />
    <DjKnob label='HI'    :model-value='state.eqHigh'    @update:model-value='(v: any) => onSetEq("high", v)' :min='0' :max='2' :default-value='1.0' />
    <DjKnob label='MID'   :model-value='state.eqMid'     @update:model-value='(v: any) => onSetEq("mid",  v)' :min='0' :max='2' :default-value='1.0' />
    <DjKnob label='LO'    :model-value='state.eqLow'     @update:model-value='(v: any) => onSetEq("low",  v)' :min='0' :max='2' :default-value='1.0' />
    <DjKnob label='FLTR'  :model-value='state.filter'    @update:model-value='(v: any) => onSetFilter(v)'  :min='-1' :max='1' :default-value='0.0' :bipolar='true' />
    <div class='dj-strip-fader'>
        <DjKnob label='VOL'  :model-value='state.userVolume' @update:model-value='(v: any) => onSetVolume(v)' :min='0' :max='1' :default-value='0.7' />
    </div>
</div>
</template>

<script lang='ts' setup>
import { computed, PropType } from 'vue';
import DjKnob from './DjKnob.vue';
import {
    DeckId, djState,
    setEq, setFilter, setGain, setDeckVolume, type EqBand,
} from '../../scripts/dj';

const props = defineProps({
    id: { required: true, type: String as PropType<DeckId> },
});
const state = computed(() => props.id === 'a' ? djState.deckA : djState.deckB);

function onSetGain(v: number) { setGain(props.id, v); }
function onSetEq(band: EqBand, v: number) { setEq(props.id, band, v); }
function onSetFilter(v: number) { setFilter(props.id, v); }
function onSetVolume(v: number) { setDeckVolume(props.id, v); }
</script>

<style lang='scss' scoped>
.dj-strip {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 8px 4px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xs, 4px);
}
.dj-strip-fader {
    margin-top: auto;
}
</style>
