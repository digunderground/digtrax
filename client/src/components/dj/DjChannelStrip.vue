<template>
<!-- Per-deck channel strip — Mixxx layout.
       2-column knob grid:  GAIN | HI
                            FLTR | MID
                              ?  | LO
       Vertical VOL fader on the side.
     Single column total ~80px wide so two strips + crossfader fit in
     a center column < 250px wide (vs. our prior 200-px strips that
     were too tall because of vertical sliders for everything). -->
<div class='dj-strip'>
    <div class='dj-strip-knobs'>
        <DjRotaryKnob label='GAIN' :model-value='state.gain'   @update:model-value='(v: any) => onSetGain(v)'   :min='0' :max='2' :default-value='1.0' />
        <DjRotaryKnob label='HI'   :model-value='state.eqHigh' @update:model-value='(v: any) => onSetEq("high", v)' :min='0' :max='2' :default-value='1.0' />
        <DjRotaryKnob label='FLTR' :model-value='state.filter' @update:model-value='(v: any) => onSetFilter(v)' :min='-1' :max='1' :default-value='0.0' :bipolar='true' />
        <DjRotaryKnob label='MID'  :model-value='state.eqMid'  @update:model-value='(v: any) => onSetEq("mid",  v)' :min='0' :max='2' :default-value='1.0' />
        <div class='dj-strip-spacer' />
        <DjRotaryKnob label='LO'   :model-value='state.eqLow'  @update:model-value='(v: any) => onSetEq("low",  v)' :min='0' :max='2' :default-value='1.0' />
    </div>
    <div class='dj-strip-fader-wrap'>
        <input
            type='range'
            class='dj-strip-fader'
            :value='state.userVolume'
            :min='0' :max='1' :step='0.01'
            @input='onVolume'
            @dblclick='onVolumeReset'
        />
    </div>
</div>
</template>

<script lang='ts' setup>
import { computed, PropType } from 'vue';
import DjRotaryKnob from './DjRotaryKnob.vue';
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
function onVolume(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    if (Number.isFinite(v)) setDeckVolume(props.id, v);
}
function onVolumeReset() { setDeckVolume(props.id, 0.7); }
</script>

<style lang='scss' scoped>
.dj-strip {
    display: flex;
    align-items: stretch;
    gap: 6px;
    padding: 6px 4px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xs, 4px);
}
.dj-strip-knobs {
    display: grid;
    grid-template-columns: 36px 36px;
    grid-auto-rows: 44px;
    column-gap: 2px;
    row-gap: 2px;
    align-content: start;
}
.dj-strip-spacer {
    /* Empty cell so LO sits on row 3 column 2, matching Mixxx. */
}

.dj-strip-fader-wrap {
    display: flex;
    align-items: stretch;
    width: 24px;
}
/* Vertical channel-volume fader. CSS appearance reset + custom thumb so
   it actually looks like a DJ-mixer fader (chunky teal handle, dim
   track). Mac browsers default to ugly cyan; we override. */
.dj-strip-fader {
    -webkit-appearance: slider-vertical;
    appearance: slider-vertical;
    writing-mode: vertical-lr;
    direction: rtl;
    width: 14px;
    margin: auto;
    background: rgba(0, 0, 0, 0.5);
    border: 1px solid var(--color-border);
    border-radius: 2px;
    cursor: ns-resize;
    accent-color: var(--color-accent);
}
</style>
