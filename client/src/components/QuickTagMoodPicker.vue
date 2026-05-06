<template>
    <!-- Inline mood chooser. Renders the current mood as a colored chip; click
         opens a Quasar menu listing every configured mood (plus "None") so
         the user can pick without bouncing to a bottom bar. Replaces the
         deprecated QuickTagMoods footer. -->
    <q-chip
        dense
        :clickable='selected'
        :color='currentMood ? currentMood.color : ""'
        :outline='!currentMood'
        :label='currentMood ? currentMood.mood : "—"'
        :class="['qt-mood-picker', { 'qt-mood-picker--readonly': !selected }]"
    >
        <q-icon v-if='selected' name='mdi-menu-down' size='14px' class='qt-mood-caret' />
        <!-- Menu only attaches when the track is selected — matches the
             established edit pattern in QuickTagTileThin (genres are
             clickable only when selected). Anchored upward so it doesn't
             flip direction based on viewport space (consistent with the
             genre subgenre menu). -->
        <q-menu v-if='selected' auto-close anchor='top middle' self='bottom middle'>
            <q-list dense style='min-width: 180px;'>
                <q-item clickable v-close-popup @click='setMood(undefined)'>
                    <q-item-section side>
                        <q-icon name='mdi-close' size='14px' />
                    </q-item-section>
                    <q-item-section>None</q-item-section>
                </q-item>
                <q-separator />
                <q-item
                    v-for='m in moods'
                    :key='m.mood'
                    clickable
                    v-close-popup
                    @click='setMood(m.mood)'
                    :class='{ "qt-mood-active": currentMood && currentMood.mood === m.mood }'
                >
                    <q-item-section side>
                        <span class='qt-mood-dot' :style='{ background: moodColorToTrack(m.color) }'></span>
                    </q-item-section>
                    <q-item-section>{{ m.mood }}</q-item-section>
                    <q-item-section side v-if='currentMood && currentMood.mood === m.mood'>
                        <q-icon name='mdi-check' size='14px' color='primary' />
                    </q-item-section>
                </q-item>
            </q-list>
        </q-menu>
    </q-chip>
</template>

<script lang='ts' setup>
import { computed, PropType } from 'vue';
import { get1t } from '../scripts/digtrax';
import { QTTrack } from '../scripts/quicktag';
import { moodColorToTrack } from '../scripts/trackColors';

const $1t = get1t();
const props = defineProps({
    track: { required: true, type: QTTrack },
    selected: { required: false, type: Boolean, default: false },
});

const moods = computed(() => $1t.settings.value.quickTag.moods);

const currentMood = computed(() => {
    const m = props.track.mood;
    if (!m) return undefined;
    return moods.value.find((x: any) => x.mood === m) ?? { mood: m, color: 'white', outline: true };
});

// Apply via the multi-track setter — every selected track gets the new
// mood (matches the bottom-bar behavior we're replacing). Only reachable
// when the row is selected (the menu gates on `selected`).
function setMood(mood: string | undefined) {
    $1t.quickTag.value.track.mood = mood;
}
</script>

<style lang='scss' scoped>
.qt-mood-picker {
    margin: 0 !important;
    height: 18px !important;
    font-size: 10px !important;
    padding: 0 6px 0 8px !important;
    text-transform: uppercase;
    letter-spacing: 0.04em;
}
.qt-mood-picker--readonly {
    cursor: default !important;
}
.qt-mood-caret {
    margin-left: 4px;
    margin-right: -2px;
    opacity: 0.7;
}
.qt-mood-dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: 0 0 4px currentColor;
}
.qt-mood-active {
    background: rgba(0, 210, 191, 0.06);
}
</style>
