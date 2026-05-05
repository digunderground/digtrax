<template>
<div class='dt-welcome' oncontextmenu="return false;">
    <!-- Ambient gradient background -->
    <div class='dt-welcome-bg'></div>

    <div class='dt-welcome-inner'>
        <!-- Brand lockup -->
        <div class='dt-welcome-brand'>
            <span class='dt-welcome-mark'>
                <span class='dig'>DIG</span><span class='trax'>TRAX</span>
            </span>
            <div class='dt-welcome-tagline'>The cross-platform music tagger for DJs</div>
            <div class='dt-welcome-version'>
                Version <span>{{$1t.info.value.version}}</span>
            </div>
        </div>

        <!-- Feature cards -->
        <div class='dt-welcome-grid'>
            <div class='dt-welcome-card' @click='help("autotagger")'>
                <div class='dt-welcome-card-icon'><q-icon name='mdi-tag-multiple' size='28px'></q-icon></div>
                <div class='dt-welcome-card-title'>// Auto Tag</div>
                <div class='dt-welcome-card-body'>
                    Automatically tag local audio files from Beatport, Traxsource, Juno, Discogs, MusicBrainz, and Spotify — based on artist + title, filename, or playlist. Or identify with Shazam.
                </div>
                <div class='dt-welcome-card-cta'>Read more →</div>
            </div>

            <div class='dt-welcome-card' @click='help("audiofeatures")'>
                <div class='dt-welcome-card-icon'><q-icon name='mdi-waveform' size='28px'></q-icon></div>
                <div class='dt-welcome-card-title'>// Audio Features</div>
                <div class='dt-welcome-card-body'>
                    Tag local audio with Spotify's audio features by ISRC tag or exact match. Free Spotify account required.
                </div>
                <div class='dt-welcome-card-cta'>Read more →</div>
            </div>

            <div class='dt-welcome-card dt-welcome-card-primary' @click='help("quicktag")'>
                <div class='dt-welcome-card-icon'><q-icon name='mdi-flash' size='28px'></q-icon></div>
                <div class='dt-welcome-card-title'>// Quick Tag</div>
                <div class='dt-welcome-card-body'>
                    Tag your audio quickly by energy, mood, genre, and custom tags. Keyboard-driven with bindings for fast review.
                </div>
                <div class='dt-welcome-card-cta'>Read more →</div>
            </div>

            <div class='dt-welcome-card' @click='help("renamer")'>
                <div class='dt-welcome-card-icon'><q-icon name='mdi-rename-box' size='28px'></q-icon></div>
                <div class='dt-welcome-card-title'>// Auto Rename</div>
                <div class='dt-welcome-card-body'>
                    Rename audio files based on tag values. Template-driven, with batch processing across folders.
                </div>
                <div class='dt-welcome-card-cta'>Read more →</div>
            </div>
        </div>

        <!-- Bottom row -->
        <div class='dt-welcome-foot'>
            <div class='dt-welcome-foot-item'>
                Need help? Click <strong>Read more</strong> on any card above.
            </div>
        </div>

        <div class='dt-welcome-credits'>
            Originally created by Marekkon5 · UI design by Bas Curtiz
        </div>
    </div>
</div>
</template>

<script lang='ts' setup>
import { get1t } from '../scripts/digtrax.js';

const $1t = get1t();
function help(route: string) {
    $1t.helpDialog.value.route = route;
    $1t.helpDialog.value.open = true;
}
</script>

<style lang="scss" scoped>
.dt-welcome {
    position: relative;
    height: 100vh;
    width: 100%;
    overflow-y: auto;
    background: var(--color-bg);
}

.dt-welcome-bg {
    position: fixed;
    inset: 0;
    pointer-events: none;
    z-index: 0;
    background:
        radial-gradient(ellipse 1200px 600px at 20% 0%, rgba(0, 210, 191, 0.08), transparent 60%),
        radial-gradient(ellipse 800px 600px at 100% 50%, rgba(167, 139, 250, 0.06), transparent 60%);
}

.dt-welcome-inner {
    position: relative;
    z-index: 1;
    max-width: 1100px;
    margin: 0 auto;
    padding: 64px 32px 80px;
}

/* Brand */
.dt-welcome-brand {
    text-align: center;
    margin-bottom: 56px;
}

.dt-welcome-mark {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 56px;
    letter-spacing: 0.02em;
    line-height: 1;
}

.dt-welcome-mark .dig {
    color: var(--color-fg);
}

.dt-welcome-mark .trax {
    color: var(--color-accent);
    text-shadow: 0 0 32px var(--color-accent-glow);
}

.dt-welcome-tagline {
    margin-top: 18px;
    color: var(--color-fg-muted);
    font-size: 15px;
}

.dt-welcome-version {
    margin-top: 12px;
    color: var(--color-fg-subtle);
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
}

.dt-welcome-version > span {
    color: var(--color-fg-muted);
}

.dt-welcome-sep {
    margin: 0 8px;
    color: var(--color-fg-subtle) !important;
}

.dt-welcome-link {
    color: var(--color-accent) !important;
    cursor: pointer;
    text-decoration: underline;
    text-decoration-color: rgba(0, 210, 191, 0.4);
    text-underline-offset: 3px;
}

.dt-welcome-link:hover {
    text-decoration-color: var(--color-accent);
}

/* Card grid */
.dt-welcome-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 14px;
    margin-bottom: 48px;
}

.dt-welcome-card {
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 24px 22px;
    cursor: pointer;
    transition: all var(--duration-base) var(--ease-standard);
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 200px;
}

.dt-welcome-card:hover {
    border-color: var(--color-border-strong);
    background: var(--color-bg-overlay);
    transform: translateY(-2px);
    box-shadow: var(--shadow-2);
}

.dt-welcome-card-primary {
    border-color: rgba(0, 210, 191, 0.3);
    background: linear-gradient(135deg, rgba(0, 210, 191, 0.06), var(--color-bg-elevated));
}

.dt-welcome-card-primary:hover {
    border-color: var(--color-accent);
    box-shadow: 0 0 24px var(--color-accent-glow);
}

.dt-welcome-card-icon {
    color: var(--color-accent);
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: rgba(0, 210, 191, 0.08);
    display: flex;
    align-items: center;
    justify-content: center;
}

.dt-welcome-card-title {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-fg);
}

.dt-welcome-card-body {
    flex: 1;
    color: var(--color-fg-muted);
    font-size: 13px;
    line-height: 1.55;
}

.dt-welcome-card-cta {
    color: var(--color-accent);
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 700;
}

/* Footer */
.dt-welcome-foot {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 16px;
    color: var(--color-fg-muted);
    font-size: 12px;
    text-align: center;
    margin-bottom: 20px;
}

.dt-welcome-foot-item {
    padding: 8px 16px;
}

.dt-welcome-credits {
    text-align: center;
    color: var(--color-fg-subtle);
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
}
</style>
