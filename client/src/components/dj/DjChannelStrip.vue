<template>
<!-- Per-deck channel strip — simplified for v1 (Mixxx parity comes
     incrementally). FILTER, FX, KEY, PAN deferred to v2.
       Layout (vertical column):
         GAIN
         HI
         MID
         LO
       Channel VOL fader on the right.
     Single 32px-wide column of rotary knobs + 24px fader = ~60px
     total per strip. Two strips fit in a ~140px center column. -->
<div class='dj-strip'>
    <div class='dj-strip-knobs'>
        <DjRotaryKnob label='GAIN' :model-value='state.gain'   @update:model-value='(v: any) => onSetGain(v)'   :min='0' :max='2' :default-value='1.0' />
        <DjRotaryKnob label='HI'   :model-value='state.eqHigh' @update:model-value='(v: any) => onSetEq("high", v)' :min='0' :max='2' :default-value='1.0' />
        <DjRotaryKnob label='MID'  :model-value='state.eqMid'  @update:model-value='(v: any) => onSetEq("mid",  v)' :min='0' :max='2' :default-value='1.0' />
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
    setEq, setGain, setDeckVolume, type EqBand,
} from '../../scripts/dj';

const props = defineProps({
    id: { required: true, type: String as PropType<DeckId> },
});
const state = computed(() => props.id === 'a' ? djState.deckA : djState.deckB);

function onSetGain(v: number) { setGain(props.id, v); }
function onSetEq(band: EqBand, v: number) { setEq(props.id, band, v); }
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
    gap: 4px;
    padding: 4px 3px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xs, 4px);
}
.dj-strip-knobs {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
}
.dj-strip-fader-wrap {
    display: flex;
    align-items: stretch;
    width: 22px;
}
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
