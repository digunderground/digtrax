<template>
    <!-- Platform tiles. Responsive grid: 1 column narrow, 2 column medium,
         3+ columns wide. Each tile mirrors the visual idiom used by Quick Tag
         tracks — a tappable card with selection state, surface tokens, and
         glanceable metadata. Drag-and-drop still works (draggable wraps the
         grid container) for reordering fallback priority. -->
    <draggable
        v-model='$1t.info.value.platforms'
        @update='syncPlatforms'
        item-key='id'
        class='at-platforms-grid'
        :animation='160'
    >
        <template #item='{ element: platform }'>
            <button
                type='button'
                class='at-platform-tile'
                :class='{ "at-platform-tile--selected": isEnabled(platform.platform.id) }'
                @click='update(platform.platform.id)'
            >
                <!-- Selection state ring at top — matches the Quick Tag tile motif -->
                <span class='at-platform-check'>
                    <q-icon
                        :name='isEnabled(platform.platform.id) ? "mdi-check-circle" : "mdi-circle-outline"'
                        size='20px'
                    />
                </span>

                <div class='at-platform-art'>
                    <img :src='platform.icon' :alt='platform.platform.name' />
                </div>

                <div class='at-platform-body'>
                    <div class='at-platform-name'>{{ platform.platform.name }}</div>
                    <div class='at-platform-desc' v-html='platform.platform.description'></div>

                    <div class='at-platform-meta'>
                        <span class='at-meta-pill' :title='threadTooltip(platform)'>
                            <q-icon
                                :name='platform.platform.maxThreads == 1 ? "mdi-speedometer-slow"
                                    : (platform.platform.maxThreads > 1 ? "mdi-speedometer-medium" : "mdi-speedometer")'
                                size='14px'
                            />
                            <span>{{ platform.platform.maxThreads || '∞' }}×</span>
                        </span>
                        <span v-if='platform.requiresAuth' class='at-meta-pill' title='Requires an account'>
                            <q-icon name='mdi-lock' size='14px' />
                            <span>auth</span>
                        </span>
                        <span v-if='hasLyrics(platform)' class='at-meta-pill' title='Can fetch lyrics'>
                            <q-icon name='mdi-microphone' size='14px' />
                            <span>lyrics</span>
                        </span>
                        <span v-if='!platform.builtIn' class='at-meta-pill at-meta-pill--ext' title='Custom (community) platform'>
                            <q-icon name='mdi-puzzle-outline' size='14px' />
                            <span class='monospace'>{{ platform.platform.id }}@{{ platform.platform.version }}</span>
                        </span>
                    </div>
                </div>
            </button>
        </template>
    </draggable>

    <!-- "Need more platforms?" — temporarily hidden for the V4 Auto Tag rework.
         The feature is parked in plan/01-ui-redesign.md (Out of scope/deferred);
         reinstate by flipping `SHOW_PLATFORMS_REPO` to true once the redesigned
         flow lands. Original markup is preserved below so we can resurrect it
         without re-deriving the layout / handler wiring. -->
    <template v-if='SHOW_PLATFORMS_REPO'>
        <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 24px; margin-bottom: 35px"' inset color="dark"/>

        <div v-if='!dense' class='q-mt-md q-mb-xl'>
            <div class='text-subtitle2 text-bold text-primary'>NEED MORE PLATFORMS?</div>
            <div class='text-subtitle2 text-grey-6'>DigTrax supports custom platforms written in Rust.<br>You can install them using the button below.</div>

            <div class="row items-center justify-center q-mt-md">
                <q-btn dense push color="primary" :loading='platformsRepoButtonLoading' :disable="platformsRepoButtonLoading" class="rounded-borders q-px-sm q-mb-xs text-black text-weight-medium text-caption" @click='openPlatformsRepo()'>Platforms Repository</q-btn>
            </div>

            <div class='text-caption text-grey-6 text-center q-py-sm'>
                <span @click='$1t.url("https://github.com/digtrax/digtrax/blob/master/CUSTOM_PLATFORMS.md")' class='clickable doc-link'>How to create a custom platform?<span class="q-ml-xs"><q-icon name='mdi-open-in-new'></q-icon></span></span>
            </div>

        </div>
    </template>

</template>

<script lang='ts' setup>
import { onMounted, ref } from 'vue';
import { get1t } from '../scripts/digtrax.js';
import draggable from 'vuedraggable';
import { AutotaggerPlatform, SupportedTag } from '../scripts/autotagger';


const { dense } = defineProps({
    dense: { type: Boolean, default: false }
});
const $1t = get1t();
const platformsRepoButtonLoading = ref(false);

// Feature flag for the "need more platforms" / Platforms Repository UI.
// Hidden during the V4 Auto Tag rework; reinstate by flipping to true once
// the redesigned flow lands. See plan/01-ui-redesign.md → Out of scope.
const SHOW_PLATFORMS_REPO = false;

// Update config
function update(platform: string) {
    let i = $1t.config.value.platforms.indexOf(platform);
    if (i == -1)
        $1t.config.value.platforms.push(platform);
    else
        $1t.config.value.platforms.splice(i, 1);
}

// Is platform enabled
function isEnabled(platform: string) {
    return $1t.config.value.platforms.includes(platform);
}

// Sync platforms order to config
function syncPlatforms() {
    $1t.config.value.platforms = $1t.info.value.platforms.map((p) => p.platform.id).filter((p) => $1t.config.value.platforms.includes(p));
}

/// Does the platform have lyrics
function hasLyrics(platform: AutotaggerPlatform) {
    return platform.supportedTags.includes(SupportedTag.UnsyncedLyrics) || platform.supportedTags.includes(SupportedTag.SyncedLyrics);
}

function threadTooltip(platform: AutotaggerPlatform): string {
    const m = platform.platform.maxThreads;
    if (!m) return 'Unlimited concurrent searches';
    if (m === 1) return 'Single-threaded';
    return `Up to ${m} concurrent searches`;
}

/// Open the platforms repo and make the button load
function openPlatformsRepo() {
    platformsRepoButtonLoading.value = true;
    $1t.send("repoManifest");
    setTimeout(() => {
        platformsRepoButtonLoading.value = false
    }, 2000);
}

onMounted(() => {
    $1t.info.value.platforms.sort((a, b) => {
        let x = $1t.config.value.platforms.indexOf(a.platform.id);
        let y = $1t.config.value.platforms.indexOf(b.platform.id);
        if (x == -1) x = 1000;
        if (y == -1) y = 1000;
        return x - y;
    });
});

</script>

<style lang='scss'>
.at-platforms-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
    user-select: none;
}

.at-platform-tile {
    position: relative;
    display: grid;
    grid-template-columns: 56px 1fr;
    column-gap: 14px;
    align-items: start;
    padding: 14px 16px 14px 16px;
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md, 12px);
    color: inherit;
    text-align: left;
    cursor: pointer;
    font-family: inherit;
    transition:
        border-color var(--duration-fast, 120ms) var(--ease-standard, ease),
        background-color var(--duration-fast, 120ms) var(--ease-standard, ease),
        box-shadow var(--duration-fast, 120ms) var(--ease-standard, ease),
        transform var(--duration-fast, 120ms) var(--ease-standard, ease);
}
.at-platform-tile:hover {
    border-color: var(--color-border-strong);
    background: rgba(255, 255, 255, 0.02);
}
.at-platform-tile:active {
    transform: translateY(1px);
}
.at-platform-tile--selected {
    border-color: var(--color-accent);
    background: rgba(0, 210, 191, 0.06);
    box-shadow:
        0 0 0 1px var(--color-accent),
        0 4px 24px rgba(0, 210, 191, 0.10);
}

.at-platform-check {
    position: absolute;
    top: 10px;
    right: 12px;
    color: var(--color-fg-subtle);
    transition: color var(--duration-fast, 120ms) var(--ease-standard, ease);
}
.at-platform-tile--selected .at-platform-check {
    color: var(--color-accent);
}

.at-platform-art {
    grid-column: 1;
    width: 56px;
    height: 56px;
    border-radius: var(--radius-sm, 8px);
    overflow: hidden;
    background: rgba(255, 255, 255, 0.04);
    display: flex;
    align-items: center;
    justify-content: center;
}
.at-platform-art img {
    max-width: 80%;
    max-height: 80%;
    object-fit: contain;
}

.at-platform-body {
    grid-column: 2;
    min-width: 0;
}
.at-platform-name {
    font-size: 15px;
    font-weight: 700;
    line-height: 1.2;
    margin-bottom: 4px;
    color: var(--color-fg);
    /* Reserve space for the check icon */
    padding-right: 22px;
}
.at-platform-desc {
    font-size: 12px;
    color: var(--color-fg-subtle);
    line-height: 1.4;
    /* Clamp to two lines so tile heights stay even */
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    margin-bottom: 8px;
}

.at-platform-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
}
.at-meta-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: var(--radius-full, 9999px);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--color-border);
    font-size: 11px;
    color: var(--color-fg-subtle);
    line-height: 1.4;
}
.at-meta-pill--ext {
    color: var(--color-fg-muted);
}

/* Legacy class kept so any external selectors keep matching, but with
   neutral defaults — the new grid styles take over visual presentation. */
.card { user-select: none; }
.cb svg { color: #000; }
.text-subtitle3 { font-size: 12px; }
</style>
