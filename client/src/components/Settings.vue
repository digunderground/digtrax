<template>
<div class="settings-route">
    <!-- Category sidebar (replaces top tabs) -->
    <aside class="settings-sidebar">
        <div class="settings-brand">// Settings</div>
        <button
            v-for="cat in categories"
            :key="cat.id"
            class="settings-cat"
            :class="{ active: tab === cat.id }"
            @click="tab = cat.id"
        >
            <q-icon :name="cat.icon" size="18px" class="settings-cat-icon"></q-icon>
            <span class="settings-cat-label">{{ cat.label }}</span>
        </button>

        <div class="settings-spacer"></div>

        <button class="settings-cat" @click="closeSettings">
            <q-icon name="mdi-arrow-left" size="18px" class="settings-cat-icon"></q-icon>
            <span class="settings-cat-label">Back</span>
        </button>
    </aside>

    <!-- Content pane -->
    <main class="settings-content">
        <div class="settings-content-inner">
            <div class="settings-page-title">{{ currentCategory.label }}</div>
            <div class="settings-page-desc" v-if="currentCategory.desc">{{ currentCategory.desc }}</div>

            <!-- Auto-detected pending changes banner. Appears on Quick Tag + Custom Tags
                 when the user has edited a frame mapping or a configured value since this
                 page mounted. One click opens the modal that lists every detected change. -->
            <div
                v-if='(tab == "quicktag" || tab == "quicktag-custom") && pendingChangeCount > 0'
                class='settings-migrate-banner q-mb-md row items-center q-pa-md'
            >
                <q-icon name='mdi-alert-circle-outline' class='q-mr-sm' size='20px' color='warning' />
                <div style='flex: 1;'>
                    <div class='text-body2 text-bold'>
                        {{ pendingChangeCount }} change{{ pendingChangeCount === 1 ? '' : 's' }} not yet propagated
                    </div>
                    <div class='text-caption text-grey-5'>
                        You changed a frame mapping or value. Apply the same change to every existing file in your library?
                    </div>
                </div>
                <q-btn
                    push dense color='primary' class='text-black q-ml-md'
                    icon='mdi-file-tree-outline'
                    label='Review & propagate…'
                    @click='openMigrateDialog'
                />
            </div>

            <!-- Quick Tag -->
            <div v-if='tab == "quicktag"'>
                <div class='select'>
                    <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-sm q-mb-sm'>Input</div>

                    <q-input filled v-model='$1t.settings.value.path'>
                        <template v-slot:append>
                            <q-btn round dense flat icon='mdi-open-in-app' class='text-grey-4' @click='browseQuickTag'></q-btn>
                        </template>
                    </q-input>

                    <div class='q-mt-sm q-pl-xs'>
                        <q-checkbox
                            v-model='$1t.settings.value.quickTag.recursive'
                            label='Include subfolders'
                            class='checkbox'
                            @input="$1t.loadQuickTag()"
                        ></q-checkbox>
                    </div>
                    <PlaylistDropZone
                        v-model='qtPlaylist'
                        @update:model-value='loadQTPlaylist'
                        class='input'
                        style='margin-bottom: 40px;'
                    ></PlaylistDropZone>
                </div>

                <q-separator class='settings-sep' inset color="darker"/>
                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg'>Energy</div>
                <div class='q-mb-sm q-mt-md text-grey-5 text-uppercase text-caption row justify-between'>
                    <span>Stars</span>
                    <span class='text-grey-5 text-uppercase text-caption'>Key binds</span>
                </div>
                <div class='row q-mb-lg'>
                    <div v-for='i in 5' :key='"energy" + i' class='col row'>
                        <div class='col-4 q-pt-xs text-center'>
                            <span>{{i}}</span>
                            <q-icon name='mdi-star' size='xs' class='q-pl-xs q-pb-md mt-6' color='yellow'></q-icon>
                        </div>
                        <div class='col-8 mt-6 text-left'>
                            <KeybindVue
                                class='energy-keybind text-center'
                                v-model='$1t.settings.value.quickTag.energyKeys[i-1]'
                            ></KeybindVue>
                        </div>
                    </div>
                </div>

                <div class='q-mb-sm row justify-between text-grey-5 text-uppercase text-caption'>
                    <span>Tag frame</span>
                    <span class='text-grey-5 text-uppercase text-caption'>Save to</span>
                </div>
                <q-select
                    v-model='$1t.settings.value.quickTag.energyTag.type'
                    dense
                    filled
                    label='Tag type'
                    :options='["rating", "symbol"]'
                    style='margin-bottom: 40px;'
                    popup-content-class='no-shadow'
                ></q-select>

                <div v-if='$1t.settings.value.quickTag.energyTag.type != "rating"' class='row'>
                    <div class='col-2 q-pr-md'>
                        <q-input v-model='$1t.settings.value.quickTag.energyTag.symbol' filled dense label='Symbol'></q-input>
                    </div>
                    <div class='col-10 q-mb-xs' :style='"margin-bottom: -4px"'>
                        <TagFields dense v-model='$1t.settings.value.quickTag.energyTag.tag'></TagFields>
                    </div>
                </div>

                <q-separator class='settings-sep' inset color="darker"/>
                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg'>Mood</div>
                <div class='text-grey-5 text-uppercase text-caption q-mt-md q-mb-sm row justify-between'>
                    <span>Tag frame</span>
                    <span class='text-grey-5 text-uppercase text-caption'>Save to</span>
                </div>
                <TagFields class='q-mb-sm' v-model='$1t.settings.value.quickTag.moodTag'></TagFields>

                <div class='q-mb-sm q-mt-md text-grey-5 text-uppercase text-caption row justify-between'>
                    <span>Values</span>
                    <span class='text-grey-5 text-uppercase text-caption'>Key binds</span>
                </div>
                <div class='q-mb-md'>
                    <draggable v-model='$1t.settings.value.quickTag.moods' :item-key='(e: any) => `mood`'>
                        <template #item='{ element: mood, index: i }'>
                            <div class='row justify-around'>
                                <q-input
                                    clearable
                                    @clear='$1t.settings.value.quickTag.moods.splice(i, 1)'
                                    v-model='$1t.settings.value.quickTag.moods[i].mood'
                                    filled
                                    dense
                                    class='col-5 q-pr-md q-mb-sm'
                                ></q-input>
                                <q-select
                                    v-model='$1t.settings.value.quickTag.moods[i].color'
                                    dense
                                    filled
                                    label='Color'
                                    :options='colors'
                                    :label-color='$1t.settings.value.quickTag.moods[i].color'
                                    :color='$1t.settings.value.quickTag.moods[i].color'
                                    class='col-5 q-pr-md'
                                    popup-content-class='no-shadow'
                                ></q-select>
                                <KeybindVue
                                    class='col-2 text-center'
                                    v-model='$1t.settings.value.quickTag.moods[i].keybind'
                                ></KeybindVue>
                            </div>
                        </template>
                    </draggable>

                    <div class='q-mt-sm q-mb-sm text-uppercase text-primary text-subtitle2'>Add new mood</div>
                    <div class='row'>
                        <q-input v-model='newMood.mood' filled dense class='col-5 q-pr-md q-mb-lg'></q-input>
                        <q-select v-model='newMood.color' :options='colors' filled dense class='col-5 q-pr-md' popup-content-class='no-shadow'></q-select>
                        <div class='col-1'>
                            <q-btn flat round icon='mdi-plus' @click='addMood' color='primary'></q-btn>
                        </div>
                    </div>

                    <div class='q-mt-sm q-mb-sm text-uppercase text-primary text-subtitle2'>Sort moods</div>
                    <div class='row'>
                        <q-btn class='q-mr-sm' flat round color='white' size='sm' icon='mdi-sort-alphabetical-ascending' @click='sortMoods(1)'></q-btn>
                        <q-btn class='q-mr-sm' flat round color='white' size='sm' icon='mdi-sort-alphabetical-descending' @click='sortMoods(-1)'></q-btn>
                    </div>
                </div>

                <q-separator class='settings-sep' inset color="darker"/>
                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg'>Genre</div>
                <div class='text-grey-5 text-uppercase text-caption q-mt-md q-mb-sm row justify-between'>
                    <span>Values</span>
                    <span class='text-grey-5 text-uppercase text-caption'>Key binds</span>
                </div>
                <div>
                    <draggable v-model='$1t.settings.value.quickTag.genres' :item-key='(e: any) => `genre`'>
                        <template #item='{ element, index: i }'>
                            <div>
                                <div class='row q-my-sm'>
                                    <q-input
                                        clearable
                                        filled
                                        dense
                                        class='col-10'
                                        v-model="$1t.settings.value.quickTag.genres[i].genre"
                                        @clear='$1t.settings.value.quickTag.genres.splice(i, 1)'
                                    ></q-input>
                                    <KeybindVue
                                        class='col-2 text-center'
                                        v-model='$1t.settings.value.quickTag.genres[i].keybind'
                                    ></KeybindVue>
                                </div>
                                <div class='row'>
                                    <div class='col-2 text-body q-mt-sm text-primary'>Subgenres: </div>
                                    <q-input
                                        clearable
                                        filled
                                        dense
                                        class='col-10'
                                        placeholder='Use , as separator'
                                        @update:model-value='(e) => onSubgenreInput(e as string, i)'
                                        :model-value='($1t.settings.value.quickTag.genres[i].subgenres||[]).join(",")'
                                    ></q-input>
                                </div>
                                <div class='q-mt-xl'></div>
                            </div>
                        </template>
                    </draggable>

                    <div class='q-mb-sm text-uppercase text-primary text-subtitle2'>Add new genre</div>
                    <div class='row'>
                        <q-input filled dense class='col-10 q-pr-md' v-model='newGenre'></q-input>
                        <div class='col-1'>
                            <q-btn flat round icon='mdi-plus' @click='addGenre' color='primary'></q-btn>
                        </div>
                    </div>

                    <div class='q-mt-md q-mb-sm'>
                        <q-checkbox
                            label='Custom subgenre tag'
                            :model-value="!!$1t.settings.value.quickTag.subgenreTag"
                            @update:model-value="(v) => enableCustomSubgenreTag(v)"
                        ></q-checkbox>
                        <TagFields v-if='$1t.settings.value.quickTag.subgenreTag' class='col-10 q-mt-sm q-mb-md' style='margin-bottom: 20px;' dense v-model='$1t.settings.value.quickTag.subgenreTag'></TagFields>
                    </div>

                    <div class='q-mt-sm q-mb-sm text-uppercase text-primary text-subtitle2'>Sort genres</div>
                    <div class='row'>
                        <q-btn class='q-mr-sm' flat round color='white' size='sm' icon='mdi-sort-alphabetical-ascending' @click='sortGenres(1)'></q-btn>
                        <q-btn class='q-mr-sm' flat round color='white' size='sm' icon='mdi-sort-alphabetical-descending' @click='sortGenres(-1)'></q-btn>
                    </div>
                </div>
            </div>

            <!-- Custom Tags -->
            <div v-if='tab == "quicktag-custom"'>
                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-sm q-mb-sm'>Separators</div>
                <div style='margin-bottom: 40px;'>
                    <Separators v-model='$1t.settings.value.quickTag.separators'></Separators>
                </div>

                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg q-mb-sm'>Tag options</div>
                <div style='margin-bottom: 20px;'>
                    <q-input
                        v-model='$1t.settings.value.quickTag.id3CommLang'
                        filled dense
                        label='ID3 COMM Language'
                        class='input'
                        :rules="[val => !val || val.length == 3]"
                    ></q-input>
                </div>
                <q-separator class='settings-sep' inset color="darker"/>

                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg q-mb-sm'>Custom note</div>
                <div class='text-grey-5 text-uppercase text-caption q-mt-md q-mb-sm row justify-between'>
                    <span>Tag frame</span>
                    <span class='text-grey-5 text-uppercase text-caption'>Key bind</span>
                </div>
                <div class='row'>
                    <TagFields class='col-10 q-mb-md' style='margin-bottom: 20px;' dense v-model='$1t.settings.value.quickTag.noteTag.tag'></TagFields>
                    <KeybindVue
                        class='col-2 text-center'
                        v-model='$1t.settings.value.quickTag.noteTag.keybind'
                    ></KeybindVue>
                </div>

                <q-separator class='settings-sep' inset color="darker"/>
                <div class='q-mb-lg' style='margin-bottom: 29px;'></div>
                <div v-for='(tag, i) in $1t.settings.value.quickTag.custom' :key='"tag"+i'>
                    <div class='row'>
                        <div class='text-subtitle1 text-bold q-mb-sm' style='margin-top: 4px;' v-if='!customQTEdit[i]'>{{tag.name}}</div>
                        <q-input dense filled v-if='customQTEdit[i]' v-model='$1t.settings.value.quickTag.custom[i].name'></q-input>
                        <div class='q-mx-md mt-2'>
                            <q-btn
                                size='sm'
                                flat
                                round
                                :icon='customQTEdit[i] ? "mdi-check" : "mdi-pencil"'
                                class='q-mr-sm'
                                color='primary'
                                @click='editCustomQT(i)'
                            ></q-btn>
                            <q-btn size='sm' class='q-mr-sm' flat round icon='mdi-chevron-up' color='primary' @click='reorderCustomQT(i, -1)' v-if='i > 0'></q-btn>
                            <q-btn size='sm' class='q-mr-sm' flat round icon='mdi-chevron-down' color='primary' @click='reorderCustomQT(i, 1)' v-if='i != $1t.settings.value.quickTag.custom.length - 1'></q-btn>
                            <q-btn size='sm' class='q-mr-sm' flat round icon='mdi-sort-alphabetical-ascending' color='primary' @click='sortCustomQT(i, 1)'></q-btn>
                            <q-btn size='sm' class='q-mr-sm' flat round icon='mdi-sort-alphabetical-descending' color='primary' @click='sortCustomQT(i, -1)'></q-btn>
                            <q-btn size='sm' flat round icon='mdi-delete' color='red' @click='deleteCustomQT(i)'></q-btn>
                        </div>
                    </div>

                    <TagFields class='q-pt-sm' v-model='$1t.settings.value.quickTag.custom[i].tag'></TagFields>

                    <draggable v-model='tag.values' :item-key='(e: any) => `qtc-${i}`'>
                        <template #item='{ element, index: j }'>
                            <div class='row'>
                                <q-btn class='col-1 q-mt-sm' round flat icon='mdi-close' @click='$1t.settings.value.quickTag.custom[i].values.splice(j, 1)'></q-btn>
                                <q-input
                                    class='col-9 q-px-sm q-pt-sm'
                                    dense
                                    filled
                                    v-model='$1t.settings.value.quickTag.custom[i].values[j].val'
                                ></q-input>
                                <KeybindVue
                                    class='col-2 text-center q-pt-sm'
                                    v-model='$1t.settings.value.quickTag.custom[i].values[j].keybind'
                                ></KeybindVue>
                            </div>
                        </template>
                    </draggable>

                    <q-btn
                        flat
                        color='primary'
                        class='q-mt-sm q-mb-sm'
                        style='margin-bottom: 22px;'
                        icon='mdi-plus'
                        @click='addNewQTValue(i)'
                    >Add new value</q-btn>
                </div>

                <div class='q-mb-lg'></div>
                <div class='row q-mt-md'>
                    <div class='q-mb-sm text-uppercase text-primary text-subtitle2 q-my-lg col-4'>Add new section</div>
                    <q-input v-model='newCustomQT' filled label='Name' class='q-mt-sm col-7 q-pr-md'></q-input>
                    <div class='q-mt-md col-1'>
                        <q-btn round flat icon='mdi-plus' size='md' color='primary' @click='addCustomQT'></q-btn>
                    </div>
                </div>
            </div>

            <!-- Preferences -->
            <div v-if='tab == "advanced"'>
                <div class='q-mb-xl'>
                    <q-btn dense push
                        color='primary'
                        class='rounded-borders q-px-md q-mt-xs text-weight-medium text-black'
                        @click='$1t.send("openSettingsFolder")'
                    >Open data folder</q-btn>
                </div>

                <q-separator class='settings-sep' inset color="darker"/>

                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg q-mb-xs'>Auto Tag</div>
                <q-checkbox v-model='$1t.settings.value.autoTaggerSinglePage' label="Show as single page" class='checkbox'></q-checkbox><br>
                <q-checkbox v-model='$1t.settings.value.showAutoTaggerProfiles' label="Show profiles" class='checkbox'></q-checkbox><br>

                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg q-mb-xs'>Quick Tag</div>
                <q-checkbox v-model='$1t.settings.value.quickTag.autosave' label='Autosave changes when switching to a different track' class='checkbox'></q-checkbox>
                <q-checkbox v-model='$1t.settings.value.continuePlayback' label='Continue playback when switching to a different track' class='checkbox'></q-checkbox><br>
                <q-checkbox v-model='$1t.settings.value.playOnSeek' label='Start/continue playback after seeking' class='checkbox'></q-checkbox><br>
                <q-checkbox v-model='$1t.settings.value.autoPlayNext' label='Go to next track when playback ends' class='checkbox'></q-checkbox><br>
                <q-checkbox v-model='$1t.settings.value.quickTag.id3v24' label='Use ID3v2.4 for MP3 and AIFF' class='checkbox'></q-checkbox><br>
                <q-checkbox v-model='$1t.settings.value.externalAudioPlayer' label='Use external audio player' class='checkbox'></q-checkbox><br>
                <q-checkbox v-model='$1t.settings.value.quickTag.thinTracks' label='Thin, dense tracks (rows view)' class='checkbox'></q-checkbox><br>

                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg q-mb-xs'>Edit Tags</div>
                <q-checkbox v-model='$1t.settings.value.tagEditorDouble' label="Show 'Your list'" class='checkbox'></q-checkbox><br>
                <q-checkbox v-model='$1t.settings.value.tagEditorAutosave' label='Autosave changes when switching to a different track' class='checkbox'></q-checkbox><br>
                <q-checkbox v-model='$1t.settings.value.tagEditorPlayer' label='Show player in tag editor' class='checkbox'></q-checkbox><br>

                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg q-mb-xs'>Display</div>
                <q-checkbox v-model='$1t.settings.value.helpButton' label='Show help button' class='checkbox'></q-checkbox>

                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg q-mb-xs'>Advanced</div>
                <q-checkbox v-model='$1t.settings.value.clientSidePlayer' label='Client side player (for server mode)' class='checkbox'></q-checkbox><br>
                <q-checkbox v-model='$1t.settings.value.nonNativeBrowser' label='Client side folder browser' class='checkbox'></q-checkbox>

                <div class='text-uppercase text-primary text-subtitle2 text-bold q-mt-lg q-mb-md'>Primary color</div>
                <q-color v-model='$1t.settings.value.primaryColor' @change='colorChange' flat></q-color>
                <q-btn @click='$1t.settings.value.primaryColor = "#00d2bf"; colorChange()' color='primary' flat class='q-mt-sm'>
                    Reset color to default
                </q-btn>
            </div>

            <!-- Backup & Restore -->
            <div v-if='tab == "backup"'>
                <div class='dt-backup-card'>
                    <div class='dt-backup-icon'><q-icon name='mdi-download' size='28px'></q-icon></div>
                    <div class='dt-backup-title'>// Export</div>
                    <div class='dt-backup-body'>
                        Save your current DigTrax settings — folder paths, mood &amp; genre &amp; energy bindings, custom tag definitions, and platform auth tokens — to a single JSON file.
                        Useful as a backup before major changes, or for moving between machines.
                    </div>
                    <q-btn color='primary' class='dt-backup-btn' @click='exportSettings'>
                        <q-icon name='mdi-download' size='16px' class='q-mr-sm'></q-icon>
                        Export current settings
                    </q-btn>
                </div>

                <div class='dt-backup-card'>
                    <div class='dt-backup-icon'><q-icon name='mdi-upload' size='28px'></q-icon></div>
                    <div class='dt-backup-title'>// Import</div>
                    <div class='dt-backup-body'>
                        Replace your current settings with a previously-exported JSON file.
                        Compatible with files from previous OneTagger installs — point this at the OneTagger <code>settings.json</code>.
                        DigTrax will reload after import.
                    </div>
                    <div class='dt-backup-warning'>
                        <q-icon name='mdi-alert-circle-outline' size='14px' class='q-mr-xs'></q-icon>
                        Importing will overwrite your current settings. Export a backup first if you want to preserve them.
                    </div>
                    <q-btn color='primary' outline class='dt-backup-btn' @click='importSettings'>
                        <q-icon name='mdi-upload' size='16px' class='q-mr-sm'></q-icon>
                        Import settings from file…
                    </q-btn>
                </div>
            </div>
        </div>
    </main>

    <!-- Migrate-tag dialog: lists every detected change since the page mounted.
         User picks a folder + recursive, deselects any change they don't want
         applied (default: all on), runs them sequentially. Streams progress
         via the global tagEditorBulkProgress events. -->
    <q-dialog v-model='migrateDialog' persistent>
        <q-card style='min-width: 600px; max-width: 760px;' class='q-pa-md'>
            <div class='text-h6 q-mb-sm'>Propagate changes to library</div>
            <div class='text-caption text-grey-5 q-mb-md'>
                Walks the chosen folder and applies every checked change. Frame
                renames migrate the tag key on each file; value renames rewrite
                the matching entry inside the configured frame. Files that
                don't match a change are left untouched.
            </div>

            <div v-if='migrateDraft.changes.length === 0' class='text-grey-5 q-py-md'>
                Nothing to propagate.
            </div>
            <div v-else class='q-mb-md' style='max-height: 320px; overflow-y: auto;'>
                <div
                    v-for='(c, i) in migrateDraft.changes'
                    :key='"chg" + i'
                    class='settings-migrate-row q-py-xs q-px-sm row items-start'
                >
                    <q-checkbox v-model='migrateDraft.changes[i].run' size='sm' class='q-mr-sm' />
                    <div style='flex: 1; min-width: 0;'>
                        <div class='text-body2'>
                            <q-badge :color='c.kind === "frame" ? "primary" : "accent"' class='q-mr-xs'>{{ c.kind }}</q-badge>
                            {{ c.label }}
                        </div>
                        <div v-if='c.kind === "frame"' class='text-caption text-grey-5 monospace'>
                            {{ c.from.id3 || '—' }} → {{ c.to.id3 || '—' }}
                            &nbsp;|&nbsp; {{ c.from.vorbis || '—' }} → {{ c.to.vorbis || '—' }}
                            &nbsp;|&nbsp; {{ c.from.mp4 || '—' }} → {{ c.to.mp4 || '—' }}
                        </div>
                        <div v-else class='text-caption text-grey-5'>
                            in <span class='monospace'>{{ c.frame.id3 || '?' }} / {{ c.frame.vorbis || '?' }} / {{ c.frame.mp4 || '?' }}</span>
                        </div>
                    </div>
                </div>
            </div>

            <q-input filled dense label='Folder' v-model='migrateDraft.folder' class='q-mb-sm'>
                <template v-slot:append>
                    <q-btn round dense flat icon='mdi-folder-open' @click='browseMigrateFolder' />
                </template>
            </q-input>
            <q-toggle v-model='migrateDraft.recursive' label='Include subfolders' class='q-mb-md' />

            <div v-if='migrateRunning' class='q-mb-md text-caption text-grey-5'>
                <q-spinner size='14px' class='q-mr-xs' />
                {{ migrateProgress.index }} / {{ migrateProgress.total }} files
                — modified {{ migrateProgress.modified }}, failed {{ migrateProgress.failed }}
                <span v-if='migrateProgress.lastPath' class='ellipsis' style='display: block;'>{{ migrateProgress.lastPath }}</span>
            </div>

            <div class='row justify-end q-gutter-sm'>
                <q-btn flat label='Cancel' @click='migrateDialog = false' :disable='migrateRunning' />
                <q-btn v-if='migrateRunning' flat color='red' label='Stop' @click='cancelMigration' />
                <q-btn
                    v-else
                    push color='primary' class='text-black'
                    label='Run'
                    :disable='migrateDraft.changes.length === 0 || !migrateDraft.changes.some((c) => c.run) || !migrateDraft.folder'
                    @click='runMigrations'
                />
            </div>
        </q-card>
    </q-dialog>
</div>
</template>

<script lang='ts' setup>
import draggable from 'vuedraggable';
import { Ref, computed, ref, onMounted, onBeforeUnmount, watch } from 'vue';
import { useRouter } from 'vue-router';
import { get1t } from '../scripts/digtrax';
import { FrameName, Keybind, Playlist } from '../scripts/utils';
import KeybindVue from './Keybind.vue';
import PlaylistDropZone from './PlaylistDropZone.vue';
import Separators from './Separators.vue';
import TagFields from './TagFields.vue';
import { setCssVar, useQuasar } from 'quasar';

const $1t = get1t();
const router = useRouter();
const $q = useQuasar();

// --- Tag migration: auto-detect changes since this Settings page mounted -----
// On mount we snapshot the user's quickTag config. Whenever the snapshot
// differs from the current state, we surface a banner offering to propagate
// each change to the user's library (frame renames + per-value renames).
// On a successful run we re-snapshot so the same change isn't offered twice.
type FrameKey = { id3: string; vorbis: string; mp4: string };
type FrameRename = { kind: 'frame'; label: string; from: FrameKey; to: FrameKey };
type ValueRename = { kind: 'value'; label: string; frame: FrameKey; oldValue: string; newValue: string };
type PendingChange = (FrameRename | ValueRename) & { run: boolean };

const settingsSnapshot = ref<any>(null);
const migrateDialog = ref(false);
const migrateRunning = ref(false);
const migrateProgress = ref<{ phase: 'idle'|'start'|'progress'|'done'; index: number; total: number; succeeded: number; failed: number; modified: number; cancelled: boolean; lastPath?: string }>({
    phase: 'idle', index: 0, total: 0, succeeded: 0, failed: 0, modified: 0, cancelled: false,
});
const migrateDraft = ref({ folder: '', recursive: false, changes: [] as PendingChange[] });

onMounted(() => {
    // Deep-clone so subsequent edits don't mutate our snapshot. JSON round-trip
    // strips the FrameName class — fine, we only need shape comparison.
    settingsSnapshot.value = JSON.parse(JSON.stringify(($1t.settings.value as any).quickTag ?? {}));
});

// Compare two FrameName-like objects, ignoring class identity.
function frameSame(a: any, b: any): boolean {
    if (!a && !b) return true;
    if (!a || !b) return false;
    return (a.id3 ?? '') === (b.id3 ?? '')
        && (a.vorbis ?? '') === (b.vorbis ?? '')
        && (a.mp4 ?? '') === (b.mp4 ?? '');
}
function frameToKey(f: any): FrameKey {
    return { id3: f?.id3 ?? '', vorbis: f?.vorbis ?? '', mp4: f?.mp4 ?? '' };
}

// Build the list of detectable changes from the snapshot vs current state.
// Frame renames are flagged when any of the 3 per-format keys changed; value
// renames use index-positional diff inside each custom's values array, which
// catches simple edits ("06 → 07 - Cool Down") cleanly. Reorders or
// length-mismatches are skipped — too ambiguous to auto-rename safely.
const pendingChanges = computed<PendingChange[]>(() => {
    const orig = settingsSnapshot.value;
    const curr = ($1t.settings.value as any).quickTag;
    if (!orig || !curr) return [];
    const out: PendingChange[] = [];

    const checkFrame = (label: string, oFrame: any, cFrame: any) => {
        if (!oFrame || !cFrame) return;
        if (!frameSame(oFrame, cFrame)) {
            out.push({ kind: 'frame', label, from: frameToKey(oFrame), to: frameToKey(cFrame), run: true });
        }
    };

    checkFrame('Mood frame', orig.moodTag, curr.moodTag);
    checkFrame('Energy frame', orig.energyTag?.tag, curr.energyTag?.tag);
    checkFrame('Note frame', orig.noteTag?.tag, curr.noteTag?.tag);
    checkFrame('Subgenre frame', orig.subgenreTag, curr.subgenreTag);

    // Mood values (positional diff against orig.moods).
    if (orig.moods && curr.moods && orig.moods.length === curr.moods.length) {
        for (let i = 0; i < orig.moods.length; i++) {
            if (orig.moods[i].mood !== curr.moods[i].mood) {
                out.push({
                    kind: 'value',
                    label: `Mood value: "${orig.moods[i].mood}" → "${curr.moods[i].mood}"`,
                    frame: frameToKey(curr.moodTag),
                    oldValue: orig.moods[i].mood,
                    newValue: curr.moods[i].mood,
                    run: true,
                });
            }
        }
    }

    // Custom — frame change + value renames per index.
    const oCustom = orig.custom ?? [];
    const cCustom = curr.custom ?? [];
    const len = Math.min(oCustom.length, cCustom.length);
    for (let i = 0; i < len; i++) {
        const o = oCustom[i];
        const c = cCustom[i];
        if (!o || !c) continue;
        checkFrame(`${c.name} frame`, o.tag, c.tag);
        const ov = o.values ?? [];
        const cv = c.values ?? [];
        if (ov.length === cv.length) {
            for (let j = 0; j < ov.length; j++) {
                if (ov[j]?.val !== cv[j]?.val) {
                    out.push({
                        kind: 'value',
                        label: `${c.name} value: "${ov[j]?.val}" → "${cv[j]?.val}"`,
                        frame: frameToKey(c.tag),
                        oldValue: ov[j]?.val ?? '',
                        newValue: cv[j]?.val ?? '',
                        run: true,
                    });
                }
            }
        }
    }
    return out;
});

const pendingChangeCount = computed(() => pendingChanges.value.length);

function openMigrateDialog() {
    migrateDraft.value = {
        folder: $1t.settings.value.path ?? '',
        recursive: false,
        changes: pendingChanges.value.map((c) => ({ ...c })),
    };
    migrateProgress.value = { phase: 'idle', index: 0, total: 0, succeeded: 0, failed: 0, modified: 0, cancelled: false };
    migrateDialog.value = true;
}

function browseMigrateFolder() {
    const prev = $1t.onTagEditorEvent;
    $1t.onTagEditorEvent = (e: any) => {
        if (e?.action === 'browse' && typeof e.path === 'string') {
            migrateDraft.value.folder = e.path;
            $1t.onTagEditorEvent = prev;
            return;
        }
        prev?.(e);
    };
    $1t.browse('te', migrateDraft.value.folder || $1t.settings.value.path || '');
}

function cancelMigration() {
    $1t.send('tagEditorCancel' as any, {} as any);
}

async function runMigrations() {
    const checked = migrateDraft.value.changes.filter((c) => c.run);
    if (checked.length === 0) {
        $q.notify({ message: 'No changes selected', timeout: 2000, position: 'top-right' });
        return;
    }
    if (!migrateDraft.value.folder) {
        $q.notify({ message: 'Pick a folder first', color: 'negative', timeout: 2500, position: 'top-right' });
        return;
    }

    migrateRunning.value = true;
    let totalModified = 0;
    let totalScanned = 0;
    let totalFailed = 0;
    const aggrErrors: string[] = [];

    // Install one-shot Tag Editor handler that processes streaming progress
    // for the active migration; we'll restore the original at the end.
    const prev = $1t.onTagEditorEvent;
    let resolveCurrent: ((info: { ok: boolean; error?: string; modified?: number; scanned?: number; failed?: number; cancelled?: boolean }) => void) | null = null;

    $1t.onTagEditorEvent = (e: any) => {
        if (!e) return;
        if (e.action === 'tagEditorBulkProgress') {
            if (e.phase === 'start') {
                migrateProgress.value = { phase: 'start', index: 0, total: e.total ?? 0, succeeded: 0, failed: 0, modified: 0, cancelled: false };
            } else if (e.phase === 'progress') {
                migrateProgress.value.phase = 'progress';
                migrateProgress.value.index = e.index ?? migrateProgress.value.index + 1;
                migrateProgress.value.total = e.total ?? migrateProgress.value.total;
                migrateProgress.value.lastPath = e.path;
                if (e.ok) migrateProgress.value.succeeded += 1;
                else migrateProgress.value.failed += 1;
                if (e.modified) migrateProgress.value.modified += 1;
            }
            return;
        }
        if (e.action === 'tagEditorMigrate' || e.action === 'tagEditorMigrateValue') {
            const succeeded = e.report?.succeeded ?? 0;
            const failed = e.report?.failed?.length ?? 0;
            const scanned = e.scanned ?? succeeded + failed;
            const modified = e.modified ?? migrateProgress.value.modified;
            const cancelled = !!e.cancelled;
            migrateProgress.value.phase = 'done';
            migrateProgress.value.cancelled = cancelled;
            resolveCurrent?.({ ok: true, modified, scanned, failed, cancelled });
            resolveCurrent = null;
            return;
        }
        prev?.(e);
    };

    for (const change of checked) {
        // Skip if user cancelled previously.
        if (migrateProgress.value.cancelled) break;

        await new Promise<void>((resolve) => {
            resolveCurrent = (info) => {
                totalScanned += info.scanned ?? 0;
                totalModified += info.modified ?? 0;
                totalFailed += info.failed ?? 0;
                if (info.cancelled) migrateProgress.value.cancelled = true;
                resolve();
            };
            if (change.kind === 'frame') {
                $1t.send('migrateTagFolder' as any, {
                    folder: migrateDraft.value.folder,
                    recursive: migrateDraft.value.recursive,
                    from: change.from,
                    to: change.to,
                } as any);
            } else {
                $1t.send('migrateValueFolder' as any, {
                    folder: migrateDraft.value.folder,
                    recursive: migrateDraft.value.recursive,
                    frame: change.frame,
                    oldValue: change.oldValue,
                    newValue: change.newValue,
                } as any);
            }
        });
    }

    $1t.onTagEditorEvent = prev;
    migrateRunning.value = false;

    if (totalFailed === 0 && !migrateProgress.value.cancelled) {
        // Re-snapshot so the same changes don't re-appear next time.
        settingsSnapshot.value = JSON.parse(JSON.stringify(($1t.settings.value as any).quickTag ?? {}));
        migrateDialog.value = false;
    }

    $q.notify({
        message: migrateProgress.value.cancelled
            ? `Cancelled. Modified ${totalModified} files`
            : `Migration done — modified ${totalModified} files across ${totalScanned} scanned${totalFailed ? `, ${totalFailed} errors` : ''}`,
        color: totalFailed ? 'negative' : (migrateProgress.value.cancelled ? 'warning' : 'positive'),
        timeout: 6000,
        position: 'top-right',
    });
}

const categories = [
    { id: 'quicktag',        label: 'Quick Tag',       icon: 'mdi-flash',           desc: 'Path, energy, mood, and genre tagging configuration.' },
    { id: 'quicktag-custom', label: 'Custom Tags',     icon: 'mdi-tag-edit',        desc: 'Custom tag definitions, separators, and the note frame.' },
    { id: 'advanced',        label: 'Preferences',     icon: 'mdi-tune',            desc: 'Behavior toggles for Auto Tag, Quick Tag, Tag Editor, display, and advanced options.' },
    { id: 'backup',          label: 'Backup & Restore', icon: 'mdi-content-save-cog', desc: 'Export your settings to a file or import from a backup. Useful for moving between machines or restoring from a previous version.' },
];

// --- Backup / Restore handlers ---

function exportSettings() {
    try {
        const blob = new Blob([JSON.stringify($1t.settings.value, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        const stamp = new Date().toISOString().split('T')[0];
        a.download = `digtrax-settings-${stamp}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    } catch (e) {
        console.error('Export failed:', e);
    }
}

function importSettings() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json,application/json';
    input.onchange = async (e: Event) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (!file) return;
        try {
            const text = await file.text();
            const data = JSON.parse(text);
            if (typeof data !== 'object' || data === null) {
                throw new Error('not an object');
            }
            // Apply + persist + reload so reactive views pick up new state
            Object.assign($1t.settings.value, data);
            $1t.saveSettings(false);
            setTimeout(() => window.location.reload(), 400);
        } catch (err) {
            console.error('Import failed:', err);
            alert('Could not import settings — file is not a valid DigTrax settings JSON.');
        }
    };
    input.click();
}

const colors = [
    'amber',
    'blue', 'blue-4',
    'blue-grey', 'blue-grey-9',
    'brown',
    'cyan', 'cyan-10',
    'deep-orange', 'deep-orange-10',
    'deep-purple', 'deep-purple-4',
    'green', 'green-10',
    'grey', 'grey-8',
    'indigo', 'indigo-5',
    'light-blue', 'light-blue-10',
    'light-green',
    'lime', 'lime-9',
    'orange',
    'pink', 'pink-4',
    'purple', 'purple-4',
    'red', 'red-10',
    'teal', 'teal-10',
    'yellow',
];

const tab = ref('quicktag');
const newMood: Ref<{ mood?: string, color: string, keybind?: Keybind }> = ref({ mood: undefined, color: 'red', keybind: undefined });
const newGenre = ref<string | undefined>();
const newCustomQT = ref('');
const customQTEdit = ref<boolean[]>([]);
const qtPlaylist = ref({});

const currentCategory = computed(() => categories.find(c => c.id === tab.value) ?? categories[0]);

// Persist defensively. Wrapped so a throw in any one step doesn't break navigation.
function persist() {
    try { $1t.saveSettings(); } catch (e) { console.warn('saveSettings failed:', e); }
}

// Save on window unload (Cmd+Q while on Settings).
const beforeUnload = () => persist();
window.addEventListener('beforeunload', beforeUnload);
onBeforeUnmount(() => {
    // Component is unmounting (route changed) — save on the way out.
    persist();
    window.removeEventListener('beforeunload', beforeUnload);
});

function closeSettings() {
    persist();
    router.push('/quicktag');
}

// Esc key as a second escape hatch — but skip when typing in inputs.
function onKeyDown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    const target = e.target as HTMLElement | null;
    if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) return;
    closeSettings();
}

onMounted(() => window.addEventListener('keydown', onKeyDown));
onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDown));

function colorChange() {
    setCssVar('primary', $1t.settings.value.primaryColor);
}

function addMood() {
    if (newMood.value.mood) {
        if ($1t.settings.value.quickTag.moods.find(m => newMood.value.mood!.toLowerCase() == m.mood.toLowerCase())) return;
        $1t.settings.value.quickTag.moods.push(JSON.parse(JSON.stringify(newMood.value)));
        newMood.value.mood = undefined;
    }
}

function sortMoods(dir: 1 | -1 = 1) {
    $1t.settings.value.quickTag.moods.sort((a, b) => {
        if (a.mood.toLowerCase() > b.mood.toLowerCase()) return 1 * dir;
        if (a.mood.toLowerCase() < b.mood.toLowerCase()) return -1 * dir;
        return 0;
    });
}

function addGenre() {
    if (!newGenre.value || newGenre.value.trim() == "") return;
    if ($1t.settings.value.quickTag.genres.find((g) => g.genre.toLowerCase() == newGenre.value!.toLowerCase())) return;
    $1t.settings.value.quickTag.genres.push({ genre: newGenre.value, keybind: undefined, subgenres: [] });
    newGenre.value = undefined;
}

function sortGenres(dir: 1 | -1 = 1) {
    $1t.settings.value.quickTag.genres.sort((a, b) => {
        if (a.genre.toLowerCase() > b.genre.toLowerCase()) return 1 * dir;
        if (a.genre.toLowerCase() < b.genre.toLowerCase()) return -1 * dir;
        return 0;
    });
}

function onSubgenreInput(e: string | null, i: number) {
    if (!e) {
        $1t.settings.value.quickTag.genres[i].subgenres = [];
        return;
    }
    $1t.settings.value.quickTag.genres[i].subgenres = e.split(",");
}

function enableCustomSubgenreTag(enable: boolean) {
    if (enable) {
        $1t.settings.value.quickTag.subgenreTag = new FrameName('TCON', 'GENRE', '©gen');
    } else {
        $1t.settings.value.quickTag.subgenreTag = undefined;
    }
}

function browseQuickTag() {
    $1t.browse('qt', $1t.settings.value.path);
}

function addCustomQT() {
    $1t.settings.value.quickTag.custom.push({
        name: newCustomQT.value,
        tag: FrameName.same('CUSTOM'),
        values: []
    });
    newCustomQT.value = '';
}

function deleteCustomQT(i: number) {
    $1t.settings.value.quickTag.custom.splice(i, 1);
}

function editCustomQT(i: number) {
    customQTEdit.value[i] = !customQTEdit.value[i];
}

function sortCustomQT(i: number, dir: 1 | -1 = 1) {
    $1t.settings.value.quickTag.custom[i].values.sort((a, b) => {
        if (a.val.toLowerCase() > b.val.toLowerCase()) return 1 * dir;
        if (a.val.toLowerCase() < b.val.toLowerCase()) return -1 * dir;
        return 0;
    });
}

function addNewQTValue(i: number) {
    $1t.settings.value.quickTag.custom[i].values.push({ val: "New", keybind: undefined });
}

function reorderCustomQT(now: number, offset: number) {
    let item = $1t.settings.value.quickTag.custom[now + offset];
    $1t.settings.value.quickTag.custom[now + offset] = $1t.settings.value.quickTag.custom[now];
    $1t.settings.value.quickTag.custom[now] = item;
}

function loadQTPlaylist(playlist?: Playlist) {
    if (!playlist || !playlist.data) {
        $1t.loadQuickTag();
        return;
    }
    $1t.loadQuickTag(playlist!);
}
</script>

<style lang='scss'>
.settings-route {
    display: flex;
    height: 100vh;
    width: 100%;
    background: var(--color-bg);
    color: var(--color-fg);
}

.settings-sidebar {
    width: 240px;
    flex-shrink: 0;
    background: var(--color-bg-elevated);
    border-right: 1px solid var(--color-border);
    padding: var(--space-3) var(--space-2);
    display: flex;
    flex-direction: column;
    gap: 1px;
    overflow-y: auto;
}

.settings-brand {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-fg-subtle);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: var(--space-2) var(--space-3) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    margin-bottom: var(--space-3);
}

.settings-cat {
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
    transition: all var(--duration-fast) var(--ease-standard);
    position: relative;
    margin: 1px 0;
    width: 100%;
}

.settings-cat:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--color-fg);
}

.settings-cat.active {
    background: linear-gradient(90deg, rgba(0, 210, 191, 0.15), rgba(0, 210, 191, 0.04));
    color: var(--color-fg);
}

.settings-cat.active::before {
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

.settings-cat-icon { flex-shrink: 0; }
.settings-cat-label {
    flex: 1;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 12px;
    font-weight: 500;
}

.settings-spacer { flex: 1; }

.settings-content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    background: var(--color-bg);
}

.settings-content-inner {
    max-width: 720px;
    padding: var(--space-7) var(--space-7) var(--space-9);
}

.settings-page-title {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 28px;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    margin-bottom: var(--space-1);
}

.settings-page-title::before {
    content: '// ';
    color: var(--color-accent);
}

.settings-page-desc {
    color: var(--color-fg-muted);
    font-size: 13px;
    margin-bottom: var(--space-6);
}

.settings-sep {
    margin-top: var(--space-5);
    margin-bottom: var(--space-5);
}

.energy-keybind {
    margin-top: -2px;
}

.mt-2 { margin-top: 2px; }
.mt-6 { margin-top: 6px; }

.custom-sep {
    min-width: 601px;
    margin-inline-start: -10%;
}

/* Backup & Restore panel */
.dt-backup-card {
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 24px;
    margin-bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.dt-backup-icon {
    color: var(--color-accent);
    width: 48px;
    height: 48px;
    border-radius: var(--radius-sm);
    background: rgba(0, 210, 191, 0.08);
    display: flex;
    align-items: center;
    justify-content: center;
}

.dt-backup-title {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 15px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-fg);
}

.dt-backup-body {
    color: var(--color-fg-muted);
    font-size: 13px;
    line-height: 1.55;
}

.dt-backup-body code {
    font-family: var(--font-mono);
    background: rgba(255, 255, 255, 0.04);
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    color: var(--color-accent);
    font-size: 11px;
}

.dt-backup-warning {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 12px;
    background: rgba(255, 182, 39, 0.08);
    border: 1px solid rgba(255, 182, 39, 0.25);
    border-radius: var(--radius-sm);
    color: var(--color-warning);
    font-size: 12px;
}

.dt-backup-btn {
    align-self: flex-start;
    margin-top: 4px;
}

.settings-migrate-banner {
    background: linear-gradient(90deg, rgba(255, 193, 7, 0.08), rgba(0, 210, 191, 0.06));
    border: 1px solid rgba(255, 193, 7, 0.25);
    border-radius: var(--radius-md, 8px);
}
.settings-migrate-row {
    border-radius: var(--radius-xs, 4px);
}
.settings-migrate-row:hover {
    background-color: rgba(255, 255, 255, 0.03);
}
</style>
