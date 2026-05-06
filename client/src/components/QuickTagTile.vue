<template>
<div class='qt-card-wrap' :style='{ "--track-c": trackC, "--track-glow": trackGlow }'>
    <q-card flat class='qt-card' :class='{"qt-card-selected": selected}'>

        <!-- Cover area: aspect-locked, gradient fallback, title + bpm/key overlays -->
        <div class='qt-card-cover'>
            <q-img
                :src='art'
                class='qt-card-art'
                :placeholder-src='PLACEHOLDER_IMG'
                no-spinner
            >
                <template v-slot:error>
                    <div class='qt-card-art-fallback'></div>
                </template>
            </q-img>

            <!-- Gradient scrim for legibility of overlays -->
            <div class='qt-card-cover-scrim'></div>

            <!-- Title overlay (uppercase, V4 mono) -->
            <div class='qt-card-title-overlay'>
                <div class='qt-card-title-text'>{{ track.title }}</div>
            </div>

            <!-- BPM / Key chip top-right -->
            <div class='qt-card-bpm-chip' v-if='track.bpm || track.key'>
                <span v-if='track.bpm' class='monospace'>{{ track.bpm }}</span>
                <span v-if='track.bpm && track.key' class='qt-card-bpm-sep'> · </span>
                <span v-if='track.key' class='monospace' :style='keyColor(track.key)'>{{ track.key }}</span>
            </div>

            <!-- Year (subtle, top-left) -->
            <div class='qt-card-year' v-if='track.year'>{{ track.year }}</div>
        </div>

        <!-- Meta block below cover -->
        <div class='qt-card-meta'>
            <div class='qt-card-name'>{{ track.title }}</div>
            <div class='qt-card-artist'>{{ track.artists.join(', ') }}</div>

            <!-- Data row: mood, energy, genre, customs -->
            <div class='qt-card-data'>
                <!-- Mood — inline picker. When the card is selected the
                     chip becomes a dropdown of every configured mood. -->
                <QuickTagMoodPicker :track='track' :selected='selected' />

                <!-- Energy pips (more compact than stars) -->
                <q-rating
                    size='14px'
                    v-model='track.energy'
                    no-reset
                    :readonly='!selected'
                    class='qt-card-rating'
                    color='primary'
                    color-selected='primary'
                ></q-rating>

                <!-- Genres (clickable when selected) -->
                <div class='qt-card-genres' v-if='track.genres.length'>
                    <span
                        v-if='selected'
                        v-for='(genre, i) in track.genres'
                        :key='"gen" + i'
                        class='qt-card-genre clickable'
                        @click='removeGenre(genre)'
                    >
                        {{ genre }}<span v-if='i != track.genres.length - 1'>, </span>
                    </span>
                    <span v-if='!selected' class='qt-card-genre'>{{ track.genres.join(', ') }}</span>
                </div>

                <!-- Hot-cue dots: one per custom tag, color hashed from value (V4) -->
                <div class='qt-card-cues' v-if='cueDots.length'>
                    <span
                        v-for='(c, i) in cueDots'
                        :key='"cdot"+i'
                        class='cue-dot'
                        :style='{ background: c, color: c }'
                    ></span>
                </div>
            </div>

            <!-- Custom tag chips at the bottom -->
            <div class='qt-card-customs' v-if='track.getAllCustom().length'>
                <q-chip
                    v-for='(tag, i) in track.getAllCustom()'
                    :key='"qtc" + i'
                    icon='mdi-close'
                    dense
                    square
                    :label='tag.value'
                    outline
                    color='primary'
                    class='qt-tile-chip qt-card-customchip'
                    @click='removeCustom(tag)'
                ></q-chip>
            </div>
        </div>

    </q-card>
</div>
</template>

<script lang='ts' setup>
import { computed, toRef } from 'vue';
import { get1t } from '../scripts/digtrax.js';
import { CAMELOT_KEYS, CustomTagInfo, KEY_COLORS, OPENKEY_KEYS, PLACEHOLDER_IMG, QTTrack } from '../scripts/quicktag.js';
import { httpUrl } from '../scripts/utils.js';
import { hashColor, trackPaint } from '../scripts/trackColors.js';
import QuickTagMoodPicker from './QuickTagMoodPicker.vue';

const $1t = get1t();
const props = defineProps({
    track: { required: true, type: QTTrack },
    noArtCache: { default: false, type: Boolean }
});
const inputTrack = toRef(props, 'track');
const noArtCache = toRef(props, 'noArtCache');

function getMood(name?: string) {
    if (!name) return;
    let mood = $1t.settings.value.quickTag.moods.find(m => m.mood == name);
    if (mood) {
        mood.outline = false;
        return mood;
    }
    return { mood: name, color: 'white', outline: true };
}

function removeMood(mood?: string) {
    if (!mood || !selected.value) return;
    track.value.mood = undefined;
}

function removeGenre(genre: string) {
    track.value.toggleGenre(genre);
}

function keyColor(key?: string) {
    if (!key) return;
    key = key.trim().toUpperCase();
    let color = KEY_COLORS[CAMELOT_KEYS[key.toUpperCase()]] || KEY_COLORS[OPENKEY_KEYS[key.toLowerCase()]];
    if (!color) {
        if (key.length < 3) key = `0${key}`;
        color = KEY_COLORS[key.toUpperCase()];
    }
    if (color) return `color: ${color};`;
}

function removeCustom(tag: CustomTagInfo) {
    if (!selected.value) return;
    if (tag.type === 'custom') {
        track.value.removeCustom(tag.index, tag.value);
        return;
    }
    let values = track.value.getNote().split(",")
        .map(n => n.trim())
        .filter(n => n && n != tag.value);
    track.value.setNote(values.join(", "));
}

const track = computed(() => {
    let track = $1t.quickTag.value.track.getTrack(inputTrack.value.path);
    if (!track) track = inputTrack.value;
    return track;
});

const selected = computed(() => $1t.quickTag.value.track.isSelected(track.value));
const art = computed(() => `${httpUrl()}/thumb?path=${encodeURIComponent(track.value.path)}${noArtCache.value ? "&_=" + Math.random().toString() : ""}`);

// Track color/glow — mood-derived when available, else hash-stable per track.
const paint = computed(() => trackPaint(track.value, $1t.settings.value));
const trackC = computed(() => paint.value.color);
const trackGlow = computed(() => paint.value.glow);

// Up to 4 colored hot-cue dots from custom tags.
const cueDots = computed(() => track.value.getAllCustom().slice(0, 4).map(t => hashColor(t.value || '')));
</script>

<style lang='scss' scoped>
.qt-card-wrap {
    height: 100%;
    width: 100%;
}

/* V4 — vertical track tile. Cover up top, meta below, glow halo when selected. */
.qt-card {
    height: 100%;
    width: 100%;
    background: var(--color-bg-elevated) !important;
    border: 1px solid var(--color-border) !important;
    border-radius: var(--radius-md) !important;
    overflow: hidden;
    cursor: pointer;
    transition: border-color var(--duration-fast) var(--ease-standard),
                background var(--duration-fast) var(--ease-standard),
                box-shadow var(--duration-fast) var(--ease-standard),
                transform var(--duration-fast) var(--ease-standard);
    box-shadow: none !important;
    display: flex;
    flex-direction: column;
}

.qt-card:hover {
    border-color: var(--color-border-strong) !important;
    background: var(--color-bg-overlay) !important;
    transform: translateY(-1px);
}

/* V4 — selected halo uses the track's own color, matching the design mockup */
.qt-card-selected {
    border-color: var(--track-c) !important;
    box-shadow: 0 0 0 2px var(--track-c), 0 0 32px var(--track-glow) !important;
    background: var(--color-bg-overlay) !important;
}

/* Cover area — 1.6 aspect ratio (V4 mockup) */
.qt-card-cover {
    position: relative;
    width: 100%;
    aspect-ratio: 1.6;
    background: linear-gradient(135deg, #1a2030, var(--color-bg));
    flex-shrink: 0;
    overflow: hidden;
}

.qt-card-art {
    width: 100%;
    height: 100%;
}

.qt-card-art-fallback {
    width: 100%;
    height: 100%;
    /* V4 — fallback gradient uses the track's color so every track-no-art is still distinct */
    background: linear-gradient(135deg, var(--track-c, var(--color-accent)), color-mix(in srgb, var(--track-c, var(--color-accent)) 25%, var(--color-bg)));
    display: flex;
    align-items: center;
    justify-content: center;
}

.qt-card-art-fallback::before {
    content: '♪';
    font-family: var(--font-mono);
    font-size: 32px;
    color: rgba(255, 255, 255, 0.6);
}

.qt-card-cover-scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg,
                rgba(5, 8, 17, 0.5) 0%,
                rgba(5, 8, 17, 0) 30%,
                rgba(5, 8, 17, 0) 60%,
                rgba(5, 8, 17, 0.7) 100%);
    pointer-events: none;
}

.qt-card-title-overlay {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 10px;
    pointer-events: none;
}

.qt-card-title-text {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 14px;
    line-height: 1.2;
    color: rgba(255, 255, 255, 0.96);
    text-transform: uppercase;
    letter-spacing: 0.02em;
    text-shadow: 0 2px 8px rgba(0, 0, 0, 0.6);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
}

.qt-card-bpm-chip {
    position: absolute;
    top: 8px;
    right: 10px;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    padding: 3px 8px;
    border-radius: var(--radius-full);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.9);
    border: 1px solid rgba(255, 255, 255, 0.08);
}

.qt-card-bpm-sep {
    color: rgba(255, 255, 255, 0.4);
}

.qt-card-year {
    position: absolute;
    top: 8px;
    left: 10px;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    padding: 2px 7px;
    border-radius: var(--radius-full);
    font-family: var(--font-mono);
    font-size: 10px;
    color: rgba(255, 255, 255, 0.75);
    border: 1px solid rgba(255, 255, 255, 0.06);
}

/* Meta block below cover */
.qt-card-meta {
    flex: 1;
    padding: 10px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 0;
}

.qt-card-name {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 13px;
    color: var(--color-fg);
    text-transform: uppercase;
    letter-spacing: 0.02em;
    line-height: 1.25;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.qt-card-artist {
    font-size: 11px;
    color: var(--color-fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 4px;
}

.qt-card-data {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: auto;
    margin-bottom: 4px;
}

.qt-card-mood-chip {
    margin: 0 !important;
    height: 20px !important;
    font-size: 10px !important;
    padding: 0 8px !important;
    text-transform: uppercase;
    letter-spacing: 0.04em;
}

.qt-card-rating {
    flex-shrink: 0;
}

.qt-card-genres {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
}

.qt-card-genre {
    font-size: 10px;
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
}

.qt-card-genre.clickable:hover {
    text-decoration: line-through;
    cursor: pointer;
}

.qt-card-cues {
    display: flex;
    gap: 5px;
    align-items: center;
    flex-shrink: 0;
}

.cue-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    box-shadow: 0 0 5px currentColor;
}

.qt-card-customs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
    overflow: hidden;
    max-height: 24px;
}

.qt-tile-chip {
    cursor: pointer;
    font-size: 11px;
    margin: 0 !important;
    height: 18px !important;
    padding: 0 6px !important;
}

.qt-tile-chip :deep(.q-chip__content) {
    color: var(--color-accent);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
}

.qt-tile-chip :deep(.q-icon) {
    display: none;
}

.qt-tile-chip:hover :deep(.q-icon) {
    display: inline;
    cursor: pointer;
}
</style>
