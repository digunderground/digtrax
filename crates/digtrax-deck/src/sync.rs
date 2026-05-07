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
//! - Constants (all Mixxx-exact, see `bpmcontrol.cpp::calcSyncAdjustment`):
//!   - error deadband 0.01 — below 1% phase error, NO adjustment
//!     (prevents jitter / oscillation when essentially in sync).
//!   - PI gain 0.7
//!   - absolute rate cap ±5%
//!   - per-buffer rate-of-change cap ±2% (delta from last adjustment;
//!     prevents lurching when error spikes).
//!   - train-wreck threshold 0.2: when this fires, we apply a steady
//!     +5% rate boost so the follower catches up over a few seconds.
//!     We do NOT teleport the follower — that was the source of the
//!     "skip forward erratically during breakdowns" bug, where every
//!     train-wreck snap left the follower at a slightly different
//!     wrong position (because beat-period × phase has rounding error
//!     when QM-DSP-derived periods are uneven).
//!   - `last_sync_adjustment` is persistent across buffers and used
//!     to clamp the per-buffer delta. Reset to 1.0 on sync-engage.
//! - Snap-on-engage IS still done (one shot, when sync flips off→on)
//!   to land the follower close to phase before the PI loop takes
//!   over. After that one snap, position is never written again.
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

/// Phase-error deadband. Below this, no rate adjustment at all
/// (Mixxx's `kErrorThreshold`). Prevents jitter when in sync.
const ERROR_THRESHOLD: f64 = 0.01;
/// PI gain (Mixxx's `kSyncAdjustmentProportional`).
const PI_GAIN: f64 = 0.7;
/// Absolute rate cap (Mixxx's `kSyncAdjustmentCap`). Adjustment
/// stays within `[1.0 - 0.05, 1.0 + 0.05]`.
const PI_ABS_CAP: f64 = 0.05;
/// Per-buffer rate-of-change cap (Mixxx's `kSyncDeltaCap`).
/// Limits how far one buffer's adjustment can move from the
/// previous buffer's, preventing lurches.
const PI_DELTA_CAP: f64 = 0.02;
/// Phase error above which we apply the catch-up boost. Mixxx's
/// `kTrainWreckThreshold` — we use the same value but, like Mixxx,
/// do NOT snap-seek; we just nudge the rate by the absolute cap so
/// the follower converges over a few seconds.
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
    /// Persistent last-applied adjustment (Mixxx's
    /// `m_dLastSyncAdjustment`). Used to clamp the per-buffer delta.
    /// Stored in "adjustment" form: 1.0 means no rate change.
    /// Per-deck because either deck can be the follower.
    last_adj_a: f64,
    last_adj_b: f64,
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
            last_adj_a: 1.0,
            last_adj_b: 1.0,
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

        // Reset persistent adjustment state on engage so the new
        // sync session starts from a clean 1.0 baseline (Mixxx's
        // `m_resetSyncAdjustment` flag).
        if sync_a_engaged { self.last_adj_a = 1.0; }
        if sync_b_engaged { self.last_adj_b = 1.0; }

        // Pick leader / follower references. We have to do this with
        // some careful borrow-splitting since both come from `&mut`s.
        let follower_id;
        let (leader_eng, follower_eng, follower_sync, follower_engaged) = match leader_code {
            1 => { follower_id = DeckId::B; (&*deck_a, deck_b, sync_b_now, sync_b_engaged) }
            2 => { follower_id = DeckId::A; (&*deck_b, deck_a, sync_a_now, sync_a_engaged) }
            _ => return,
        };
        if !follower_sync { return; }

        // Only run sync corrections when BOTH decks are actively
        // producing audio. If either is paused, the PI loop can yank
        // the follower's rate around against a frozen leader phase.
        if !leader_eng.is_playing() || !follower_eng.is_playing() { return; }

        let (Some(leader_phase), Some(follower_phase)) =
            (leader_eng.beat_distance(), follower_eng.beat_distance())
        else { return; };

        let leader_bpm = leader_eng.file_bpm() as f64;
        let follower_bpm = follower_eng.file_bpm() as f64;
        if leader_bpm <= 0.0 || follower_bpm <= 0.0 { return; }
        // Leader's user-driven rate (tempo slider). Follower's effective
        // tempo target = leader.file_bpm × leader.rate, so when the user
        // pushes the leader's tempo slider the follower tracks it.
        let leader_user_rate = leader_eng.current_rate() as f64;

        // Shortest circular distance ∈ (-0.5, 0.5]. err > 0 → leader is
        // ahead of follower → follower must speed up.
        let err = ((leader_phase - follower_phase + 1.5) % 1.0) - 0.5;

        // Snap-seek ONLY on engage to land near phase. NEVER on
        // train-wreck — see file-level comment for why.
        if follower_engaged {
            snap_follower(follower_eng, leader_phase);
            // After snap, write the base rate immediately; the next
            // buffer's err will be ~0 and the deadband holds it there.
            let base_rate = (leader_bpm * leader_user_rate) / follower_bpm;
            follower_eng.write_rate(base_rate as f32);
            // Also zero our persistent state — we're now aligned.
            match follower_id {
                DeckId::A => self.last_adj_a = 1.0,
                DeckId::B => self.last_adj_b = 1.0,
            }
            return;
        }

        // Mixxx-exact `calcSyncAdjustment`. Read previous adjustment.
        let last_adj = match follower_id {
            DeckId::A => self.last_adj_a,
            DeckId::B => self.last_adj_b,
        };

        let adjustment = if err.abs() > TRAIN_WRECK {
            // Way off — apply the absolute cap as a steady boost
            // toward the leader. NEVER snap. The follower converges
            // smoothly over a few seconds at 5% rate excess.
            // Sign: err > 0 means follower is BEHIND, so speed up.
            if err > 0.0 { 1.0 + PI_ABS_CAP } else { 1.0 - PI_ABS_CAP }
        } else if err.abs() > ERROR_THRESHOLD {
            // PI control with rate-of-change clamp. err > 0 → follower
            // behind → speed up, so adjust > 1.
            let target = 1.0 + (err * PI_GAIN);
            let delta = (target - last_adj).clamp(-PI_DELTA_CAP, PI_DELTA_CAP);
            // Final adjustment = last + delta, clamped to absolute cap.
            // Phrased as Mixxx phrases it: 1.0 + clamp((last - 1.0) + delta, ±cap).
            1.0 + ((last_adj - 1.0) + delta).clamp(-PI_ABS_CAP, PI_ABS_CAP)
        } else {
            // Deadband — already in sync, leave the rate alone.
            1.0
        };

        // Persist for next buffer.
        match follower_id {
            DeckId::A => self.last_adj_a = adjustment,
            DeckId::B => self.last_adj_b = adjustment,
        }

        // base_rate = effective leader BPM (file BPM × leader's user
        // rate) / follower file BPM. Bakes in the leader's tempo
        // slider so the follower tracks user pitch shifts on the
        // leader without saturating the cap.
        let base_rate = (leader_bpm * leader_user_rate) / follower_bpm;
        let new_rate = base_rate * adjustment;
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
