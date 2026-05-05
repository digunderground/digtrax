<template>
<div class='autotag-page'>

    <!-- Header strip: page title + status hint + START + CLI buttons.
         Sticky so the primary action stays visible as the user scrolls
         through Platforms and the secondary settings tabs below. -->
    <div class='autotag-header row items-center q-px-lg q-py-md'>
        <div class='col'>
            <div class='text-h6 text-bold' style='letter-spacing: 0.02em;'>Auto Tag</div>
            <div class='text-caption text-grey-5'>
                Pick platforms, point at a folder or playlist, hit Start.
                <span v-if='!canStart' class='text-warning q-ml-xs'>{{ blockerHint }}</span>
            </div>
        </div>
        <q-btn
            flat round dense
            icon='mdi-console'
            color='grey-6'
            @click='cliDialog = true'
            class='q-mr-sm'
        >
            <q-tooltip anchor='top middle' self='bottom middle' :offset='[10, 10]'>CLI version of this config</q-tooltip>
        </q-btn>
        <q-btn
            push dense
            icon='mdi-play'
            color='primary' text-color='black'
            class='rounded-borders q-px-md text-weight-medium'
            label='Start'
            :disable='!canStart'
            @click='startTagging'
        />
    </div>

    <div class='autotag-grid q-px-lg q-pb-xl'>

        <!-- Profiles — slim card right under the header. Controlled by the
             same setting that gated the old single-page profile pane. -->
        <section
            v-if='$1t.settings.value.showAutoTaggerProfiles'
            class='dt-card autotag-card autotag-card--slim'
        >
            <header class='dt-card-header'>
                <q-icon name='mdi-bookmark-multiple-outline' size='18px' class='dt-card-icon' />
                <div class='col'>
                    <div class='dt-card-title'>Profiles</div>
                    <div class='dt-card-subtitle'>Save, switch, or delete a tagging configuration.</div>
                </div>
            </header>
            <div class='dt-card-body'>
                <AutotaggerProfile />
            </div>
        </section>

        <!-- Select Platforms — primary surface, full-width. The platform
             tile grid lives inside its own card to anchor the scan. -->
        <section class='dt-card autotag-card'>
            <header class='dt-card-header'>
                <q-icon name='mdi-web' size='18px' class='dt-card-icon' />
                <div class='col'>
                    <div class='dt-card-title'>Select platforms</div>
                    <div class='dt-card-subtitle'>Tap a tile to enable. Drag to reorder fallback priority.</div>
                </div>
                <span class='at-selected-badge q-ml-sm'>
                    {{ $1t.config.value.platforms.length }} selected
                </span>
            </header>
            <div class='dt-card-body'>
                <AutotaggerPlatforms />
            </div>
        </section>

        <!-- Secondary settings — tabbed so the page stays scannable when
             only the primary action (platforms + Start) matters. -->
        <section class='dt-card autotag-card'>
            <q-tabs
                v-model='settingsTab'
                dense
                inline-label
                no-caps
                align='left'
                class='autotag-tabs'
                indicator-color='primary'
                active-color='primary'
                active-bg-color='transparent'
            >
                <q-tab name='tags' icon='mdi-label-multiple' label='Input & tags' />
                <q-tab name='platform' icon='mdi-tune' label='Platform settings' />
                <q-tab name='advanced' icon='mdi-cog-outline' label='Advanced' />
            </q-tabs>
            <q-separator class='autotag-tabs-sep' />
            <q-tab-panels v-model='settingsTab' animated swipeable class='autotag-tab-panels'>
                <q-tab-panel name='tags' class='autotag-tab-panel'>
                    <AutotaggerTags />
                </q-tab-panel>
                <q-tab-panel name='platform' class='autotag-tab-panel'>
                    <AutotaggerPlatformSpecific />
                </q-tab-panel>
                <q-tab-panel name='advanced' class='autotag-tab-panel'>
                    <AutotaggerAdvanced />
                </q-tab-panel>
            </q-tab-panels>
        </section>

    </div>

    <!-- Floating Start FAB — keeps the primary action one click away even
         when the user has scrolled past the header. -->
    <q-page-sticky position='bottom-right' :offset='[36, 32]'>
        <q-btn
            fab push
            icon='mdi-play'
            color='primary'
            :disable='!canStart'
            @click='startTagging'
        >
            <q-tooltip anchor='top middle' self='bottom middle' :offset='[10, 10]'>Start tagging</q-tooltip>
        </q-btn>
    </q-page-sticky>

    <q-dialog v-model='cliDialog'>
        <CliDialog :config='$1t.config.value' command='autotagger' />
    </q-dialog>
</div>
</template>

<script lang='ts' setup>
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { get1t } from '../scripts/digtrax';

import AutotaggerPlatforms from '../components/AutotaggerPlatforms.vue';
import AutotaggerTags from '../components/AutotaggerTags.vue';
import AutotaggerPlatformSpecific from '../components/AutotaggerPlatformSpecific.vue';
import AutotaggerAdvanced from '../components/AutotaggerAdvanced.vue';
import AutotaggerProfile from '../components/AutotaggerProfile.vue';
import CliDialog from '../components/CliDialog.vue';

const $1t = get1t();
const $router = useRouter();
const cliDialog = ref(false);
const settingsTab = ref<'tags' | 'platform' | 'advanced'>('tags');

async function startTagging() {
    $1t.saveSettings();
    $1t.config.value.type = 'autoTagger';

    let playlist: any = null;
    if ($1t.autoTaggerPlaylist.value && $1t.autoTaggerPlaylist.value.data)
        playlist = $1t.autoTaggerPlaylist.value;

    if ($1t.settings.value.audioFeatures.spotifyClientId && $1t.settings.value.audioFeatures.spotifyClientSecret) {
        $1t.config.value.spotify = {
            clientId: $1t.settings.value.audioFeatures.spotifyClientId,
            clientSecret: $1t.settings.value.audioFeatures.spotifyClientSecret,
        };
    } else {
        $1t.config.value.spotify = undefined;
    }

    setTimeout(() => {
        $1t.send('startTagging', { config: $1t.config.value, playlist });
    }, 100);
    setTimeout(async () => {
        await $router.push('/autotagger/status');
    }, 10);
}

const hasInput = computed(() => !!$1t.config.value.path
    || !!($1t.autoTaggerPlaylist.value && $1t.autoTaggerPlaylist.value.data));
const hasPlatform = computed(() => $1t.config.value.platforms.length > 0);
const canStart = computed(() => hasInput.value && hasPlatform.value);
const blockerHint = computed(() => {
    if (!hasInput.value && !hasPlatform.value) return '— pick a folder/playlist and at least one platform';
    if (!hasInput.value) return '— pick a folder or drop a playlist below';
    if (!hasPlatform.value) return '— select at least one platform';
    return '';
});
</script>


<style lang='scss'>
.autotag-page {
    background: var(--color-bg);
    min-height: 100%;
}
.autotag-header {
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-elevated);
    position: sticky;
    top: 0;
    z-index: 5;
    backdrop-filter: saturate(140%) blur(8px);
}
.autotag-grid {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding-top: 18px;
    max-width: 1180px;
    margin: 0 auto;
}

/* V4 card primitive used across the Auto Tag rework. */
.dt-card {
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md, 12px);
    padding: 18px 20px 20px;
    transition: border-color var(--duration-fast, 120ms) var(--ease-standard, ease);
}
.dt-card:hover { border-color: var(--color-border-strong); }
.dt-card-header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 14px;
}
.dt-card-icon { color: var(--color-accent); margin-top: 2px; flex: 0 0 auto; }
.dt-card-title {
    font-size: 14px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-fg);
    line-height: 1.2;
}
.dt-card-subtitle {
    font-size: 12px;
    color: var(--color-fg-subtle);
    margin-top: 3px;
    line-height: 1.4;
}
.autotag-card--slim { padding: 14px 18px; }

/* Selected-platforms badge in the platforms card header */
.at-selected-badge {
    flex: 0 0 auto;
    align-self: center;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 3px 10px;
    border-radius: var(--radius-full, 9999px);
    border: 1px solid var(--color-accent);
    color: var(--color-accent);
    background: rgba(0, 210, 191, 0.08);
}

/* Tabs strip — quiet, low-chrome so the cards still dominate. */
.autotag-tabs {
    margin: -4px -8px 0 -8px;
}
.autotag-tabs .q-tab {
    min-height: 38px;
    padding: 0 12px;
    text-transform: none;
    font-weight: 600;
    letter-spacing: 0.01em;
}
.autotag-tabs-sep {
    background: var(--color-border);
    margin: 0 -20px;
}
.autotag-tab-panels {
    background: transparent;
}
.autotag-tab-panel {
    padding: 20px 4px 4px 4px;
}

/* Legacy helpers kept for sub-components that still reference them. */
.input { max-width: 526px; margin: 8px auto 0; padding: 0 16px; }
.select { max-width: 526px; margin: 8px auto 0; padding: 0 16px; }
.slider { max-width: 550px !important; }
</style>
