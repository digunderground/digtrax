<template>
<!-- Mixxx-style deck panel. Top-down stack:
       Header  : artwork | meta | times | BPM | deck letter
       Sync row: SYNC + MASTER + beatjump
       Waveform: scrolling main + mini overview
       Transport: play / pause / stop, time label
     This whole thing also doubles as the drag-target — drop a track
     anywhere on the panel to load it. -->
<div
    class='dj-deck'
    :class='{ "dj-deck--empty": !state.path, "dj-deck--drag": dragOver, "dj-deck--loading": state.loading, "dj-deck--mirror": id === "b" }'
    @dragenter.prevent='onDragEnter'
    @dragover.prevent='onDragOver'
    @dragleave='onDragLeave'
    @drop.prevent='onDrop'
>
    <div v-if='!state.path' class='dj-deck-empty'>
        <q-icon name='mdi-tray-arrow-down' size='28px' class='dj-deck-empty-icon' />
        <div class='dj-deck-empty-label'>Drop track on Deck {{ id.toUpperCase() }}</div>
    </div>

    <template v-else>
        <!-- Header strip: art / meta / time / BPM / deck letter -->
        <div class='dj-deck-header'>
            <q-img
                :src='artUrl'
                class='dj-deck-art'
                :placeholder-src='PLACEHOLDER_IMG'
            >
                <template v-slot:error>
                    <q-img :src='PLACEHOLDER_IMG' class='dj-deck-art' />
                </template>
            </q-img>
            <div class='dj-deck-meta'>
                <div class='dj-deck-title' :title='state.title'>
                    {{ state.title || filename(state.path) }}
                </div>
                <div class='dj-deck-artist'>{{ state.artists.join(', ') || '—' }}</div>
            </div>
            <div class='dj-deck-times'>
                <div class='dj-deck-time-elapsed'>-{{ formatTime(remainingMs) }}</div>
                <div class='dj-deck-time-total'>{{ formatTime(state.duration) }}</div>
            </div>
            <div v-if='state.bpm > 0' class='dj-deck-bpm-block' :title='bpmTitle'>
                <div class='dj-deck-bpm-value'>{{ effectiveBpmStr }}</div>
                <div v-if='ratePctStr' class='dj-deck-rate-pct' :class='ratePctClass'>{{ ratePctStr }}</div>
            </div>
            <div class='dj-deck-id'>{{ id.toUpperCase() }}</div>
        </div>

        <!-- SYNC + MASTER + beatjump  (mirrored on deck B) -->
        <div class='dj-deck-sync-row'>
            <button
                class='dj-deck-pill dj-deck-pill--sync'
                :class='{ "dj-deck-pill--on": state.syncOn }'
                :disabled='state.bpm <= 0'
                @click='toggleSync'
                :title='syncTitle'
            >SYNC</button>
            <button
                class='dj-deck-pill dj-deck-pill--master'
                :class='{ "dj-deck-pill--on": state.isLeader }'
                :disabled='state.bpm <= 0'
                @click='toggleMaster'
                :title='masterTitle'
            >MASTER</button>
            <!-- KEY LOCK toggle (Mixxx-equivalent). Default ON: tempo
                 changes preserve pitch via RubberBand. OFF: tempo and
                 pitch couple (vinyl-style). Always usable — doesn't
                 depend on beat analysis. -->
            <button
                class='dj-deck-pill dj-deck-pill--key'
                :class='{ "dj-deck-pill--on": state.keyLock }'
                @click='toggleKeyLock'
                :title='keyLockTitle'
            >KEY</button>
            <div class='dj-deck-jump' :class='{ "dj-deck-jump--disabled": !state.beats.length }'>
                <button class='dj-deck-jump-btn' :disabled='!state.beats.length' @click='onJump(-4)' title='-4 beats'>⟪4</button>
                <button class='dj-deck-jump-btn' :disabled='!state.beats.length' @click='onJump(-1)' title='-1 beat'>⟨1</button>
                <button class='dj-deck-jump-btn' :disabled='!state.beats.length' @click='onJump(1)'  title='+1 beat'>1⟩</button>
                <button class='dj-deck-jump-btn' :disabled='!state.beats.length' @click='onJump(4)'  title='+4 beats'>4⟫</button>
            </div>
            <!-- Beat-grid translate (Mixxx beats_translate_*). Lets the
                 user fix kick-vs-snare (½) or nudge a few ms either way. -->
            <div class='dj-deck-translate' :class='{ "dj-deck-translate--disabled": !state.beats.length }' :title='translateTitle'>
                <button class='dj-deck-translate-btn' :disabled='!state.beats.length' @click='onTranslate(-10)' title='Nudge grid 10 ms earlier'>‹</button>
                <button class='dj-deck-translate-btn dj-deck-translate-btn--half' :disabled='!state.beats.length' @click='onHalfBeat' title='Shift grid by ½ beat (kick ↔ snare)'>½</button>
                <button class='dj-deck-translate-btn' :disabled='!state.beats.length' @click='onTranslate(10)' title='Nudge grid 10 ms later'>›</button>
            </div>
        </div>

        <!-- Waveform + per-deck tempo slider on the right (Mixxx
             convention). Slider is read-only on a SYNC follower:
             the sync engine writes the rate atomic ~100×/sec, and a
             user drag would just be overwritten. The follower's
             slider still moves visually so the user can see what
             the engine is doing. -->
        <div class='dj-deck-wave-row'>
            <DjWaveform :id='id' class='dj-deck-wave' />
            <div class='dj-deck-tempo' :class='{ "dj-deck-tempo--follower": isFollower }'>
                <div class='dj-deck-tempo-pct' :class='ratePctClass'>{{ ratePctStr || "0.0%" }}</div>
                <input
                    type='range'
                    class='dj-deck-tempo-slider'
                    :value='state.rate'
                    :min='0.92' :max='1.08' :step='0.0005'
                    :disabled='isFollower'
                    @input='onTempo'
                    @dblclick='onTempoReset'
                    :title='isFollower
                        ? "SYNC is following the leader — set this deck as MASTER to drive tempo"
                        : "Drag for tempo (±8%); double-click to reset to native tempo"'
                />
            </div>
        </div>

        <!-- Transport row: play / pause + analyzing / loading hint -->
        <div class='dj-deck-transport'>
            <q-btn
                round dense unelevated
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
            <div class='dj-deck-status'>
                <span v-if='state.loading' class='dj-deck-loading-text'>decoding…</span>
                <span v-else-if='state.analyzing' class='dj-deck-loading-text'>analyzing beats…</span>
                <span v-else class='dj-deck-pos-text'>{{ formatTime(state.position) }}</span>
            </div>
        </div>
    </template>
</div>
</template>

<script lang='ts' setup>
import { computed, PropType, ref } from 'vue';
import DjWaveform from './DjWaveform.vue';
import {
    DeckId, djState,
    loadDeck, playDeck, pauseDeck, stopDeck,
    setLeader, setSync, setKeyLock, beatJump, setDeckRate, translateBeats,
} from '../../scripts/dj';
import { PLACEHOLDER_IMG } from '../../scripts/quicktag';
import { httpUrl } from '../../scripts/utils';

const props = defineProps({
    id: { required: true, type: String as PropType<DeckId> },
});

const state = computed(() =>
    props.id === 'a' ? djState.deckA : djState.deckB
);

const dragOver = ref(false);

const artUrl = computed(() =>
    state.value.path
        ? `${httpUrl()}/thumb?path=${encodeURIComponent(state.value.path)}`
        : ''
);

function onDragEnter(e: DragEvent) {
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
    dragOver.value = true;
}
function onDragOver(e: DragEvent) {
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
    dragOver.value = true;
}
function onDragLeave(_e: DragEvent) { dragOver.value = false; }
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
    } catch (_) { /* not JSON */ }
    if (typeof raw === 'string' && raw.startsWith('/')) loadDeck(props.id, raw);
}

function togglePlay() {
    if (state.value.playing) pauseDeck(props.id);
    else playDeck(props.id);
}
function onStop() { stopDeck(props.id); }
function onJump(beats: number) { beatJump(props.id, beats); }

/// Beat-grid translation handlers (Mixxx beats_translate_*).
/// onTranslate(±10) is the small earlier/later nudge.
/// onHalfBeat shifts by exactly half a beat = the kick/snare flip.
function onTranslate(offsetMs: number) {
    translateBeats(props.id, offsetMs);
}
function onHalfBeat() {
    if (!state.value.bpm || state.value.bpm <= 0) return;
    const halfBeatMs = (60_000 / state.value.bpm) / 2;
    translateBeats(props.id, halfBeatMs);
}
const translateTitle = computed(() => state.value.beats.length
    ? 'Beat-grid: ‹ nudge -10ms · ½ shift half-beat (kick↔snare) · › nudge +10ms'
    : 'Beat-grid translate available after analysis');
function toggleSync() { setSync(props.id, !state.value.syncOn); }
function toggleMaster() {
    if (state.value.isLeader) setLeader(null);
    else setLeader(props.id);
}
function toggleKeyLock() { setKeyLock(props.id, !state.value.keyLock); }
const keyLockTitle = computed(() => state.value.keyLock
    ? 'KEY LOCK on — tempo changes preserve pitch (RubberBand)'
    : 'KEY LOCK off — pitch follows tempo (vinyl-style)');

/// Tempo slider input handler. Disabled on followers (the sync engine
/// owns their rate atomic), but otherwise sends djRate. Double-click
/// resets to native tempo (1.0).
function onTempo(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    if (Number.isFinite(v)) setDeckRate(props.id, v);
}
function onTempoReset() { setDeckRate(props.id, 1.0); }

/// True when this deck is following another deck via SYNC. The tempo
/// slider goes read-only because the sync engine writes the rate
/// atomic every audio buffer.
const isFollower = computed(() => state.value.syncOn && !state.value.isLeader);

const remainingMs = computed(() =>
    Math.max(0, state.value.duration - state.value.position)
);

const effectiveBpmStr = computed(() => {
    const b = state.value.bpm || 0;
    const r = state.value.rate || 1.0;
    if (b <= 0) return '—';
    return (b * r).toFixed(2);
});

const ratePctStr = computed(() => {
    const r = state.value.rate || 1.0;
    if (Math.abs(r - 1.0) < 0.001) return '';
    const pct = (r - 1.0) * 100;
    return `${pct >= 0 ? '+' : ''}${pct.toFixed(1)}%`;
});
const ratePctClass = computed(() => {
    const r = state.value.rate || 1.0;
    if (r > 1.0) return 'dj-deck-rate-pct--up';
    if (r < 1.0) return 'dj-deck-rate-pct--down';
    return '';
});

const bpmTitle = computed(() =>
    `Detected ${(state.value.bpm || 0).toFixed(2)} BPM × rate ${(state.value.rate || 1.0).toFixed(3)}× = ${(((state.value.bpm || 0) * (state.value.rate || 1.0))).toFixed(2)} BPM`
);
const syncTitle = computed(() => {
    if (state.value.bpm <= 0) return 'Waiting for beat analysis…';
    if (state.value.syncOn) return 'SYNC on — click to disengage';
    return 'Sync to MASTER deck\'s tempo + phase';
});
const masterTitle = computed(() => {
    if (state.value.bpm <= 0) return 'Waiting for beat analysis…';
    if (state.value.isLeader) return 'This deck is the LEADER — click to clear';
    return 'Make this deck the tempo + phase LEADER';
});

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
    gap: 4px;
    padding: 6px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm, 6px);
    background: rgba(255, 255, 255, 0.02);
    /* Just enough vertical space for the header + sync row + waveform
       (96px main + 22px mini) + transport row, no slack. */
    min-height: 210px;
}
.dj-deck--drag {
    border-color: var(--color-accent);
    background: rgba(0, 210, 191, 0.06);
}
.dj-deck--loading { opacity: 0.85; }
/* Mirror deck B layout where the deck letter sits on the right (Mixxx
   convention: A on left, B on right). For now both decks use the same
   header order but flag is here for further mirroring polish. */
.dj-deck--mirror {}

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

/* Header — artwork / meta / times / BPM / deck letter */
.dj-deck-header {
    display: grid;
    grid-template-columns: 44px 1fr auto auto auto;
    grid-template-rows: auto;
    gap: 8px;
    align-items: center;
}
.dj-deck-art {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-xs, 4px);
    flex-shrink: 0;
    border: 1px solid var(--color-border);
}
.dj-deck-meta {
    min-width: 0;
}
.dj-deck-title {
    font-weight: 700;
    color: var(--color-fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 13px;
}
.dj-deck-artist {
    font-size: 11px;
    color: var(--color-fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.dj-deck-times {
    text-align: right;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-fg-muted);
    line-height: 1.3;
}
.dj-deck-time-elapsed { color: var(--color-fg); font-weight: 700; }
.dj-deck-bpm-block {
    text-align: right;
    line-height: 1.1;
}
.dj-deck-bpm-value {
    font-family: var(--font-mono);
    font-weight: 800;
    font-size: 16px;
    color: var(--color-accent);
}
.dj-deck-rate-pct {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
}
.dj-deck-rate-pct--up   { color: #ff5a4b; }
.dj-deck-rate-pct--down { color: #5ab9ff; }
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
}

/* Sync + master + beatjump row */
.dj-deck-sync-row {
    display: flex;
    align-items: center;
    gap: 6px;
}
.dj-deck-pill {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xs, 3px);
    background: transparent;
    color: var(--color-fg-muted);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    cursor: pointer;
    transition: all var(--duration-fast, 120ms) ease;
}
.dj-deck-pill:disabled { opacity: 0.4; cursor: not-allowed; }
.dj-deck-pill:not(:disabled):hover {
    color: var(--color-fg);
    border-color: var(--color-border-strong);
}
.dj-deck-pill--on {
    color: #001f1c !important;
    background: var(--color-accent) !important;
    border-color: var(--color-accent) !important;
}
.dj-deck-pill--master.dj-deck-pill--on {
    background: #ff8c2e !important;
    border-color: #ff8c2e !important;
    color: #1a0e00 !important;
}
/* KEY LOCK pill: same shape, distinct sky-blue when on so users
   can distinguish it from SYNC's mint and MASTER's orange at a
   glance. Default state is ON; we render that as the active fill. */
.dj-deck-pill--key.dj-deck-pill--on {
    background: var(--t-sky, #38BDF8) !important;
    border-color: var(--t-sky, #38BDF8) !important;
    color: #002030 !important;
}

.dj-deck-jump {
    display: inline-flex;
    gap: 2px;
    margin-left: auto;
}
.dj-deck-jump--disabled { opacity: 0.4; }
.dj-deck-jump-btn {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: var(--radius-xs, 3px);
    border: 1px solid var(--color-border);
    background: transparent;
    color: var(--color-fg-muted);
    cursor: pointer;
}
.dj-deck-jump-btn:disabled { cursor: not-allowed; }
.dj-deck-jump-btn:not(:disabled):hover {
    color: var(--color-fg);
    border-color: var(--color-accent);
    background: rgba(0, 210, 191, 0.08);
}

/* Beat-grid translate (Mixxx beats_translate_* family). Subtle
   amber tint distinguishes them from beat-jump (which moves the
   playhead) since these move the GRID instead. */
.dj-deck-translate {
    display: inline-flex;
    gap: 2px;
    margin-left: 4px;
}
.dj-deck-translate--disabled { opacity: 0.4; }
.dj-deck-translate-btn {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
    min-width: 18px;
    border-radius: var(--radius-xs, 3px);
    border: 1px solid var(--color-border);
    background: transparent;
    color: var(--color-fg-muted);
    cursor: pointer;
    transition: all var(--duration-fast, 120ms) ease;
}
.dj-deck-translate-btn:disabled { cursor: not-allowed; }
.dj-deck-translate-btn:not(:disabled):hover {
    color: #1a0e00;
    border-color: #ffc832;
    background: #ffc832;
}
.dj-deck-translate-btn--half {
    font-weight: 800;
    color: #ffc832;
    border-color: rgba(255, 200, 50, 0.5);
}
.dj-deck-translate-btn--half:not(:disabled):hover {
    color: #1a0e00;
    background: #ffc832;
    border-color: #ffc832;
}

.dj-deck-wave-row {
    display: flex;
    flex: 1;
    gap: 6px;
    min-height: 0;
}
.dj-deck-wave {
    flex: 1;
    min-height: 0;
    min-width: 0;
}

/* Per-deck tempo slider (Mixxx-style "pitch fader"). Vertical, ±8%
   range. Double-click center-detents to native tempo. Disabled
   visual when the deck is a sync follower — the sync engine drives
   the rate atomic and a user drag would be instantly overwritten. */
.dj-deck-tempo {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 38px;
    flex-shrink: 0;
    gap: 2px;
}
.dj-deck-tempo-pct {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--color-fg-muted);
    line-height: 1;
}
.dj-deck-tempo-slider {
    -webkit-appearance: slider-vertical;
    appearance: slider-vertical;
    writing-mode: vertical-lr;
    direction: rtl;
    flex: 1;
    width: 14px;
    background: rgba(0, 0, 0, 0.5);
    border: 1px solid var(--color-border);
    border-radius: 2px;
    cursor: ns-resize;
    accent-color: var(--color-accent);
    margin: 0;
}
.dj-deck-tempo--follower .dj-deck-tempo-slider {
    cursor: not-allowed;
    opacity: 0.7;
    accent-color: #ffc832; /* amber — "this is being driven by sync" */
}
.dj-deck-tempo--follower .dj-deck-tempo-pct {
    color: #ffc832;
}

/* Transport row */
.dj-deck-transport {
    display: flex;
    align-items: center;
    gap: 6px;
}
.dj-deck-play   { color: var(--color-accent) !important; }
.dj-deck-status {
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-fg-muted);
}
.dj-deck-loading-text {
    color: var(--color-fg-subtle);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 10px;
}
.dj-deck-pos-text {
    color: var(--color-fg);
}
</style>
