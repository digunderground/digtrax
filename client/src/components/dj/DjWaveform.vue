<template>
<!-- Per-deck waveform renderer.
       MAIN  — scrolling 30-second zoom. Playhead anchored at 1/3 of
               the canvas width so the user sees ~10 s of past +
               ~20 s of upcoming material. Each bar is a mirrored
               stack of three bands (low/mid/high → red/teal/blue).
               Yellow ticks every 4 beats (bar boundary), bright cyan
               every 16 beats (phrase boundary). Click to seek.
       MINI  — full-track overview. Translucent box marks the visible
               window of the main view. Drag to scrub.
     Both canvases are HiDPI-aware. -->
<div class='dj-wave-stack'>
    <canvas
        ref='mainCanvas'
        class='dj-waveform dj-waveform--main'
        :class='{ "dj-waveform--empty": !state.spectrum.length || !state.duration, "dj-waveform--analyzing": state.analyzing }'
        :title='formatTime(state.position) + " / " + formatTime(state.duration)'
        @mousemove='onMainMove'
        @mouseleave='hover = false'
        @click='onMainClick'
        @wheel.prevent='onWheel'
    />
    <canvas
        ref='miniCanvas'
        class='dj-waveform dj-waveform--mini'
        :class='{ "dj-waveform--empty": !state.spectrum.length || !state.duration }'
        @mousedown='onMiniDown'
    />
</div>
</template>

<script lang='ts' setup>
import { computed, onMounted, onUnmounted, PropType, ref, watch, nextTick } from 'vue';
import { djState, DeckId, seekDeck, zoomIn, zoomOut, type DeckState } from '../../scripts/dj';

const props = defineProps({
    id: { required: true, type: String as PropType<DeckId> },
});
const state = computed<DeckState>(() =>
    props.id === 'a' ? djState.deckA : djState.deckB
);

const mainCanvas = ref<HTMLCanvasElement | null>(null);
const miniCanvas = ref<HTMLCanvasElement | null>(null);
const hover = ref(false);
const hoverPos = ref(0);

/// Where the playhead sits horizontally within the main canvas (0..1).
/// 1/3 = past on left + ahead on right — Mixxx's default.
const PLAYHEAD_RATIO = 1 / 3;

/// Wheel handler: scroll up = zoom IN (less time visible / more detail),
/// scroll down = zoom OUT. Ignores horizontal scroll. Both decks share
/// the same zoom value so the user can compare at the same scale.
function onWheel(e: WheelEvent) {
    if (Math.abs(e.deltaY) < 1) return;
    if (e.deltaY < 0) zoomIn(); else zoomOut();
}

const BAND_PLAYED = [
    'rgba(255, 90, 75, 1.0)',    // low — red/orange (kicks)
    'rgba(0, 230, 200, 1.0)',    // mid — accent teal (vox/snare)
    'rgba(140, 190, 255, 1.0)',  // high — cool blue (hats)
];
const BAND_UNPLAYED = [
    'rgba(220, 75, 60, 0.85)',
    'rgba(0, 200, 175, 0.85)',
    'rgba(120, 170, 235, 0.85)',
];

function drawBar(
    ctx: CanvasRenderingContext2D, x: number, barWidth: number,
    h: number, band: number[], played: boolean, minHeight: number,
) {
    const lo = Math.max(0, Math.min(1, band[0] || 0));
    const mid = Math.max(0, Math.min(1, band[1] || 0));
    const hi = Math.max(0, Math.min(1, band[2] || 0));
    const palette = played ? BAND_PLAYED : BAND_UNPLAYED;
    const totalAmp = Math.max(lo, mid, hi);
    const totalH = Math.max(minHeight, totalAmp * h);
    const sumAmp = lo + mid + hi;
    const loH = sumAmp > 0 ? (lo / sumAmp) * totalH : totalH;
    const midH = sumAmp > 0 ? (mid / sumAmp) * totalH : 0;
    const hiH = sumAmp > 0 ? (hi / sumAmp) * totalH : 0;
    const cy = h / 2;
    const halfLo = loH / 2;
    const halfMid = midH / 2;
    const halfHi = hiH / 2;
    ctx.fillStyle = palette[0];
    ctx.fillRect(x, cy - halfLo, barWidth, halfLo * 2);
    ctx.fillStyle = palette[1];
    ctx.fillRect(x, cy - halfLo - halfMid, barWidth, halfMid);
    ctx.fillRect(x, cy + halfLo, barWidth, halfMid);
    ctx.fillStyle = palette[2];
    ctx.fillRect(x, cy - halfLo - halfMid - halfHi, barWidth, halfHi);
    ctx.fillRect(x, cy + halfLo + halfMid, barWidth, halfHi);
}

function prep(canvas: HTMLCanvasElement): { ctx: CanvasRenderingContext2D, w: number, h: number } | null {
    const rect = canvas.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return null;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(rect.width * dpr);
    canvas.height = Math.round(rect.height * dpr);
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, rect.width, rect.height);
    return { ctx, w: rect.width, h: rect.height };
}

function drawMain() {
    const c = mainCanvas.value;
    if (!c) return;
    const p = prep(c);
    if (!p) return;
    const { ctx, w, h } = p;
    const bars = state.value.spectrum;
    if (!bars.length || !state.value.duration) return;

    const bps = state.value.spectrumBarsPerSec || 10;
    const windowDurationMs = djState.zoomSeconds * 1000;
    const windowStartMs = state.value.position - PLAYHEAD_RATIO * windowDurationMs;
    const windowEndMs = windowStartMs + windowDurationMs;
    const pxPerMs = w / windowDurationMs;
    const msPerBar = 1000 / bps;

    const firstBar = Math.max(0, Math.floor(windowStartMs / msPerBar));
    const lastBar = Math.min(bars.length - 1, Math.ceil(windowEndMs / msPerBar));
    const minHeight = Math.max(2, h * 0.06);
    const barWidth = Math.max(1, msPerBar * pxPerMs - 0.5);
    const playedX = PLAYHEAD_RATIO * w;

    for (let i = firstBar; i <= lastBar; i++) {
        const barStartMs = i * msPerBar;
        const x = (barStartMs - windowStartMs) * pxPerMs;
        const cx = x + barWidth / 2;
        const played = cx <= playedX;
        drawBar(ctx, x, barWidth, h, bars[i] || [0, 0, 0], played, minHeight);
    }

    // Beat grid — render directly from the detected beats array. By
    // construction the markers land where the algorithm found beats,
    // so any visible drift between yellow lines and red kicks IS the
    // detection error (no compounding from a synthetic grid).
    const beats = state.value.beats;
    if (beats && beats.length > 1) {
        ctx.save();
        // Binary-search the first visible beat — at 30s zoom × ~140 BPM
        // we have 70 beats visible out of potentially 1500+ total.
        let lo = 0, hi = beats.length;
        while (lo < hi) {
            const mid = (lo + hi) >>> 1;
            if (beats[mid] < windowStartMs) lo = mid + 1; else hi = mid;
        }
        for (let k = lo; k < beats.length; k++) {
            const t = beats[k];
            if (t >= windowEndMs) break;
            const bx = (t - windowStartMs) * pxPerMs;
            const isPhrase = (k % 16) === 0;
            const isBar = (k % 4) === 0;
            if (isPhrase) {
                ctx.fillStyle = 'rgba(0, 230, 255, 0.95)';
                ctx.fillRect(bx - 1, 0, 2.5, h);
            } else if (isBar) {
                ctx.fillStyle = 'rgba(255, 200, 50, 0.9)';
                ctx.fillRect(bx - 0.5, 0, 1.6, h);
            } else {
                ctx.fillStyle = 'rgba(255, 255, 255, 0.32)';
                ctx.fillRect(bx, 0, 1, h);
            }
        }
        ctx.restore();
    }

    // Hover cursor.
    if (hover.value) {
        ctx.fillStyle = 'rgba(255, 255, 255, 0.55)';
        ctx.fillRect(hoverPos.value * w - 1, 0, 2, h);
    }

    // Playhead.
    ctx.fillStyle = '#00E6C8';
    ctx.fillRect(playedX - 1, 0, 2, h);
}

function drawMini() {
    const c = miniCanvas.value;
    if (!c) return;
    const p = prep(c);
    if (!p) return;
    const { ctx, w, h } = p;
    const bars = state.value.spectrum;
    if (!bars.length || !state.value.duration) return;

    const minHeight = Math.max(1, h * 0.1);
    const cellWidth = w / bars.length;
    const playedRatio = state.value.duration > 0
        ? Math.max(0, Math.min(1, state.value.position / state.value.duration))
        : 0;
    const playedX = playedRatio * w;

    if (bars.length > w * 1.5) {
        // Many more bars than pixels — group `step` bars per pixel column.
        const step = Math.ceil(bars.length / w);
        const cols = Math.ceil(bars.length / step);
        const colWidth = w / cols;
        const barWidth = Math.max(1, colWidth);
        for (let col = 0; col < cols; col++) {
            const start = col * step;
            const end = Math.min(bars.length, start + step);
            let lo = 0, mid = 0, hi = 0;
            for (let i = start; i < end; i++) {
                const b = bars[i] || [0, 0, 0];
                if (b[0] > lo) lo = b[0];
                if (b[1] > mid) mid = b[1];
                if (b[2] > hi) hi = b[2];
            }
            const x = col * colWidth;
            const cx = x + barWidth / 2;
            const played = cx <= playedX;
            drawBar(ctx, x, barWidth, h, [lo, mid, hi], played, minHeight);
        }
    } else {
        const barWidth = Math.max(1, cellWidth);
        for (let i = 0; i < bars.length; i++) {
            const x = i * cellWidth;
            const cx = x + barWidth / 2;
            const played = cx <= playedX;
            drawBar(ctx, x, barWidth, h, bars[i] || [0, 0, 0], played, minHeight);
        }
    }

    // Visible-window overlay.
    const windowDurationMs = djState.zoomSeconds * 1000;
    const windowStartMs = state.value.position - PLAYHEAD_RATIO * windowDurationMs;
    const winX = (windowStartMs / state.value.duration) * w;
    const winW = (windowDurationMs / state.value.duration) * w;
    ctx.save();
    ctx.fillStyle = 'rgba(0, 230, 200, 0.10)';
    ctx.fillRect(winX, 0, winW, h);
    ctx.strokeStyle = 'rgba(0, 230, 200, 0.55)';
    ctx.lineWidth = 1;
    ctx.strokeRect(winX + 0.5, 0.5, winW - 1, h - 1);
    ctx.restore();

    ctx.fillStyle = '#00E6C8';
    ctx.fillRect(playedX - 0.5, 0, 1, h);
}

function redraw() { drawMain(); drawMini(); }

function onMainClick(e: MouseEvent) {
    const c = mainCanvas.value;
    if (!c || !state.value.duration) return;
    const rect = c.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const windowDurationMs = djState.zoomSeconds * 1000;
    const windowStartMs = state.value.position - PLAYHEAD_RATIO * windowDurationMs;
    const target = windowStartMs + ratio * windowDurationMs;
    const clamped = Math.max(0, Math.min(state.value.duration, target));
    seekDeck(props.id, Math.round(clamped));
}

function onMainMove(e: MouseEvent) {
    const c = mainCanvas.value;
    if (!c) return;
    const rect = c.getBoundingClientRect();
    hoverPos.value = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    hover.value = true;
    drawMain();
}

const miniScrubbing = ref(false);
function onMiniDown(e: MouseEvent) {
    miniScrubbing.value = true;
    seekFromMini(e);
    window.addEventListener('mousemove', onMiniScrub);
    window.addEventListener('mouseup', onMiniUp, { once: true });
}
function onMiniScrub(e: MouseEvent) {
    if (!miniScrubbing.value) return;
    seekFromMini(e);
}
function onMiniUp() {
    miniScrubbing.value = false;
    window.removeEventListener('mousemove', onMiniScrub);
}
function seekFromMini(e: MouseEvent) {
    const c = miniCanvas.value;
    if (!c || !state.value.duration) return;
    const rect = c.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    seekDeck(props.id, Math.round(ratio * state.value.duration));
}

function formatTime(ms: number): string {
    const s = Math.max(0, Math.round((ms || 0) / 1000));
    return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, '0')}`;
}

watch(() => state.value.spectrum, () => redraw(), { flush: 'post', deep: false });
watch(() => state.value.position, () => redraw(), { flush: 'post' });
watch(() => state.value.duration, () => redraw(), { flush: 'post' });
watch(() => state.value.beats, () => redraw(), { flush: 'post', deep: false });
watch(() => djState.zoomSeconds, () => redraw(), { flush: 'post' });

let ro: ResizeObserver | null = null;
onMounted(() => {
    nextTick(() => redraw());
    if (typeof ResizeObserver !== 'undefined') {
        ro = new ResizeObserver(() => redraw());
        if (mainCanvas.value) ro.observe(mainCanvas.value);
        if (miniCanvas.value) ro.observe(miniCanvas.value);
    }
});
onUnmounted(() => {
    if (ro) ro.disconnect();
    window.removeEventListener('mousemove', onMiniScrub);
});
</script>

<style lang='scss' scoped>
.dj-wave-stack {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    min-width: 0;
}
.dj-waveform {
    width: 100%;
    cursor: pointer;
    border-radius: var(--radius-xs, 4px);
    background-color: rgba(0, 0, 0, 0.35);
    display: block;
}
.dj-waveform--main {
    height: 96px;
    cursor: crosshair;
}
.dj-waveform--mini {
    height: 22px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    cursor: ew-resize;
}
.dj-waveform--empty {
    cursor: default;
    opacity: 0.4;
}
.dj-waveform--analyzing {
    animation: dj-wave-pulse 1.6s ease-in-out infinite;
}
@keyframes dj-wave-pulse {
    0%, 100% { opacity: 0.6; }
    50%      { opacity: 1.0; }
}
</style>
