<template>
<div class='renamer-page'>

    <!-- Header strip: title + status hint + Start (matches Auto Tag idiom). -->
    <div class='renamer-header row items-center q-px-lg q-py-md' v-if='!$1t.lock.value.locked'>
        <div class='col'>
            <div class='text-h6 text-bold' style='letter-spacing: 0.02em;'>Renamer</div>
            <div class='text-caption text-grey-5'>
                Pick an input folder and a filename template, preview, then run.
                <span v-if='!startable' class='text-warning q-ml-xs'>{{ blockerHint }}</span>
            </div>
        </div>
        <q-btn
            push dense
            icon='mdi-play'
            color='primary' text-color='black'
            class='rounded-borders q-px-md text-weight-medium'
            label='Start'
            :disable='!startable'
            @click='start(false)'
        />
    </div>

    <!-- Loading overlay while a run is in flight. -->
    <div v-if='$1t.lock.value.locked' class='renamer-loading'>
        <q-circular-progress indeterminate size='64px' color='primary'></q-circular-progress>
    </div>

    <div v-if='!$1t.lock.value.locked' class='renamer-grid q-px-lg q-pb-xl'>

        <!-- Input + output -->
        <section class='dt-card'>
            <header class='dt-card-header'>
                <q-icon name='mdi-folder-multiple-outline' size='18px' class='dt-card-icon' />
                <div class='col'>
                    <div class='dt-card-title'>Input &amp; output</div>
                    <div class='dt-card-subtitle'>Drag &amp; drop, paste a path, or browse with the folder icon. Leave Output empty to rename in place.</div>
                </div>
            </header>
            <div class='dt-card-body'>
                <q-input
                    filled dense
                    label='Input folder'
                    v-model='config.path'
                    @update:model-value='updatePreview()'
                    class='q-mb-sm'
                >
                    <template v-slot:append>
                        <q-btn round dense flat icon='mdi-open-in-app' @click='browse(false)' />
                    </template>
                </q-input>
                <q-input
                    filled dense
                    label='Output folder (optional — same as input if blank)'
                    v-model='config.outDir'
                    @update:model-value='updatePreview()'
                >
                    <template v-slot:append>
                        <q-btn round dense flat icon='mdi-open-in-app' @click='browse(true)' />
                    </template>
                </q-input>
            </div>
        </section>

        <!-- Template editor + suggestions popup. The fake-cursor / overlay-text
             technique is preserved verbatim — it's load-bearing for the syntax
             highlight, and rewriting it is out of scope for this turn. -->
        <section class='dt-card'>
            <header class='dt-card-header'>
                <q-icon name='mdi-code-tags' size='18px' class='dt-card-icon' />
                <div class='col'>
                    <div class='dt-card-title'>Template</div>
                    <div class='dt-card-subtitle'>Mix dynamic placeholders (e.g. <span class='monospace text-accent'>%artist% - %title%</span>) and static text. <q-icon name='mdi-help-circle-outline' size='14px' class='q-ml-xs' /> for the full reference.</div>
                </div>
            </header>
            <div class='dt-card-body'>
                <div class='renamer-editor'>
                    <div class='fake-cursor' :style='cursorStyle'>|</div>
                    <div class='template-text'>
                        <span v-if='config.template' v-html='highlighted'></span>
                        <span v-if='!config.template' class='template-input-placeholder'>Filename template</span>
                    </div>
                    <input
                        class='template-input monospace'
                        spellcheck='false'
                        ref='templateInputElem'
                        @blur='onBlur'
                        @focus='onSelectionChange'
                        @selectionchange='onSelectionChange'
                        @keyup='onSelectionChange'
                        @keydown='onKeyDown'
                        @input='(e) => templateInput(e as InputEvent)'
                        @click='onSelectionChange'
                        @paste='onPaste'
                    >
                </div>

                <div v-if='suggestions.length > 0'>
                    <div class='suggestions-box' :style='suggestionsStyle'>
                        <div style='width: 40%'>
                            <div
                                v-for='(suggestion, i) in suggestions'
                                :key="'s'+i"
                                class='q-mr-sm q-pa-xs'
                                :class='{"help-suggestion-selected": i == suggestionIndex}'
                            >
                                <q-icon name='mdi-variable' class='q-mb-xs' v-if='suggestion.kind == "variable"'></q-icon>
                                <q-icon name='mdi-information-outline' class='q-mb-xs' v-if='suggestion.kind == "property"'></q-icon>
                                <q-icon name='mdi-function' class='q-mb-xs' v-if='suggestion.kind == "function"'></q-icon>
                                <span class='q-ml-sm' :class='{"text-primary": i == suggestionIndex}'>
                                    <RenamerTokenName :token='suggestion' :params='false'></RenamerTokenName>
                                </span>
                                <span v-if='i == suggestionIndex' style='float: right;'>
                                    <q-icon name='mdi-chevron-right' class='q-mb-xs' color='primary'></q-icon>
                                </span>
                            </div>
                        </div>
                        <div style='width: 60%'>
                            <div v-if='suggestions[suggestionIndex]'>
                                <div v-if='suggestions[suggestionIndex].kind == "function"' class='q-mb-sm suggestion-help-function'>
                                    <RenamerTokenName :token='suggestions[suggestionIndex]'></RenamerTokenName>
                                    <br>
                                </div>
                                <div v-html='suggestions[suggestionIndex].doc'></div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>

        <!-- Preview list -->
        <section class='dt-card' v-if='preview.length > 0'>
            <header class='dt-card-header'>
                <q-icon name='mdi-eye-outline' size='18px' class='dt-card-icon' />
                <div class='col'>
                    <div class='dt-card-title'>Preview</div>
                    <div class='dt-card-subtitle'>What the next run will write — sample of resolved filenames.</div>
                </div>
                <span class='renamer-preview-count'>{{ preview.length }}</span>
            </header>
            <div class='dt-card-body renamer-preview'>
                <div v-for='(file, i) in preview' :key='"prev"+i' class='renamer-preview-row monospace'>
                    {{ file[1] }}
                </div>
            </div>
        </section>

        <!-- Options -->
        <section class='dt-card'>
            <header class='dt-card-header'>
                <q-icon name='mdi-cog-outline' size='18px' class='dt-card-icon' />
                <div class='col'>
                    <div class='dt-card-title'>Options</div>
                    <div class='dt-card-subtitle'>Move vs. copy, overwrite policy, recursion, separator for joined values.</div>
                </div>
            </header>
            <div class='dt-card-body renamer-options'>
                <div class='renamer-options-toggles'>
                    <q-toggle left-label class='justify-between' label='Copy files instead of moving' v-model='config.copy'></q-toggle>
                    <q-toggle left-label class='justify-between' label='Overwrite existing target files' v-model='config.overwrite'></q-toggle>
                    <q-toggle left-label class='justify-between' label='Include subfolders' v-model='config.subfolders'></q-toggle>
                    <q-toggle left-label class='justify-between' label='Keep original subfolders' v-model='config.keepSubfolders'></q-toggle>
                </div>
                <div class='renamer-options-sep q-mt-md'>
                    <q-input
                        v-model='config.separator'
                        label='Multi-value separator'
                        filled dense
                        style='max-width: 240px;'
                    />
                </div>
            </div>
        </section>

    </div>

    <!-- Floating Start FAB — stays for parity when scrolled past header. -->
    <q-page-sticky position='bottom-right' :offset='[36, 32]' v-if='!$1t.lock.value.locked'>
        <q-btn
            fab push
            icon='mdi-play'
            color='primary'
            :disable='!startable'
            @click='start(false)'
        >
            <q-tooltip anchor='top middle' self='bottom middle' :offset='[10, 10]'>Start renaming</q-tooltip>
        </q-btn>
    </q-page-sticky>

    <!-- Hidden span used to measure character width for cursor positioning. -->
    <div>
        <span style='visibility: hidden; font-size: 16px;' class='monospace' ref='textWidthRef'>abcdefghijklmnopqrstuvwxyz0123456789</span>
    </div>

</div>
</template>

<script lang='ts' setup>
import RenamerTokenName from '../components/RenamerTokenName.vue';
import { computed, onMounted, onUnmounted, ref, watch, watchEffect } from 'vue';
import { get1t } from '../scripts/digtrax';
import { useQuasar } from 'quasar';

class RenamerConfig {
    path?: string;
    outDir?: string;
    template = '%artist% - %title%';
    copy = false;
    subfolders = true;
    overwrite = false;
    separator = ', ';
    keepSubfolders = false;
}

const $1t = get1t();
const $q = useQuasar();
const config = ref(new RenamerConfig());
const highlighted = ref(undefined);
const cursor = ref(-99999);
const charWidth = ref(1.0);
const suggestions = ref<any[]>([]);
const suggestionIndex = ref(0);
const suggestionOffset = ref(0);
const suggestionsTop = ref(0);
const preview = ref([]);

const templateInputElem = ref<HTMLInputElement | undefined>();
const textWidthRef = ref<HTMLElement | undefined>();

// Browse folder
function browse(output = false) {
    $1t.browse(output ? 'rnOutput' : 'rn', config.value.path);
};

// Handle typing into the template box
function templateInput(e: InputEvent) {
    if (!config.value.template) {
        if (!e.data) return;
        config.value.template = e.data;
    }
    
    // Autoclose
    let pos = cursor.value;
    if (e.data == '(') {
        injectTemplate(cursor.value + 1, ')');
        moveCursor(pos + 1);
    }
    if (e.data == '"') {
        injectTemplate(cursor.value, '"');
        moveCursor(pos + 1);
    }
    if (e.data == '%') {
        injectTemplate(cursor.value, '%');
        moveCursor(pos + 1);
    }

    // @ts-ignore
    config.value.template = e.target.value;
    updateTemplate();
};

// Fetch syntax highlighting and ac
function updateTemplate() {
    $1t.send('renamerSyntaxHighlight', { template: config.value.template });
    $1t.send('renamerAutocomplete', { 
        template: config.value.template.substring(0, cursor.value + 1) 
    });
    // Update cursor
    onSelectionChange();
};

// Handle paste event
function onPaste(e: ClipboardEvent) {
    setTimeout(() => {
        const value = (e as any).target.value;
        if (value) {
            config.value.template = value;
            updateTemplate();
        }
    }, 25);
}

// Handle global selection change to update fake cursor (yes, pain)
function onSelectionChange() {
    cursor.value = templateInputElem.value!.selectionStart!;
};

/// Template blur
function onBlur() {
    cursor.value = -6969;
    suggestions.value = [];
};

/// Template key down
function onKeyDown(e: KeyboardEvent) {
    // Control suggestions
    if (e.key == "ArrowDown") {
        if (suggestionIndex.value < suggestions.value.length - 1) suggestionIndex.value += 1;
        e.preventDefault();
        return;
    }
    if (e.key == "ArrowUp") {
        if (suggestionIndex.value > 0) suggestionIndex.value -= 1;
        e.preventDefault();
        return;
    }
    // Enter override
    if (e.key == "Enter") {
        if (suggestions.value[suggestionIndex.value]) {
            // Fill suggestion
            let text = suggestions.value[suggestionIndex.value].name.substring(suggestionOffset.value);
            let pos = cursor.value;
            injectTemplate(cursor.value, text);
            updateTemplate();
            moveCursor(pos + text.length);
        }
        e.preventDefault();
        return;
    }
    // Don't close again
    if (templateInputElem.value?.selectionStart == templateInputElem.value?.selectionEnd) {
        if (e.key == ')') {
            if (config.value.template[cursor.value] == ')') {
                e.preventDefault();
                moveCursor(cursor.value + 1);
            }
        }
        if (e.key == '"') {
            if (config.value.template[cursor.value] == '"') {
                e.preventDefault();
                moveCursor(cursor.value + 1);
            }
        }
        if (e.key == '%') {
            if (config.value.template[cursor.value] == '%') {
                e.preventDefault();
                moveCursor(cursor.value + 1);
            }
        }
    }
    
    return true;
}

/// Move cursor in template field
function moveCursor(pos: number) {
    templateInputElem.value?.setSelectionRange(pos, pos);
}

/// Add text to template
function injectTemplate(index: number, text: string) {
    templateInputElem.value!.value = templateInputElem.value!.value.substring(0, index) + text + templateInputElem.value!.value.substring(index);
    config.value.template = templateInputElem.value!.value;
}

/// Update the preview
function updatePreview() {
    $1t.send('renamerPreview', { config: config.value });
}

// Start renaming
function start(force = false) {
    // Dialog
    if (!force) {
        $q.dialog({
            title: 'Warning',
            message: 'Many DJ apps store cue points and other metadata based on the original file name. When renamed, this information will be lost and you will have to reimport these files.',
            html: true,
            ok: {
                color: 'primary',
                label: 'Start'
            },
            cancel: {
                color: 'primary',
                flat: true
            }
        })
        .onOk(() => {
            start(true);
        });
        return;
    }

    // Prevent reference
    $1t.settings.value.renamer = JSON.parse(JSON.stringify(config.value));
    $1t.saveSettings(true);
    $1t.lock.value.locked = true;
    $1t.send('renamerStart', { config: config.value });
}

/// Move suggestions box
function onScroll(e: Event) {
    // @ts-ignore
    suggestionsTop.value = e.target.scrollTop;
}

onMounted(() => {
    $1t.onRenamerEvent = (json: any) => {
        switch (json.action) {
            // Browse folder
            case 'browse':
                if (json.context == 'rnOutput')
                    config.value.outDir = json.path
                else 
                    config.value.path = json.path;

                updatePreview();
                break;
            // Syntax highlight
            case 'renamerSyntaxHighlight':
                highlighted.value = json.html;
                break;
            // Finished
            case 'renamerDone':
                $1t.lock.value.locked = false;
                $q.dialog({
                    title: 'Done',
                    message: 'Renaming finished!',
                    html: true,
                    ok: {
                        color: 'primary',
                        label: 'Open Folder'
                    },
                    cancel: {
                        color: 'primary',
                        flat: true
                    }
                }).onOk(() => {
                    $1t.send('openFolder', { path: config.value.outDir??config.value.path });
                });
                break;
            // Suggestions
            case 'renamerAutocomplete':
                suggestions.value = json.suggestions;
                suggestionOffset.value = json.offset;
                if (suggestionIndex.value > suggestions.value.length)
                    suggestionIndex.value = 0;
                break;
            // Preview renamed files
            case 'renamerPreview':
                preview.value = json.files;
                break;
            default:
                console.error(`Unknown action: ${json}`);
        }
    }

    // Restore settings
    if ($1t.settings.value.renamer) {
        config.value = Object.assign({}, config.value, $1t.settings.value.renamer);
        // console.log(config.value);
        if (config.value.template) {
            $1t.send('renamerSyntaxHighlight', { template: config.value.template });
            // @ts-ignore
            document.getElementsByClassName('template-input')[0].value = config.value.template;
        }
    }

    // Fix scroll suggestions box
    document.addEventListener('scroll', onScroll, true);
});

// Calculate character width after render
watchEffect(() => {
    setTimeout(() => {
        charWidth.value = textWidthRef.value!.offsetWidth / 36.0;
    }, 100);
})

onUnmounted(() => {
    // Remove event
    document.removeEventListener('scroll', onScroll, true);
});

const startable = computed(() => config.value.path && config.value.template);
const blockerHint = computed(() => {
    if (!config.value.path && !config.value.template) return '— pick an input folder and write a template';
    if (!config.value.path) return '— pick an input folder';
    if (!config.value.template) return '— write a template';
    return '';
});
const cursorStyle = computed(() => `margin-left: ${12 + cursor.value * charWidth.value}px`);
const suggestionsStyle = computed(() => {
    let top = `margin-top: -${suggestionsTop}px;`;
        if ((cursor.value * charWidth.value) > 500) {
            return `${top} margin-left: ${12 + cursor.value * charWidth.value - 500}px`;
        }
    return `${top} margin-left: ${12 + cursor.value * charWidth.value}px`;
});

watch(() => config.value.template, () => {
    // Debounce and render preview
    let cur = config.value.template;
    setTimeout(() => {
        if (cur != config.value.template || !config.value.template || !startable.value) return;
        updatePreview();
    }, 400);
});
</script>

<style lang='scss'>
.renamer-page {
    background: var(--color-bg);
    min-height: 100%;
}
.renamer-header {
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-elevated);
    position: sticky;
    top: 0;
    z-index: 5;
    backdrop-filter: saturate(140%) blur(8px);
}
.renamer-grid {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding-top: 18px;
    max-width: 1080px;
    margin: 0 auto;
}
.renamer-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: calc(100vh - 160px);
}

/* Template editor — preserve the fake-cursor + overlay-text mechanic. The
   .template-input is invisible (transparent text) and the .template-text
   layer renders syntax-highlighted HTML on top of it. */
.renamer-editor {
    position: relative;
    margin-top: 4px;
}
.template-input {
    text-align: left;
    background-color: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--color-border);
    padding-left: 12px;
    padding-right: 12px;
    padding-top: 20px;
    padding-bottom: 20px;
    outline: none !important;
    border-radius: var(--radius-sm);
    font-size: 16px;
    width: 100%;
    box-sizing: border-box;
    color: rgba(255, 255, 255, 0);
    transition: border-color var(--duration-fast) var(--ease-standard),
                box-shadow var(--duration-fast) var(--ease-standard);
}
.template-input:focus {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-glow);
}
.template-input-placeholder {
    color: var(--color-fg-subtle);
    border-style: none;
}
.template-text {
    position: absolute;
    pointer-events: none;
    top: 20px;
    left: 12px;
    right: 12px;
    font-size: 16px;
    text-align: left;
    z-index: 10;
    line-height: 1.4;
}
.template-text span { font-family: monospace !important; }

.fake-cursor {
    position: absolute;
    top: 20px;
    z-index: 20;
    height: 20px;
    width: 4px;
    margin-left: 12px;
    font-weight: bold;
    font-size: 20px;
    transition: margin-left 0.1s;
    animation-name: blink;
    animation-duration: 2s;
    animation-iteration-count: infinite;
}
@keyframes blink {
    0% { opacity: 0.25; }
    50% { opacity: 0.64; }
    100% { opacity: 0.25; }
}

.suggestions-box {
    background-color: var(--color-bg-overlay);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-2);
    max-width: 500px;
    width: 500px;
    font-size: 14px;
    text-align: left;
    padding: 8px;
    display: flex;
    position: absolute;
    z-index: 10;
    margin-top: 6px;
}
.suggestion-help-function { font-size: 13px; }
.help-suggestion-selected {
    background-color: rgba(0, 210, 191, 0.08);
    border-radius: var(--radius-xs);
}

/* Preview rows */
.renamer-preview { max-height: 360px; overflow-y: auto; }
.renamer-preview-row {
    font-size: 12px;
    color: var(--color-fg-muted);
    padding: 6px 8px;
    border-bottom: 1px dashed var(--color-border);
    word-break: break-all;
}
.renamer-preview-row:last-child { border-bottom: none; }
.renamer-preview-count {
    flex: 0 0 auto;
    align-self: center;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 3px 10px;
    border-radius: var(--radius-full, 9999px);
    border: 1px solid var(--color-accent);
    color: var(--color-accent);
    background: rgba(0, 210, 191, 0.08);
}

.renamer-options-toggles {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 360px;
}

/* Legacy helper kept for the keybind-icon used in older copy. */
.keybind-icon {
    padding: 3px 7px;
    border-radius: var(--radius-xs);
    background: var(--color-bg-overlay);
    border: 1px solid var(--color-border);
    margin-bottom: 4px;
    margin-left: 4px;
    font-family: var(--font-mono);
}
.text-accent { color: var(--color-accent); }
</style>
