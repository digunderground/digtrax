<template>
<!-- Equal-power crossfader. Maps `model-value` ∈ [-1, 1] (engine
     convention; -1 = full A, +1 = full B, 0 = center detent) to a
     horizontal slider. Double-click resets to center. The center
     glows when within 5% of 0 to give the user a visible "you're
     centered" cue without making the slider feel sticky. -->
<div class='xfade'>
    <div class='xfade-label'>A</div>
    <div
        class='xfade-track'
        :class='{ "xfade-track--centered": centered }'
        @dblclick='reset'
    >
        <input
            type='range'
            :min='-1'
            :max='1'
            :step='0.001'
            :value='modelValue'
            @input='onInput'
            class='xfade-input'
        />
        <div class='xfade-detent' />
    </div>
    <div class='xfade-label'>B</div>
</div>
</template>

<script lang='ts' setup>
import { computed } from 'vue';

const props = defineProps({
    modelValue: { type: Number, default: 0 },
});
const emit = defineEmits<{ (e: 'update:modelValue', v: number): void }>();

const centered = computed(() => Math.abs(props.modelValue) < 0.05);

function onInput(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    if (Number.isFinite(v)) emit('update:modelValue', v);
}
function reset() { emit('update:modelValue', 0); }
</script>

<style lang='scss' scoped>
.xfade {
    display: flex;
    align-items: center;
    gap: 10px;
}
.xfade-label {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-fg-muted);
    width: 12px;
    text-align: center;
}
.xfade-track {
    position: relative;
    flex: 1;
    height: 32px;
    display: flex;
    align-items: center;
}
.xfade-detent {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 2px;
    height: 14px;
    background: rgba(255, 255, 255, 0.18);
    pointer-events: none;
    transition: background var(--duration-fast, 120ms) var(--ease-standard, ease);
}
.xfade-track--centered .xfade-detent {
    background: var(--color-accent);
    box-shadow: 0 0 6px var(--color-accent-glow);
}
.xfade-input {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 6px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 3px;
    outline: none;
    cursor: ew-resize;
}
.xfade-input::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 22px;
    border-radius: 3px;
    background: var(--color-accent);
    box-shadow: 0 0 6px var(--color-accent-glow);
    cursor: ew-resize;
}
</style>
