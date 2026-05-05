<template>
<div class='dt-fb'>

    <!-- Path -->
    <div class='dt-fb-section'>// Path</div>
    <div class='dt-fb-path' v-if='!editPath' @click='editPath = true' :title='path'>
        <q-icon name='mdi-folder' size='14px' class='dt-fb-path-icon'></q-icon>
        <span class='dt-fb-path-text'>{{ pathLabel }}</span>
        <q-icon name='mdi-pencil' size='12px' class='dt-fb-path-edit'></q-icon>
    </div>
    <div class='dt-fb-pathedit' v-if='editPath'>
        <form @submit.prevent='loadFiles()'>
            <q-input outlined dense v-model='path' @blur='editPath = false' autofocus></q-input>
        </form>
    </div>
    <button class='dt-fb-browse' @click='browse'>
        <q-icon name='mdi-folder-open' size='14px'></q-icon>
        <span>Browse…</span>
    </button>

    <!-- Filter -->
    <div class='dt-fb-section'>// Filter</div>
    <q-input dense filled placeholder='Filter files' class='dt-fb-filter' @update:model-value='applyFilter' v-model='filter'>
        <template v-slot:prepend>
            <q-icon name='mdi-magnify' size='14px'></q-icon>
        </template>
    </q-input>

    <!-- Files -->
    <div class='dt-fb-section'>// Files</div>

    <button class='dt-fb-item dt-fb-parent' @click='loadFiles("..")'>
        <q-icon name='mdi-folder-upload' size='14px' class='dt-fb-item-icon'></q-icon>
        <span class='dt-fb-item-label'>Parent folder</span>
    </button>

    <button
        v-for='file in files'
        :key='file.filename'
        class='dt-fb-item'
        :class='{ "dt-fb-active": isSelected(file.path) }'
        @click='(file.dir || file.playlist) ? loadFiles(file.filename) : loadFiles(file.path)'
        :title='file.filename'
    >
        <q-icon
            size='14px'
            class='dt-fb-item-icon'
            :name="file.dir ? 'mdi-folder' : (file.playlist ? 'mdi-playlist-music' : 'mdi-music')"
        ></q-icon>
        <span class='dt-fb-item-label'>{{ file.filename }}</span>
    </button>

</div>
</template>

<script lang='ts' setup>
import { computed, onMounted, ref } from 'vue';
import { get1t } from '../scripts/digtrax.js';

const $1t = get1t();
const path = ref($1t.settings.value.path);
const files = ref<any[]>([]);
const originalFiles = ref<any[]>([]);
const filter = ref<string | undefined>(undefined);
const initial = ref(true);
const editPath = ref(false);

// Show just the last 2 path segments to keep the sidebar compact.
const pathLabel = computed(() => {
    const p = path.value || '';
    const sep = p.includes('\\') ? '\\' : '/';
    const parts = p.split(sep).filter(Boolean);
    if (parts.length === 0) return 'No folder';
    if (parts.length === 1) return parts[0];
    return '…/' + parts.slice(-2).join('/');
});

function loadFiles(f?: string) {
    $1t.send('quickTagFolder', {path: path.value, subdir: f});
}

function browse() {
    $1t.browse('qt', path.value);
}

function applyFilter() {
    if (!filter.value || filter.value.trim().length == 0) {
        files.value = originalFiles.value;
        return;
    }
    files.value = originalFiles.value.filter(f => f.filename.toLowerCase().includes(filter.value?.toLowerCase()));
}

function isSelected(p: string) {
    return p == $1t.settings.value.path;
}

onMounted(() => {
    path.value = $1t.settings.value.path;
    $1t.onQuickTagBrowserEvent = (json) => {
        switch (json.action) {
            case 'quickTagFolder':
                if (!initial.value) {
                    $1t.settings.value.path = json.path;
                    $1t.loadQuickTag();
                }
                initial.value = false;
                if (json.files.length == 0) return;
                files.value = json.files;
                originalFiles.value = json.files;
                path.value = json.path;
                break;
            case 'pathUpdate':
                initial.value = true;
                $1t.send('quickTagFolder', { path: $1t.settings.value.path, subdir: '..' });
        }
    }
    initial.value = true;
    loadFiles('..');
});
</script>

<style lang="scss" scoped>
.dt-fb {
    height: 100%;
    width: 100%;
    padding: 12px 8px 16px;
    overflow-y: auto;
    color: var(--color-fg);
}

.dt-fb-section {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    color: var(--color-fg-subtle);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 14px 10px 6px;
    user-select: none;
}

.dt-fb-section:first-child {
    padding-top: 4px;
}

/* Path display */
.dt-fb-path {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
    cursor: pointer;
    margin: 0 4px 4px;
    transition: border-color var(--duration-fast) var(--ease-standard),
                background var(--duration-fast) var(--ease-standard);
}

.dt-fb-path:hover {
    border-color: var(--color-border-strong);
    background: rgba(255, 255, 255, 0.05);
}

.dt-fb-path-icon {
    color: var(--color-accent);
    flex-shrink: 0;
}

.dt-fb-path-text {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.dt-fb-path-edit {
    color: var(--color-fg-subtle);
    flex-shrink: 0;
}

.dt-fb-pathedit {
    margin: 0 4px 4px;
}

/* Browse button */
.dt-fb-browse {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 12px;
    background: transparent;
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-sm);
    color: var(--color-fg-muted);
    font: inherit;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 500;
    cursor: pointer;
    margin: 0 4px;
    transition: all var(--duration-fast) var(--ease-standard);
}

.dt-fb-browse:hover {
    border-color: var(--color-accent);
    color: var(--color-accent);
    box-shadow: 0 0 12px var(--color-accent-glow);
}

/* Filter */
.dt-fb-filter {
    margin: 0 4px;
}

.dt-fb-filter :deep(.q-field__control) {
    background: rgba(255, 255, 255, 0.03) !important;
    border-radius: var(--radius-sm) !important;
    height: 32px !important;
    min-height: 32px !important;
}

.dt-fb-filter :deep(.q-field__native) {
    min-height: 32px !important;
    font-size: 12px;
}

.dt-fb-filter :deep(.q-field__control::before) {
    border: none !important;
}

/* File items */
.dt-fb-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: transparent;
    border: none;
    border-radius: var(--radius-xs);
    color: var(--color-fg-muted);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    margin: 1px 4px;
    width: calc(100% - 8px);
    text-align: left;
    transition: background var(--duration-fast) var(--ease-standard),
                color var(--duration-fast) var(--ease-standard);
    position: relative;
}

.dt-fb-item:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--color-fg);
}

.dt-fb-active {
    background: rgba(0, 210, 191, 0.08) !important;
    color: var(--color-fg) !important;
}

.dt-fb-active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 2px;
    background: var(--color-accent);
    border-radius: 1px;
    box-shadow: 0 0 6px var(--color-accent-glow);
}

.dt-fb-parent {
    color: var(--color-fg-subtle);
}

.dt-fb-parent:hover {
    color: var(--color-fg-muted);
}

.dt-fb-item-icon {
    flex-shrink: 0;
    color: inherit;
    opacity: 0.85;
}

.dt-fb-item-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
</style>
