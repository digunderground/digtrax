<template>
    <q-card flat class='qt-tile-thin' :class='{"qt-tile-thin-selected": selected, "qt-tile-thin-odd": odd}' :style='{ "--track-c": trackC, "--track-glow": trackGlow }'>
        <!-- Title cell: small gradient cover + title text -->
        <div v-if='isVis("title")' class='qt-cell qt-cell-title'>
            <div class='qt-row-cover'></div>
            <span class='qt-row-title'>{{ track.title }}</span>
        </div>

        <!-- Artist -->
        <div v-if='isVis("artist")' class='qt-cell qt-cell-artist'>{{ track.artists.join(", ") }}</div>

        <!-- Mood — inline picker (replaces the deprecated bottom bar). Row
             click handler still fires so clicking the cell first selects
             the track; once selected, the chip becomes a dropdown. -->
        <div v-if='isVis("mood")' class='qt-cell qt-cell-mood'>
            <QuickTagMoodPicker :track='track' :selected='selected' />
        </div>

        <!-- Energy: 5 horizontal pips (V4) -->
        <div v-if='isVis("energy")' class='qt-cell qt-cell-energy'>
            <div class='energy-pips'>
                <span
                    v-for='i in 5'
                    :key='"epip"+i'
                    class='epip'
                    :class='{ on: i <= track.energy }'
                    @click.stop='onEnergyClick(i)'
                ></span>
            </div>
        </div>

        <!-- Genres -->
        <div v-if='isVis("genre")' class='qt-cell qt-cell-genre'>
            <span v-if='selected' class='qt-genre-list'>
                <span
                    v-for='(genre, i) in track.genres'
                    :key='"gen"+i'
                    class='hover-strike'
                    @click.stop='track.toggleGenre(genre)'
                >{{genre}}<span v-if='i != track.genres.length - 1'>, </span></span>
            </span>
            <span v-else>{{track.genres.join(', ')}}</span>
        </div>

        <!-- Year -->
        <div v-if='isVis("year")' class='qt-cell qt-cell-year monospace'>{{ track.year || '' }}</div>

        <!-- BPM -->
        <div v-if='isVis("bpm")' class='qt-cell qt-cell-bpm monospace'>{{ track.bpm || '' }}</div>

        <!-- Key -->
        <div v-if='isVis("key")' class='qt-cell qt-cell-key monospace' :style='keyColor(track.key)'>{{ track.key || '' }}</div>

        <!-- Per-custom-field cells. One cell per *visible* configured QuickTag
             custom field, showing only that field's chips. Clearer than the
             old single "Custom" column that mashed every value together. -->
        <div
            v-for='col in visibleCustomCols'
            :key='col.key'
            class='qt-cell qt-cell-customfield'
        >
            <div class='qt-thin-customs'>
                <q-chip
                    v-for='(tag, i) in customsForField(col.customIndex!)'
                    :key='col.key + "-" + i'
                    icon='mdi-close'
                    dense
                    square
                    :label='tag.value'
                    outline
                    color='primary'
                    class='qt-thin-customchip'
                    @click.stop='removeCustom(tag)'
                ></q-chip>
            </div>
        </div>
    </q-card>
</template>

<script lang='ts' setup>
import { computed, toRef } from 'vue';
import { get1t } from '../scripts/digtrax';
import { CustomTagInfo, keyColor, QTTrack } from '../scripts/quicktag';
import { trackPaint } from '../scripts/trackColors';
import { columns, isColumnVisible } from '../scripts/qtColumns';
import QuickTagMoodPicker from './QuickTagMoodPicker.vue';

const $1t = get1t();
const props = defineProps({
    track: { required: true, type: QTTrack },
    odd: { required: false, type: Boolean, default: false }
});
const inputTrack = toRef(props, 'track');
const odd = props.odd;

// Visibility helper for the static columns. Columns that don't render produce
// no grid cell at all so the column-template stays in lock-step with the header.
function isVis(key: string): boolean {
    return isColumnVisible(key);
}

// Per-custom-field column definitions for v-for. The header drives this same
// list, so cells line up by index. Pre-filtered to visible to side-step
// Vue 3's v-for/v-if-on-same-element limitation.
const visibleCustomCols = computed(() => columns.value.filter((c) => c.kind === 'custom' && c.visible));

// Filter the track's custom tags to only the values stored under the given
// custom-field index. `track.getAllCustom()` already returns CustomTagInfo
// with .index — we just slice by that.
function customsForField(customIndex: number): CustomTagInfo[] {
    return track.value.getAllCustom().filter((t) => t.index === customIndex && t.type === 'custom');
}

function getMood(name?: string) {
    if (!name) return;
    let mood = $1t.settings.value.quickTag.moods.find(m => m.mood == name);
    if (mood) {
        mood.outline = false;
        return mood;
    }
    return { mood: name, color: 'white', outline: true };
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

function onEnergyClick(value: number) {
    if (!selected.value) return;
    // Click same value to clear (matches q-rating "no-reset=false" behavior).
    track.value.energy = track.value.energy === value ? value : value;
}

const track = computed(() => {
    let track = $1t.quickTag.value.track.getTrack(inputTrack.value.path);
    if (!track) track = inputTrack.value;
    return track;
});
const selected = computed(() => $1t.quickTag.value.track.isSelected(track.value));

// Track color/glow — mood-derived when available, else hash-stable per track.
const paint = computed(() => trackPaint(track.value, $1t.settings.value));
const trackC = computed(() => paint.value.color);
const trackGlow = computed(() => paint.value.glow);
</script>

<style lang='scss' scoped>
/* V4 — Row tile is a grid that mirrors the column header. Cells overflow-ellipsis. */
.qt-tile-thin {
    display: grid !important;
    grid-template-columns: var(--col-template);
    align-items: center;
    height: 40px;
    min-height: 40px;
    margin: 1px 0;
    background: transparent !important;
    border: 1px solid transparent !important;
    border-radius: var(--radius-sm) !important;
    box-shadow: none !important;
    transition: background var(--duration-fast) var(--ease-standard),
                border-color var(--duration-fast) var(--ease-standard),
                box-shadow var(--duration-fast) var(--ease-standard);
    position: relative;
    overflow: hidden;
    cursor: pointer;
}

.qt-tile-thin::before {
    content: '';
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 3px;
    background: transparent;
    border-radius: 2px;
    transition: background var(--duration-fast) var(--ease-standard),
                box-shadow var(--duration-fast) var(--ease-standard);
}

.qt-tile-thin:hover {
    background: rgba(255, 255, 255, 0.04) !important;
}

/* V4 — selected uses the track's own color, matching the design mockup */
.qt-tile-thin-selected {
    background: color-mix(in srgb, var(--track-c) 10%, transparent) !important;
    border-color: var(--track-c) !important;
    box-shadow: 0 0 0 1px var(--track-c) inset, 0 0 16px var(--track-glow) !important;
}

.qt-tile-thin-selected::before {
    background: var(--track-c);
    box-shadow: 0 0 8px var(--track-glow);
}

/* Subtle track-color strip on every row when not selected (visual indicator of mood) */
.qt-tile-thin::after {
    content: '';
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 3px;
    background: var(--track-c);
    opacity: 0;
    border-radius: 2px;
    transition: opacity var(--duration-fast) var(--ease-standard);
    pointer-events: none;
}

.qt-tile-thin:hover::after {
    opacity: 0.4;
}

.qt-tile-thin-selected::after {
    opacity: 0;
}

.qt-cell {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 12px;
    font-size: 12px;
    color: var(--color-fg-muted);
    line-height: 1;
    display: flex;
    align-items: center;
    gap: 10px;
}

/* Title cell: cover thumb on left, title text right */
.qt-cell-title {
    color: var(--color-fg);
    padding-left: 14px;
}

.qt-row-cover {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-xs);
    background: linear-gradient(135deg, var(--track-c, var(--color-accent)), color-mix(in srgb, var(--track-c, var(--color-accent)) 25%, var(--color-bg-overlay)));
    flex-shrink: 0;
    border: 1px solid rgba(255, 255, 255, 0.06);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
}

.qt-row-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    font-size: 12px;
    color: var(--color-fg);
}

.qt-cell-artist {
    color: var(--color-fg-muted);
}

.qt-cell-mood,
.qt-cell-energy {
    justify-content: flex-start;
}

/* V4 — 5 horizontal energy pips */
.energy-pips {
    display: flex;
    gap: 3px;
    align-items: center;
}

.epip {
    width: 12px;
    height: 6px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.08);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-standard),
                box-shadow var(--duration-fast) var(--ease-standard);
}

.epip:hover {
    background: rgba(255, 255, 255, 0.16);
}

.epip.on {
    background: var(--color-accent);
    box-shadow: 0 0 4px var(--color-accent-glow);
}

.qt-cell-genre .qt-genre-list {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    font-size: 11px;
}

.qt-cell-genre {
    text-transform: uppercase;
    letter-spacing: 0.02em;
    font-size: 11px;
}

.qt-cell-year,
.qt-cell-bpm,
.qt-cell-key {
    font-size: 12px;
    color: var(--color-fg-muted);
    justify-content: flex-end;
}

.qt-cell-bpm {
    color: var(--color-fg);
}

.qt-cell-customfield {
    overflow: hidden;
}

.qt-thin-customs {
    display: flex;
    gap: 6px;
    overflow: hidden;
    flex-wrap: nowrap;
    align-items: center;
}

.qt-thin-chip {
    margin: 0 !important;
    height: 18px !important;
    font-size: 10px !important;
    padding: 0 8px !important;
    text-transform: uppercase;
    letter-spacing: 0.04em;
}

.qt-thin-customchip {
    margin: 0 !important;
    height: 18px !important;
    font-size: 10px !important;
    padding: 0 6px !important;
    flex-shrink: 0;
}

.qt-thin-customchip :deep(.q-chip__content) {
    color: var(--color-accent);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
}

.qt-thin-customchip :deep(.q-icon) {
    display: none;
}

.qt-thin-customchip:hover :deep(.q-icon) {
    display: inline;
    cursor: pointer;
}

.hover-strike:hover {
    text-decoration: line-through;
    cursor: pointer;
}
</style>
