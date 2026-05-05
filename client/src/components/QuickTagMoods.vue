<template>
<div class='dt-moods'>
    <button
        v-for='(mood, i) in $1t.settings.value.quickTag.moods'
        :key='"mood"+i'
        class='dt-mood-chip'
        :class='{ active: moodSelected(mood) }'
        :style='moodStyle(mood)'
        @click='moodSelect(mood)'
        @mousemove='moodHover = i'
        @mouseleave="moodHover = -1"
    >
        {{ mood.mood }}
    </button>
</div>
</template>

<script lang='ts' setup>
import { ref } from 'vue';
import { get1t } from '../scripts/digtrax.js';
import { QuickTagMood } from '../scripts/quicktag.js';
import { moodColorToTrack, colorToGlow } from '../scripts/trackColors';

const $1t = get1t();
const moodHover = ref(-1);

function moodSelected(mood: QuickTagMood) {
    return $1t.quickTag.value.track.mood == mood.mood;
}

function moodSelect(mood: QuickTagMood) {
    if ($1t.quickTag.value.track.mood == mood.mood) $1t.quickTag.value.track.mood = undefined;
    else $1t.quickTag.value.track.mood = mood.mood;
}

function moodStyle(mood: QuickTagMood) {
    const color = moodColorToTrack(mood.color);
    return {
        '--mood-c': color,
        '--mood-glow': colorToGlow(color),
    };
}
</script>

<style lang="scss" scoped>
.dt-moods {
    display: flex;
    gap: 6px;
    padding: 8px 16px;
    flex-wrap: wrap;
    justify-content: center;
}

.dt-mood-chip {
    padding: 5px 14px;
    border: 1px solid var(--mood-c, var(--color-border));
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--mood-c, var(--color-fg-muted));
    font: inherit;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-standard);
    white-space: nowrap;
}

.dt-mood-chip:hover {
    background: color-mix(in srgb, var(--mood-c, var(--color-accent)) 12%, transparent);
}

.dt-mood-chip.active {
    background: var(--mood-c, var(--color-accent));
    color: #002b27;
    box-shadow: 0 0 12px var(--mood-glow, var(--color-accent-glow));
    font-weight: 700;
}
</style>
