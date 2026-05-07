<template>
<!-- Compact EQ + filter strip. Four sliders side-by-side: HIGH, MID,
     LOW, FILTER. Each EQ slider goes 0..2 (1 = unity, double-click to
     reset). FILTER goes -1..+1 with 0 = bypass / center detent. -->
<div class='dj-eq-strip'>
    <div class='dj-eq-cell' :class='{ "dj-eq-cell--off": state.eqHigh !== 1.0 }'>
        <button class='dj-eq-label' @dblclick='reset("high")' title='Double-click to reset'>HI</button>
        <q-slider
            :model-value='state.eqHigh' @update:model-value='(v: any) => onEq("high", v)'
            :min='0' :max='2' :step='0.01' vertical reverse class='dj-eq-slider'
        />
    </div>
    <div class='dj-eq-cell' :class='{ "dj-eq-cell--off": state.eqMid !== 1.0 }'>
        <button class='dj-eq-label' @dblclick='reset("mid")' title='Double-click to reset'>MID</button>
        <q-slider
            :model-value='state.eqMid' @update:model-value='(v: any) => onEq("mid", v)'
            :min='0' :max='2' :step='0.01' vertical reverse class='dj-eq-slider'
        />
    </div>
    <div class='dj-eq-cell' :class='{ "dj-eq-cell--off": state.eqLow !== 1.0 }'>
        <button class='dj-eq-label' @dblclick='reset("low")' title='Double-click to reset'>LOW</button>
        <q-slider
            :model-value='state.eqLow' @update:model-value='(v: any) => onEq("low", v)'
            :min='0' :max='2' :step='0.01' vertical reverse class='dj-eq-slider'
        />
    </div>
    <div class='dj-eq-cell' :class='{ "dj-eq-cell--off": Math.abs(state.filter) > 0.02 }'>
        <button class='dj-eq-label dj-eq-label--filter' @dblclick='resetFilter' title='Double-click to bypass'>FLTR</button>
        <q-slider
            :model-value='state.filter' @update:model-value='onFilter'
            :min='-1' :max='1' :step='0.01' vertical reverse class='dj-eq-slider dj-eq-slider--filter'
        />
    </div>
</div>
</template>

<script lang='ts' setup>
import { computed, PropType } from 'vue';
import { DeckId, djState, setEq, setFilter, type EqBand } from '../../scripts/dj';

const props = defineProps({
    id: { required: true, type: String as PropType<DeckId> },
});
const state = computed(() => props.id === 'a' ? djState.deckA : djState.deckB);

function onEq(band: EqBand, v: number | null) {
    if (typeof v === 'number') setEq(props.id, band, v);
}
function reset(band: EqBand) { setEq(props.id, band, 1.0); }
function onFilter(v: number | null) {
    if (typeof v === 'number') setFilter(props.id, v);
}
function resetFilter() { setFilter(props.id, 0); }
</script>

<style lang='scss' scoped>
.dj-eq-strip {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 8px;
}
.dj-eq-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    height: 100px;
}
.dj-eq-label {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-fg-subtle);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
    transition: color var(--duration-fast, 120ms) ease;
}
.dj-eq-cell--off .dj-eq-label {
    color: var(--color-accent);
}
.dj-eq-slider {
    height: 80px;
    width: 16px;
}
</style>
