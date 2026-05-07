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
                    <span v-if='state.loading' class='dj-deck-loading'>analyzing…</span>
                </div>
                <div class='dj-deck-artists'>{{ state.artists.join(', ') || '—' }}</div>
            </div>
        </div>

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
        </div>

        <div class='dj-deck-scrub'>
            <span class='dj-deck-time'>{{ formatTime(state.position) }}</span>
            <q-slider
                :model-value='scrubValue'
                @update:model-value='onScrub'
                :min='0' :max='state.duration || 1' :step='100'
                class='dj-deck-scrub-slider'
            />
            <span class='dj-deck-time'>{{ formatTime(state.duration) }}</span>
        </div>
    </template>
</div>
</template>

<script lang='ts' setup>
import { computed, PropType, ref } from 'vue';
import {
    DeckId, djState,
    loadDeck, playDeck, pauseDeck, stopDeck, seekDeck, setDeckVolume,
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
function onScrub(v: number | null) {
    if (typeof v === 'number') seekDeck(props.id, v);
}

const scrubValue = computed(() => state.value.position || 0);

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
    min-height: 130px;
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
