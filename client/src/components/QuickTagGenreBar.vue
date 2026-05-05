<template>
<div class='dt-genres' @mouseleave="onMouseLeave">
    <div class='dt-genres-scroll'>
        <div
            v-for='(genre, i) in $1t.settings.value.quickTag.genres'
            :key='"genre"+i'
            class='dt-genre-wrap'
        >
            <button
                @mouseenter="mouseOver = i"
                class='dt-genre'
                :class='{ active: isSelected(genre.genre), hover: mouseOver == i }'
                @click='setGenre(genre.genre)'
            >
                {{ genre.genre }}

                <q-menu v-if='genre.subgenres' :model-value='mouseOver == i' class='no-shadow'>
                    <q-list @mouseleave="mouseOver = -1" class='dt-subgenre-list'>
                        <q-item
                            v-for='(subgenre, j) in genre.subgenres'
                            :key='"sg"+j'
                            clickable
                            @click='setGenre(subgenre)'
                            class='dt-subgenre-item'
                        >
                            <q-icon name='mdi-check' size='14px' class='dt-subgenre-check' v-if='isSelected(subgenre)'></q-icon>
                            <span class='dt-subgenre-label'>{{ subgenre }}</span>
                        </q-item>
                    </q-list>
                </q-menu>
            </button>
        </div>
    </div>
</div>
</template>

<script lang='ts' setup>
import { ref } from 'vue';
import { get1t } from '../scripts/digtrax.js';

const $1t = get1t();
const mouseOver = ref(-1);

function isSelected(genre: string) {
    return $1t.quickTag.value.track.genres.includes(genre);
}

function setGenre(genre: string) {
    $1t.quickTag.value.track.toggleGenre(genre);
}

function onMouseLeave() {
    if (mouseOver.value == -1) return;
    if ($1t.settings.value.quickTag.genres[mouseOver.value].subgenres.length > 0) return;
    mouseOver.value = -1;
}
</script>

<style lang="scss" scoped>
.dt-genres {
    height: 40px;
    display: flex;
    align-items: center;
    border-top: 1px solid var(--color-border);
}

.dt-genres-scroll {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 16px;
    overflow-x: auto;
    overflow-y: hidden;
    width: 100%;
    height: 100%;
}

.dt-genres-scroll::-webkit-scrollbar:horizontal {
    height: 4px !important;
}

.dt-genre-wrap {
    flex-shrink: 0;
}

.dt-genre {
    padding: 5px 12px;
    background: transparent;
    border: none;
    color: var(--color-fg-subtle);
    font: inherit;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    border-radius: var(--radius-xs);
    transition: all var(--duration-fast) var(--ease-standard);
    white-space: nowrap;
}

.dt-genre:hover,
.dt-genre.hover {
    color: var(--color-fg);
    background: rgba(255, 255, 255, 0.04);
}

.dt-genre.active {
    color: var(--color-accent);
    background: rgba(0, 210, 191, 0.08);
    text-shadow: 0 0 6px var(--color-accent-glow);
    font-weight: 700;
}

/* Subgenre menu */
.dt-subgenre-list {
    background: var(--color-bg-overlay) !important;
    border: 1px solid var(--color-border) !important;
    border-radius: var(--radius-md);
    padding: 4px;
    min-width: 180px;
}

.dt-subgenre-item {
    border-radius: var(--radius-xs);
    padding: 6px 10px !important;
    min-height: 0 !important;
    color: var(--color-fg-muted);
    font-size: 12px;
}

.dt-subgenre-item:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--color-fg);
}

.dt-subgenre-check {
    color: var(--color-accent);
    margin-right: 6px;
}

.dt-subgenre-label {
    text-transform: capitalize;
}
</style>
