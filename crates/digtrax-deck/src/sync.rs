//! Closed-loop beat-distance sync engine.
//!
//! Architecture (Mixxx-style, see `research/mixxx/src/engine/sync/`):
//!
//! - One deck is the **leader** (called "master" in older Mixxx; we use
//!   "leader" internally and "MASTER" only in user-facing strings).
//! - Other decks with sync enabled are **followers**: every audio
//!   buffer (~10ms) we read the leader's beat-distance phase, compute
//!   the shortest circular distance to the follower's phase, and write
//!   a small rate correction to the follower's rate atomic.
//! - On the transition from sync-off to sync-on we **snap-seek** the
//!   follower's playhead to the nearest beat that aligns with the
//!   leader's current phase. Without this, the PI controller can take
//!   seconds to drag the follower across half a beat at the rate cap.
//! - Constants:
//!   - gain 0.7 (Mixxx default)
//!   - rate cap **±5%** (Mixxx uses 2%; we widen because our BPM
//!     detection is only ±0.5 BPM where Mixxx's is ±0.05). At 124 BPM
//!     a 1 BPM detection error is 0.8% — Mixxx's 2% cap leaves only
//!     1.2% headroom for phase corrections, which can saturate. 5%
//!     gives us 4.2% headroom even at the edge of detection error.
//!   - train-wreck threshold 25% of a beat (we snap-seek instead of
//!     trying to PI-correct).
//! - Runs in the cpal callback ([`SyncEngine::process`]), so the loop
//!   closes against **real audio position** — no frontend / WS round-
//!   trip in the hot path.
//!
//! Followers without a beat sequence yet, or with no leader BPM
//! available, get no correction (the loop bails). The user's manual
//! rate slider then takes over via the same rate atomic — there's only
//! ONE rate per deck, the sync engine and the slider race for it. When
//! sync is on the engine wins (writes ~100×/sec); when off the slider's
//! last value sticks.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use crate::engine::DeckEngine;
use crate::DeckId;

/// PI gain — Mixxx's exact `bpmcontrol.cpp::calcSyncedRate` value.
const PI_GAIN: f64 = 0.7;
/// Rate-correction cap — Mixxx's exact value, restored. Now valid
/// because we run QM-DSP for beat detection (sub-frame BPM precision)
/// and snap-seek on engage so the PI controller only handles steady-
/// state corrections within ±2%.
const PI_CAP: f64 = 0.02;
/// Beyond this absolute phase error, snap-seek instead of PI-correcting.
const TRAIN_WRECK: f64 = 0.2;

/// Audio-thread side. Owned by the cpal callback closure alongside
/// the two [`DeckEngine`]s.
pub(crate) struct SyncEngine {
    leader: Arc<AtomicU8>,
    sync_a: Arc<AtomicBool>,
    sync_b: Arc<AtomicBool>,
    /// Edge-detection state for "sync just engaged" → trigger snap.
    /// Audio-thread-only, doesn't need to be atomic.
    last_sync_a: bool,
    last_sync_b: bool,
}

/// `Send + Clone` controller. The WS layer holds one and updates
/// `leader` / `sync_*` from any thread.
#[derive(Clone)]
pub struct SyncHandle {
    leader: Arc<AtomicU8>,
    sync_a: Arc<AtomicBool>,
    sync_b: Arc<AtomicBool>,
}

impl SyncEngine {
    pub(crate) fn new() -> (SyncEngine, SyncHandle) {
        let leader = Arc::new(AtomicU8::new(0));
        let sync_a = Arc::new(AtomicBool::new(false));
        let sync_b = Arc::new(AtomicBool::new(false));
        let engine = SyncEngine {
            leader: leader.clone(),
            sync_a: sync_a.clone(),
            sync_b: sync_b.clone(),
            last_sync_a: false,
            last_sync_b: false,
        };
        let handle = SyncHandle { leader, sync_a, sync_b };
        (engine, handle)
    }

    /// Run one PI iteration. Called by the cpal callback every buffer,
    /// AFTER both decks have ticked but BEFORE either renders audio.
    /// Takes `&mut` to both decks because snap-seek (on engage / on
    /// train-wreck) writes the follower's position directly.
    pub(crate) fn process(&mut self, deck_a: &mut DeckEngine, deck_b: &mut DeckEngine) {
        let leader_code = self.leader.load(Ordering::Acquire);
        if leader_code == 0 {
            // Reset edges — re-engaging requires a real off→on transition.
            self.last_sync_a = false;
            self.last_sync_b = false;
            return;
        }
        let sync_a_now = self.sync_a.load(Ordering::Acquire);
        let sync_b_now = self.sync_b.load(Ordering::Acquire);

        // Edge detection — we only act on the rising edge of sync per deck.
        let sync_a_engaged = sync_a_now && !self.last_sync_a;
        let sync_b_engaged = sync_b_now && !self.last_sync_b;
        self.last_sync_a = sync_a_now;
        self.last_sync_b = sync_b_now;

        // Pick leader / follower references. We have to do this with
        // some careful borrow-splitting since both come from `&mut`s.
        let (leader_eng, follower_eng, follower_sync, follower_engaged) = match leader_code {
            1 => (&*deck_a, deck_b, sync_b_now, sync_b_engaged),
            2 => (&*deck_b, deck_a, sync_a_now, sync_a_engaged),
            _ => return,
        };
        if !follower_sync { return; }

        // Only run sync corrections when BOTH decks are actively
        // producing audio. If either is paused, the PI loop can yank
        // the follower's playhead around (each buffer the follower
        // drifts further from a frozen leader, eventually crossing
        // the train-wreck threshold and snap-seeking — which makes
        // the visible waveform jump erratically).
        if !leader_eng.is_playing() || !follower_eng.is_playing() { return; }

        let (Some(leader_phase), Some(follower_phase)) =
            (leader_eng.beat_distance(), follower_eng.beat_distance())
        else { return; };

        let leader_bpm = leader_eng.file_bpm() as f64;
        let follower_bpm = follower_eng.file_bpm() as f64;
        if leader_bpm <= 0.0 || follower_bpm <= 0.0 { return; }

        // Shortest circular distance ∈ (-0.5, 0.5]. err > 0 → leader is
        // ahead of follower → follower must speed up.
        let err = ((leader_phase - follower_phase + 1.5) % 1.0) - 0.5;

        // Snap conditions: (a) sync just engaged, OR (b) we're more
        // than 25% of a beat off and the PI cap can't realistically
        // catch up in reasonable time. In both cases write the
        // follower's playhead directly to the nearest aligned beat.
        if follower_engaged || err.abs() > TRAIN_WRECK {
            snap_follower(follower_eng, leader_phase);
        }

        // Steady-state PI correction.
        let correction = (PI_GAIN * err).clamp(-PI_CAP, PI_CAP);
        let base_rate = leader_bpm / follower_bpm;
        let new_rate = base_rate * (1.0 + correction);
        follower_eng.write_rate(new_rate as f32);
    }
}

/// Move the follower's playhead to the nearest beat aligned to
/// `leader_phase`. "Aligned" means: at the target frame, the follower's
/// own beat-distance fraction equals the leader's. The follower's
/// nearest beat boundary is the anchor; we add `phase × period` to
/// land mid-beat.
fn snap_follower(follower: &mut DeckEngine, leader_phase: f64) {
    let beats = follower.beats_frames();
    if beats.len() < 2 { return; }
    let pos = follower.position_frames();

    // Find the nearest beat to current position. Linear scan is fine —
    // beat counts are small (a few thousand at most), and this only
    // runs on engage / train-wreck (not every buffer).
    let mut nearest_idx = 0usize;
    let mut nearest_dist = f64::INFINITY;
    for (i, &b) in beats.iter().enumerate() {
        let d = (b as f64 - pos).abs();
        if d < nearest_dist {
            nearest_dist = d;
            nearest_idx = i;
        }
    }
    // Compute the period using the bracket beat we'll be sitting on.
    // For the last beat we don't have a "next", so fall back to the
    // previous interval.
    let (anchor, period) = if nearest_idx + 1 < beats.len() {
        let a = beats[nearest_idx] as f64;
        let p = beats[nearest_idx + 1] as f64 - a;
        (a, p)
    } else if nearest_idx > 0 {
        let p = beats[nearest_idx] as f64 - beats[nearest_idx - 1] as f64;
        (beats[nearest_idx] as f64, p)
    } else {
        return;
    };
    if period <= 0.0 { return; }

    let target = anchor + leader_phase * period;
    follower.write_position_frames(target);
}

impl SyncHandle {
    /// Pick the leader deck. `None` clears (no leader). Idempotent.
    pub fn set_leader(&self, leader: Option<DeckId>) {
        let v = match leader {
            None => 0,
            Some(DeckId::A) => 1,
            Some(DeckId::B) => 2,
        };
        self.leader.store(v, Ordering::Release);
    }

    /// Toggle sync for one deck. The deck only follows when (a) sync
    /// is on for it, AND (b) some OTHER deck is the leader. (You
    /// can't follow yourself.)
    pub fn set_sync(&self, deck: DeckId, on: bool) {
        match deck {
            DeckId::A => self.sync_a.store(on, Ordering::Release),
            DeckId::B => self.sync_b.store(on, Ordering::Release),
        }
    }

    /// Read current leader for the WS push.
    pub fn leader(&self) -> Option<DeckId> {
        match self.leader.load(Ordering::Acquire) {
            1 => Some(DeckId::A),
            2 => Some(DeckId::B),
            _ => None,
        }
    }

    /// Read whether a given deck has sync on.
    pub fn sync(&self, deck: DeckId) -> bool {
        match deck {
            DeckId::A => self.sync_a.load(Ordering::Acquire),
            DeckId::B => self.sync_b.load(Ordering::Acquire),
        }
    }
}
