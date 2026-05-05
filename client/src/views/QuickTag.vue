<template>
<div>
    <div class="qt-shell">
      <aside class="qt-pane qt-pane-left">
        <QuickTagFileBrowser></QuickTagFileBrowser>
      </aside>
      <main class="qt-pane qt-pane-center" :style="{ '--col-template': colTemplate }">
        <!-- Compact toolbar: search + stats + cards/rows toggle -->
        <div class="qt-toolbar">
            <q-input
                dense
                v-model='filter'
                filled
                placeholder='Filter'
                class='qt-search'
                @update:model-value='filterTracks()'
            >
                <template v-slot:prepend>
                    <q-icon name="mdi-magnify" size="16px"></q-icon>
                </template>
                <template v-slot:append v-if='filter'>
                    <q-icon name="mdi-close" size="16px" class='cursor-pointer' @click='filter = ""; filterTracks()'></q-icon>
                </template>
            </q-input>

            <div class="qt-stats">
                <span class="qt-stats-num">{{ tracks.length }}</span>
                <span class="qt-stats-of">of</span>
                <span class="qt-stats-num">{{ $1t.quickTag.value.tracks.length }}</span>
                <span
                    v-if='$1t.quickTag.value.failed.length != 0'
                    class="qt-stats-warn"
                    @click='failedDialog = true'
                    title="Failed to load — click for details"
                >
                    · {{ $1t.quickTag.value.failed.length }} failed
                </span>
                <span
                    v-if='$1t.quickTag.value.isLimited()'
                    class="qt-stats-warn"
                    @click='$1t.loadQuickTag(undefined, false)'
                    title="Capped at 500 — click to load all"
                >
                    · capped
                </span>
            </div>

            <div class="qt-toolbar-spacer"></div>

            <!-- Column visibility menu — only meaningful in rows mode where the
                 grid header drives layout. Custom fields (Vibe, Situation, …)
                 appear in this list automatically because they're derived from
                 settings.quickTag.custom in qtColumns.ts. -->
            <q-btn
                v-if='$1t.settings.value.quickTag.thinTracks'
                flat dense
                size='sm'
                icon='mdi-view-column-outline'
                class='qt-cols-btn'
                title='Show / hide columns'
            >
                <q-menu class='qt-cols-menu'>
                    <q-list dense>
                        <q-item-label header class='qt-cols-header'>Columns</q-item-label>
                        <q-item
                            v-for='col in allColumns'
                            :key='col.key'
                            clickable
                            v-close-popup.once
                            @click='toggleColumn(col.key)'
                        >
                            <q-item-section side>
                                <q-icon
                                    :name='col.visible ? "mdi-checkbox-marked" : "mdi-checkbox-blank-outline"'
                                    :color='col.visible ? "primary" : "grey-6"'
                                    size='16px'
                                />
                            </q-item-section>
                            <q-item-section>
                                <q-item-label>{{ col.label }}</q-item-label>
                                <q-item-label caption v-if='col.kind === "custom"'>custom field</q-item-label>
                            </q-item-section>
                        </q-item>
                    </q-list>
                </q-menu>
            </q-btn>

            <div class="qt-view-toggle">
                <button
                    class="qt-view-btn"
                    :class="{ active: !$1t.settings.value.quickTag.thinTracks }"
                    @click="$1t.settings.value.quickTag.thinTracks = false"
                    title="Cards view"
                >
                    <q-icon name="mdi-view-grid" size="14px"></q-icon>
                    <span>Cards</span>
                </button>
                <button
                    class="qt-view-btn"
                    :class="{ active: $1t.settings.value.quickTag.thinTracks }"
                    @click="$1t.settings.value.quickTag.thinTracks = true"
                    title="Rows view (↑↓ to navigate)"
                >
                    <q-icon name="mdi-view-list" size="14px"></q-icon>
                    <span>Rows</span>
                </button>
            </div>
        </div>

        <!-- Sortable column header (click to sort, drag dividers to resize in Rows mode) -->
        <QuickTagColumnHeader
            :sort-option='sortOption'
            :sort-descending='sortDescending'
            :cards='!$1t.settings.value.quickTag.thinTracks'
            @sort='sort'
        ></QuickTagColumnHeader>

    <!-- Tracks -->
    <div class='tracklist qt-full-height' v-if='$1t.quickTag.value.tracks.length > 0' ref='tracklist' :class='{"qt-height": $1t.quickTag.value.track}' @scroll='onScroll'>

        <!-- Card grid (V4) -->
        <div class='qt-cards-grid' v-if='!$1t.settings.value.quickTag.thinTracks'>
            <div v-for='item in tracks' :key='item.path' class='qt-card-grid-item'>
                <q-intersection style='height: 100%;' @click.native='(e: MouseEvent) => trackClick(item, e)' once>
                    <QuickTagTile :track='item' :no-art-cache="noArtCacheList.includes(item.path)"></QuickTagTile>
                    <QuickTagContextMenu
                        @manual-tag="onManualTag(item.path)"
                        :path="item.path"
                    ></QuickTagContextMenu>
                </q-intersection>
            </div>
        </div>
        <!-- Thin tracks -->
        <div :style='`width: ${tracklistWidth}`'>
            <div v-for='(item, i) in tracks' :key='item.path' v-if='$1t.settings.value.quickTag.thinTracks'>
                <q-intersection style='height: 40px;' @click.native='(e: MouseEvent) => trackClick(item, e)' once>
                    <QuickTagTileThin :track='item' :odd='i % 2 == 1'></QuickTagTileThin>
                    <QuickTagContextMenu 
                        @manual-tag="onManualTag(item.path)"
                        :path="item.path"
                    ></QuickTagContextMenu>
                </q-intersection>
            </div>
        </div>

        <!-- No results -->
        <div v-if='tracks.length == 0'>
            <div class='text-center text-h5 text-grey-6 q-my-lg'>No results!</div>
        </div>
    </div>

    <div v-if='$1t.quickTag.value.tracks.length == 0' class='qtbg-container qt-full-height'>
        <!-- Loading -->
        <div v-if='$1t.lock.value.locked' class='row justify-center'>
            <q-circular-progress indeterminate color='primary' size='64px'></q-circular-progress>
        </div>

        <!-- No path selected -->
        <div @click='selectFolder' v-if='!$1t.lock.value.locked'>
            <div class='text-center text-subtitle2 text-bold text-primary q-my-sm'>NO FOLDER SELECTED</div>
            <div class='text-center text-subtitle2 text-grey-6'><span class='keybind-icon q-px-sm text-caption text-bold'>CLICK</span> here to select folder</div>
            
            <div class="q-pa-lg q-mt-lg">
                <div class="row q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Play / Pause:
                    </div>

                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                    <q-icon name='mdi-keyboard-space' class='keybind-icon'></q-icon>
                    </div>            
                </div>

                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Seek back / forwards:        
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-chevron-left' class='keybind-icon q-mr-xs'></q-icon> / <q-icon name='mdi-chevron-right' class='keybind-icon'></q-icon>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Switch Track:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-chevron-up' class='keybind-icon q-mr-xs'></q-icon> / <q-icon name='mdi-chevron-down' class='keybind-icon'></q-icon>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Select multiple:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-apple-keyboard-control' class='keybind-icon q-mr-xs'></q-icon> + <span class='keybind-icon q-px-sm'>CLICK</span>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Save:      
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-apple-keyboard-control' class='keybind-icon q-mr-xs'></q-icon> + <span class='keybind-icon q-px-sm'>S</span>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Delete:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-apple-keyboard-control' class='keybind-icon q-mr-xs'></q-icon> + <span class='keybind-icon q-px-sm'>DEL</span>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Confirm:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-keyboard-return' class='keybind-icon'></q-icon>
                    </div> 
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Context Menu:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <span class='keybind-icon q-pl-sm'>RIGHT</span><span class='keybind-icon q-px-sm'>CLICK</span>
                    </div>
                </div>
            </div>
        </div>
    </div>
      </main>
      <aside class="qt-pane qt-pane-right" v-if="$1t.quickTag.value.track">
        <QuickTagRight></QuickTagRight>
      </aside>
    </div>

    <!-- Save dialog -->
    <q-dialog v-model='saveDialog'>
        <q-card>
            <q-card-section>
                <div class='text-h6 text-grey-3'>Unsaved changes</div>
            </q-card-section>
            <q-card-section>
                <div class='text-grey-3'>Warning, there are unsaved changes on this track, do you want to save them?</div>
            </q-card-section>
            <q-card-actions align='right'>
                <q-btn color='red' flat text @click='saveDialogCallback(false)'>Discard</q-btn>
                <q-btn color='primary' flat text @click='saveDialogCallback(true)' ref='saveButton'>Save</q-btn>
            </q-card-actions>
        </q-card>
    </q-dialog>

    <!-- Note tag dialog -->
    <q-dialog v-model='noteDialog' @show='onNoteDialogShow'>
        <q-card v-if='$1t.quickTag.value.track'>
            <q-card-section>
                <div class='text-h6'>Custom note</div>
            </q-card-section>
            <q-card-section>
                <q-input
                    filled
                    dense
                    label="Note tag"
                    style='width: 256px;'
                    :model-value='$1t.quickTag.value.track.getNote()'
                    @update:model-value='(d: any) => $1t.quickTag.value.track!.setNote(d)'
                    @keyup.enter="noteDialog = false"
                    ref='noteDialogInput'
                ></q-input>
            </q-card-section>
        </q-card>
    </q-dialog>

    <!-- Failed files dialog -->
    <q-dialog v-model='failedDialog'>
        <q-card class='q-pa-md'>
            <q-card-section>
                <div class='text-subtitle2 text-bold text-center text-red'>FAILED TO LOAD</div>
            </q-card-section>
            <q-card-section>
                <div>
                    <div v-for='failed in $1t.quickTag.value.failed' class='q-my-sm'>
                        <div class='text-subtitle3 text-grey-4 monospace'>{{failed.path}}</div>
                        <div class='text-body2 text-red'>{{failed.error}}</div>
                    </div>
                </div>
            </q-card-section>
            <q-card-section horizontal>
                <q-space></q-space>
                <q-btn flat color='primary' @click='failedDialog = false'>Close</q-btn>
            </q-card-section>
        </q-card>
    </q-dialog>

    <!-- Manual Tagger -->
    <ManualTag :path='manualTagPath' @exit='onManualTagDone'></ManualTag>

</div>
</template>

<script lang='ts' setup>
import { scroll, useQuasar } from 'quasar';
import { Ref, computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { get1t } from '../scripts/digtrax.js';
import { CustomTagInfo, QTTrack } from '../scripts/quicktag.js';

import ManualTag from '../components/ManualTag.vue';
import QuickTagTile from '../components/QuickTagTile.vue';
import QuickTagTileThin from '../components/QuickTagTileThin.vue';
import QuickTagContextMenu from '../components/QuickTagContextMenu.vue';
import QuickTagFileBrowser from '../components/QuickTagFileBrowser.vue';
import QuickTagRight from '../components/QuickTagRight.vue';
import QuickTagColumnHeader from '../components/QuickTagColumnHeader.vue';
import { colTemplate, columns as allColumns, setColumnVisible } from '../scripts/qtColumns';

function toggleColumn(key: string) {
    const col = allColumns.value.find((c) => c.key === key);
    if (!col) return;
    setColumnVisible(key, !col.visible);
}

const { setVerticalScrollPosition } = scroll;

const $1t = get1t();
const $q = useQuasar();
const sortOptions = ['title', 'artist', 'mood', 'energy', 'genre', 'year', 'bpm', 'key', 'custom'];
const saveDialog = ref(false);
const noteDialog = ref(false);
const filter = ref<string | undefined>(undefined);
const sortDescending = ref(false);
const sortOption = ref('title');
const failedDialog = ref(false);
const manualTagPath = ref<string | undefined>(undefined);
const noArtCacheList = ref<string[]>([])

let afterSave: undefined | Function = undefined;

// Click on track card
function trackClick(track: QTTrack, event: MouseEvent) {
    // Add track to list
    if (event.ctrlKey || event.metaKey || ($1t.info.value.os == 'macos' && event.altKey)) {
        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation();
        
        selectionCursor = tracks.value.findIndex(t => t.path == track.path);
        $1t.toggleQTTrack(track);
        return;
    }

    // Expand to add range of tracks to list
    if (event.shiftKey) {
        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation();

        // No existing selection to expand
        if (selectionCursor === -1) {
            return;
        }

        const currentIndex = tracks.value.findIndex(t => t.path == track.path);
        const startIndex = Math.min(selectionCursor, currentIndex);
        const endIndex = Math.max(selectionCursor, currentIndex);

        for (let i = startIndex; i <= endIndex; i++) {
            $1t.addQTTrack(tracks.value[i]);
        }

        selectionCursor = currentIndex;
        return;
    }

    // Prevent clicking on same track
    if ($1t.quickTag.value.track.isSelected(track)) return;
    selectionCursor = tracks.value.findIndex(t => t.path == track.path);
    $1t.loadQTTrack(track);
}

// Save dialog callback
async function saveDialogCallback(save: boolean) {
    if (save) {
        await $1t.saveQTTrack();
        $q.notify({
            message: "Tags saved!",
            color: 'primary',
            textColor: 'black',
            timeout: 500,
            position: 'top-right'
        });
    }
    $1t.loadQTTrack(undefined, true);
    saveDialog.value = false;
    // focus on custom tags fix
    setTimeout(() => { $1t.quickTagUnfocus(); }, 50);

    // Do after save action
    if (afterSave) {
        afterSave();
        afterSave = undefined;
    }
}

// Select folder and load tracks
function selectFolder() {
    $1t.browse('qt');
}

// Focus
const noteDialogInput = ref<HTMLElement | undefined>();
function onNoteDialogShow() {
    noteDialogInput.value?.focus();
}

// Sort by option
function sort(option: string) {
    if (sortOption.value != option) {
        // reset sort direction
        sortDescending.value = false;
        sortOption.value = option;
    } else {
        sortDescending.value = !sortDescending.value;
    }
    // Unselect first
    $1t.quickTag.value.track.removeAll();
    filterTracks();
}

/// Filter tracks with search and sorting
function filterTracks() {
    let t = (() => {
        let tracks = $1t.quickTag.value.tracks;

        if (filter.value) {
            let newFilter = filter.value.toLowerCase();
            // title, artist or track or tags
            tracks = $1t.quickTag.value.tracks.filter((t) => 
                t.title.toLowerCase().match(newFilter) || t.path.toLowerCase().match(newFilter) ||
                t.artists.filter((a: any) => a.toLowerCase().match(newFilter)).length > 0 ||
                (t.mood??'').toLowerCase().match(newFilter) ||
                t.getAllCustom().some((i: CustomTagInfo) => i.value.toLowerCase().match(newFilter)) ||
                (t.genres??[]).some((i: any) => i.toLowerCase().match(newFilter)) 
            );
        }
        if (!sortOption.value) return tracks;

        // Sort
        tracks.sort((a, b) => {
            let va, vb;
            switch (sortOption.value) {
                // Arrays
                case 'artist':
                case 'genre':
                    va = a[`${sortOption.value}s`].join(', ').toLowerCase();
                    vb = b[`${sortOption.value}s`].join(', ').toLowerCase();
                    break;
                default:
                    va = (a as any)[sortOption.value]??''.toLowerCase();
                    vb = (b as any)[sortOption.value]??''.toLowerCase();
                    break;
            }

            // Compare
            if (va < vb) {
                return -1;
            }
            if (va > vb) {
                return 1;
            }
            return 0;
        });
        if (sortDescending.value) tracks.reverse();

        return tracks;
    })();

    // Unselect
    if (tracks.value.length != t.length) {
        $1t.quickTag.value.track.removeAll();
    }
    tracks.value = t;

    // Fix width
    fixTracklistWidth();
}

/// Find index of selected track in tracklist
function findIndex(highest: boolean = true) {
    var finalIndex = -1;
    for (let i=0; i < $1t.quickTag.value.track.tracks.length; i++) {
        let index = tracks.value.findIndex(t => t.path == $1t.quickTag.value.track.tracks[i].path);
        // Get at least some index
        if (finalIndex == -1 && index != -1) {
            finalIndex = index;
            continue;
        }
        // Highest index
        if (highest && index > finalIndex) {
            finalIndex = index;
            continue;
        }
        // Lowest index
        if (!highest && index != -1 && index < finalIndex) {
            finalIndex = index;
            continue;
        }
    }
    return finalIndex;
}

// On scroll event
function onScroll(e: Event) {
    // Fix width
    fixTracklistWidth(true);
}

// Open manual tag
async function onManualTag(path: string) {
    // Wait for save
    if ($1t.quickTag.value.track.isChanged()) {
        let promise = new Promise((res, _) => afterSave = res);
        $1t.onQuickTagEvent('onUnsavedChanges');
        await promise;
    }
    $1t.quickTag.value.track.removeAll();

    // Open
    manualTagPath.value = path;
}

// Manual tagging done
function onManualTagDone() {
    noArtCacheList.value.push(manualTagPath.value!);
    manualTagPath.value = undefined;
    $1t.loadQuickTag();
}

/// Export playlist from selected tracks or filtered
function generatePlaylist() {
    let paths = $1t.quickTag.value.track.tracks.map(t => t.path);
    if (paths.length == 0) {
        paths = tracks.value.map(t => t.path);
    }
    $1t.send('generatePlaylist', { paths });
}

// Scroll to track index
const tracklist = ref<HTMLElement | undefined>();
function scrollToIndex(index: number) {
    if ($1t.settings.value.quickTag.thinTracks) {
        setVerticalScrollPosition(tracklist.value!, index * 33 - (tracklist.value!.clientHeight / 68) * 34, 250);
        return;
    }
    setVerticalScrollPosition(tracklist.value!, index * 116 - 154, 250);
}

/// Update tracklist width to fit
const tracklistWidth = ref('100%');
function fixTracklistWidth(force = false) {
    if (force) {
        if (tracklist.value) {
            tracklistWidth.value = `${tracklist.value!.scrollWidth}px`;
        }
        return;
    }
    tracklistWidth.value = '100%';
    setTimeout(() => {
        if (tracklist.value) {
            tracklistWidth.value = `${tracklist.value!.scrollWidth}px`;
        }
    }, 20);
}
let resizeListener = () => fixTracklistWidth();
window.addEventListener('resize', resizeListener);

// Update track list
let tracks: Ref<QTTrack[]> = ref([]);
watch(() => $1t.quickTag.value.tracks, () => filterTracks());

/// Index of track for selection cursor
let selectionCursor = -1;
let selectionDirection = 0;


const saveButton = ref<any>();
onMounted(() => {
    $1t.onQuickTagEvent = (action, data) => {
        switch (action) {
            // Save dialog
            case 'onUnsavedChanges':
                // Autosave enabled
                if ($1t.settings.value.quickTag.autosave) {
                    saveDialogCallback(true);
                    return;
                }

                saveDialog.value = true;
                setTimeout(() => {
                    saveButton.value?.$el.focus()
                }, 100);
                break;

            // Note tag updated
            case 'onNoteTag':
                noteDialog.value = true;
                break;

            // Change track position relatively
            case 'changeTrack':
                var offset = data.offset as number;
                // Get largest index from selected tracks
                var i = findIndex(offset > 0);
                // Load next track
                if (i != -1 && (i + offset) != tracks.value.length && (i + offset) >= 0) {
                    $1t.loadQTTrack(tracks.value[i + offset], data.force??false);
                }
                break;

            // Add track to selection
            case 'addTrack':
                var offset = data.offset as number;
                
                // Update cursor
                if (offset == 0 || $1t.quickTag.value.track.tracks.length == 0) {
                    break;
                }
                if ($1t.quickTag.value.track.tracks.length == 1) {
                    selectionCursor = findIndex();
                }
                var i = selectionCursor;

                // Save directions and offsets to make the shift select working
                var normOffset = Math.min(Math.max(offset, -1), 1);
                if ($1t.quickTag.value.track.tracks.length > 1 && selectionDirection != 0 && selectionDirection != normOffset) {
                    offset = 0;
                }

                // Load next track
                if (i != -1 && (i + offset) != tracks.value.length && (i + offset) >= 0) {
                    // Save correct direction and offset
                    selectionCursor = i + offset;
                    selectionDirection = normOffset;

                    $1t.toggleQTTrack(tracks.value[i + offset]);
                }
                break;

            case 'focusSearch':
                break

            case 'quickTagLoad':
                if ($1t.settings.value.quickTag.trackIndex == -1 || $1t.quickTag.value.tracks.length == 0 || $1t.lock.value.locked) return;
                // Reload last opened track track
                setTimeout(() => {
                    $1t.loadQTTrack($1t.quickTag.value.tracks[$1t.settings.value.quickTag.trackIndex]);
                    $1t.settings.value.quickTag.trackIndex = -1;
                }, 50);

                break;

            case 'onDeleteTrack':
                // Confirm dialog
                $q.dialog({
                    title: 'Delete File',
                    message: 'Do you really want to delete the selected file(s)?',
                    persistent: false,
                    ok: {
                        color: 'red'                        
                    },
                    cancel: {
                        color: ''
                    }
                }).onOk(() => {
                    $1t.player.value.stop();
                    $1t.send('deleteFiles', { paths: $1t.quickTag.value.track.tracks.map(t => t.path) });
                    setTimeout(() => {
                        $1t.quickTag.value.track.removeAll();
                        $1t.loadQuickTag();
                    }, 50);
                });
                break;

            case 'quickTagSaved':
                filterTracks();
                break;

            // Manual tag trigger
            case 'onManualTag':
                onManualTag(data.path);
                return;

            case 'generatePlaylist':
                generatePlaylist();
                break;
                
            default:
                console.log(`Unknown QT Event: ${action} ${data}`);
                break;
        }
    }

    // Restore sort state
    sortOption.value = $1t.settings.value.quickTag.sortOption||'title';
    sortDescending.value = $1t.settings.value.quickTag.sortDescending === true;

    // Load tracks if path available
    $1t.loadQuickTag();
});

onUnmounted(() => {
    // Save track index if single
    if ($1t.quickTag.value.track.tracks.length == 1) {
        $1t.settings.value.quickTag.trackIndex = $1t.quickTag.value.tracks.findIndex((t) => $1t.quickTag.value.track.tracks[0].path == t.path);
    } else {
        $1t.settings.value.quickTag.trackIndex = -1;
    }

    // Save sorting
    $1t.settings.value.quickTag.sortOption = sortOption.value;
    $1t.settings.value.quickTag.sortDescending = sortDescending.value;

    // Unregister listener
    window.removeEventListener('resize', resizeListener);
});

/// Scroll to position
watch($1t.quickTag.value.track, () => {
    if ($1t.quickTag.value.track.tracks.length != 1) return;
    let index = tracks.value.findIndex((t) => $1t.quickTag.value.track.tracks[0].path == t.path);
    scrollToIndex(index);
});

</script>

<style lang='scss'>
.tracklist {
    overflow-y: auto;
    overflow-x: auto;
}

.qtbg-container {
    display: flex;
    flex-direction: column;
    justify-content: center;
    cursor: pointer;
    margin-top: -30px;
}

.qt-full-height {
    height: calc(100vh - 195px);
}

.qt-height {
    height: calc(100vh - 279px);
}

.keybind-icon {
    padding: 4px;
    border-radius: 2px;
    background: #262828;
    margin-bottom: 4px;
    margin-left: 4px;
}

.bar-bg {
    background: #00ff00;
}

.show-link {
    color: gray;
    text-decoration-line: underline;
}
.show-link:hover {
    color: #f0f0f0;
    text-decoration-line: underline;
}


.qt-search-bar {
    background-color: transparent !important;
    background: transparent !important;
}

.qt-search-bar * {
    background-color: transparent !important;
    background: transparent !important;
}

/* V4 — Quick Tag 3-pane shell. File browser on left, track content center, inspector right. */
.qt-shell {
    display: flex;
    height: calc(100vh - 8px);
    overflow: hidden;
    align-items: stretch;
}

.qt-pane {
    overflow-y: auto;
    overflow-x: hidden;
}

.qt-pane-left {
    width: 240px;
    flex-shrink: 0;
    background: var(--color-bg-elevated);
    border-right: 1px solid var(--color-border);
}

.qt-pane-center {
    flex: 1;
    min-width: 0;
    background: var(--color-bg);
    padding: 0 8px;
}

.qt-pane-right {
    width: 300px;
    flex-shrink: 0;
    background: var(--color-bg-elevated);
    border-left: 1px solid var(--color-border);
}

/* Override the legacy full-height calc inside the new pane (the pane itself scrolls). */
.qt-pane-center .qt-full-height {
    height: auto !important;
    min-height: calc(100vh - 240px);
}
.qt-pane-center .qt-height {
    height: auto !important;
    min-height: calc(100vh - 320px);
}

/* V4 — Cards grid (vertical tiles, multi-column) */
.qt-cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
    padding: 8px 4px 16px;
}

.qt-card-grid-item {
    height: 280px;
}

/* V4 — Compact toolbar (search + stats + view toggle on one row) */
.qt-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 4px;
    flex-wrap: nowrap;
}

.qt-search {
    width: 240px;
    flex-shrink: 0;
}

.qt-search :deep(.q-field__control) {
    background: rgba(255, 255, 255, 0.04) !important;
    border-radius: var(--radius-sm) !important;
    height: 32px !important;
    min-height: 32px !important;
}

.qt-search :deep(.q-field__native) {
    padding: 0 !important;
    min-height: 32px !important;
    font-size: 13px;
}

.qt-search :deep(.q-field__control::before) {
    border: none !important;
}

.qt-stats {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--color-fg-muted);
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    flex-shrink: 0;
}

.qt-stats-num {
    color: var(--color-fg);
    font-weight: 600;
}

.qt-stats-of {
    color: var(--color-fg-subtle);
    text-transform: lowercase;
}

.qt-stats-warn {
    color: var(--color-warning);
    cursor: pointer;
    margin-left: 4px;
    text-transform: none;
}

.qt-stats-warn:hover {
    text-decoration: underline;
}

.qt-toolbar-spacer {
    flex: 1;
}

/* Column visibility button — sits left of the Rows/Cards toggle. */
.qt-cols-btn {
    color: var(--color-fg-muted);
    margin-right: 8px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0 6px;
}
.qt-cols-btn:hover { color: var(--color-fg); }
.qt-cols-menu {
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    min-width: 200px;
}
.qt-cols-header {
    color: var(--color-fg-subtle);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 8px 12px 4px;
}

/* V4 — Rows / Cards segmented control */
.qt-view-toggle {
    display: inline-flex;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 2px;
    gap: 2px;
}

.qt-view-btn {
    background: transparent;
    border: none;
    color: var(--color-fg-muted);
    font: inherit;
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 5px 10px;
    border-radius: var(--radius-xs);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    transition: all var(--duration-fast) var(--ease-standard);
    white-space: nowrap;
}

.qt-view-btn:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--color-fg);
}

.qt-view-btn.active {
    background: var(--color-accent);
    color: #002b27;
    box-shadow: 0 0 8px var(--color-accent-glow);
    font-weight: 700;
}

</style>
