// DJ Mixer store — reactive state for the dual-deck DJ engine on
// `feature/dj-mixer`. This is the v1 ground-up rewrite that lives next
// to (not on top of) the legacy single-deck preview at `$1t.player`.
//
// Architecture (see `plan/ok-lets-take-a-hazy-church.md`):
//
// - The Rust audio engine lives in `crates/digtrax-deck/`. cpal owns
//   the audio callback; sample-accurate playback / mixing / sync all
//   happen there.
// - Backend → frontend is one-way push: `djLoadStart`, `djLoaded`,
//   `djPosition` (Phase 1g — coming next), all routed by
//   `digtrax.ts::onDjEvent`.
// - Frontend → backend is the `Dj*` action family — see Action::Dj* in
//   `crates/digtrax-ui/src/socket.rs`.

import { reactive } from 'vue';
import { get1t } from './digtrax';

export type DeckId = 'a' | 'b';

export interface DeckState {
    id: DeckId;
    /// Loaded file path (undefined = empty deck — drop target shows).
    path?: string;
    title?: string;
    artists: string[];
    /// Track length in ms, set by backend after decode finishes.
    duration: number;
    /// Current playhead in ms. Backend pushes ~30 Hz from the audio
    /// thread; the local ticker doesn't need to estimate.
    position: number;
    /// True while audio is being produced (backend confirms via push).
    playing: boolean;
    /// True while decode is in flight. djLoaded clears it.
    loading: boolean;
    /// True while beat analysis is running. djAnalyzed clears it.
    analyzing: boolean;
    /// 0..1 fader position. Sent to backend via `djVolume`.
    userVolume: number;
    /// Playback rate multiplier (0.5..2.0, 1.0 = native). Pitch-couples
    /// to speed for now (vinyl-style); Phase 2c may add pitch
    /// preservation. Phase 3's sync controller writes here too.
    rate: number;
    /// Detected BPM (Phase 2a — kick tracker). 0 = analysis hasn't
    /// completed yet or failed.
    bpm: number;
    /// Position of beat 1 in ms (file-time). 0 = unknown / pre-analysis.
    firstBeat: number;
    /// 0..1 confidence score. Low = the kick tracker found a weak /
    /// inconsistent pulse and the user shouldn't trust the BPM display.
    bpmConfidence: number;
    /// Per-bar [low, mid, high] spectrum amplitude triplets (the deck's
    /// scrolling RGB waveform draws these). Sampled at `barsPerSec`
    /// per second of audio.
    spectrum: number[][];
    /// Spectrum density (bars per second of audio).
    spectrumBarsPerSec: number;
    /// Every detected beat in ms (file-time). Length ≈ duration*bpm/60.
    /// Drives the yellow beat-grid markers on the deck waveform.
    beats: number[];
    /// True if this deck is currently the sync LEADER. The follower's
    /// rate is corrected to match this deck's beat phase. Backend
    /// confirms via 33 Hz `djPosition.isLeader`.
    isLeader: boolean;
    /// True if SYNC is engaged on this deck. Means: "if a different
    /// deck is the leader and this deck has analyzed beats, follow it."
    syncOn: boolean;
    /// 3-band EQ. Each ∈ [0, 2], 1 = unity, 0 = kill, 2 = +6 dB.
    eqLow: number;
    eqMid: number;
    eqHigh: number;
    /// DJ filter knob, [-1, 1]. 0 = bypass, -1 = full LPF, +1 = full HPF.
    filter: number;
}

function emptyDeck(id: DeckId): DeckState {
    return {
        id,
        path: undefined,
        title: undefined,
        artists: [],
        duration: 0,
        position: 0,
        playing: false,
        loading: false,
        analyzing: false,
        userVolume: 0.7,
        rate: 1.0,
        bpm: 0,
        firstBeat: 0,
        bpmConfidence: 0,
        spectrum: [],
        spectrumBarsPerSec: 10,
        beats: [],
        isLeader: false,
        syncOn: false,
        eqLow: 1.0,
        eqMid: 1.0,
        eqHigh: 1.0,
        filter: 0.0,
    };
}

export const djState = reactive({
    deckA: emptyDeck('a'),
    deckB: emptyDeck('b'),
    /// Crossfader in [-1, 1]. -1 = full A, +1 = full B, 0 = center.
    crossfader: 0,
    /// Master output gain in [0, 2]. 1.0 = unity.
    masterGain: 1.0,
});

function refOf(id: DeckId): DeckState {
    return id === 'a' ? djState.deckA : djState.deckB;
}

// ─── Outbound (frontend → backend) ────────────────────────────────────

/**
 * Drop a track on a deck. Triggers backend decode + load. The deck stays
 * in `loading: true` state until `djLoaded` arrives.
 */
export function loadDeck(id: DeckId, path: string, title?: string, artists?: string[]) {
    const $1t = get1t();
    const slot = refOf(id);
    // Reset deck state so the UI immediately reflects "new track loading".
    Object.assign(slot, emptyDeck(id), {
        path,
        title,
        artists: artists ?? [],
        loading: true,
        userVolume: slot.userVolume, // preserve fader position across loads
    });
    $1t.send('djLoad', { deck: id, path });
}

export function playDeck(id: DeckId) {
    const slot = refOf(id);
    if (!slot.path) return;
    // Optimistic — the backend will confirm via djPosition pushes.
    slot.playing = true;
    get1t().send('djPlay', { deck: id });
}

export function pauseDeck(id: DeckId) {
    const slot = refOf(id);
    slot.playing = false;
    get1t().send('djPause', { deck: id });
}

export function stopDeck(id: DeckId) {
    const slot = refOf(id);
    slot.playing = false;
    slot.position = 0;
    get1t().send('djStop', { deck: id });
}

export function seekDeck(id: DeckId, posMs: number) {
    const slot = refOf(id);
    slot.position = Math.max(0, Math.min(slot.duration, posMs));
    get1t().send('djSeek', { deck: id, pos: Math.round(slot.position) });
}

export function setDeckVolume(id: DeckId, value: number) {
    const slot = refOf(id);
    const v = Math.max(0, Math.min(1, value));
    slot.userVolume = v;
    get1t().send('djVolume', { deck: id, value: v });
}

/// Set deck playback rate. 1.0 = native, 1.5 = 1.5× speed (and 1.5×
/// higher pitch — pitch coupling acknowledged). Clamped to [0.5, 2.0].
export function setDeckRate(id: DeckId, value: number) {
    const slot = refOf(id);
    const v = Math.max(0.5, Math.min(2.0, value));
    slot.rate = v;
    get1t().send('djRate', { deck: id, value: v });
}

export function setCrossfader(value: number) {
    const v = Math.max(-1, Math.min(1, value));
    djState.crossfader = v;
    get1t().send('djCrossfader', { value: v });
}

export function setMasterGain(value: number) {
    const v = Math.max(0, Math.min(2, value));
    djState.masterGain = v;
    get1t().send('djMasterGain', { value: v });
}

/// Pick the sync leader. `null` clears (no leader). The follower
/// deck(s) need `setSync(id, true)` separately.
export function setLeader(deck: DeckId | null) {
    get1t().send('djSetLeader', { deck });
}

/// Toggle sync on a deck. Backend will only correct rate when (a) sync
/// is on, AND (b) some OTHER deck is the leader, AND (c) both have
/// analyzed beats. The 33Hz djPosition push reports the actual state.
export function setSync(deck: DeckId, on: boolean) {
    get1t().send('djSync', { deck, on });
}

export type EqBand = 'low' | 'mid' | 'high';

/// Set one EQ band per deck. `value` ∈ [0, 2] where 1 = unity, 0 = kill,
/// 2 = +6 dB. Optimistic state update so the knob feels responsive.
export function setEq(deck: DeckId, band: EqBand, value: number) {
    const v = Math.max(0, Math.min(2, value));
    const slot = refOf(deck);
    if (band === 'low') slot.eqLow = v;
    else if (band === 'mid') slot.eqMid = v;
    else slot.eqHigh = v;
    get1t().send('djEq', { deck, band, value: v });
}

/// Set the DJ filter knob. `value` ∈ [-1, 1]. 0 = bypass, -1 = full
/// LPF, +1 = full HPF.
export function setFilter(deck: DeckId, value: number) {
    const v = Math.max(-1, Math.min(1, value));
    refOf(deck).filter = v;
    get1t().send('djFilter', { deck, value: v });
}

/// Beat jump. Positive = forward, negative = backward. No-op until the
/// deck's beat analysis has finished. The audio thread quantizes from
/// the current bracket beat.
export function beatJump(deck: DeckId, beats: number) {
    get1t().send('djBeatJump', { deck, beats });
}

// ─── Inbound (backend → frontend) ─────────────────────────────────────

/**
 * Wire WebSocket events from the audio engine into reactive deck state.
 * Called from `digtrax.ts` for every action whose name starts with `dj`.
 */
export function onDjEvent(json: any) {
    if (!json || typeof json.action !== 'string') return;
    const id: DeckId | undefined = json.deck;
    switch (json.action) {
        case 'djLoadStart': {
            // Title/artists arrive before decode finishes — show them
            // immediately so the deck slot looks loaded even while the
            // file is still being parsed.
            if (!id) return;
            const slot = refOf(id);
            slot.title = json.title ?? slot.title;
            slot.artists = json.artists ?? slot.artists;
            slot.loading = true;
            return;
        }
        case 'djLoaded': {
            if (!id) return;
            const slot = refOf(id);
            slot.duration = Number(json.duration) || 0;
            slot.loading = false;
            // Beat analysis runs immediately after Load on the backend;
            // mark this deck as analyzing so the UI can show a spinner.
            slot.analyzing = true;
            return;
        }
        case 'djAnalyzed': {
            if (!id) return;
            const slot = refOf(id);
            slot.bpm = Number(json.bpm) || 0;
            slot.firstBeat = Number(json.firstBeatMs) || 0;
            slot.bpmConfidence = Number(json.confidence) || 0;
            slot.spectrumBarsPerSec = Number(json.barsPerSecond) || 10;
            slot.spectrum = Array.isArray(json.spectrumBars)
                ? json.spectrumBars.map((b: any) =>
                    Array.isArray(b) ? [Number(b[0])||0, Number(b[1])||0, Number(b[2])||0] : [0, 0, 0])
                : [];
            slot.beats = Array.isArray(json.beatsMs)
                ? json.beatsMs.map((b: any) => Number(b) || 0)
                : [];
            slot.analyzing = false;
            return;
        }
        case 'djPosition': {
            if (!id) return;
            const slot = refOf(id);
            const pos = Number(json.pos);
            if (Number.isFinite(pos)) slot.position = pos;
            if (typeof json.playing === 'boolean') slot.playing = json.playing;
            const rate = Number(json.rate);
            if (Number.isFinite(rate) && rate > 0) slot.rate = rate;
            if (typeof json.isLeader === 'boolean') slot.isLeader = json.isLeader;
            if (typeof json.syncOn === 'boolean') slot.syncOn = json.syncOn;
            return;
        }
    }
}
