<template>
    <div>
        <q-layout view="hHh lpR fFf" class="bg-background">
            <!-- Permanent left nav rail (replaces top tabs) -->
            <q-drawer :model-value="true" side="left" :width="220" :breakpoint="0" persistent bordered>
                <DigTraxNavRail></DigTraxNavRail>
            </q-drawer>

            <!-- Help button -->
            <HelpButton v-if='$1t.info.value.ready'></HelpButton>

            <!-- Content -->
            <q-page-container class="content" ref="contentContainer">
                <router-view v-slot='{ Component }' v-if='$1t.info.value.ready'>
                    <transition name="fade">
                        <keep-alive :include='["AudioFeatures"]'>
                            <component :is='Component'></component>
                        </keep-alive>
                    </transition>
                </router-view>
                <!-- Loading -->
                <div v-if='!$1t.info.value.ready' class='row justify-center items-center' style='height: calc(100vh - 64px)'>
                    <q-circular-progress indeterminate color='primary' size='64px'></q-circular-progress>
                </div>
            </q-page-container>

            <!-- Footer -->
            <q-footer reveal class="bg-darker text-white" v-if="footer">
                <div v-if='isRoute("quicktag")'>
                    <QuickTagMoods v-if="$1t.quickTag.value.track"></QuickTagMoods>
                    <QuickTagGenreBar v-if="$1t.quickTag.value.track"></QuickTagGenreBar>
                </div>

                <PlayerBar v-if='($1t.settings.value.tagEditorPlayer && isRoute("tageditor")) || isRoute("quicktag")'></PlayerBar>
            </q-footer>
        </q-layout>

        <!-- Min size dialog -->
        <q-dialog v-model="sizeDialog" persistent>
            <q-card>
                <q-card-section>
                    <div class="text-h6">Warning</div>
                </q-card-section>
                <q-card-section>
                    DigTrax requires atleast 1200x550 window size. Please
                    resize to continue.
                </q-card-section>
            </q-card>
        </q-dialog>

        <!-- Update dialog -->
        <q-dialog v-model="updateDialog">
            <q-card v-if="update" style="min-width: 400px; max-width: 560px;">
                <q-card-section class="text-center">
                    <div class="text-h5">New update available</div>
                </q-card-section>
                <q-card-section>
                    <div class="text-center">
                        <div class="text-h6 text-weight-bold q-mb-md">
                            v{{ update.version }}
                        </div>
                        <!-- Plaintext, NOT v-html (no XSS surface). Full notes on the GitHub release page. -->
                        <pre class="update-snippet">{{ update.changelog || 'See the release page for full notes.' }}</pre>
                    </div>
                </q-card-section>
                <q-card-section class="justify-center row">
                    <q-btn color="primary" class="text-black" @click="$1t.url(update!.url)">View release</q-btn>
                </q-card-section>
            </q-card>
        </q-dialog>

        <!-- Folder browser dialog -->
        <q-dialog v-model='$1t.folderBrowser.value.open'>
            <FolderBrowser v-if='$1t.folderBrowser.value.open' :base='$1t.folderBrowser.value.basePath'></FolderBrowser>
        </q-dialog>
    </div>
</template>

<script lang='ts' setup>
import axios from 'axios';

import { compareVersions } from 'compare-versions';
import { useQuasar } from 'quasar';
import { onMounted, onUpdated, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { get1t } from "./scripts/digtrax.js";

import HelpButton from './components/HelpButton.vue';
import PlayerBar from './components/PlayerBar.vue';
import FolderBrowser from './components/FolderBrowser.vue';
import DigTraxNavRail from './components/DigTraxNavRail.vue';
import QuickTagGenreBar from './components/QuickTagGenreBar.vue';
import QuickTagMoods from './components/QuickTagMoods.vue';

const $1t = get1t();
const $q = useQuasar();
const router = useRouter();

const footer = ref(false);
const sizeDialog = ref(false);
const update = ref<undefined | { url: string, version: string, changelog: string }>(undefined);
const updateDialog = ref(false);

/// Check if is on route
function isRoute(route: string) {
    return router.currentRoute.value.path.includes(route);
}

/// Check for updates via the GitHub Releases API.
/// No custom infra — GitHub serves the latest release as JSON. Update DT_UPDATE_REPO when
/// the public DigTrax repo is live; until then this 404s and the function silently no-ops.
const DT_UPDATE_REPO = 'digtrax/digtrax';

async function checkUpdates() {
    const url = `https://api.github.com/repos/${DT_UPDATE_REPO}/releases/latest`;
    let data: any = null;
    try {
        const res = await axios.get(url, {
            headers: { 'Accept': 'application/vnd.github+json' },
            timeout: 5000,
        });
        data = res.data;
    } catch {
        // 404 / 403 / network — fail silently
        return;
    }
    if (!data || !data.tag_name) return;

    // GitHub tags are usually `v1.8.0`; strip the `v` prefix for compareVersions
    const remoteVersion = String(data.tag_name).replace(/^v/, '');
    let isNewer = false;
    try {
        isNewer = compareVersions(remoteVersion, $1t.info.value.version) === 1;
    } catch {
        return;
    }
    if (!isNewer) return;

    update.value = {
        version: remoteVersion,
        url: data.html_url,
        // Plaintext snippet only — full markdown notes live on the GitHub release page.
        changelog: (data.body || '').slice(0, 600),
    };
    $q.notify({
        message: `New update available (${remoteVersion})!`,
        timeout: 10000,
        progress: true,
        actions: [
            {
                label: "Show",
                handler: () => { updateDialog.value = true; },
            },
        ],
        position: 'top-right'
    });
}

function setWaveformWidth() {
    document.documentElement.style.setProperty('--waveform-wave', Math.min(Math.round(20 + ((window.innerWidth - 1200) / 10)), 70).toString());
}

// Setup
onMounted(() => {
    // Quasar dark mode is independent of our V4 token-based theme switcher.
    // Keep Quasar in dark mode so its built-in components (dialogs, dropdowns, etc.) render dark surfaces.
    // Light mode for Quasar is a Phase 1 Step 2 follow-up.
    $q.dark.set(true);

    setWaveformWidth();
    window.addEventListener("resize", () => {
        setWaveformWidth();
        if (window.innerHeight < 550 || window.innerWidth < 1200) {
            sizeDialog.value = true;
        } else {
            sizeDialog.value = false;
        }
    });

    // Show footer where the route warrants it
    if (isRoute('quicktag') || isRoute('tageditor')) {
        footer.value = true;
    }

    setTimeout(() => checkUpdates(), 5000);

    // First-launch OneTagger config detection. One-shot via localStorage flag.
    if (!localStorage.getItem(LEGACY_FLAG)) {
        $1t.send('detectLegacySettings', {});
    }
});

// Legacy detect — toast once when OneTagger settings are found on a fresh DigTrax install.
const LEGACY_FLAG = 'digtrax-legacy-prompt-handled';

watch(() => $1t.legacyConfig.value, (v) => {
    if (!v.exists || !v.settings) return;
    if (localStorage.getItem(LEGACY_FLAG)) return;
    localStorage.setItem(LEGACY_FLAG, '1');

    $q.notify({
        message: 'Found OneTagger settings — import them?',
        color: 'dark',
        position: 'top',
        timeout: 0,
        actions: [
            {
                label: 'Import',
                color: 'primary',
                handler: () => {
                    if (!v.settings || typeof v.settings !== 'object') return;
                    // Replace settings + persist + reload so reactive views pick up the new state
                    Object.assign($1t.settings.value, v.settings);
                    $1t.saveSettings(false);
                    $q.notify({
                        message: 'Imported. Reloading…',
                        color: 'positive',
                        timeout: 1500,
                    });
                    setTimeout(() => window.location.reload(), 800);
                }
            },
            { label: 'Skip', color: 'grey', handler: () => {} }
        ]
    });
}, { deep: true });

// Update footer visibility on route change
const contentContainer = ref(null);
watch(useRoute(), (r) => {
    // @ts-ignore
    contentContainer.value!.$el.style.overflowY = "hidden";
    footer.value = r.path.includes('/quicktag') || r.path.includes('/tageditor');
});

onUpdated(() => {
    setTimeout(() => {
        // @ts-ignore
        contentContainer.value!.$el.style.overflowY = "auto";
    }, 250);
});
</script>

<style lang='scss'>
.content {
    overflow-y: auto !important;
    height: calc(100vh);
    min-height: 100vh;
}

.fade-enter-active,
.fade-leave-active {
    transition-property: opacity;
    transition-duration: 0.25s;
}
.fade-enter-active {
    transition-delay: 0.25s;
}
.fade-enter,
.fade-leave-active {
    opacity: 0;
}

.update-snippet {
    text-align: left;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 12px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-fg-muted);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 280px;
    overflow-y: auto;
    margin: 0;
}
</style>
