<template>
<aside class="dt-nav">
    <!-- Brand mark -->
    <div class="dt-nav-brand">
        <img src="../assets/icon.png" height="32" class="dt-nav-icon" :class="{ spin: $1t.lock.value.locked }" />
        <span class="dt-nav-brandtext">
            <span class="dig">DIG</span><span class="trax">TRAX</span>
        </span>
    </div>

    <div class="dt-nav-section">// Tools</div>

    <router-link
        v-for="route in routes"
        :key="route.to"
        :to="route.to"
        custom
        v-slot="{ navigate, isActive }"
    >
        <button
            class="dt-nav-item"
            :class="{ active: isActive }"
            :disabled="$1t.lock.value.locked"
            @click="onNav(route, navigate)"
        >
            <q-icon :name="route.icon" size="20px" class="dt-nav-itemicon"></q-icon>
            <span class="dt-nav-itemlabel">{{ route.label }}</span>
        </button>
    </router-link>

    <div class="dt-nav-spacer"></div>

    <div class="dt-nav-section">// Theme</div>

    <div class="dt-nav-themetoggle">
        <button
            v-for="opt in themeOptions"
            :key="opt.value"
            class="dt-nav-themebtn"
            :class="{ active: currentTheme === opt.value }"
            :title="opt.label"
            @click="onThemeChange(opt.value)"
        >
            <q-icon :name="opt.icon" size="16px"></q-icon>
        </button>
    </div>

    <router-link to="/settings" custom v-slot="{ navigate, isActive }">
        <button
            class="dt-nav-item dt-nav-settings"
            :class="{ active: isActive }"
            @click="navigate"
            :disabled="$1t.lock.value.locked"
        >
            <q-icon name="mdi-cog" size="20px" class="dt-nav-itemicon"></q-icon>
            <span class="dt-nav-itemlabel">Settings</span>
        </button>
    </router-link>
</aside>
</template>

<script lang="ts" setup>
import { ref } from 'vue';
import { get1t } from '../scripts/digtrax.js';
import { getTheme, setTheme, type ThemeMode } from '../scripts/theme.js';

const $1t = get1t();

// Feature flag — hide Audio Features for the V4 redesign cycle. The view
// itself still exists at /audiofeatures so deep links keep working; only the
// sidebar entry is hidden. Flip to true once the rework lands.
// See plan/01-ui-redesign.md → "Deferred — Auto Tag follow-ups".
const SHOW_AUDIO_FEATURES = false;

const routes = [
    { to: '/quicktag',      label: 'Quick Tag',      icon: 'mdi-flash' },
    { to: '/tageditor',     label: 'Tag Editor',     icon: 'mdi-pencil' },
    { to: '/autotagger',    label: 'Auto Tag',       icon: 'mdi-tag-multiple' },
    ...(SHOW_AUDIO_FEATURES
        ? [{ to: '/audiofeatures', label: 'Audio Features', icon: 'mdi-waveform' }]
        : []),
    { to: '/renamer',       label: 'Renamer',        icon: 'mdi-rename-box' },
];

const themeOptions: { value: ThemeMode; label: string; icon: string }[] = [
    { value: 'dark',   label: 'Dark',   icon: 'mdi-weather-night' },
    { value: 'light',  label: 'Light',  icon: 'mdi-weather-sunny' },
    { value: 'system', label: 'System', icon: 'mdi-monitor' },
];

const currentTheme = ref<ThemeMode>(getTheme());

function onThemeChange(t: ThemeMode) {
    currentTheme.value = t;
    setTheme(t);
}

function onNav(_route: any, navigate: () => void) {
    if ($1t.lock.value.locked) return;
    navigate();
}
</script>

<style lang="scss" scoped>
.dt-nav {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    padding: var(--space-3) var(--space-2);
    background: var(--color-bg-elevated);
    border-right: 1px solid var(--color-border);
    color: var(--color-fg);
    overflow-y: auto;
}

.dt-nav-brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    margin-bottom: var(--space-3);
}

.dt-nav-icon {
    cursor: pointer;
}

.dt-nav-brandtext {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 18px;
    letter-spacing: 0.02em;
    line-height: 1;
}
.dt-nav-brandtext .dig { color: var(--color-fg); }
.dt-nav-brandtext .trax {
    color: var(--color-accent);
    text-shadow: 0 0 12px var(--color-accent-glow);
}

.dt-nav-section {
    font-size: 10px;
    font-weight: 700;
    color: var(--color-fg-subtle);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: var(--space-3) var(--space-2) var(--space-1);
    user-select: none;
}

.dt-nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: none;
    background: transparent;
    color: var(--color-fg-muted);
    font: inherit;
    font-size: 13px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    transition: background var(--duration-fast) var(--ease-standard),
                color var(--duration-fast) var(--ease-standard);
    position: relative;
    margin: 1px 0;
    width: 100%;
}

.dt-nav-item:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.04);
    color: var(--color-fg);
}

.dt-nav-item.active {
    background: linear-gradient(90deg, rgba(0, 210, 191, 0.15), rgba(0, 210, 191, 0.04));
    color: var(--color-fg);
}

.dt-nav-item.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    background: var(--color-accent);
    border-radius: 2px;
    box-shadow: 0 0 10px var(--color-accent-glow);
}

.dt-nav-item:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}

.dt-nav-itemicon {
    flex-shrink: 0;
}

.dt-nav-itemlabel {
    flex: 1;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 12px;
    font-weight: 500;
}

.dt-nav-spacer {
    flex: 1;
}

.dt-nav-themetoggle {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    margin: 0 var(--space-2) var(--space-2);
}

.dt-nav-themebtn {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--color-fg-muted);
    padding: 6px;
    border-radius: var(--radius-xs);
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-standard);
    display: flex;
    align-items: center;
    justify-content: center;
}

.dt-nav-themebtn:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--color-fg);
}

.dt-nav-themebtn.active {
    background: var(--color-accent);
    color: #002b27;
    box-shadow: 0 0 8px var(--color-accent-glow);
}

.dt-nav-settings {
    margin-bottom: var(--space-2);
}

@keyframes rotation {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
}

.spin {
    animation: rotation 2s infinite linear;
}
</style>
