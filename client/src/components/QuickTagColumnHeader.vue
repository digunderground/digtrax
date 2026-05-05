<template>
<div class="qt-colheader" :class="{ 'qt-colheader-cards': cards }" :style="{ '--col-template': colTemplate }">
    <div
        v-for="(col, i) in visibleColumns"
        :key="col.key"
        class="qt-col"
        :class="{ active: sortOption === col.key, 'qt-col-last': i === visibleColumns.length - 1 }"
        @click="col.sortable ? onSort(col.key) : null"
        :title="col.sortable ? `Sort by ${col.label}` : col.label"
        :style="{ cursor: col.sortable ? 'pointer' : 'default' }"
    >
        <span class="qt-col-label">{{ col.label }}</span>
        <span class="qt-col-arrow" v-if="sortOption === col.key">
            <q-icon :name="sortDescending ? 'mdi-arrow-down' : 'mdi-arrow-up'" size="13px"></q-icon>
        </span>
        <!-- Resize handle (rows mode only — cards have no aligned columns to size against) -->
        <span
            v-if="!cards && i < visibleColumns.length - 1"
            class="qt-col-resize"
            @mousedown.stop="(e) => startResize(e, col.key)"
            @click.stop
            title="Drag to resize"
        ></span>
    </div>
</div>
</template>

<script lang="ts" setup>
import { visibleColumns, colTemplate, setColumnWidth } from '../scripts/qtColumns';

defineProps<{
    sortOption: string;
    sortDescending: boolean;
    cards: boolean;
}>();

const emit = defineEmits<{
    (e: 'sort', key: string): void;
}>();

function onSort(key: string) {
    emit('sort', key);
}

let resizing: { key: string; startX: number; startWidth: number } | null = null;

function startResize(e: MouseEvent, key: string) {
    const col = visibleColumns.value.find(c => c.key === key);
    if (!col) return;
    resizing = { key, startX: e.clientX, startWidth: col.width };
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    window.addEventListener('mousemove', onResize);
    window.addEventListener('mouseup', endResize, { once: true });
    e.preventDefault();
}

function onResize(e: MouseEvent) {
    if (!resizing) return;
    const delta = e.clientX - resizing.startX;
    setColumnWidth(resizing.key, resizing.startWidth + delta);
}

function endResize() {
    resizing = null;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    window.removeEventListener('mousemove', onResize);
}
</script>

<style lang="scss" scoped>
.qt-colheader {
    display: grid;
    grid-template-columns: var(--col-template);
    align-items: stretch;
    height: 36px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-elevated);
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    overflow-x: auto;
    overflow-y: hidden;
    user-select: none;
    position: sticky;
    top: 0;
    z-index: 5;
}

.qt-colheader-cards {
    /* Cards mode — header still sortable, just no resize handles */
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    margin-bottom: 8px;
}

.qt-col {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 12px;
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-fg-subtle);
    cursor: pointer;
    position: relative;
    transition: color var(--duration-fast) var(--ease-standard),
                background var(--duration-fast) var(--ease-standard);
    border-right: 1px solid var(--color-border);
}

.qt-col.qt-col-last {
    border-right: none;
}

.qt-col:hover {
    color: var(--color-fg-muted);
    background: rgba(255, 255, 255, 0.02);
}

.qt-col.active {
    color: var(--color-accent);
    background: rgba(0, 210, 191, 0.05);
}

.qt-col-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.qt-col-arrow {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    color: var(--color-accent);
    text-shadow: 0 0 6px var(--color-accent-glow);
}

/* Resize handle: thin strip on the right edge, wider hit area for easier grabbing */
.qt-col-resize {
    position: absolute;
    top: 0;
    right: -3px;
    width: 6px;
    height: 100%;
    cursor: col-resize;
    z-index: 6;
}

.qt-col-resize:hover {
    background: var(--color-accent);
    box-shadow: 0 0 6px var(--color-accent-glow);
}
</style>
