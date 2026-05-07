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
//! - Constants come straight from `bpmcontrol.cpp::calcSyncedRate`:
//!   gain 0.7, ±2% rate cap, train-wreck threshold 20% of a beat.
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

/// Mixxx's `bpmcontrol.cpp::calcSyncedRate` constants.
const PI_GAIN: f64 = 0.7;
const PI_CAP: f64 = 0.02;
const TRAIN_WRECK: f64 = 0.2;

/// Audio-thread side. Owned by the cpal callback closure alongside
/// the two [`DeckEngine`]s.
pub(crate) struct SyncEngine {
    leader: Arc<AtomicU8>,
    sync_a: Arc<AtomicBool>,
    sync_b: Arc<AtomicBool>,
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
    /// Construct a paired engine + handle. Engine moves into the
    /// audio-thread callback; handle goes to the WS layer.
    pub(crate) fn new() -> (SyncEngine, SyncHandle) {
        let leader = Arc::new(AtomicU8::new(0));
        let sync_a = Arc::new(AtomicBool::new(false));
        let sync_b = Arc::new(AtomicBool::new(false));
        let engine = SyncEngine {
            leader: leader.clone(),
            sync_a: sync_a.clone(),
            sync_b: sync_b.clone(),
        };
        let handle = SyncHandle { leader, sync_a, sync_b };
        (engine, handle)
    }

    /// Run one PI iteration. Called by the cpal callback every buffer,
    /// AFTER both decks have ticked but BEFORE either renders audio.
    pub(crate) fn process(&self, deck_a: &DeckEngine, deck_b: &DeckEngine) {
        let leader_code = self.leader.load(Ordering::Acquire);
        if leader_code == 0 { return; }
        let (leader_eng, follower_eng, follower_sync) = match leader_code {
            1 => (deck_a, deck_b, &self.sync_b),
            2 => (deck_b, deck_a, &self.sync_a),
            _ => return,
        };
        if !follower_sync.load(Ordering::Acquire) { return; }

        let (Some(leader_phase), Some(follower_phase)) =
            (leader_eng.beat_distance(), follower_eng.beat_distance())
        else { return; };

        let leader_bpm = leader_eng.file_bpm() as f64;
        let follower_bpm = follower_eng.file_bpm() as f64;
        if leader_bpm <= 0.0 || follower_bpm <= 0.0 { return; }

        // Shortest circular distance ∈ (-0.5, 0.5]. err > 0 → leader is
        // ahead of follower → follower must speed up.
        let err = ((leader_phase - follower_phase + 1.5) % 1.0) - 0.5;

        // Train-wreck path: |err| > 20% of a beat. Pegging the rate
        // correction at ±2% can't catch up; ideally we'd snap-seek the
        // follower here. v1 just lets the rate cap pull as hard as it
        // can — over a few seconds the loop converges. Real snap-seek
        // is a v2 follow-up (needs frame-aligned cmd_tx into the deck).
        let _ = TRAIN_WRECK;

        let correction = (PI_GAIN * err).clamp(-PI_CAP, PI_CAP);
        let base_rate = leader_bpm / follower_bpm;
        let new_rate = base_rate * (1.0 + correction);
        follower_eng.write_rate(new_rate as f32);
    }
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
