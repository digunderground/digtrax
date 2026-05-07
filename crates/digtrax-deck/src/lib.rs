//! DigTrax dual-deck DJ engine.
//!
//! Sample-accurate, cpal-based, no Qt. The public surface is:
//!
//! - [`MasterMixer::new`] — opens a cpal output stream and spins up two
//!   [`DeckEngine`] instances. Returns a [`MixerHandle`] that callers
//!   (typically the WS layer in `digtrax-ui`) clone freely to drive
//!   playback, volume, crossfader, etc. from any thread.
//! - [`MixerHandle`] — `Send + Clone`. All control is via lock-free
//!   atomics or a non-blocking `crossbeam-channel`. Never blocks the
//!   audio thread.
//! - [`DeckId`] — identifies which deck a handle method targets.
//!
//! Architecture (see `plan/ok-lets-take-a-hazy-church.md` for full design):
//!
//! ```text
//! cpal callback (RT thread)
//!   └─ MasterMixer::process(out_buffer)
//!        ├─ DeckEngine[A].process(deck_buf_a)  ─┐
//!        │    decode-from-memory + resample      │
//!        │    Phase 2: + rubato time-stretch     ├─→ sum × crossfade × master gain
//!        │    Phase 4: + EQ + filter             │
//!        ├─ DeckEngine[B].process(deck_buf_b)  ─┘
//!        └─ write `out_buffer`
//! ```
//!
//! Phase 1 ships:
//! - cpal output stream (default device, default config)
//! - Pre-decoded-to-memory playback (no streaming yet)
//! - Per-deck linear playback at native sample rate, with rubato as a
//!   sample-rate converter so 44.1 kHz sources play correctly into a
//!   48 kHz output stream.
//! - Per-deck volume and equal-power crossfader.
//! - Position reporting via atomic (audio thread writes; readers convert).
//!
//! Phase 2+ extends the in-callback chain (rubato rate change, beats,
//! sync, EQ) without changing this public API.

mod beats;
mod decode;
mod engine;
mod eq;
mod filter;
mod master;
mod sync;

pub use beats::{analyze, BeatAnalysis};
pub use decode::{decode_file, DecodedAudio};
pub use engine::{DeckHandle, DeckSnapshot, EqBand};
pub use master::{MasterMixer, MixerHandle};
pub use sync::SyncHandle;

/// Which deck a [`MixerHandle`] method targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckId {
    A,
    B,
}

impl DeckId {
    /// All deck ids — useful for iterating both decks in WS push loops.
    pub const ALL: [DeckId; 2] = [DeckId::A, DeckId::B];

    /// Stable lowercase string for WS protocol serialisation.
    pub fn as_str(self) -> &'static str {
        match self {
            DeckId::A => "a",
            DeckId::B => "b",
        }
    }
}
