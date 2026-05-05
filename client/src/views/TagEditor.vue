<template>
<div class='full-height'>

    <div class='row full-height'>
        <!-- File browser -->
        <div 
            @contextmenu.prevent="" 
            class='q-px-md q-pt-md bg-darker' 
            :class='{"col-4": !$1t.settings.value.tagEditorDouble, "col-3": $1t.settings.value.tagEditorDouble}'
            style='max-height: 100%; overflow-y: scroll;'
        >
            <div class='text-weight-bold text-subtitle2 clickable path-display' @click='browse'>
                <div class='row inline'>
                    <span style="direction:ltr;" class='text-primary monospace'>{{path}}</span>
                </div>
            </div>
            <div class='q-mt-sm'>

                <!-- Filter (filename) -->
                <q-input dense filled label='Filter' class='q-mb-sm' @update:model-value='(v: any) => applyFilter(v as string)' v-model='filter'></q-input>

                <!-- Selection toolbar -->
                <div class='row items-center q-mb-sm'>
                    <q-btn dense flat size='sm' class='q-mr-xs' @click='toggleSelectAll' icon='mdi-select-all' label='All'></q-btn>
                    <q-btn dense flat size='sm' class='q-mr-xs' @click='clearSelection' icon='mdi-close' :disable='selectedPaths.length === 0' label='Clear'></q-btn>
                    <q-btn dense flat size='sm' class='q-mr-xs' @click='filterDialog = true' icon='mdi-filter-variant' label='Filter by tag'></q-btn>
                    <q-btn dense flat size='sm' @click='renameDialog = true' icon='mdi-rename' label='Rename tag' :disable='selectedPaths.length === 0'></q-btn>
                </div>
                <div v-if='selectedPaths.length > 0' class='text-caption text-primary q-mb-xs'>
                    {{ selectedPaths.length }} selected
                </div>

                <!-- Parent -->
                <div class='q-mb-sm clickable te-file' @click='loadFiles("..")'>
                    <q-icon size='xs' class='q-mb-xs text-grey-4' name='mdi-folder-upload'></q-icon>
                    <span class='q-ml-sm text-caption text-grey-4'>Parent folder</span>
                </div>

                <draggable
                    id='fileList'
                    :move='onFileMove'
                    group='files'
                    :list='files'
                    item-key='filename'
                    @change='onFileDrag'>
                    <template #item='{ element: file }'>
                        <div
                            class='clickable te-file row items-center no-wrap'
                            @click='(file.dir || file.playlist) ? loadFiles(file.filename) : loadFile(file.path, $event)'
                            :class='{"te-file-selected": isSelected(file.path), "text-primary": isSelected(file.path), "text-grey-4": !isSelected(file.path)}'
                        >
                            <q-checkbox
                                v-if='!file.dir && !file.playlist'
                                dense
                                size='xs'
                                :model-value='isSelected(file.path)'
                                @update:model-value='loadFile(file.path, { metaKey: true } as any)'
                                @click.stop
                                class='q-mr-xs'
                            />
                            <q-icon size='xs' class='q-mb-xs text-grey-4' v-if='!file.dir && !file.playlist' name='mdi-music'></q-icon>
                            <q-icon size='xs' class='q-mb-xs text-grey-4' v-if='file.dir' name='mdi-folder'></q-icon>
                            <q-icon size='xs' class='q-mb-xs text-grey-4' v-if='file.playlist' name='mdi-playlist-music'></q-icon>
                            <span class='q-ml-sm text-caption ellipsis' style='flex: 1; min-width: 0;'>{{file.filename}}</span>
                        </div>
                    </template>


                </draggable>
            </div>
        </div>

        <!-- Custom list -->
        <div 
            @contextmenu.prevent="" 
            class='col-3 bg-darker q-px-md q-pt-sm' 
            v-if='$1t.settings.value.tagEditorDouble'
            style='max-height: 100%; overflow-y: scroll;'
        >
            <div class='bg-darker separator'></div>
            <div class='row justify-between'>
                <div class='text-weight-bold text-subtitle2 text-primary q-pb-sm'>Your list</div>
                <div>
                    <q-btn round dense size='xs' flat style='margin-right: 2px;' @click='clearCustom'>
                        <q-icon name='mdi-close' color='red'></q-icon>
                    </q-btn>
                </div>
            </div>
            
            <draggable 
                group='files' 
                :move='onFileMove' 
                :list='customList' 
                @change='onFileDrag' 
                style='height: calc(100% - 32px)'
                :item-key="(e: any) => `//CUSTOM${e}`"
            >
                <template #item='{ element: f }'>
                    <div class='row'>
                        <div 
                            @click='loadFile(f)' 
                            class='te-file clickable q-my-xs q-mr-sm' 
                            style='width: calc(100% - 32px)' 
                            :class='{"text-primary": isSelected(f), "text-grey-4": !isSelected(f)}'
                        >
                            <span>{{filename(f)}}</span>
                        </div>
                        <div>
                            <q-btn size='xs' class='q-mt-xs' flat round style='float: right;' @click='removeCustom(f)'>
                                <q-icon name='mdi-close' color='red'></q-icon>
                            </q-btn>
                        </div>
                    </div>
                </template>
            </draggable>
        </div>

        <!-- Tags -->
        <div 
            :class='{"col-8": !$1t.settings.value.tagEditorDouble, "col-6": $1t.settings.value.tagEditorDouble}'
            style='max-height: 100%; overflow-y: scroll;'>
            <div v-if='!file' class='justify-center items-center content-center row full-height'>
                
                <div class='col-12 text-subtitle2 text-bold text-primary text-center q-my-sm'>NO FILE SELECTED</div><br>
                <span class='text-center text-subtitle2 text-grey-6'>Tip: <span class='keybind-icon q-px-sm text-caption text-bold'>CLICK</span> the path to open a folder and select an audio file</span>
            </div>

            <div v-if='file' class='q-px-md'>
                <div class='text-center q-py-md text-subtitle2 text-grey-5 monospace'>
                    <span v-if='!isMultiSelect'>{{file.filename}}</span>
                    <span v-else>{{ selectedPaths.length }} files selected — bulk edit</span>
                </div>
                <div class='q-mt-md'>
                    <div v-for='(tag, i) in allTagKeys' :key='i' class='row q-my-sm items-center'>
                        <div class='col-3 text-body2 text-uppercase text-primary text-weight-medium q-mt-sm q-pr-xs' style='text-overflow: ellipsis; overflow: hidden;'>
                            
                            <span v-if='customLabel(tag)'><span class='text-uppercase text-weight-medium' style='color: var(--color-accent, #00D2BF);'>{{customLabel(tag)}} </span><span class='text-grey-4 monospace text-caption'>{{tag}}</span></span>
                            <span v-else-if='ABSTRACTIONS[tag]'><span class='text-uppercase text-primary text-weight-medium'>{{ABSTRACTIONS[tag]}} </span><span class="text-grey-4 monospace text-caption"> {{tag}}</span></span>
                            <span v-else>{{tag}}</span>
                            <q-badge
                                v-if='isMultiSelect && tagSummary(tag).distinct > 1'
                                color='warning' text-color='black' class='q-ml-xs'
                            >{{ tagSummary(tag).distinct }} values</q-badge>
                            <q-badge
                                v-else-if='isMultiSelect && tagSummary(tag).present < selectedPaths.length'
                                color='grey-7' class='q-ml-xs'
                            >{{ tagSummary(tag).present }}/{{ selectedPaths.length }}</q-badge>
                        </div>
                        
                        
                        <!-- Chip-mode: when the tag has configured suggestions or a small
                             multi-value set, edit chips directly instead of free text. -->
                        <div v-if='tagChips(tag).show' class='col-8 row items-center q-gutter-xs te-chips-row'>
                            <q-chip
                                v-for='val in tagChips(tag).current'
                                :key='"sel-" + tag + "-" + val'
                                :color='customDefFor(tag) ? "accent" : "primary"'
                                text-color='black'
                                size='sm'
                                removable
                                @remove='removeChipValue(tag, val)'
                            >{{ val }}</q-chip>
                            <q-chip
                                v-for='val in tagChips(tag).suggestions'
                                :key='"sug-" + tag + "-" + val'
                                outline
                                clickable
                                size='sm'
                                @click='addChipValue(tag, val)'
                                class='te-ghost-chip'
                            >+ {{ val }}</q-chip>
                            <q-input
                                :model-value="''"
                                dense
                                outlined
                                placeholder='Add custom…'
                                style='width: 140px;'
                                @keyup.enter='(e: any) => { addChipValue(tag, e.target.value); e.target.value = ""; }'
                            />
                        </div>
                        <q-input
                            v-else
                            :model-value='bufferFor(tag)'
                            @update:model-value='(v: any) => setBuffer(tag, String(v))'
                            filled
                            dense
                            class='col-8'
                            :placeholder='isMultiSelect && tagSummary(tag).distinct > 1 ? "(multiple — typing replaces in all)" : ""'
                        ></q-input>

                        <div class='col-1 q-pl-md q-pt-xs'>
                            <q-btn round dense flat @click='removeTag(tag)'>
                                <q-icon name='mdi-delete' class='text-red'></q-icon>
                            </q-btn>
                        </div>
                    </div>
                </div>
                <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 40px;"' inset color="dark"/>

                <!-- Add new tag (single-file or bulk both supported) -->
                <div class='row q-mt-lg' style='margin-top: 40px;'>
                    <div class='col-3 q-pt-sm text-weight-medium text-grey-4 text-body2'>Add new text tag</div>
                    <TagField tageditor class='col-8' dense :format='tagFormat!' @update:model-value='newTag = $event'></TagField>
                    <div class='col-1 q-pl-md q-pt-xs'>
                        <q-btn round dense flat @click='addNewTag'>
                            <q-icon name='mdi-plus' class='text-primary'></q-icon>
                        </q-btn>
                    </div>
                </div>
                <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 20px; margin-bottom: 25px;"' inset color="dark"/>

                <!-- Album art (single-file only — bulk art editing is a different feature) -->
                <div v-if='!isMultiSelect' class='text-uppercase text-primary text-weight-medium'>
                    Album art
                    <q-btn round flat class='q-mb-xs q-ml-sm' @click='addAlbumArtDialog = true'>
                        <q-icon name='mdi-plus' color='primary'></q-icon>
                    </q-btn>
                </div>
                <div v-if='!isMultiSelect' class='text-grey-4 albumart-container text-center'>
                    <div v-for='(image, i) in file.images' :key='"art"+i' class='q-mr-md'>
                        <!-- <q-img :src='image.data' class='albumart clickable' @click='albumArt = image.data; showAlbumArt = true'></q-img>
                        <div class='q-pt-sm q-mb-md'>
                            <div v-if='file.format != "mp4"' class='text-caption'>{{image.kind}}</div>
                            <div v-if='file.format != "mp4"' class='text-caption'>{{image.description}}</div>
                            <div class='text-subtitle3 text-grey-6 monospace'>{{image.mime}} {{image.width}}x{{image.height}}</div>
                            <q-btn dense push color='red' class='rounded-borders q-px-md q-mt-sm text-weight-medium' @click='removeArt(i)'>Remove</q-btn>
                        </div> -->
                        <TagEditorAlbumArt 
                            :image='image' 
                            @click='albumArt = image.data; showAlbumArt = true' 
                            @remove='removeArt(i)'
                            @replace='addAlbumArt'
                        ></TagEditorAlbumArt>
                    </div>
                </div>

                <!-- ID3 specific tags — single-file only -->
                <div v-if='file.id3 && !isMultiSelect'>
                    <!-- Comments -->
                    <div class='text-uppercase text-primary text-weight-medium'>
                        Comments <span class="text-grey-4 monospace text-caption q-pl-xs">COMM</span>
                        <q-btn round flat class='q-mb-xs q-ml-sm' @click='addID3Comment'>
                            <q-icon name='mdi-plus' color='primary'></q-icon>
                        </q-btn>
                    </div>
                    <div>
                        <div v-for='(comment, i) in file.id3.comments' :key='"comm"+i' class='row q-py-sm'>
                            <q-input
                                filled
                                dense
                                label='Language'
                                class='col-2'
                                v-model='file.id3.comments[i].lang'
                                maxlength='3'
                                @change='id3CommentsChange'
                            ></q-input>
                            <q-input
                                filled
                                dense
                                label='Description'
                                class='col-4 q-pl-sm'
                                v-model='file.id3.comments[i].description'
                                @change='id3CommentsChange'
                            ></q-input>
                            <q-input
                                filled
                                dense
                                label='Text'
                                class='col-5 q-pl-sm'
                                v-model='file.id3.comments[i].text'
                                @change='id3CommentsChange'
                            ></q-input>
                            <div class='col-1 q-pl-md q-pt-xs'>
                                <q-btn round dense flat @click='removeID3Comment(i)'>
                                    <q-icon name='mdi-delete' class='text-red'></q-icon>
                                </q-btn>
                            </div>
                        </div>
                    </div>

                    <!-- Unsynchronized lyrics -->
                    <div class='text-uppercase text-primary text-weight-medium'>
                        Unsynchronized lyrics <span class="text-grey-4 monospace text-caption q-pl-xs">USLT</span>
                        <q-btn round flat class='q-mb-xs q-ml-sm' @click='addID3USLT'>
                            <q-icon name='mdi-plus' color='primary'></q-icon>
                        </q-btn>
                    </div>
                    <div>
                        <div v-for='(lyric, i) in file.id3.unsync_lyrics' :key='"uslt"+i' class='q-py-sm'>
                            <div class='row'>
                                <q-input
                                    filled
                                    dense
                                    label='Language'
                                    class='col-3'
                                    v-model='file.id3.unsync_lyrics[i].lang'
                                    maxlength='3'
                                    @change='id3USLTChange'
                                ></q-input>
                                <q-input
                                    filled
                                    dense
                                    label='Description'
                                    class='col-8 q-pl-md'
                                    v-model='file.id3.unsync_lyrics[i].description'
                                    @change='id3USLTChange'
                                ></q-input>
                                <div class='col-1 q-pl-md q-pt-xs'>
                                    <q-btn round dense flat @click='removeID3USLT(i)'>
                                        <q-icon name='mdi-delete' class='text-red'></q-icon>
                                    </q-btn>
                                </div>
                            </div>
                            <q-input
                                filled
                                dense
                                label='Text'
                                v-model='file.id3.unsync_lyrics[i].text'
                                type='textarea'
                                class='q-pt-sm q-pb-sm'
                                @change='id3USLTChange'
                            ></q-input>
                        </div>
                    </div>

                    <!-- Popularimeter -->
                    <div>
                        <div class='text-uppercase text-primary text-weight-medium'>
                            Popularimeter <span class="text-grey-4 monospace text-caption q-pl-xs">POPM</span>
                            <q-btn v-if='!file.id3.popularimeter' round flat class='q-mb-xs q-ml-sm' @click='addPOPM'>
                                <q-icon name='mdi-plus' color='primary'></q-icon>
                            </q-btn>
                        </div>
                        <div v-if='file.id3.popularimeter' class='row q-py-sm'>
                            <q-input
                                filled
                                dense
                                label='Email'
                                class='col-4'
                                v-model='file.id3.popularimeter.email'
                                @change='id3POPMChange'
                            ></q-input>
                            <q-input
                                filled
                                dense
                                type='number'
                                label='Play count'
                                class='col-3 q-pl-sm'
                                v-model='file.id3.popularimeter.counter'
                                maxlength='9'
                                @change='id3POPMChange'
                            ></q-input>
                            <div class='col-4 q-pl-md'>
                                <q-slider
                                    :min='0'
                                    :max='255'
                                    label
                                    label-text-color='black'
                                    :label-value='POPMLabel'
                                    v-model='file.id3.popularimeter.rating'
                                    @change='id3POPMChange'
                                ></q-slider>
                            </div>
                            <div class='col-1 q-pl-md q-pt-xs'>
                                <q-btn round dense flat @click='removePOPM'>
                                    <q-icon name='mdi-delete' class='text-red'></q-icon>
                                </q-btn>
                            </div>
                        </div>
                    </div>
                    <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 32px; margin-bottom: 25px;"' inset color="dark"/>
                    
                    <!-- ID3v2.4 -->
                    <div class='q-mt-lg text-center'>
                        <div class='text-subtitle2 text-bold text-primary custom-margin'>
                            OPTIONS
                        </div>
                    </div>
                    <div class='column flex-center'>
                        <q-toggle label='Use ID3v2.4' left-label style='width: 160px;' class='justify-between' v-model='id3v24'></q-toggle>
                    </div>
            </div>

            <!-- Save, Manual tag -->
            <q-page-sticky position='bottom-right' :offset='[36, 18]'>
                <div class='row'>
                    <q-btn v-if='!isMultiSelect' dense
                        push
                        @click='manualTagPath = file.path'
                        color="primary"
                        class='rounded-borders q-px-md q-mt-xs text-black text-weight-medium q-mr-md'
                        label="Manual Tag"
                    ></q-btn>

                    <q-btn dense
                        push
                        @click='save'
                        :disable='selectedPaths.length === 0'
                        color="primary"
                        class='rounded-borders q-px-md q-mt-xs text-black text-weight-medium'
                        :label='isMultiSelect ? `Save ${selectedPaths.length} files` : "Save"'
                    ></q-btn>
                </div>
            </q-page-sticky>

            </div>
        </div>
    </div>

    <!-- Album art dialog -->
    <q-dialog v-model='showAlbumArt' @hide='albumArt = null'>
        <q-img :src='albumArt' style='max-width: 50%;'></q-img>
    </q-dialog>

    <!-- Add album art dialog -->
    <q-dialog v-model='addAlbumArtDialog'>
        <AddAlbumArt :types='albumArtTypes' @close='addAlbumArtDialog = false' @save='addAlbumArt'></AddAlbumArt>
    </q-dialog>

    <!-- Manual Tag -->
    <ManualTag :path='manualTagPath' @exit='loadFile(manualTagPath!); manualTagPath = undefined;'></ManualTag>

    <!-- Filter by tag dialog -->
    <q-dialog v-model='filterDialog'>
        <q-card style='min-width: 480px;' class='q-pa-md'>
            <div class='text-h6 q-mb-sm'>Filter by tag</div>
            <div class='text-caption text-grey-5 q-mb-md'>
                Walks {{ filterDraft.recursive ? 'this folder and all subfolders' : 'this folder' }} and selects every file whose tags match.
            </div>
            <q-select
                filled dense label='Preset (auto-expands to ID3 / Vorbis / MP4 frame names)'
                v-model='filterDraft.preset'
                :options="[{label: '— manual tag below —', value: -1}].concat(customTagDefs.map((d, i) => ({label: d.label + ' (' + (d.frame.id3 || '?') + ' / ' + (d.frame.vorbis || '?') + ' / ' + (d.frame.mp4 || '?') + ')', value: i})))"
                emit-value map-options
                class='q-mb-sm'
            />
            <q-input
                filled dense
                :label="filterDraft.preset >= 0 ? 'Tag — overridden by preset above' : 'Tag (e.g. GENRE, ARTIST, TXXX:MOOD)'"
                v-model='filterDraft.tag'
                :disable='filterDraft.preset >= 0'
                class='q-mb-sm'
            />
            <q-input
                filled dense
                :label='filterDraft.preset >= 0 && filterPresetValues.length > 0 ? "Value (type freely or pick from list)" : "Value (empty = tag has any value)"'
                v-model='filterDraft.value'
                clearable
                class='q-mb-sm'
            >
                <template v-slot:append>
                    <q-icon
                        v-if='filterPresetValues.length > 0'
                        name='mdi-menu-down'
                        class='cursor-pointer'
                    >
                        <q-menu fit>
                            <q-list dense>
                                <q-item
                                    v-for='v in filterPresetValues'
                                    :key='v'
                                    clickable
                                    v-close-popup
                                    @click='filterDraft.value = v'
                                >
                                    <q-item-section class='monospace'>{{ v }}</q-item-section>
                                </q-item>
                            </q-list>
                        </q-menu>
                    </q-icon>
                </template>
            </q-input>
            <q-select
                filled dense label='Match'
                v-model='filterDraft.mode'
                :options="[
                    {label: 'Contains (case-insensitive)', value: 'contains'},
                    {label: 'Exact (case-insensitive)', value: 'exact'},
                    {label: 'Regex', value: 'regex'},
                ]"
                emit-value map-options
                class='q-mb-sm'
            />
            <q-toggle v-model='filterDraft.recursive' label='Include subfolders' class='q-mb-md' />
            <div class='row justify-end'>
                <q-btn flat label='Cancel' v-close-popup class='q-mr-sm' />
                <q-btn push color='primary' class='text-black' label='Run filter' @click='applyFilterQuery' />
            </div>
        </q-card>
    </q-dialog>

    <!-- Rename custom tag dialog -->
    <q-dialog v-model='renameDialog'>
        <q-card style='min-width: 460px;' class='q-pa-md'>
            <div class='text-h6 q-mb-sm'>Rename tag frame</div>
            <div class='text-caption text-grey-5 q-mb-md'>
                Reads the value at <span class='monospace text-primary'>From</span>, writes it to <span class='monospace text-primary'>To</span>, deletes the old key. Queues the rename — click Save to commit across {{ selectedPaths.length }} file(s).
            </div>
            <q-select
                filled dense label='Pick "From" preset (resolves to current file format)'
                v-model='renameDraft.presetFrom'
                :options="[{label: '— manual frame below —', value: -1}].concat(customTagDefs.map((d, i) => ({label: d.label + ' (' + (d.frame.id3 || '?') + ')', value: i})))"
                emit-value map-options
                class='q-mb-sm'
            />
            <q-input
                filled dense
                :label="renameDraft.presetFrom >= 0 ? 'From — overridden by preset above' : 'From (e.g. TXXX:OLD)'"
                v-model='renameDraft.from'
                :disable='renameDraft.presetFrom >= 0'
                class='q-mb-sm monospace'
            />
            <q-input filled dense label='To (e.g. TXXX:MOOD)' v-model='renameDraft.to' class='q-mb-md monospace' />
            <div class='row justify-end'>
                <q-btn flat label='Cancel' v-close-popup class='q-mr-sm' />
                <q-btn push color='primary' class='text-black' label='Queue rename' @click='applyRename' />
            </div>
        </q-card>
    </q-dialog>

    <!-- Bottom progress drawer (visible during/after a bulk save) -->
    <div v-if="bulk.phase !== 'idle'" class='te-progress-drawer'>
        <div class='row items-center q-px-md q-py-sm'>
            <q-circular-progress
                show-value
                :value='bulk.total === 0 ? 0 : (bulk.index / bulk.total) * 100'
                size='32px' :thickness='0.22' color='primary' track-color='grey-9'
                class='q-mr-md'
            >
                <span class='text-caption'>{{ bulk.index }}/{{ bulk.total }}</span>
            </q-circular-progress>
            <div style='flex: 1; min-width: 0;'>
                <div class='text-body2'>
                    <span v-if="bulk.phase === 'start'">Preparing…</span>
                    <span v-else-if="bulk.phase === 'progress'">
                        Writing {{ bulk.index }} of {{ bulk.total }}
                        <span v-if='bulk.failed.length' class='text-red q-ml-sm'>{{ bulk.failed.length }} failed</span>
                    </span>
                    <span v-else-if="bulk.phase === 'done'">
                        <span v-if='bulk.cancelled' class='text-warning'>Cancelled</span>
                        <span v-else-if='bulk.failed.length' class='text-red'>Done — {{ bulk.failed.length }} failures</span>
                        <span v-else class='text-positive'>Done — {{ bulk.succeeded }} written</span>
                    </span>
                </div>
                <div class='text-caption text-grey-5 ellipsis' v-if='bulk.lastPath'>{{ bulk.lastPath }}</div>
            </div>
            <q-btn
                v-if="bulk.phase === 'progress' || bulk.phase === 'start'"
                dense flat color='red' icon='mdi-close' label='Cancel' @click='cancelBulk'
            />
            <q-btn
                v-else
                dense flat icon='mdi-close' @click="bulk.phase = 'idle'"
            />
        </div>
    </div>

</div>
</template>

<script lang='ts' setup>
import TagField from '../components/TagField.vue';
import AddAlbumArt from '../components/AddAlbumArt.vue';
import draggable from 'vuedraggable';
import { ABSTRACTIONS } from '../scripts/tags';
import { computed, onDeactivated, onMounted, ref } from 'vue';
import { get1t } from '../scripts/digtrax';
import { useQuasar } from 'quasar';
import ManualTag from '../components/ManualTag.vue';
import TagEditorAlbumArt from '../components/TagEditorAlbumArt.vue';

const $1t = get1t();
const $q = useQuasar();
const path = ref($1t.settings.value.path);
const files = ref<any[]>([]);
const originalFiles = ref<any[]>([]);
// Phase 2 multi-select model: every loaded file lives in `loadedFiles`,
// `selectedPaths` is the canonical selection used at save time. `file`
// stays as a computed pointing at the first loaded file so the existing
// single-file template (album art, ID3 panes) keeps working untouched.
const loadedFiles = ref<any[]>([]);
const selectedPaths = ref<string[]>([]);
const selectionAnchor = ref<string | null>(null);
const file = computed<any>(() => loadedFiles.value[0]);
const filter = ref<any>(undefined);
const changes = ref<any[]>([]);
const newTag = ref<any>(undefined);
const albumArt = ref<any>(undefined);
const showAlbumArt = ref(false);
const addAlbumArtDialog = ref(false);
const customList = ref($1t.settings.value.tagEditorCustom);
const id3v24 = ref(false);
const manualTagPath = ref<string | undefined>(undefined);

// Bulk save progress. `phase: idle` hides the drawer; `start` and `progress`
// keep it visible. Reset to `idle` when the final tagEditorSave event arrives.
type BulkProgress = {
    phase: 'idle' | 'start' | 'progress' | 'done';
    index: number;
    total: number;
    succeeded: number;
    failed: { path: string; error: string }[];
    cancelled: boolean;
    lastPath?: string;
};
const bulk = ref<BulkProgress>({
    phase: 'idle', index: 0, total: 0, succeeded: 0, failed: [], cancelled: false
});

// Filter dialog state (Step 3). `preset` carries the index into
// customTagDefs.value when the user picks one — that's how we tell the
// query builder to span all 3 format-specific frame keys.
const filterDialog = ref(false);
const filterDraft = ref({ tag: '', value: '', mode: 'contains', recursive: false, preset: -1 });

// Rename-tag dialog state (Step 4). `presetFrom` mirrors the filter preset
// for the From side so users don't have to memorise raw frame names.
const renameDialog = ref(false);
const renameDraft = ref({ from: '', to: '', presetFrom: -1 });
// Configured values for the active filter preset — feeds the Value combobox.
const filterPresetValues = computed<string[]>(() => {
    const i = filterDraft.value.preset;
    if (i < 0) return [];
    return customTagDefs.value[i]?.values ?? [];
});

const isMultiSelect = computed(() => selectedPaths.value.length > 1);

// User-configured custom fields from QuickTag settings. Each entry is a
// (friendly label, FrameName) pair — Mood/Energy/Note/Subgenre plus every
// `custom[]` entry. These let the editor surface meaningful rows even when
// the underlying frame isn't yet present on the loaded files.
type CustomDef = { label: string; frame: any; values?: string[] };
const customTagDefs = computed<CustomDef[]>(() => {
    const qt: any = ($1t.settings.value as any).quickTag;
    if (!qt) return [];
    const defs: CustomDef[] = [];
    if (qt.moodTag) defs.push({
        label: 'Mood', frame: qt.moodTag,
        values: (qt.moods ?? []).map((m: any) => m?.mood).filter(Boolean),
    });
    if (qt.energyTag?.tag) defs.push({ label: 'Energy', frame: qt.energyTag.tag });
    if (qt.noteTag?.tag) defs.push({ label: 'Note', frame: qt.noteTag.tag });
    if (qt.subgenreTag) defs.push({
        label: 'Subgenre', frame: qt.subgenreTag,
        values: (qt.genres ?? []).flatMap((g: any) => g?.subgenres ?? []),
    });
    for (const c of (qt.custom ?? []) as any[]) {
        if (c?.name && c?.tag) {
            defs.push({
                label: c.name,
                frame: c.tag,
                values: (c.values ?? []).map((v: any) => v?.val).filter(Boolean),
            });
        }
    }
    return defs;
});

function frameKey(frame: any, format: string | undefined): string | undefined {
    if (!frame) return undefined;
    // FrameName carries id3/vorbis/mp4 fields and (when class-instanced) a byFormat method.
    if (typeof frame.byFormat === 'function') return frame.byFormat(format ?? 'mp3');
    if (!format) return frame.id3;
    if (format === 'mp3' || format === 'aif' || format === 'aiff' || format === 'wav') return frame.id3;
    if (format === 'mp4') return frame.mp4;
    return frame.vorbis ?? frame.id3;
}

// All formats actually present in the loaded set. Used to decide which custom
// frame keys are relevant — single-format selections show one row per custom
// def; mixed-format selections show one row per (def × format) actually used.
const presentFormats = computed<string[]>(() => {
    const set = new Set<string>();
    for (const f of loadedFiles.value) if (f?.format) set.add(f.format);
    return [...set];
});

// Resolved custom-tag rows. Each row is a real frame key (e.g. TMOO, MOOD,
// iTunes:MOOD) tagged with the friendly label that produced it.
const customTagRows = computed<Array<{ key: string; label: string }>>(() => {
    const rows: Array<{ key: string; label: string }> = [];
    const seen = new Set<string>();
    for (const def of customTagDefs.value) {
        for (const fmt of presentFormats.value) {
            const k = frameKey(def.frame, fmt);
            if (k && !seen.has(k)) {
                seen.add(k);
                rows.push({ key: k, label: def.label });
            }
        }
        // Even with no files loaded yet, surface ID3 form so the editor isn't empty.
        if (presentFormats.value.length === 0) {
            const k = frameKey(def.frame, 'mp3');
            if (k && !seen.has(k)) { seen.add(k); rows.push({ key: k, label: def.label }); }
        }
    }
    return rows;
});

// Friendly label for a key — custom defs first, then the static ABSTRACTIONS
// table for standard format-specific frame IDs.
function customLabel(key: string): string | undefined {
    return customTagRows.value.find(r => r.key === key)?.label;
}

// Union of tag keys across loaded files plus the custom tag rows. Custom
// tags lead so the user always sees them at the top; then file tags in
// first-file order, then alphabetical extras.
const allTagKeys = computed<string[]>(() => {
    const seen = new Set<string>();
    const ordered: string[] = [];
    for (const r of customTagRows.value) {
        if (!seen.has(r.key)) { seen.add(r.key); ordered.push(r.key); }
    }
    if (loadedFiles.value.length === 0) return ordered;
    for (const k of Object.keys(loadedFiles.value[0]?.tags ?? {})) {
        if (!seen.has(k)) { seen.add(k); ordered.push(k); }
    }
    const extra: string[] = [];
    for (let i = 1; i < loadedFiles.value.length; i++) {
        for (const k of Object.keys(loadedFiles.value[i]?.tags ?? {})) {
            if (!seen.has(k)) { seen.add(k); extra.push(k); }
        }
    }
    extra.sort();
    return ordered.concat(extra);
});

// ID3-binary frames (USLT/COMM/POPM) carry structured payloads outside the
// flat tags map — peek into the per-file id3 sub-object so the bulk editor
// can show their values and round-trip edits.
type Id3Bin = 'uslt' | 'comm' | 'popm' | null;
function id3Bin(key: string): Id3Bin {
    // Match plain frame ID and any "FRAME:descriptor" form.
    const head = key.split(':')[0];
    if (head === 'USLT') return 'uslt';
    if (head === 'COMM') return 'comm';
    if (head === 'POPM') return 'popm';
    return null;
}

function getValueForKey(f: any, key: string): string | undefined {
    if (!f) return undefined;
    const flat = f.tags?.[key];
    if (flat !== undefined && flat !== '') return flat;
    if (!f.id3) return undefined;
    switch (id3Bin(key)) {
        case 'uslt': return f.id3.unsync_lyrics?.[0]?.text || undefined;
        case 'comm': return f.id3.comments?.[0]?.text || undefined;
        case 'popm': {
            const r = f.id3.popularimeter?.rating;
            return (typeof r === 'number') ? String(r) : undefined;
        }
        default: return undefined;
    }
}

// Per-key summary across the loaded set:
//   distinct  — number of distinct values (1 = unified)
//   unified   — the common value if distinct === 1, else ''
//   present   — how many files have a non-empty value for this key
function tagSummary(key: string): { distinct: number; unified: string; present: number } {
    const set = new Set<string>();
    let present = 0;
    for (const f of loadedFiles.value) {
        const v = getValueForKey(f, key);
        if (v !== undefined && v !== '') {
            present += 1;
            set.add(v);
        }
    }
    if (set.size === 1) {
        return { distinct: 1, unified: set.values().next().value ?? '', present };
    }
    return { distinct: set.size, unified: '', present };
}

// Local "edit value" buffer keyed by tag — what's in the input box right now,
// as a pending bulk change. Distinct from the per-file `tags` map: in
// multi-select mode the input is detached from any single file and only feeds
// the change list when the user types.
const editBuffer = ref<Record<string, string>>({});
const overrides = ref<Set<string>>(new Set()); // keys the user has actually edited

function bufferFor(key: string): string {
    if (overrides.value.has(key)) return editBuffer.value[key] ?? '';
    return tagSummary(key).unified;
}

function setBuffer(key: string, value: string) {
    overrides.value.add(key);
    editBuffer.value[key] = value;
    const bin = id3Bin(key);
    if (bin === 'uslt') queueId3Lyrics(value);
    else if (bin === 'comm') queueId3Comments(value);
    else if (bin === 'popm') queueId3Popularimeter(value);
    else queueRawChange(key, value);
}

// ID3-binary writers — synthesize default lang/description so a single text
// field is enough for bulk edit. Multi-block lyrics/comments are flattened to
// one block on save; if the user needs surgical control they can drop into
// single-file mode where the dedicated ID3 panes are still available.
function queueId3Lyrics(text: string) {
    const lyrics = [{ lang: 'eng', description: '', text }];
    const idx = changes.value.findIndex((c: any) => c.type === 'id3UnsynchronizedLyrics');
    if (idx !== -1) (changes.value[idx] as any).lyrics = lyrics;
    else changes.value.push({ type: 'id3UnsynchronizedLyrics', lyrics } as any);
}
function queueId3Comments(text: string) {
    const comments = [{ lang: 'eng', description: '', text }];
    const idx = changes.value.findIndex((c: any) => c.type === 'id3Comments');
    if (idx !== -1) (changes.value[idx] as any).comments = comments;
    else changes.value.push({ type: 'id3Comments', comments } as any);
}
function queueId3Popularimeter(value: string) {
    const rating = parseInt(value, 10);
    if (!Number.isFinite(rating) || rating < 0 || rating > 255) return;
    const popm = { email: 'no@email', rating, counter: 0 };
    const idx = changes.value.findIndex((c: any) => c.type === 'id3Popularimeter');
    if (idx !== -1) (changes.value[idx] as any).popm = popm;
    else changes.value.push({ type: 'id3Popularimeter', popm } as any);
}

function queueRawChange(key: string, value: string) {
    // Mirror the format-aware splitting from onChange: MP3 keeps a single
    // value, other formats split on comma.
    let arr: string[];
    const fmt = file.value?.format;
    if (fmt && fmt !== 'mp3') arr = value.split(',');
    else arr = [value];
    const idx = changes.value.findIndex((c: any) => c.type === 'raw' && c.tag === key);
    if (idx !== -1) changes.value[idx].value = arr;
    else changes.value.push({ type: 'raw', tag: key, value: arr });
}

// Parse a comma-joined tag value into a deduped, trimmed list.
function parseValues(s: string | undefined | null): string[] {
    if (!s) return [];
    const seen = new Set<string>();
    const out: string[] = [];
    for (const part of s.split(',')) {
        const t = part.trim();
        if (t && !seen.has(t)) { seen.add(t); out.push(t); }
    }
    return out;
}

// Effective values for a key — pending edits override file state. Returns the
// union of values across loaded files when no override exists; once the user
// has touched the row (override flag set) reflects only what's in the buffer.
function effectiveValues(key: string): string[] {
    if (overrides.value.has(key)) return parseValues(editBuffer.value[key]);
    const set = new Set<string>();
    for (const f of loadedFiles.value) {
        for (const v of parseValues(getValueForKey(f, key))) set.add(v);
    }
    return [...set];
}

// Resolve the matching custom def for a key — used to surface configured
// suggestion chips. Tries every present format so customs that use different
// frame names per format still match.
function customDefFor(key: string): CustomDef | undefined {
    const fmts = presentFormats.value.length ? presentFormats.value : ['mp3'];
    return customTagDefs.value.find((d) => fmts.some((f) => frameKey(d.frame, f) === key));
}

// Chip-mode info: current values, suggestion chips (configured but not
// currently set), and a flag for whether to render chips at all. We render
// chips when: the key has a custom def with values to suggest, OR the existing
// tag is multi-valued and under 10 distinct values (the user can keep
// adding/removing without losing track).
function tagChips(key: string): { current: string[]; suggestions: string[]; show: boolean } {
    const current = effectiveValues(key);
    const def = customDefFor(key);
    const configured = def?.values ?? [];
    const suggestions = configured.filter((v) => !current.includes(v));
    // POPM is a numeric score; chip-mode would be misleading. USLT (lyrics)
    // is free-form text — also not chip-friendly.
    const bin = id3Bin(key);
    if (bin === 'popm' || bin === 'uslt') return { current, suggestions: [], show: false };
    const show = (configured.length > 0) || (current.length > 0 && current.length < 10);
    return { current, suggestions, show };
}

// Persist a new chip set for the key. Picks the right TagChange variant
// (Raw vs ID3 binary) based on the key.
function commitChips(key: string, values: string[]) {
    overrides.value.add(key);
    const joined = values.join(', ');
    editBuffer.value[key] = joined;
    const bin = id3Bin(key);
    if (bin === 'comm') {
        queueId3Comments(joined);
    } else {
        // Use the array form directly — preserves chip-per-value semantics
        // even on MP3 where free-text input would join into one ID3 frame.
        const idx = changes.value.findIndex((c: any) => c.type === 'raw' && c.tag === key);
        if (idx !== -1) (changes.value[idx] as any).value = values;
        else changes.value.push({ type: 'raw', tag: key, value: values } as any);
    }
}

function addChipValue(key: string, value: string) {
    const v = (value ?? '').trim();
    if (!v) return;
    const current = effectiveValues(key);
    if (current.includes(v)) return;
    commitChips(key, [...current, v]);
}

function removeChipValue(key: string, value: string) {
    const current = effectiveValues(key).filter((x) => x !== value);
    commitChips(key, current);
}

function loadFiles(f?: string) {
    $1t.send('tagEditorFolder', {path: path.value, subdir: f});
}

function browse() {
    $1t.browse('te', path.value);
}

function loadSelected() {
    // Push selectedPaths over the wire. Empty selection clears the editor pane.
    if (selectedPaths.value.length === 0) {
        loadedFiles.value = [];
        return;
    }
    if ($1t.settings.value.tagEditorPlayer && selectedPaths.value.length === 1)
        $1t.player.value.loadTrack(selectedPaths.value[0]);
    $1t.send('tagEditorLoad', { paths: [...selectedPaths.value] });
}

function loadFile(p: string, ev?: MouseEvent) {
    // Autosave on switch (existing behavior, single-file only — bulk autosave
    // would silently rewrite many files).
    if (file.value && !isMultiSelect.value && $1t.settings.value.tagEditorAutosave) {
        save();
    }
    changes.value = [];
    overrides.value = new Set();
    editBuffer.value = {};

    const cmd = ev?.metaKey || ev?.ctrlKey;
    const shift = ev?.shiftKey;

    if (shift && selectionAnchor.value) {
        // Range-select between anchor and clicked file (within the current
        // visible folder listing).
        const visible = files.value.filter((x: any) => !x.dir && !x.playlist).map((x: any) => x.path);
        const a = visible.indexOf(selectionAnchor.value);
        const b = visible.indexOf(p);
        if (a !== -1 && b !== -1) {
            const [lo, hi] = a < b ? [a, b] : [b, a];
            selectedPaths.value = visible.slice(lo, hi + 1);
            loadSelected();
            return;
        }
    }

    if (cmd) {
        // Toggle membership without losing existing selection.
        const i = selectedPaths.value.indexOf(p);
        if (i === -1) selectedPaths.value = [...selectedPaths.value, p];
        else selectedPaths.value = selectedPaths.value.filter((x) => x !== p);
        selectionAnchor.value = p;
        loadSelected();
        return;
    }

    // Plain click: replace selection.
    selectedPaths.value = [p];
    selectionAnchor.value = p;
    loadSelected();
}

function toggleSelectAll() {
    const visible = files.value.filter((x: any) => !x.dir && !x.playlist).map((x: any) => x.path);
    if (selectedPaths.value.length === visible.length && visible.every((p: string) => selectedPaths.value.includes(p))) {
        selectedPaths.value = [];
        loadedFiles.value = [];
    } else {
        selectedPaths.value = visible;
        loadSelected();
    }
}

function clearSelection() {
    selectedPaths.value = [];
    loadedFiles.value = [];
    changes.value = [];
    overrides.value = new Set();
    editBuffer.value = {};
}

// If file is in the active selection set
function isSelected(path: string) {
    return selectedPaths.value.includes(path);
}

function applyFilter(v: string) {
    filter.value = v;
    if (!filter.value || filter.value.trim().length == 0) {
        files.value = originalFiles.value;
        return;
    }
    files.value = originalFiles.value.filter(f => f.filename.toLowerCase().includes(filter.value));
}


/*
    Custom list
*/

// Vue draggable file drag process
function onFileDrag(e: any) {
    if (e.added) {
        if (e.added.element.dir || e.added.element.playlist) {
            $1t.send('tagEditorFolder', {path: path.value, subdir: e.added.element.filename, recursive: true});
            // Don't copy
            customList.value.splice(e.added.newIndex, 1);
        } else {
            // Duplicate
            if (!customList.value.find((i) => i == e.added.element.path)) 
                customList.value.splice(e.added.newIndex, 1, e.added.element.path);
            else 
                customList.value.splice(e.added.newIndex, 1);
        }
    }
    // Read again
    if (e.removed) {
        files.value.splice(e.removed.oldIndex, 0, e.removed.element);
    }
    saveSettings();
}

// Allow only one way drag
function onFileMove(e: any) {
    if (e.relatedContext.component.$el.id == 'fileList') return false;
}
function removeCustom(i: string) {
    customList.value.splice(customList.value.indexOf(i), 1);
    saveSettings();
}

// Get filename from path
function filename(path: string) {
    path = path.toString();
    if (path.trim().startsWith('/')) {
        let s = path.split('/');
        return s[s.length - 1];
    }
    let s = path.split('\\');
    return s[s.length - 1];
}
function clearCustom() {
    customList.value = [];
    saveSettings();
}

/*
    Text Tags
*/

// Delete tag
function removeTag(tag: string) {
    delete file.value.tags[tag];
    changes.value.push({
        type: 'remove',
        tag: tag
    })
}

// Create new tag
function addNewTag() {
    if (!newTag.value) return;
    if (file.value.tags[newTag.value]) {
        $q.notify({
            message: "Tag already exists!",
            timeout: 2000,
            position: 'top-right'
        });
        return;
    }
    // Remove removal of tag
    let i = changes.value.findIndex((c) => c.type == 'remove' && c.tag == newTag.value);
    if (i > -1) changes.value.splice(i, 1);

    file.value.tags[newTag.value] = '';
    changes.value.push({
        type: 'raw',
        tag: newTag.value,
        value: []
    });
}

function onChange(tag: string) {
    // Used by ID3-specific single-file inputs (album art etc.). Bulk-aware
    // text-tag editing flows through `setBuffer` instead.
    if (!file.value) return;
    let value = file.value.tags[tag]
    // Split only for tags, MP3 writes to single tag as id3 separator
    if (file.value.format != 'mp3') {
        value = value.split(',');
    } else {
        value = [value];
    }
    let index = changes.value.findIndex((c: any) => c.tag == tag);
    if (index != -1) {
        changes.value[index].value = value;
    } else {
        changes.value.push({ type: 'raw', tag: tag, value: value });
    }
}

/*
    Album Art
*/

// Add new album art
function addAlbumArt(data: any) {
    // Find old image
    file.value.images = file.value.images.filter((i: any) => i.kind != data.kind);
    changes.value = changes.value.filter((c) => c.type != 'addPictureBase64' || c.kind != data.kind);

    // Add
    changes.value.push({
        type: 'addPictureBase64',
        mime: data.mime,
        data: data.data,
        kind: data.kind,
        description: data.description
    });
    data.data = `data:${data.mime};base64,${data.data}`;
    file.value.images.push(data);
}

// Delete album art
function removeArt(i: number) {
    let kind = file.value.images[i].kind;
    file.value.images.splice(i, 1);
    //Remove newly added image
    let index = changes.value.findIndex((c) => c.type == "addPictureBase64" && c.kind == kind);
    if (index != -1) {
        changes.value.splice(index, 1);
        return;
    }
    changes.value.push({
        type: 'removePicture',
        kind
    });
}

/*
    ID3 Comments
*/

// Generate new change for ID3 comments
function id3CommentsChange() {
    let i = changes.value.findIndex((c) => c.type == 'id3Comments');
    if (i > -1) {
        changes.value.splice(i, 1);
    }
    changes.value.push({
        type: 'id3Comments',
        comments: file.value.id3.comments
    });
}

function addID3Comment() {
    file.value.id3.comments.push({
        lang: "eng",
        description: "",
        text: ""
    });
    id3CommentsChange();
}

function removeID3Comment(i: number) {
    file.value.id3.comments.splice(i, 1);
    id3CommentsChange();
}


/*
    ID3 Unsynchronized lyrics
*/
function id3USLTChange() {
    let i = changes.value.findIndex((c) => c.type == 'id3UnsynchronizedLyrics');
    if (i > -1) changes.value.splice(i, 1);
    changes.value.push({
        type: 'id3UnsynchronizedLyrics',
        lyrics: file.value.id3.unsync_lyrics
    });
}
function removeID3USLT(i: number) {
    file.value.id3.unsync_lyrics.splice(i, 1);
    id3USLTChange();
}
function addID3USLT() {
    file.value.id3.unsync_lyrics.push({
        lang: 'eng',
        description: '',
        text: ''
    });
    id3USLTChange();
}

/*
    ID3 Popularimeter
*/

function id3POPMChange() {
    // Remove existing popm changes
    let i = changes.value.findIndex((c) => c.type == 'id3Popularimeter');
    if (i > -1) changes.value.splice(i, 1);
    i = changes.value.findIndex((c) => c.type == "remove" && c.tag == "POPM");
    if (i > -1) changes.value.splice(i, 1);
    // Add new changes
    if (file.value.id3.popularimeter) {
        file.value.id3.popularimeter.counter = parseInt(file.value.id3.popularimeter.counter.toString());
        changes.value.push({
            type: 'id3Popularimeter',
            popm: file.value.id3.popularimeter
        });
    } else {
        changes.value.push({
            type: 'remove',
            tag: 'POPM'
        });
    }
}
function addPOPM() {
    file.value.id3.popularimeter = {
        email: "no@email",
        rating: 0,
        counter: 0
    }
    id3POPMChange();
}
function removePOPM() {
    file.value.id3.popularimeter = null;
    id3POPMChange();
}


/*
    Saving and backend
*/

// Save to all files in `selectedPaths`. Phase 2 bulk save: the same change
// set goes to every selected file via `commit_multi` on the backend.
function save() {
    if (selectedPaths.value.length === 0) return;
    if (changes.value.length === 0) {
        $q.notify({ message: 'No changes to save', timeout: 1500, position: 'top-right' });
        return;
    }
    $1t.send('tagEditorSave', {
        paths: [...selectedPaths.value],
        changes: {
            changes: changes.value,
            separators: {id3: ', ', vorbis: null, mp4: ', '},
            id3v24: id3v24.value
        }
    });
    // Don't reset `changes` until the save round-trips successfully — the user
    // can still cancel mid-flight.
}

function buildPredicate(tag: string, value: string | null | undefined, mode: string): any {
    const v = (value ?? '').trim();
    if (v === '') return { type: 'hasTag', tag };
    return { type: 'equals', tag, value: v, mode };
}

// When a preset is picked, expand to ID3 / Vorbis / MP4 frame keys and combine
// with Or so the query matches files of any format.
function buildPresetQuery(preset: number, value: string, mode: string): any | undefined {
    const def = customTagDefs.value[preset];
    if (!def) return undefined;
    const keys = ['id3', 'vorbis', 'mp4']
        .map((fmt) => frameKey(def.frame, fmt === 'id3' ? 'mp3' : fmt))
        .filter((k): k is string => !!k);
    const uniq = [...new Set(keys)];
    if (uniq.length === 0) return undefined;
    if (uniq.length === 1) return buildPredicate(uniq[0], value, mode);
    return { type: 'or', children: uniq.map((k) => buildPredicate(k, value, mode)) };
}

function applyFilterQuery() {
    const draft = filterDraft.value;
    let query: any;
    if (draft.preset >= 0) {
        query = buildPresetQuery(draft.preset, draft.value, draft.mode);
        if (!query) {
            $q.notify({ message: 'Preset has no frame mapping', timeout: 2000, position: 'top-right' });
            return;
        }
    } else {
        if (!draft.tag.trim()) return;
        query = buildPredicate(draft.tag, draft.value, draft.mode);
    }
    console.log('[tagEditorFilter]', { value: draft.value, mode: draft.mode, preset: draft.preset, query });
    $1t.send('tagEditorFilter', {
        folder: path.value,
        recursive: draft.recursive,
        query,
    });
    filterDialog.value = false;
}

function applyRename() {
    // If user picked a preset for "From", expand to the most relevant per-format
    // frame name based on the loaded files. Falls back to id3 if nothing loaded.
    const draft = renameDraft.value;
    let from = draft.from.trim();
    if (!from && draft.presetFrom >= 0) {
        const def = customTagDefs.value[draft.presetFrom];
        const fmt = (loadedFiles.value[0]?.format) ?? 'mp3';
        from = frameKey(def?.frame, fmt) ?? '';
    }
    const to = draft.to.trim();
    if (!from || !to || from === to) {
        $q.notify({ message: 'Provide distinct from / to tag names', timeout: 2000, position: 'top-right' });
        return;
    }
    if (selectedPaths.value.length === 0) {
        $q.notify({ message: 'Select files first', timeout: 2000, position: 'top-right' });
        return;
    }
    // Queue a RenameFrame change. User still hits Save to commit it.
    const idx = changes.value.findIndex((c: any) => c.type === 'renameFrame' && c.from === from);
    if (idx !== -1) changes.value[idx].to = to;
    else changes.value.push({ type: 'renameFrame', from, to });
    renameDialog.value = false;
    $q.notify({
        message: `Queued rename: ${from} → ${to} for ${selectedPaths.value.length} file(s). Click Save to commit.`,
        timeout: 3500,
        position: 'top-right',
    });
}

function cancelBulk() {
    $1t.send('tagEditorCancel', {} as any);
}

function saveSettings() {
    $1t.settings.value.path = path.value;
    $1t.settings.value.tagEditorCustom = customList.value;
    $1t.saveSettings(false);
}

// Websocket callback
function wsCallback(e: any) {
    switch (e.action) {
        case 'browse':
            path.value = e.path;
            loadFiles();
            break;
        case 'tagEditorFolder':
            if (e.recursive) {
                // Add dir to custom list
                let files = customList.value.concat(e.files.sort((a: any, b: any) => {
                    return a.filename.toLowerCase().localeCompare(b.filename.toLowerCase());
                }).map((f: any) => f.path));
                // Deduplicate
                customList.value = [... new Set(files)];
            } else {
                path.value = e.path;
                //Dirs first and sort
                originalFiles.value = e.files.sort((a: any, b: any) => {
                    if (a.dir && !b.dir) return -1;
                    if (b.dir && !a.dir) return 1;
                    return a.filename.toLowerCase().localeCompare(b.filename.toLowerCase());
                });
                applyFilter(filter.value);
            }
            saveSettings();
            break;
        case 'tagEditorLoad': {
            // LoadedFileSet { files: TagEditorFile[], failed: [(path, error)] }
            const set = e.data ?? {files: [], failed: []};
            loadedFiles.value = set.files ?? [];
            // Reset the buffer/override layer so the inputs reflect the freshly
            // loaded files. `changes` was already cleared in loadFile().
            overrides.value = new Set();
            editBuffer.value = {};
            if (set.failed?.length) {
                console.warn('TagEditor load failures:', set.failed);
                $q.notify({
                    message: `${set.failed.length} file(s) failed to load`,
                    color: 'warning', timeout: 3000, position: 'top-right'
                });
            }
            break;
        }
        case 'tagEditorBulkProgress': {
            // phase: 'start' | 'progress'
            if (e.phase === 'start') {
                bulk.value = {
                    phase: 'start',
                    index: 0,
                    total: e.total ?? selectedPaths.value.length,
                    succeeded: 0,
                    failed: [],
                    cancelled: false,
                };
            } else if (e.phase === 'progress') {
                bulk.value.phase = 'progress';
                bulk.value.index = e.index ?? bulk.value.index + 1;
                bulk.value.total = e.total ?? bulk.value.total;
                bulk.value.lastPath = e.path;
                if (e.ok) bulk.value.succeeded += 1;
                else bulk.value.failed.push({ path: e.path, error: e.error ?? 'unknown' });
            }
            break;
        }
        case 'tagEditorSave': {
            const failed = e.report?.failed?.length ?? 0;
            const succeeded = e.report?.succeeded ?? 0;
            const cancelled = !!e.cancelled;
            $q.notify({
                message: cancelled
                    ? `Cancelled. Wrote ${succeeded}, failed ${failed}`
                    : (failed
                        ? `Wrote ${succeeded}, failed ${failed}`
                        : `Wrote ${succeeded} file${succeeded === 1 ? '' : 's'}`),
                color: failed ? 'negative' : (cancelled ? 'warning' : undefined),
                timeout: 4000,
                position: 'top-right'
            });
            // Save round-tripped — clear pending changes and finalize the drawer.
            changes.value = [];
            overrides.value = new Set();
            editBuffer.value = {};
            bulk.value.phase = 'done';
            bulk.value.cancelled = cancelled;
            // Auto-dismiss the drawer after a short pause (gives user time to read failures).
            setTimeout(() => { if (bulk.value.phase === 'done') bulk.value.phase = 'idle'; }, 5000);
            break;
        }
        case 'tagEditorFilter': {
            const matches: string[] = e.paths ?? [];
            $q.notify({
                message: `Filter: matched ${matches.length} of ${e.scanned ?? '?'} scanned`,
                timeout: 3000, position: 'top-right'
            });
            if (matches.length > 0) {
                selectedPaths.value = matches;
                selectionAnchor.value = matches[0];
                loadSelected();
            }
            break;
        }
        case 'tagEditorCancel': {
            // ack only — the actual termination happens in tagEditorSave when
            // the loop notices the flag.
            break;
        }
        // Internal callback
        case '_tagEditorSave':
            save();
            break;
        default: 
            console.log(e);
            break;
    }
}

const tagFormat = computed(() => {
    if (!file.value) return null;
    if (file.value.format == 'flac' || file.value.format == 'ogg') return 'vorbis';
    if (file.value.format == 'mp4') return 'mp4';
    return 'id3';
});

// Filter used types
const albumArtTypes = computed(() => {
    let types = ["CoverFront", "CoverBack", "Other", "Artist", "Icon", "OtherIcon", 
        "Leaflet", "Media", "LeadArtist", "Conductor", "Band", "Composer", "Lyricist", 
        "RecordingLocation", "DuringRecording", "DuringPerformance", "ScreenCapture", 
        "BrightFish", "Illustration", "BandLogo", "PublisherLogo"];
    if (!file.value) return types;
    return types.filter((t) => file.value.images.find((i: any) => i.kind == t) ? false : true);
});

const POPMLabel = computed(() => {
    let v = file.value.id3.popularimeter.rating;
    let stars = Math.ceil(v / 51);
    if (stars == 0) stars = 1;
    return `${v} (${stars}⭐)`;
});

// Register callback
onMounted(() => {
    $1t.onTagEditorEvent = wsCallback;
    loadFiles();

    // Load QT track
    if ($1t.quickTag.value.toTagEditor) {
        loadFile($1t.quickTag.value.toTagEditor);
        $1t.quickTag.value.toTagEditor = undefined;
    } else if ($1t.quickTag.value.track.tracks.length == 1) {
        loadFile($1t.quickTag.value.track.tracks[0].path);
    }
})

// Unregister
onDeactivated(() => {
    $1t.onTagEditorEvent = () => {};
})

</script>

<style>
.te-file {
    padding: 4px 8px;
    border-radius: var(--radius-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
    overflow: hidden;
    transition: background var(--duration-fast) var(--ease-standard);
}
.te-file:hover {
    background-color: rgba(255, 255, 255, 0.04);
}
.te-file-selected {
    background-color: rgba(0, 210, 191, 0.12);
}
.te-chips-row {
    min-height: 40px;
    padding: 4px 8px;
    border-radius: var(--radius-xs);
    background-color: rgba(255, 255, 255, 0.02);
}
.te-ghost-chip {
    opacity: 0.55;
    transition: opacity var(--duration-fast) var(--ease-standard);
}
.te-ghost-chip:hover {
    opacity: 1;
}
.te-progress-drawer {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--color-surface, #1a1a1a);
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    z-index: 1000;
    box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.4);
}
.path-display {
    text-overflow: ellipsis;
    white-space: nowrap;
    overflow: hidden;
    direction: rtl;
    text-align: left;
}
.albumart {
    min-width: 128px;
    width: 128px;
    max-width: 128px;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
}
.albumart-container {
    display: flex;
    width: 180px;
}
.separator {
    width: 2px; 
    margin-left: -17px; 
    position: absolute;
    height: 100%;
}
</style>