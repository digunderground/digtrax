// Quick Tag column configuration, sort state, visibility, and width persistence.
// Used by QuickTagColumnHeader, QuickTag.vue (toolbar visibility menu), and QuickTagTileThin.
//
// Columns are derived dynamically from settings: a fixed prefix of static
// columns (Title..Key) followed by one column per configured QuickTag custom
// field (Vibe, Situation, …). Splitting customs into their own columns gives
// each a clear, labeled header instead of a single mystery "Custom" cell.
//
// Persisted-to-localStorage:
//   - per-column width
//   - per-column visibility
//
// The last *visible* column flexes to fill remaining width — fixes the
// "custom column gets cut off" issue when the window narrows.

import { ref, computed, watch } from 'vue';
import { get1t } from './digtrax';

export interface QtColumn {
    key: string;          // sort key — must match track property used in sort() in QuickTag.vue
    label: string;        // display label in column header
    width: number;        // current width in px
    minWidth: number;     // resize lower bound
    sortable: boolean;
    visible: boolean;
    kind: 'static' | 'custom';
    customIndex?: number; // for kind === 'custom', index into quickTag.custom[]
}

const WIDTH_KEY = 'digtrax-qt-columns';        // legacy: { key: width }
const VIS_KEY = 'digtrax-qt-columns-visible';  // { key: bool }

const STATIC_DEFAULTS: Omit<QtColumn, 'visible'>[] = [
    { key: 'title',  label: 'Title',  width: 280, minWidth: 120, sortable: true, kind: 'static' },
    { key: 'artist', label: 'Artist', width: 220, minWidth: 100, sortable: true, kind: 'static' },
    { key: 'mood',   label: 'Mood',   width: 120, minWidth: 70,  sortable: true, kind: 'static' },
    { key: 'energy', label: 'Energy', width: 110, minWidth: 80,  sortable: true, kind: 'static' },
    { key: 'genre',  label: 'Genre',  width: 180, minWidth: 90,  sortable: true, kind: 'static' },
    { key: 'year',   label: 'Year',   width: 70,  minWidth: 50,  sortable: true, kind: 'static' },
    { key: 'bpm',    label: 'BPM',    width: 70,  minWidth: 50,  sortable: true, kind: 'static' },
    { key: 'key',    label: 'Key',    width: 60,  minWidth: 40,  sortable: true, kind: 'static' },
];

function loadJson<T>(key: string): T | undefined {
    try {
        const raw = localStorage.getItem(key);
        if (!raw) return undefined;
        const parsed = JSON.parse(raw);
        return parsed as T;
    } catch (_) { return undefined; }
}
function saveJson(key: string, val: unknown) {
    try { localStorage.setItem(key, JSON.stringify(val)); } catch (_) { /* ignore */ }
}

const widthMap = ref<Record<string, number>>(loadJson<Record<string, number>>(WIDTH_KEY) ?? {});
const visMap = ref<Record<string, boolean>>(loadJson<Record<string, boolean>>(VIS_KEY) ?? {});

watch(widthMap, (v) => saveJson(WIDTH_KEY, v), { deep: true });
watch(visMap, (v) => saveJson(VIS_KEY, v), { deep: true });

// Derive the column list from settings. Computed so it updates live when the
// user edits qt.custom in Settings (adding/renaming custom fields auto-syncs
// the row table).
export const columns = computed<QtColumn[]>(() => {
    const $1t = get1t();
    const customs = (($1t.settings.value as any)?.quickTag?.custom ?? []) as Array<{ name: string }>;
    const customCols: QtColumn[] = customs.map((c, i) => ({
        key: `custom_${i}`,
        label: c?.name ?? `Custom ${i + 1}`,
        width: widthMap.value[`custom_${i}`] ?? 140,
        minWidth: 80,
        sortable: false,
        visible: visMap.value[`custom_${i}`] ?? true,
        kind: 'custom' as const,
        customIndex: i,
    }));
    const statics: QtColumn[] = STATIC_DEFAULTS.map((d) => ({
        ...d,
        width: widthMap.value[d.key] ?? d.width,
        visible: visMap.value[d.key] ?? true,
    }));
    return [...statics, ...customCols];
});

// Visible-only subset — what the header + row should render in column order.
export const visibleColumns = computed<QtColumn[]>(() => columns.value.filter((c) => c.visible));

// Computed CSS grid-template-columns string for the row + header. The last
// *visible* column uses minmax(width, 1fr) so it absorbs remaining space.
export const colTemplate = computed(() => {
    const visible = visibleColumns.value;
    return visible.map((c, i) => {
        if (i === visible.length - 1) return `minmax(${c.width}px, 1fr)`;
        return `${c.width}px`;
    }).join(' ');
});

export function setColumnWidth(key: string, width: number) {
    const col = columns.value.find((c) => c.key === key);
    if (!col) return;
    widthMap.value = { ...widthMap.value, [key]: Math.max(col.minWidth, Math.round(width)) };
}

export function setColumnVisible(key: string, on: boolean) {
    visMap.value = { ...visMap.value, [key]: on };
}

export function isColumnVisible(key: string): boolean {
    return visMap.value[key] ?? true;
}

export function resetColumns() {
    widthMap.value = {};
    visMap.value = {};
}
