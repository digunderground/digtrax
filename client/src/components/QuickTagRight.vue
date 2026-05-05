<template>
<div class='dt-insp'>
    <!-- Quick action: add custom note -->
    <button
        class='dt-insp-action'
        v-if='$1t.quickTag.value.track'
        @click='$1t.onQuickTagEvent("onNoteTag")'
    >
        <q-icon name='mdi-note-plus-outline' size='14px'></q-icon>
        <span>Add custom note</span>
    </button>

    <!-- Manual tag (when single track selected) -->
    <button
        class='dt-insp-action dt-insp-action-primary'
        v-if='$1t.quickTag.value.track && $1t.quickTag.value.track.tracks.length == 1'
        @click='$1t.onQuickTagEvent("onManualTag", {path: $1t.quickTag.value.track.tracks[0].path})'
    >
        <q-icon name='mdi-tag-search' size='14px'></q-icon>
        <span>Manual tag</span>
    </button>

    <!-- Custom tag groups -->
    <div v-for='(tag, i) in $1t.settings.value.quickTag.custom' :key='"tag"+i' class='dt-insp-group'>
        <q-expansion-item
            :label='tag.name'
            class='dt-insp-expansion'
            default-opened
            :model-value="true"
            :switch-toggle-side='false'
            header-class='dt-insp-group-header'
        >
            <div class='dt-insp-values'>
                <q-checkbox
                    v-for='(value, j) in tag.values'
                    :key='i+"value"+j'
                    :label='value.val'
                    :model-value='selected(i, value.val)'
                    @update:model-value='valueClick(i, value.val)'
                    dense
                    class='dt-insp-checkbox'
                ></q-checkbox>
            </div>

            <q-input
                ref='addNewTagRef'
                dense filled
                placeholder="Add value"
                @keypress.enter="addNewTag"
                v-if='newTag == i'
                v-model='newTagValue'
                class='dt-insp-input'
            ></q-input>

            <button
                v-if='newTag == -1'
                class='dt-insp-add'
                @click='showNewTag(i)'
            >
                <q-icon name='mdi-plus' size='12px'></q-icon>
                <span>Add value</span>
            </button>
        </q-expansion-item>
    </div>
</div>
</template>

<script lang='ts' setup>
import { ref } from 'vue';
import { get1t } from '../scripts/digtrax.js';

const $1t = get1t();
const newTag = ref(-1);
const newTagValue = ref<string | undefined>(undefined);

function selected(tag: number, value: string) {
    return $1t.quickTag.value.track.getCustom(tag, value);
}

function valueClick(tag: number, value: string) {
    $1t.quickTag.value.track.toggleCustom(tag, value);
}

const addNewTagRef = ref<any>();
function showNewTag(i: number) {
    newTag.value = i;
    setTimeout(() => {
        addNewTagRef.value[0].focus();
    }, 25);
}

function addNewTag() {
    if (newTagValue.value) {
        $1t.settings.value.quickTag.custom[newTag.value].values.push({val: newTagValue.value, keybind: undefined});
        $1t.saveSettings();
    }
    newTag.value = -1;
    newTagValue.value = undefined;
}
</script>

<style lang="scss" scoped>
.dt-insp {
    height: 100%;
    width: 100%;
    padding: 14px 12px;
    overflow-y: auto;
    color: var(--color-fg);
}

/* Action buttons (Add note, Manual tag) */
.dt-insp-action {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-fg-muted);
    font: inherit;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 500;
    cursor: pointer;
    margin-bottom: 8px;
    transition: all var(--duration-fast) var(--ease-standard);
}

.dt-insp-action:hover {
    border-color: var(--color-border-strong);
    color: var(--color-fg);
    background: rgba(255, 255, 255, 0.06);
}

.dt-insp-action-primary {
    background: rgba(0, 210, 191, 0.08);
    border-color: rgba(0, 210, 191, 0.4);
    color: var(--color-accent);
}

.dt-insp-action-primary:hover {
    background: rgba(0, 210, 191, 0.16);
    border-color: var(--color-accent);
    color: var(--color-accent);
    box-shadow: 0 0 12px var(--color-accent-glow);
}

/* Custom tag groups */
.dt-insp-group {
    margin-top: 16px;
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    overflow: hidden;
}

.dt-insp-expansion :deep(.q-expansion-item__container) > :deep(.q-item) {
    padding: 0 !important;
    min-height: 0 !important;
}

.dt-insp-group :deep(.q-item__label) {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-fg);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 8px 12px;
}

.dt-insp-group :deep(.q-item__section--side) {
    color: var(--color-fg-subtle);
    min-width: 0;
    padding: 0 8px;
}

.dt-insp-values {
    padding: 4px 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.dt-insp-checkbox {
    margin: 0 !important;
}

.dt-insp-checkbox :deep(.q-checkbox__label) {
    font-size: 12px;
    color: var(--color-fg-muted);
    margin-left: 6px !important;
}

.dt-insp-input {
    margin: 6px 12px 8px;
}

.dt-insp-add {
    display: flex;
    align-items: center;
    gap: 6px;
    width: calc(100% - 24px);
    margin: 4px 12px 8px;
    padding: 6px 10px;
    background: transparent;
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-xs);
    color: var(--color-fg-subtle);
    font: inherit;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-standard);
}

.dt-insp-add:hover {
    border-color: var(--color-accent);
    color: var(--color-accent);
}
</style>
