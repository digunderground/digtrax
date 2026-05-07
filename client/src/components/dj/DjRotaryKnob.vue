<template>
<!-- Compact rotary knob — Mixxx / Pioneer style.
     ~28px diameter. Drag up/down to change, wheel to fine-tune,
     double-click to reset to defaultValue. Bipolar mode (FILTER)
     shows fill from center; unipolar (GAIN/EQ) shows fill from min.
     Pointer indicates current value via a 270° sweep (-135° → +135°). -->
<div
    class='dj-rknob'
    :class='{ "dj-rknob--at-default": atDefault, "dj-rknob--bipolar": bipolar }'
    @mousedown='onDown'
    @wheel.prevent='onWheel'
    @dblclick='onReset'
>
    <svg viewBox='0 0 32 32' class='dj-rknob-svg'>
        <!-- Outer dim ring (always visible, marks the sweep range). -->
        <path :d='ringPath' class='dj-rknob-ring' fill='none' />
        <!-- Active arc — fills from baseline (min for unipolar,
             center for bipolar) to current angle. -->
        <path :d='arcPath' class='dj-rknob-arc' fill='none' />
        <!-- Knob body (filled circle). -->
        <circle cx='16' cy='16' r='9' class='dj-rknob-body' />
        <!-- Pointer — short line from center toward current angle. -->
        <line :x1='pointerX1' :y1='pointerY1' :x2='pointerX2' :y2='pointerY2' class='dj-rknob-pointer' />
    </svg>
    <div class='dj-rknob-label'>{{ label }}</div>
</div>
</template>

<script lang='ts' setup>
import { computed, ref } from 'vue';

const props = defineProps({
    label: { type: String, required: true },
    modelValue: { type: Number, required: true },
    min: { type: Number, default: 0 },
    max: { type: Number, default: 2 },
    defaultValue: { type: Number, default: 1.0 },
    bipolar: { type: Boolean, default: false },
    wheelStep: { type: Number, default: 0.05 },
});
const emit = defineEmits<{ (e: 'update:modelValue', v: number): void }>();

/// Sweep range: from -135° (min) to +135° (max), measured from 12 o'clock
/// (clockwise positive). 270° total of usable angle.
const SWEEP_DEG = 270;
const SWEEP_START = -135; // degrees relative to 12 o'clock

const norm = computed(() => (props.modelValue - props.min) / (props.max - props.min));
const atDefault = computed(() => Math.abs(props.modelValue - props.defaultValue) < 0.005);
const baselineNorm = computed(() => (props.defaultValue - props.min) / (props.max - props.min));

function angleAt(t: number) {
    return SWEEP_START + SWEEP_DEG * t;
}
function polar(angleDeg: number, radius: number) {
    // SVG: x right, y down. We want 0° at 12 o'clock, clockwise positive.
    const rad = ((angleDeg - 90) * Math.PI) / 180;
    return { x: 16 + radius * Math.cos(rad), y: 16 + radius * Math.sin(rad) };
}
function arc(fromDeg: number, toDeg: number, radius: number) {
    const start = polar(fromDeg, radius);
    const end = polar(toDeg, radius);
    const sweep = toDeg - fromDeg;
    const largeArc = Math.abs(sweep) > 180 ? 1 : 0;
    const dir = sweep >= 0 ? 1 : 0;
    return `M ${start.x.toFixed(2)} ${start.y.toFixed(2)} A ${radius} ${radius} 0 ${largeArc} ${dir} ${end.x.toFixed(2)} ${end.y.toFixed(2)}`;
}

const ringPath = computed(() => arc(SWEEP_START, SWEEP_START + SWEEP_DEG, 13));
const arcPath = computed(() => {
    const fromT = props.bipolar ? baselineNorm.value : 0;
    const toT = norm.value;
    const fromDeg = angleAt(fromT);
    const toDeg = angleAt(toT);
    if (Math.abs(toDeg - fromDeg) < 0.5) {
        // Avoid zero-length arcs (SVG renders them weirdly).
        return '';
    }
    return arc(fromDeg, toDeg, 13);
});

const pointerEnd = computed(() => polar(angleAt(norm.value), 8));
const pointerStart = computed(() => polar(angleAt(norm.value), 3));
const pointerX1 = computed(() => pointerStart.value.x);
const pointerY1 = computed(() => pointerStart.value.y);
const pointerX2 = computed(() => pointerEnd.value.x);
const pointerY2 = computed(() => pointerEnd.value.y);

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
    const dy = dragStartY.value - e.clientY;
    // Slower sensitivity than the fader: 200px = full range. Knobs reward fine control.
    const range = props.max - props.min;
    const next = clamp(dragStartValue.value + (dy / 200) * range, props.min, props.max);
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
function onReset() { emit('update:modelValue', props.defaultValue); }
function clamp(v: number, lo: number, hi: number) { return Math.max(lo, Math.min(hi, v)); }
</script>

<style lang='scss' scoped>
.dj-rknob {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    width: 36px;
    user-select: none;
    cursor: ns-resize;
}
.dj-rknob-svg {
    width: 32px;
    height: 32px;
    display: block;
}
.dj-rknob-ring {
    stroke: rgba(255, 255, 255, 0.10);
    stroke-width: 2;
    stroke-linecap: round;
}
.dj-rknob-arc {
    stroke: var(--color-accent);
    stroke-width: 2;
    stroke-linecap: round;
}
.dj-rknob--at-default .dj-rknob-arc { stroke: rgba(255, 255, 255, 0.30); }
.dj-rknob--bipolar .dj-rknob-arc { stroke: #ffc832; }
.dj-rknob-body {
    fill: rgba(0, 0, 0, 0.5);
    stroke: rgba(255, 255, 255, 0.18);
    stroke-width: 1;
}
.dj-rknob-pointer {
    stroke: var(--color-fg);
    stroke-width: 2;
    stroke-linecap: round;
}
.dj-rknob-label {
    font-family: var(--font-mono);
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--color-fg-subtle);
    user-select: none;
    line-height: 1;
}
.dj-rknob:hover .dj-rknob-body {
    stroke: var(--color-accent);
}
</style>
