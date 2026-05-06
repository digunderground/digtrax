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

                <q-menu
                    v-if='genre.subgenres'
                    :model-value='mouseOver == i'
                    class='no-shadow'
                    anchor='top middle'
                    self='bottom middle'
                >
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
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-top: 1px solid var(--color-border);
    background: var(--color-bg-elevated);
}

/* Centered when content fits; scrolls horizontally when content overflows.
   `justify-content: center` works inside an overflow:auto container so long
   as the inner content is wider than the container — the scroll adopts a
   left-aligned start, which is fine. When the genre list fits the viewport
   it stays centered. */
.dt-genres-scroll {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 0 16px;
    overflow-x: auto;
    overflow-y: hidden;
    height: 100%;
    max-width: 100%;
}

.dt-genres-scroll::-webkit-scrollbar:horizontal {
    height: 4px !important;
}

.dt-genre-wrap {
    flex-shrink: 0;
}

/* Pill-styled menu items: each genre reads as its own button rather than a
   row of plain labels. Subtle border so the bar still feels like chrome,
   not a hero CTA. */
.dt-genre {
    padding: 6px 14px;
    background: transparent;
    border: 1px solid var(--color-border);
    color: var(--color-fg-muted);
    font: inherit;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    border-radius: var(--radius-full, 9999px);
    transition: all var(--duration-fast) var(--ease-standard);
    white-space: nowrap;
    line-height: 1.2;
}

.dt-genre:hover,
.dt-genre.hover {
    color: var(--color-fg);
    background: rgba(255, 255, 255, 0.04);
    border-color: var(--color-border-strong);
}

.dt-genre.active {
    color: var(--color-accent);
    background: rgba(0, 210, 191, 0.08);
    border-color: var(--color-accent);
    box-shadow: 0 0 12px var(--color-accent-glow);
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
