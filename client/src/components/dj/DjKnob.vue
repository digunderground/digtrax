<template>
<!-- Vertical channel-strip knob/slider. Used for GAIN / HI / MID / LO /
     FLTR / fader on each deck. Behaves like a small vertical fader:
     drag up to increase, double-click to reset to `defaultValue`,
     mouse-wheel for fine adjust. Center detent visible when bipolar
     (e.g. FILTER) and at unity for unipolar (e.g. EQ). -->
<div class='dj-knob'>
    <div
        class='dj-knob-track'
        :class='{ "dj-knob-track--at-default": atDefault, "dj-knob-track--bipolar": bipolar }'
        @mousedown='onDown'
        @wheel.prevent='onWheel'
        @dblclick='onReset'
    >
        <div class='dj-knob-fill' :style='fillStyle' />
        <div class='dj-knob-detent' v-if='bipolar' />
        <div class='dj-knob-thumb' :style='thumbStyle' />
    </div>
    <div class='dj-knob-label'>{{ label }}</div>
</div>
</template>

<script lang='ts' setup>
import { computed, ref } from 'vue';

const props = defineProps({
    label: { type: String, required: true },
    modelValue: { type: Number, required: true },
    min: { type: Number, default: 0 },
    max: { type: Number, default: 2 },
    /// Where the slider snaps to on double-click. For unity-EQ: 1.0.
    /// For bipolar FILTER: 0.0.
    defaultValue: { type: Number, default: 1.0 },
    /// Bipolar = visual emphasis on center. Filter knob only for now.
    bipolar: { type: Boolean, default: false },
    /// How much the wheel moves per detent.
    wheelStep: { type: Number, default: 0.05 },
});
const emit = defineEmits<{ (e: 'update:modelValue', v: number): void }>();

const norm = computed(() => {
    return (props.modelValue - props.min) / (props.max - props.min);
});
const atDefault = computed(() => Math.abs(props.modelValue - props.defaultValue) < 0.005);

/// Thumb position, 0..1 from bottom. Translated to CSS bottom percent.
const thumbStyle = computed(() => ({ bottom: `calc(${(norm.value * 100).toFixed(1)}% - 5px)` }));

/// Fill from the default-value baseline (so bipolar shows fill above OR
/// below center depending on sign; unipolar fills from bottom).
const fillStyle = computed(() => {
    if (props.bipolar) {
        const base = (props.defaultValue - props.min) / (props.max - props.min);
        const lo = Math.min(base, norm.value);
        const hi = Math.max(base, norm.value);
        return {
            bottom: `${(lo * 100).toFixed(1)}%`,
            top: `${((1 - hi) * 100).toFixed(1)}%`,
        };
    }
    return {
        bottom: `0`,
        top: `${((1 - norm.value) * 100).toFixed(1)}%`,
    };
});

const dragging = ref(false);
const dragStartY = ref(0);
const dragStartValue = ref(0);

function onDown(e: MouseEvent) {
    dragging.value = true;
    dragStartY.value = e.clientY;
    dragStartValue.value = props.modelValue;
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp, { once: true });
}

function onMove(e: MouseEvent) {
    if (!dragging.value) return;
    const dy = dragStartY.value - e.clientY; // up = positive
    // Sensitivity: full range over ~120 pixels. Adjust as needed.
    const range = props.max - props.min;
    const delta = (dy / 120) * range;
    const next = clamp(dragStartValue.value + delta, props.min, props.max);
    emit('update:modelValue', next);
}

function onUp() {
    dragging.value = false;
    window.removeEventListener('mousemove', onMove);
}

function onWheel(e: WheelEvent) {
    const dir = e.deltaY < 0 ? 1 : -1;
    const next = clamp(props.modelValue + dir * props.wheelStep, props.min, props.max);
    emit('update:modelValue', next);
}

function onReset() {
    emit('update:modelValue', props.defaultValue);
}

function clamp(v: number, lo: number, hi: number) { return Math.max(lo, Math.min(hi, v)); }
</script>

<style lang='scss' scoped>
.dj-knob {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    width: 32px;
}
.dj-knob-track {
    position: relative;
    width: 14px;
    height: 88px;
    background: rgba(0, 0, 0, 0.45);
    border: 1px solid var(--color-border);
    border-radius: 3px;
    cursor: ns-resize;
    overflow: hidden;
}
.dj-knob-fill {
    position: absolute;
    left: 0;
    right: 0;
    background: linear-gradient(to top, rgba(0, 210, 191, 0.55), rgba(0, 210, 191, 0.85));
    pointer-events: none;
}
.dj-knob-track--at-default .dj-knob-fill {
    background: linear-gradient(to top, rgba(255, 255, 255, 0.10), rgba(255, 255, 255, 0.16));
}
.dj-knob-track--bipolar .dj-knob-fill {
    background: linear-gradient(to top, rgba(255, 200, 50, 0.55), rgba(255, 200, 50, 0.85));
}
.dj-knob-detent {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
    background: rgba(255, 255, 255, 0.4);
    pointer-events: none;
}
.dj-knob-thumb {
    position: absolute;
    left: -3px;
    right: -3px;
    height: 10px;
    background: var(--color-fg);
    border: 1px solid rgba(0, 0, 0, 0.6);
    border-radius: 2px;
    pointer-events: none;
    transition: bottom var(--duration-fast, 80ms) linear;
}
.dj-knob-label {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--color-fg-subtle);
    user-select: none;
}
</style>
