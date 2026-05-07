<template>
<!-- DJ Mode toggle pill — always rendered, top-right, gives the user a
     way back to the single-deck preview if they flipped DJ Mode on by
     mistake mid-mix. -->
<button
    class='dj-mode-toggle'
    :class='{ "dj-mode-toggle--on": $1t.settings.value.djMode }'
    @click='toggleDjMode'
    :title='$1t.settings.value.djMode ? "Exit DJ Mode (back to preview player)" : "Enter DJ Mode (dual-deck mixer)"'
>
    <q-icon
        :name='$1t.settings.value.djMode ? "mdi-disc-player" : "mdi-headphones"'
        size='12px'
        class='q-mr-xs'
    />
    <span>{{ $1t.settings.value.djMode ? 'DJ MODE' : 'DJ' }}</span>
</button>

<MixerPanel v-if='$1t.settings.value.djMode' />
<div v-else class="dt-player">
    <!-- Track meta -->
    <div class="dt-player-meta">
        <q-img
            :src='art'
            class='dt-player-art'
            :placeholder-src='PLACEHOLDER_IMG'
        >
            <template v-slot:error>
                <q-img :src='PLACEHOLDER_IMG' class='dt-player-art'></q-img>
            </template>
        </q-img>

        <div class="dt-player-meta-text">
            <div class="dt-player-title" v-if="$1t.player.value.title">
                {{ $1t.player.value.title }}
            </div>
            <div class="dt-player-artist" v-if="$1t.player.value.artists">
                {{ $1t.player.value.artists.join(', ') }}
            </div>
        </div>
    </div>

    <!-- Transport + waveform -->
    <div class="dt-player-center">
        <div class="dt-player-transport">
            <q-btn
                round flat dense
                icon="mdi-skip-previous"
                size="sm"
                class="dt-transport-btn"
                :ripple="false"
            ></q-btn>
            <q-btn
                v-if="!$1t.player.value.playing"
                round dense unelevated
                icon="mdi-play"
                class="dt-transport-play"
                :ripple="false"
                @click="$1t.player.value.play()"
                ref='playButton'
            ></q-btn>
            <q-btn
                v-else
                round dense unelevated
                icon="mdi-pause"
                class="dt-transport-play"
                :ripple="false"
                @click="$1t.player.value.pause()"
                ref='playButton'
            ></q-btn>
            <q-btn
                round flat dense
                icon="mdi-skip-next"
                size="sm"
                class="dt-transport-btn"
                :ripple="false"
            ></q-btn>
        </div>

        <div class="dt-player-wave">
            <Waveform></Waveform>
        </div>
    </div>

    <!-- Right side: playlist drop + volume -->
    <div class="dt-player-right">
        <div v-if='enablePlaylist' class="dt-player-playlist">
            <PlaylistDropZone
                tiny
                v-model="qtPlaylist"
                @update:model-value="loadQTPlaylist(); $1t.quickTagUnfocus()"
                @click.native='$1t.onQuickTagEvent("generatePlaylist"); $1t.quickTagUnfocus()'
            ></PlaylistDropZone>
        </div>

        <div class="dt-player-volume">
            <q-icon name="mdi-volume-medium" size="16px" class="dt-volume-icon"></q-icon>
            <q-slider
                v-model="$1t.player.value.volume"
                :min="0.0"
                :max="1.0"
                :step="0.01"
                @update:model-value="(v: any) => $1t.player.value.setVolume(v)"
                @change="$1t.saveSettings(false)"
                class="dt-volume-slider"
            ></q-slider>
        </div>
    </div>
</div>
</template>

<script lang='ts' setup>
import Waveform from './Waveform.vue';
import MixerPanel from './dj/MixerPanel.vue';
import PlaylistDropZone from "./PlaylistDropZone.vue";
import { Playlist, httpUrl } from '../scripts/utils';
import { computed, onDeactivated, onMounted, ref, watch } from 'vue';
import { get1t } from '../scripts/digtrax';
import { useRoute, useRouter } from 'vue-router';
import { PLACEHOLDER_IMG } from '../scripts/quicktag';
import { pauseDeck, djState } from '../scripts/dj';

const $1t = get1t();
const qtPlaylist = ref<Playlist>({});
const enablePlaylist = ref(true);
const playButton = ref<any>();

/// Flip DJ Mode. Going OFF pauses both decks so we don't leave audio
/// playing in the background. Going ON pauses the single-deck preview
/// for the same reason (a foot in both modes is confusing).
function toggleDjMode() {
    const next = !$1t.settings.value.djMode;
    if (next) {
        // Entering DJ Mode — pause the preview player.
        $1t.player.value.pause();
    } else {
        // Leaving DJ Mode — pause both decks.
        if (djState.deckA.playing) pauseDeck('a');
        if (djState.deckB.playing) pauseDeck('b');
    }
    $1t.settings.value.djMode = next;
    $1t.saveSettings(false);
}

function loadQTPlaylist() {
    if (!qtPlaylist.value || !qtPlaylist.value.data) {
        $1t.loadQuickTag();
        return;
    }
    $1t.loadQuickTag(qtPlaylist.value);
}

onMounted(() => {
    $1t.quickTagUnfocus = () => {
        playButton.value!.$el.focus();
        playButton.value!.$el.blur();
    }
    enablePlaylist.value = useRouter().currentRoute.value.path.includes('quicktag');
});

onDeactivated(() => $1t.quickTagUnfocus = () => {});

watch(useRoute(), (r) => {
    enablePlaylist.value = r.path == '/quicktag';
});

const art = computed(() => `${httpUrl()}/thumb?path=${encodeURIComponent($1t.player.value.path??'')}`);
</script>

<style lang="scss" scoped>
/* DJ Mode toggle pill — lives at the top-right of the footer in both
   states (preview and mixer) so the user can always flip back. */
.dj-mode-toggle {
    position: absolute;
    right: 14px;
    top: -10px;
    z-index: 10;
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-full, 9999px);
    background: var(--color-bg-elevated);
    color: var(--color-fg-muted);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    cursor: pointer;
    transition: all var(--duration-fast, 120ms) var(--ease-standard, ease);
}
.dj-mode-toggle:hover {
    color: var(--color-fg);
    border-color: var(--color-border-strong);
}
.dj-mode-toggle--on {
    color: #001f1c;
    background: var(--color-accent);
    border-color: var(--color-accent);
    box-shadow: 0 0 8px var(--color-accent-glow);
}

.dt-player {
    display: grid;
    grid-template-columns: 280px 1fr 220px;
    gap: 24px;
    align-items: center;
    padding: 12px 20px;
    background: var(--color-bg-elevated);
    border-top: 1px solid var(--color-border);
}

/* Meta — album art + title/artist */
.dt-player-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
}

.dt-player-art {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
    border: 1px solid var(--color-border);
}

.dt-player-meta-text {
    min-width: 0;
    overflow: hidden;
}

.dt-player-title {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 13px;
    color: var(--color-fg);
    text-transform: uppercase;
    letter-spacing: 0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.dt-player-artist {
    font-size: 11px;
    color: var(--color-fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 2px;
}

/* Center: transport + waveform */
.dt-player-center {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
}

.dt-player-transport {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
}

.dt-transport-btn {
    color: var(--color-fg-muted) !important;
}

.dt-transport-btn:hover {
    color: var(--color-fg) !important;
}

/* V4 — accent play button with glow */
.dt-transport-play {
    background: var(--color-accent) !important;
    color: #002b27 !important;
    width: 36px !important;
    height: 36px !important;
    box-shadow: 0 4px 16px var(--color-accent-glow) !important;
    transition: transform var(--duration-fast) var(--ease-standard),
                box-shadow var(--duration-fast) var(--ease-standard) !important;
}

.dt-transport-play :deep(.q-icon) {
    font-size: 20px;
}

.dt-transport-play:hover {
    transform: scale(1.05);
    box-shadow: 0 6px 24px var(--color-accent-glow) !important;
}

.dt-player-wave {
    width: 100%;
    min-width: 0;
}

/* Right: playlist + volume */
.dt-player-right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
}

.dt-player-playlist {
    flex-shrink: 0;
}

.dt-player-volume {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 130px;
}

.dt-volume-icon {
    color: var(--color-fg-muted);
    flex-shrink: 0;
}

.dt-volume-slider {
    flex: 1;
}

.dt-volume-slider :deep(.q-slider__track-container) {
    color: var(--color-accent);
}
</style>
