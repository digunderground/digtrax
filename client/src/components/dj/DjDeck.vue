<template>
<!-- Single deck panel. Phase 1 ships:
       - Drop zone showing "Drop a track" placeholder when empty
       - Track meta (deck letter, title, artists)
       - Transport (play/pause/stop)
       - Volume fader
       - Position scrubber + time display
     Phase 2 adds the waveform; Phase 3 adds SYNC/MASTER; Phase 4 adds
     EQ + filter; Phase 5 adds beat-jump. -->
<div
    class='dj-deck'
    :class='{ "dj-deck--empty": !state.path, "dj-deck--drag": dragOver, "dj-deck--loading": state.loading }'
    @dragenter.prevent='onDragEnter'
    @dragover.prevent='onDragOver'
    @dragleave='onDragLeave'
    @drop.prevent='onDrop'
>
    <!-- Empty deck = drop zone -->
    <div v-if='!state.path' class='dj-deck-empty'>
        <q-icon name='mdi-tray-arrow-down' size='28px' class='dj-deck-empty-icon' />
        <div class='dj-deck-empty-label'>Drop track on Deck {{ id.toUpperCase() }}</div>
    </div>

    <!-- Loaded deck -->
    <template v-else>
        <div class='dj-deck-meta-row'>
            <div class='dj-deck-id'>{{ id.toUpperCase() }}</div>
            <div class='dj-deck-meta'>
                <div class='dj-deck-title' :title='state.title'>
                    {{ state.title || filename(state.path) }}
                    <span v-if='state.loading' class='dj-deck-loading'>decoding…</span>
                    <span v-else-if='state.analyzing' class='dj-deck-loading'>analyzing…</span>
                </div>
                <div class='dj-deck-artists'>{{ state.artists.join(', ') || '—' }}</div>
            </div>
            <div v-if='state.bpm > 0' class='dj-deck-bpm' :title='bpmTitle'>
                <span class='dj-deck-bpm-value'>{{ effectiveBpmStr }}</span>
                <span class='dj-deck-bpm-unit'>BPM</span>
            </div>
        </div>

        <div class='dj-deck-sync-row'>
            <button
                class='dj-deck-pill dj-deck-pill--sync'
                :class='{ "dj-deck-pill--on": state.syncOn }'
                :disabled='state.bpm <= 0'
                @click='toggleSync'
                :title='syncTitle'
            >
                <q-icon name='mdi-sync' size='12px' class='q-mr-xs' />
                SYNC
            </button>
            <button
                class='dj-deck-pill dj-deck-pill--master'
                :class='{ "dj-deck-pill--on": state.isLeader }'
                :disabled='state.bpm <= 0'
                @click='toggleMaster'
                :title='masterTitle'
            >
                <q-icon name='mdi-crown-outline' size='12px' class='q-mr-xs' />
                {{ state.isLeader ? 'MASTER' : 'SET MASTER' }}
            </button>
        </div>

        <DjWaveform :id='id' class='dj-deck-wave' />

        <div class='dj-deck-transport'>
            <q-btn
                round flat dense size='md'
                :icon='state.playing ? "mdi-pause" : "mdi-play"'
                class='dj-deck-play'
                :disable='state.loading'
                @click='togglePlay'
            />
            <q-btn
                round flat dense size='sm'
                icon='mdi-stop'
                class='dj-deck-stop'
                :disable='state.loading'
                @click='onStop'
            />
            <div class='dj-deck-volume'>
                <q-icon name='mdi-volume-medium' size='14px' class='q-mr-xs' />
                <q-slider
                    :model-value='state.userVolume'
                    @update:model-value='onVolume'
                    :min='0' :max='1' :step='0.01'
                    class='dj-deck-volume-slider'
                />
            </div>
            <span class='dj-deck-time'>{{ formatTime(state.position) }} / {{ formatTime(state.duration) }}</span>
        </div>

        <!-- Pitch / rate slider — vinyl-style. Center detent at 1.0 ±
             5%, double-click resets to 1.0. Range -50%..+50% covers
             the working DJ tempo-match range (Mixxx default).
             Pitch couples to speed for now; Phase 2c may add
             pitch-preserve. -->
        <div class='dj-deck-rate'>
            <button
                class='dj-deck-rate-label'
                :class='{ "dj-deck-rate-label--off": !rateAtNative }'
                @dblclick='onRateReset'
                :title='"Double-click to reset to 1.000×"'
            >{{ rateLabel }}</button>
            <q-slider
                :model-value='state.rate'
                @update:model-value='onRate'
                :min='0.5' :max='1.5' :step='0.001'
                class='dj-deck-rate-slider'
            />
        </div>
    </template>
</div>
</template>

<script lang='ts' setup>
import { computed, PropType, ref } from 'vue';
import DjWaveform from './DjWaveform.vue';
import {
    DeckId, djState,
    loadDeck, playDeck, pauseDeck, stopDeck, setDeckVolume, setDeckRate,
    setLeader, setSync,
} from '../../scripts/dj';

const props = defineProps({
    id: { required: true, type: String as PropType<DeckId> },
});

const state = computed(() =>
    props.id === 'a' ? djState.deckA : djState.deckB
);

const dragOver = ref(false);

function onDragEnter(e: DragEvent) {
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
    dragOver.value = true;
}
function onDragOver(e: DragEvent) {
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
    dragOver.value = true;
}
function onDragLeave(_e: DragEvent) { dragOver.value = false; }

/// Drop handler. Reads `application/x-digtrax-track` (rich payload from
/// Quick Tag / Tag Editor rows) or falls back to `text/plain` (raw path).
function onDrop(e: DragEvent) {
    dragOver.value = false;
    const dt = e.dataTransfer;
    if (!dt) return;
    const raw = dt.getData('application/x-digtrax-track') || dt.getData('text/plain');
    if (!raw) return;
    try {
        const payload = JSON.parse(raw);
        if (payload?.kind === 'digtrax-track' && typeof payload.path === 'string') {
            loadDeck(props.id, payload.path, payload.title, payload.artists);
            return;
        }
    } catch (_) { /* not JSON — try as path */ }
    // Plain string path fallback (legacy).
    if (typeof raw === 'string' && raw.startsWith('/')) {
        loadDeck(props.id, raw);
    }
}

function togglePlay() {
    if (state.value.playing) pauseDeck(props.id);
    else playDeck(props.id);
}
function onStop() { stopDeck(props.id); }
function onVolume(v: number | null) {
    setDeckVolume(props.id, typeof v === 'number' ? v : 0);
}

function onRate(v: number | null) {
    if (typeof v === 'number') setDeckRate(props.id, v);
}
function onRateReset() { setDeckRate(props.id, 1.0); }

/// SYNC: toggle this deck's follow flag. The audio thread will only
/// actually correct rate if (a) this is on, AND (b) some OTHER deck is
/// the leader, AND (c) both decks have analyzed beats. Backend confirms
/// the actual state via the next djPosition push.
function toggleSync() {
    setSync(props.id, !state.value.syncOn);
}

/// MASTER: make this deck the leader (or clear if already leader). The
/// other deck with SYNC on follows.
function toggleMaster() {
    if (state.value.isLeader) {
        setLeader(null);
    } else {
        setLeader(props.id);
    }
}

const syncTitle = computed(() => {
    if (state.value.bpm <= 0) return 'Waiting for beat analysis…';
    if (state.value.syncOn) return 'SYNC on — click to disengage';
    return 'Match this deck\'s tempo + phase to the LEADER deck';
});
const masterTitle = computed(() => {
    if (state.value.bpm <= 0) return 'Waiting for beat analysis…';
    if (state.value.isLeader) return 'This deck is the LEADER — click to clear';
    return 'Make this deck the tempo + phase LEADER';
});

const rateAtNative = computed(() => Math.abs(state.value.rate - 1.0) < 0.005);
const rateLabel = computed(() => {
    const r = state.value.rate || 1.0;
    if (rateAtNative.value) return '1.000×';
    return r.toFixed(3) + '×';
});

/// Effective playback BPM = file BPM × current rate. Shown in the
/// badge so the user can see what the deck is actually outputting
/// when the rate slider is engaged.
const effectiveBpmStr = computed(() => {
    const b = state.value.bpm || 0;
    const r = state.value.rate || 1.0;
    if (b <= 0) return '—';
    return (b * r).toFixed(1);
});
const bpmTitle = computed(() =>
    `Detected ${(state.value.bpm || 0).toFixed(2)} BPM × rate ${(state.value.rate || 1.0).toFixed(3)}× = effective ${(((state.value.bpm || 0) * (state.value.rate || 1.0))).toFixed(2)} BPM`
);

function filename(p?: string): string {
    if (!p) return '';
    const slashes = p.replace(/\\/g, '/');
    return slashes.slice(slashes.lastIndexOf('/') + 1);
}

function formatTime(ms: number): string {
    const s = Math.max(0, Math.round((ms || 0) / 1000));
    return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, '0')}`;
}
</script>

<style lang='scss' scoped>
.dj-deck {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm, 6px);
    background: rgba(255, 255, 255, 0.02);
    min-height: 200px; /* taller now that the deck owns the waveform */
}
.dj-deck--drag {
    border-color: var(--color-accent);
    background: rgba(0, 210, 191, 0.06);
}
.dj-deck--loading {
    opacity: 0.85;
}
.dj-deck-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--color-fg-muted);
    gap: 6px;
    pointer-events: none;
}
.dj-deck-empty-label {
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
}
.dj-deck-meta-row {
    display: flex;
    align-items: center;
    gap: 10px;
}
.dj-deck-id {
    width: 28px;
    height: 28px;
    border-radius: 4px;
    background: var(--color-accent);
    color: #001f1c;
    font-family: var(--font-mono);
    font-weight: 800;
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}
.dj-deck-meta {
    flex: 1;
    min-width: 0;
}
.dj-deck-bpm {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    padding: 3px 10px;
    border-radius: var(--radius-full, 9999px);
    border: 1px solid var(--color-accent);
    background: rgba(0, 210, 191, 0.10);
    box-shadow: 0 0 6px rgba(0, 210, 191, 0.18);
    flex-shrink: 0;
}
.dj-deck-bpm-value {
    font-family: var(--font-mono);
    font-weight: 800;
    font-size: 14px;
    color: var(--color-accent);
}
.dj-deck-bpm-unit {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-accent);
    opacity: 0.7;
}
.dj-deck-wave {
    flex: 1;
    min-height: 0;
}

.dj-deck-sync-row {
    display: flex;
    gap: 6px;
    align-items: center;
}
.dj-deck-pill {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-full, 9999px);
    background: transparent;
    color: var(--color-fg-muted);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition: all var(--duration-fast, 120ms) var(--ease-standard, ease);
    white-space: nowrap;
}
.dj-deck-pill:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}
.dj-deck-pill:not(:disabled):hover {
    color: var(--color-fg);
    border-color: var(--color-border-strong);
    background: rgba(255, 255, 255, 0.04);
}
.dj-deck-pill--on {
    color: #001f1c !important;
    background: var(--color-accent) !important;
    border-color: var(--color-accent) !important;
    box-shadow: 0 0 8px var(--color-accent-glow);
}

.dj-deck-rate {
    display: flex;
    align-items: center;
    gap: 8px;
}
.dj-deck-rate-label {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 2px 8px;
    border-radius: var(--radius-full, 9999px);
    border: 1px solid var(--color-border);
    background: transparent;
    color: var(--color-fg-muted);
    cursor: pointer;
    min-width: 54px;
    text-align: center;
    transition: all var(--duration-fast, 120ms) var(--ease-standard, ease);
}
.dj-deck-rate-label--off {
    color: var(--color-accent);
    border-color: var(--color-accent);
}
.dj-deck-rate-slider {
    flex: 1;
}
.dj-deck-title {
    font-weight: 700;
    color: var(--color-fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.dj-deck-loading {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-fg-subtle);
    margin-left: 6px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
}
.dj-deck-artists {
    font-size: 12px;
    color: var(--color-fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.dj-deck-transport {
    display: flex;
    align-items: center;
    gap: 6px;
}
.dj-deck-play {
    color: var(--color-accent) !important;
}
.dj-deck-volume {
    display: flex;
    align-items: center;
    flex: 1;
    gap: 4px;
    color: var(--color-fg-muted);
}
.dj-deck-volume-slider {
    flex: 1;
}
.dj-deck-scrub {
    display: flex;
    align-items: center;
    gap: 8px;
}
.dj-deck-time {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-fg-muted);
    min-width: 36px;
    text-align: center;
}
.dj-deck-scrub-slider {
    flex: 1;
}
</style>
