use std::collections::HashMap;
use anyhow::Error;
use axum::extract::ws::{WebSocket, Message};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use digtrax_renamer::ac::Autocomplete;
use digtrax_renamer::docs::FullDocs;
use digtrax_renamer::{Renamer, TemplateParser, RenamerConfig};
use serde_json::{Value, json};
use serde::{Serialize, Deserialize};
use dunce::canonicalize;
use digtrax_tag::{TagChanges, TagSeparators, Tag, Field, TagFilterQuery, BatchReport, TagChange, AudioFileFormat};

/// Global cancel flag for in-progress Tag Editor bulk saves. Set by
/// `TagEditorCancel`; checked between files inside the save loop. Reset at the
/// start of every save so a previous cancel doesn't leak into the next batch.
static TAG_EDITOR_CANCEL: AtomicBool = AtomicBool::new(false);

/// Per-format frame name mapping. Mirrors the frontend `FrameName` shape so
/// the migration handler can rename across MP3/AIFF/WAV (id3), FLAC/OGG
/// (vorbis), and M4A/MP4 in one round trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameMapping {
    id3: String,
    vorbis: String,
    mp4: String,
}
use digtrax_tagger::{TaggerConfig, AudioFileInfo, TrackMatch};
use digtrax_autotag::{Tagger, AudioFileInfoImpl, TaggerConfigExt, AUTOTAGGER_PLATFORMS};
use digtrax_autotag::audiofeatures::{AudioFeaturesConfig, AudioFeatures};
use digtrax_platforms::spotify::Spotify;
use digtrax_player::{AudioSources, AudioPlayer};
use digtrax_deck::{DeckId, EqBand, MasterMixer};
use digtrax_shared::{Settings, COMMIT};
use digtrax_playlist::{UIPlaylist, PLAYLIST_EXTENSIONS, get_files_from_playlist_file};

use crate::StartContext;
use crate::quicktag::{QuickTag, QuickTagFile, QuickTagData};
use crate::tageditor::TagEditor;
use crate::browser::{FileBrowser, FolderBrowser};


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Action {
    Init,
    Exit,
    SaveSettings { settings: Value },
    LoadSettings,
    DefaultCustomPlatformSettings,
    Browse { path: Option<String>, context: Option<String> },
    Browser { url: String },
    OpenSettingsFolder,
    OpenFolder { path: PathBuf },
    OpenFile { path: PathBuf },
    DeleteFiles { paths: Vec<String> },
    GetLog,
    GeneratePlaylist { paths: Vec<String> },

    LoadPlatforms,
    StartTagging { config: TaggerConfigs, playlist: Option<UIPlaylist> },
    StopTagging,
    ConfigCallback { config: Value, platform: String, id: String },
    RepoManifest,
    #[serde(rename_all = "camelCase")]
    InstallPlatform { id: String, version: String, is_native: bool },

    Waveform { path: PathBuf },
    PlayerLoad { path: PathBuf },
    PlayerPlay,
    PlayerPause,
    PlayerSeek { pos: u64 },
    PlayerVolume { volume: f32 },
    PlayerStop,

    // ─── DJ mixer (feature/dj-mixer) ─────────────────────────────────
    // Dual-deck DJ engine, parallel to the single-deck Player above.
    // The MasterMixer is constructed lazily on first DjLoad so users
    // who never enter DJ Mode never spin up a cpal output stream.
    DjLoad { deck: WireDeckId, path: PathBuf },
    DjPlay { deck: WireDeckId },
    DjPause { deck: WireDeckId },
    DjStop { deck: WireDeckId },
    DjSeek { deck: WireDeckId, pos: u64 },
    DjVolume { deck: WireDeckId, value: f32 },
    /// Manual rate multiplier per deck. 1.0 = native; clamped [0.5, 2.0].
    /// Phase 3's sync controller writes here too — same atomic on the
    /// audio thread.
    DjRate { deck: WireDeckId, value: f32 },
    DjUnload { deck: WireDeckId },
    DjCrossfader { value: f32 },
    DjMasterGain { value: f32 },

    /// Pick the sync leader. `None` clears it. The follower decks need
    /// `DjSync` enabled to actually follow.
    DjSetLeader { deck: Option<WireDeckId> },
    /// Toggle sync on a deck. When on AND a different deck is the
    /// leader, the audio thread's PI controller corrects this deck's
    /// rate every buffer to lock to the leader's beat phase.
    DjSync { deck: WireDeckId, on: bool },
    /// 3-band EQ per deck. `band` ∈ "low"|"mid"|"high"; `value` ∈
    /// [0, 2] (1 = unity, 0 = kill, 2 = +6 dB).
    #[serde(rename_all = "camelCase")]
    DjEq { deck: WireDeckId, band: WireEqBand, value: f32 },
    /// DJ-style filter knob. `value` ∈ [-1, 1]; 0 = bypass, -1 = full
    /// LPF, +1 = full HPF.
    DjFilter { deck: WireDeckId, value: f32 },
    /// Quantized seek: jump `beats` (positive = forward, negative =
    /// backward) relative to the current bracket beat.
    DjBeatJump { deck: WireDeckId, beats: i32 },

    QuickTagLoad { path: Option<String>, playlist: Option<UIPlaylist>, recursive: Option<bool>, separators: TagSeparators, limit: Option<bool> },
    QuickTagSave { path: PathBuf, changes: TagChanges },
    QuickTagFolder { path: Option<String>, subdir: Option<String> },

    #[serde(rename_all = "camelCase")]
    SpotifyAuthorize { client_id: String, client_secret: String },
    SpotifyAuthorized,

    TagEditorFolder { path: Option<String>, subdir: Option<String>, recursive: Option<bool>  },
    TagEditorLoad { paths: Vec<PathBuf> },
    TagEditorSave { paths: Vec<PathBuf>, changes: TagChanges },
    TagEditorFilter { folder: PathBuf, recursive: bool, query: TagFilterQuery },
    TagEditorCancel,
    // Walk `folder` (optionally recursively) and rename a frame across every
    // audio file. `from` and `to` carry per-format frame keys (id3/vorbis/mp4)
    // so a single user action migrates all formats. Streams the same
    // `tagEditorBulkProgress` events as TagEditorSave so the UI can reuse the
    // bulk drawer + cancel button.
    MigrateTagFolder { folder: PathBuf, recursive: bool, from: FrameMapping, to: FrameMapping },
    // Walk `folder` and rewrite a single value within a frame across every
    // audio file. For example: rename "06 - Cool Down" → "07 - Cool Down" inside
    // the configured frame for the user's "Situation" custom. Per-file, looks
    // up the format-specific frame key, splits the existing values, replaces
    // any entry equal to `old_value` with `new_value`, writes back. Streams
    // progress; only files where a replacement actually fires count as modified.
    #[serde(rename_all = "camelCase")]
    MigrateValueFolder {
        folder: PathBuf,
        recursive: bool,
        frame: FrameMapping,
        old_value: String,
        new_value: String,
    },

    RenamerSyntaxHighlight { template: String },
    RenamerAutocomplete { template: String },
    RenamerPreview { config: RenamerConfig },
    RenamerStart { config: RenamerConfig },

    FolderBrowser { path: PathBuf, child: String, base: bool },

    ManualTag { config: TaggerConfig, path: PathBuf },
    ManualTagApply { matches: Vec<TrackMatch>, path: PathBuf, config: TaggerConfig },

    // Detect legacy OneTagger settings (one-time first-launch migration prompt).
    // Looks for a settings.json under the OneTagger ProjectDirs path; returns the parsed
    // contents so the UI can offer a one-click import without a second round-trip.
    DetectLegacySettings,
}

/// Wire-format deck id. Lower-case strings ("a"/"b") so the front-end
/// JS can use plain string literals. Mapped to the engine's [`DeckId`]
/// when dispatching to the mixer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum WireDeckId {
    A,
    B,
}

impl From<WireDeckId> for DeckId {
    fn from(w: WireDeckId) -> Self {
        match w {
            WireDeckId::A => DeckId::A,
            WireDeckId::B => DeckId::B,
        }
    }
}

/// Wire-format EQ band selector.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum WireEqBand { Low, Mid, High }

impl From<WireEqBand> for EqBand {
    fn from(w: WireEqBand) -> Self {
        match w {
            WireEqBand::Low => EqBand::Low,
            WireEqBand::Mid => EqBand::Mid,
            WireEqBand::High => EqBand::High,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum TaggerConfigs {
    AutoTagger(TaggerConfig), 
    AudioFeatures(AudioFeaturesConfig)
}

impl TaggerConfigs {
    // Print to log for later easier debug
    pub fn debug_print(&self) {
        match self {
            TaggerConfigs::AutoTagger(c) => {
                let mut c = c.clone();
                // don't leak secrets
                c.custom = HashMap::new().into();
                c.spotify = None;
                info!("AutoTagger config: {:?}", c);
            },
            TaggerConfigs::AudioFeatures(c) => {
                info!("AudioFeatures Config: {:?}", c);
            }
        }
    }
}

// Shared variables in socket
struct SocketContext {
    player: AudioPlayer,
    /// DJ Mode mixer. Lazily constructed on first DjLoad — opening a cpal
    /// output stream is heavyweight (claims the audio device) and pointless
    /// for users who never enter DJ Mode. Once constructed it stays for the
    /// life of the WS connection.
    mixer: Option<MasterMixer>,
    spotify: Option<Spotify>,
    start_context: StartContext
}

impl SocketContext {
    pub fn new(start_context: StartContext) -> SocketContext {
        SocketContext {
            player: AudioPlayer::new(),
            mixer: None,
            spotify: None,
            start_context
        }
    }

    /// Get-or-construct the mixer. First call opens a cpal output stream;
    /// subsequent calls reuse the same one. Returns the error from
    /// `MasterMixer::new` (typically "no default output device") if the
    /// system audio is misconfigured — surfaced to the UI as a WS error.
    fn mixer(&mut self) -> Result<&MasterMixer, Error> {
        if self.mixer.is_none() {
            self.mixer = Some(MasterMixer::new()?);
        }
        Ok(self.mixer.as_ref().expect("mixer just initialized"))
    }
}


/// Reply to init call
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitData {
    action: &'static str,
    version: &'static str,
    os: &'static str,
    arch: &'static str,
    custom_platform_compat: i32,
    start_context: StartContext,
    renamer_docs: FullDocs,
    commit: &'static str,
    work_dir: PathBuf,
    data_dir: PathBuf
}

impl InitData {
    /// Create new default instance
    pub fn new(start_context: StartContext) -> InitData {
        InitData {
            action: "init",
            version: digtrax_shared::VERSION,
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            custom_platform_compat: digtrax_tagger::custom::CUSTOM_PLATFORM_COMPATIBILITY,
            start_context,
            renamer_docs: FullDocs::get().html(),
            commit: COMMIT,
            work_dir: std::env::current_dir().unwrap_or_default(),
            data_dir: Settings::get_folder().unwrap_or_default(),
        }
    }
}

pub(crate) async fn handle_ws_connection(mut websocket: WebSocket, context: StartContext) -> Result<(), Error> {
    let mut context = SocketContext::new(context);

    // 30 Hz position pusher for the DJ mixer. Closed-loop sync needs
    // sub-50ms freshness on the playhead; without it the front-end
    // can only correct against a phantom and audio drifts even though
    // visualisation looks locked. `Skip` missed-tick behaviour avoids
    // a flood after a slow handler (analyze, big save) finishes.
    let mut position_tick = tokio::time::interval(std::time::Duration::from_millis(33));
    position_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased;
            // Inbound user actions get priority — a delayed position
            // push is recoverable, a delayed user click is not.
            msg = websocket.recv() => {
                match msg {
                    Some(Ok(msg)) => {
                        match msg.to_text() {
                            Ok(text) => {
                                match handle_message(text, &mut websocket, &mut context).await {
                                    Ok(_) => {},
                                    Err(err) => {
                                        error!("Websocket: {:?}, Data: {}", err, text);
                                        send_socket(&mut websocket, json!({
                                            "action": "error",
                                            "message": &format!("{}", err)
                                        })).await.ok();
                                    }
                                }
                            },
                            Err(e) => warn!("WebSocket Message is not text: {e}"),
                        }
                    },
                    Some(Err(e)) => warn!("WebSocket error: {e}"),
                    None => break, // closed
                }
            }

            _ = position_tick.tick() => {
                // Push playhead snapshots whenever the mixer is alive
                // (lazy-init only happens on first DjLoad). No-op for
                // users who never enter DJ Mode.
                if let Some(mixer) = &context.mixer {
                    let leader = mixer.handle().sync().leader();
                    for id in DeckId::ALL {
                        let snap = mixer.handle().deck(id).snapshot();
                        if !snap.loaded { continue; }
                        let is_leader = leader == Some(id);
                        let sync_on = mixer.handle().sync().sync(id);
                        let _ = send_socket(&mut websocket, json!({
                            "action": "djPosition",
                            "deck": id.as_str(),
                            "pos": snap.position_ms,
                            "duration": snap.duration_ms,
                            "playing": snap.playing,
                            "rate": snap.rate,
                            "isLeader": is_leader,
                            "syncOn": sync_on,
                        })).await;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Serialize and send to socket with warning intercept
async fn send_socket<D: Serialize>(ws: &mut WebSocket, json: D) -> Result<(), Error> {
    match send_socket_inner(ws, json).await {
        Ok(_) => Ok(()),
        Err(e) => {
            warn!("Failed sending to socket: {e}");
            Err(e)
        },
    }
}

/// Serialize and send to socket
async fn send_socket_inner<D: Serialize>(ws: &mut WebSocket, json: D) -> Result<(), Error> {
    ws.send(Message::from(serde_json::to_string(&json)?)).await?;
    Ok(())
}

async fn handle_message(text: &str, websocket: &mut WebSocket, context: &mut SocketContext) -> Result<(), Error> {
    // Parse JSON
    let action: Action = serde_json::from_str(text)?;
    match action {
        // Get initial info
        Action::Init => {
            send_socket(websocket, InitData::new(context.start_context.clone())).await.ok();
        },
        Action::Exit => std::process::exit(0),
        Action::SaveSettings { settings } => Settings::from_ui(&settings).save()?,
        Action::DetectLegacySettings => {
            // Look for ~/Library/Preferences/com.OneTagger.OneTagger/settings.json (or platform equiv).
            // If present and parseable, return content so UI can offer one-click import.
            let legacy = directories::ProjectDirs::from("com", "OneTagger", "OneTagger");
            let result = legacy.and_then(|dirs| {
                let path = dirs.preference_dir().join("settings.json");
                if !path.exists() { return None; }
                let content = std::fs::read_to_string(&path).ok()?;
                let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
                Some((path.to_string_lossy().to_string(), parsed))
            });
            let (exists, path, settings) = match result {
                Some((p, s)) => (true, Some(p), Some(s)),
                None => (false, None, None),
            };
            send_socket(websocket, json!({
                "action": "detectLegacySettings",
                "exists": exists,
                "path": path,
                "settings": settings
            })).await.ok();
        },
        Action::LoadSettings => match Settings::load() {
            Ok(settings) => {
                send_socket(websocket, json!({
                    "action": "loadSettings",
                    "settings": settings.ui
                })).await.ok();
            }
            // Ignore settings if they don't exist (might be initial load)
            Err(e) => error!("Failed loading settings, using defaults. {}", e)
        },
        // Get the default custom platform options
        Action::DefaultCustomPlatformSettings => {
            send_socket(websocket, json!({
                "action": "defaultCustomPlatformSettings",
                "custom": TaggerConfig::custom_default().custom
            })).await.ok();
        }
        // Browse for folder
        Action::Browse { path, context } => {
            let mut initial = path.unwrap_or(".".to_string());
            if initial.is_empty() || !Path::new(&initial).exists() {
                initial = ".".to_string()
            }
            if let Some(path) = tinyfiledialogs::select_folder_dialog("Select path", &initial) {
                send_socket(websocket, json!({
                    "action": "browse",
                    "path": path,
                    "context": context
                })).await.ok();
            }
        },
        // Get 1t Log
        Action::GetLog => {
            log::logger().flush();
            let log = std::fs::read_to_string(&Settings::get_folder()?.join("digtrax.log"))?;
            send_socket(websocket, json!({
                "action": "log",
                "log": log
            })).await.ok();
        },
        // Open URL in external browser
        Action::Browser { url } => { webbrowser::open(&url)?; },
        Action::OpenSettingsFolder => opener::open(Settings::get_folder()?.to_str().unwrap())?,
        Action::OpenFolder { path } => { opener::open(&path).ok(); },
        Action::OpenFile { path } => { opener::open(&path).ok(); },
        Action::DeleteFiles { paths } => { trash::delete_all(&paths)?; },

        Action::GeneratePlaylist { paths } => {
            let playlist = digtrax_playlist::create_m3u_playlist(&paths.into_iter().map(|i| i.into()).collect::<Vec<_>>());
            if let Some(path) = tinyfiledialogs::save_file_dialog_with_filter(
                "Save playlist", 
                &std::env::current_dir()?.to_string_lossy().to_string(), 
                &["m3u", "m3u8"], 
                "Save playlist"
            ) {
                std::fs::write(&path, playlist)?;
                send_socket(websocket, json!({
                    "action": "notify",
                    "message": format!("Playlist saved to: {path}")
                })).await.ok();
            }
        }

        Action::LoadPlatforms => {
            let platforms = tokio::task::spawn_blocking(|| {
                let mut platforms = AUTOTAGGER_PLATFORMS.lock().unwrap();
                platforms.load_all();
                platforms.platforms.iter().map(|p| p.info.clone()).collect::<Vec<_>>()
            }).await?;
            send_socket(websocket, json!({
                "action": "loadPlatforms",
                "platforms": platforms
            })).await.ok();
        },
        Action::ConfigCallback { config, platform, id } => {
            let platform_clone = platform.clone();
            let response = tokio::task::spawn_blocking(move || {
                if let Some(p) = AUTOTAGGER_PLATFORMS.lock().unwrap().get_builder(&platform) {
                    Some(p.config_callback(&id, config))
                } else {
                    None
                }
            }).await?;
            if let Some(r) = response {
                send_socket(websocket, json!({
                    "action": "configCallback",
                    "platform": platform_clone,
                    "response": r
                })).await.ok();
            }
        }
        Action::StartTagging { config, playlist } => {
            config.debug_print();

            // Load playlist
            let mut files = if let Some(playlist) = playlist {
                playlist.get_files()?
            } else { vec![] };
            let mut file_count = files.len();
            let mut folder_path = None;
            let tagger_finished = Arc::new(Mutex::new(None));
            // Load taggers
            let (tagger_type, rx) = match config {
                TaggerConfigs::AutoTagger(c) => {
                    // Load file list
                    if files.is_empty() {
                        let path = c.path.as_ref().map(|p| p.to_owned()).unwrap_or_default();
                        files = AudioFileInfo::get_file_list(&path, c.include_subfolders);
                        file_count = files.len();
                        folder_path = Some(path);
                    }
                    let rx = Tagger::tag_files(&c, files, tagger_finished.clone());
                    ("autoTagger", rx)
                },
                TaggerConfigs::AudioFeatures(c) => {
                    if files.is_empty() {
                        let path = c.path.as_ref().map(|i| i.to_owned()).unwrap_or_default().to_owned();
                        files = AudioFileInfo::get_file_list(&path, c.include_subfolders);
                        folder_path = Some(path);
                        file_count = files.len();
                    }
                    // Authorize spotify
                    let spotify = context.spotify.as_ref().ok_or(anyhow!("Spotify unauthorized!"))?.to_owned().to_owned();
                    let rx = AudioFeatures::start_tagging(c.clone(), spotify, files);
                    ("audioFeatures", rx)
                },
            };

            // Start
            let start = timestamp!();
            send_socket(websocket, json!({
                "action": "startTagging",
                "files": file_count,
                "type": tagger_type
            })).await.ok();
            // Tagging
            for status in rx {
                send_socket(websocket, json!({
                    "action": "taggingProgress",
                    "status": status
                })).await.ok();
            }
            info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
            // Done
            send_socket(websocket, json!({
                "action": "taggingDone",
                "path": folder_path,
                "data": *tagger_finished.lock().unwrap()
            })).await.ok();
        },
        Action::StopTagging => {
            digtrax_autotag::STOP_TAGGING.store(true, Ordering::SeqCst);
        },
        Action::Waveform { path } => {
            let source = AudioSources::from_path(&path)?;
            let (waveform_rx, cancel_tx) = source.generate_waveform(180)?;
            // Streamed
            for wave in waveform_rx {
                send_socket(websocket, json!({
                    "action": "waveformWave",
                    "wave": wave
                })).await.ok();
                // Check reply
                if websocket.recv().await.is_none() {
                    cancel_tx.send(true).ok();
                }
            }
            // Done
            send_socket(websocket, json!({
                "action": "waveformDone",
            })).await.ok();
        },
        // Load player file
        Action::PlayerLoad { path } => {
            let source = AudioSources::from_path(&path)?;
            // Meta
            let tag = Tag::load_file(&path, false)?;
            let title = tag.tag().get_field(Field::Title).map(|i| i.first().map(String::from)).flatten();
            let artists = tag.tag().get_field(Field::Artist).unwrap_or(vec![]);
            // Send to UI
            send_socket(websocket, json!({
                "action": "playerLoad",
                "title": title,
                "artists": artists,
                "duration": source.duration() as u64
            })).await.ok();
            // Load
            context.player.load_file(source);
        },
        //  Controls
        Action::PlayerPlay => context.player.play(),
        Action::PlayerPause => context.player.pause(),
        Action::PlayerSeek { pos } => {
            send_socket(websocket, json!({
                "action": "playerSync",
                "playing": context.player.seek(pos)
            })).await.ok();
        },
        Action::PlayerVolume { volume } => context.player.volume(volume),
        Action::PlayerStop => context.player.stop(),

        // ─── DJ mixer (feature/dj-mixer) ──────────────────────────────
        // First DjLoad lazily opens the cpal output stream + spins up the
        // mixer. Decode runs on a worker via spawn_blocking so the WS
        // task stays responsive while a 5-min track parses.
        Action::DjLoad { deck, path } => {
            let mixer = context.mixer()?;
            let deck_handle = mixer.handle().deck(deck.into()).clone();
            // Read tags first → echo title/artists immediately so the
            // deck UI stops looking empty before decode finishes.
            let tag = Tag::load_file(&path, false)?;
            let title = tag.tag().get_field(Field::Title)
                .and_then(|i| i.first().map(String::from));
            let artists = tag.tag().get_field(Field::Artist).unwrap_or(vec![]);
            send_socket(websocket, json!({
                "action": "djLoadStart",
                "deck": deck,
                "path": &path,
                "title": title,
                "artists": artists,
            })).await.ok();

            // Decode + analyze on the blocking pool — analyze depends on
            // the decoded buffer, so they run sequentially here. ~3s
            // decode + ~2s analyze for a 5-min track = ~5s total before
            // the deck plays. Track is playable as soon as we hand it
            // to the mixer (after decode); the analyze just adds beat
            // info that gets pushed in a follow-up djAnalyzed event.
            let path_owned = path.clone();
            let decoded = tokio::task::spawn_blocking(move || {
                digtrax_deck::decode_file(&path_owned)
            }).await??;
            let duration_ms = decoded.duration_ms();
            // Clone Arc<Vec<f32>> via a fresh DecodedAudio handle so
            // the analyzer can keep working after we hand the track to
            // the deck. Both share the same sample buffer (Arc'd).
            let decoded_for_analysis = decoded.clone();
            deck_handle.load(decoded)?;
            send_socket(websocket, json!({
                "action": "djLoaded",
                "deck": deck,
                "duration": duration_ms,
            })).await.ok();

            // Beat tracking. 30 bars/sec spectrum density = 33 ms per
            // bin; for a 5-min track that's 9000 bins (900 visible at
            // 30s zoom — about 3 bars per pixel on a 1414px wide
            // canvas, so the waveform reads as a continuous shape
            // rather than blocky tiles).
            let analysis = tokio::task::spawn_blocking(move || {
                digtrax_deck::analyze(&decoded_for_analysis, 30.0)
            }).await??;
            // Push the beat sequence into the audio thread so the sync
            // engine can compute beat-distance against it. Must happen
            // BEFORE the user can engage Sync.
            deck_handle.set_beats(&analysis.beats_ms, analysis.bpm);
            send_socket(websocket, json!({
                "action": "djAnalyzed",
                "deck": deck,
                "bpm": analysis.bpm,
                "firstBeatMs": analysis.first_beat_ms,
                "confidence": analysis.confidence,
                "barsPerSecond": 30.0,
                "spectrumBars": analysis.spectrum_bars,
                "beatsMs": analysis.beats_ms,
            })).await.ok();
        },
        Action::DjPlay { deck } => {
            context.mixer()?.handle().deck(deck.into()).play();
        },
        Action::DjPause { deck } => {
            context.mixer()?.handle().deck(deck.into()).pause();
        },
        Action::DjStop { deck } => {
            context.mixer()?.handle().deck(deck.into()).stop();
        },
        Action::DjSeek { deck, pos } => {
            context.mixer()?.handle().deck(deck.into()).seek_ms(pos);
        },
        Action::DjVolume { deck, value } => {
            context.mixer()?.handle().deck(deck.into()).set_volume(value);
        },
        Action::DjRate { deck, value } => {
            context.mixer()?.handle().deck(deck.into()).set_rate(value);
        },
        Action::DjUnload { deck } => {
            context.mixer()?.handle().deck(deck.into()).unload();
        },
        Action::DjCrossfader { value } => {
            context.mixer()?.handle().set_crossfader(value);
        },
        Action::DjMasterGain { value } => {
            context.mixer()?.handle().set_master_gain(value);
        },
        Action::DjSetLeader { deck } => {
            context.mixer()?.handle().sync().set_leader(deck.map(Into::into));
        },
        Action::DjSync { deck, on } => {
            context.mixer()?.handle().sync().set_sync(deck.into(), on);
        },
        Action::DjEq { deck, band, value } => {
            context.mixer()?.handle().deck(deck.into()).set_eq(band.into(), value);
        },
        Action::DjFilter { deck, value } => {
            context.mixer()?.handle().deck(deck.into()).set_filter(value);
        },
        Action::DjBeatJump { deck, beats } => {
            context.mixer()?.handle().deck(deck.into()).beat_jump(beats);
        },

        // Load quicktag files or playlist
        Action::QuickTagLoad { path, playlist, recursive, separators, limit } => {
            let mut data = QuickTagData::default();
            // Playlist
            if let Some(playlist) = playlist {
                data = QuickTag::load_files_playlist(&playlist, &separators)?;
            }
            // Path
            if let Some(path) = path {
                if PLAYLIST_EXTENSIONS.iter().any(|e| path.to_lowercase().ends_with(e)) {
                    data = QuickTag::load_files(get_files_from_playlist_file(&path)?, &separators)?;
                } else {
                    data = QuickTag::load_files_path(
                        &path, 
                        recursive.unwrap_or(false), 
                        &separators, 
                        0, 
                        limit.map(|l| l.then_some(500)).flatten().unwrap_or(usize::MAX)
                    )?;
                }
            }
            send_socket(websocket, json!({
                "action": "quickTagLoad",
                "data": data
            })).await.ok();
        },
        // Save quicktag changes
        Action::QuickTagSave { path, changes } => {
            let tag = changes.commit_one(&path)?;
            send_socket(websocket, json!({
                "action": "quickTagSaved",
                "path": &path,
                "file": QuickTagFile::from_tag(&path, &tag)?
            })).await.ok();
        },
        // List dir
        Action::QuickTagFolder { path, subdir } => {
            let (new_path, files) = FileBrowser::list_dir_or_default(path.clone().map(|p| PathBuf::from(p)), subdir, true, false, false)?;
            send_socket(websocket, json!({
                "action": "quickTagFolder",
                "files": files,
                "path": new_path,
            })).await.ok();
        }
        Action::SpotifyAuthorize { client_id, client_secret } => {
            // Authorize cached
            if let Some(spotify) = Spotify::try_cached_token(&client_id, &client_secret) {
                context.spotify = Some(spotify);
            // Authorize new
            } else {
                let (auth_url, client) = Spotify::generate_auth_url(&client_id, &client_secret)?;
                webbrowser::open(&auth_url)?;
                let spotify = tokio::task::spawn_blocking(move || {
                    Spotify::auth_server(client)
                }).await??;
                context.spotify = Some(spotify);
            }
            send_socket(websocket, json!({
                "action": "spotifyAuthorized",
                "value": true
            })).await.ok();
            debug!("Spotify Authorized!");
        },
        // Check if authorized
        Action::SpotifyAuthorized => {
            send_socket(websocket, json!({
                "action": "spotifyAuthorized",
                "value": context.spotify.is_some()
            })).await.ok();
        },
        Action::TagEditorFolder { path, subdir, recursive } => {
            let recursive = recursive.unwrap_or(false);
            let (new_path, files) = FileBrowser::list_dir_or_default(path.clone().map(|p| PathBuf::from(p)), subdir, true, true, recursive)?;
            send_socket(websocket, json!({
                "action": "tagEditorFolder",
                "files": files,
                "path": new_path,
                // Stateless
                "recursive": recursive
            })).await.ok();
        },
        // Load tags of one or more files
        Action::TagEditorLoad { paths } => {
            let data = TagEditor::load_files(&paths)?;
            send_socket(websocket, json!({
                "action": "tagEditorLoad",
                "data": data
            })).await.ok();
        },
        // Save changes to one or more files. Streams a TagEditorBulkProgress
        // event after each file lands so the UI's progress drawer can update
        // incrementally; checks the cancel flag between files so the user can
        // abort 5,000-file batches without waiting for the whole set.
        Action::TagEditorSave { paths, changes } => {
            TAG_EDITOR_CANCEL.store(false, Ordering::Relaxed);
            let total = paths.len();
            let mut report = BatchReport { total, ..Default::default() };

            // Emit an initial event so the UI can spin up the progress drawer
            // before the first file lands (useful on slow disks).
            send_socket(websocket, json!({
                "action": "tagEditorBulkProgress",
                "phase": "start",
                "total": total
            })).await.ok();

            for (i, path) in paths.iter().enumerate() {
                if TAG_EDITOR_CANCEL.load(Ordering::Relaxed) {
                    break;
                }
                let result = changes.commit_one(path);
                let ok = result.is_ok();
                let err = result.as_ref().err().map(|e| format!("{e:#}"));
                match &result {
                    Ok(_) => report.succeeded += 1,
                    Err(_) => report.failed.push((path.clone(), err.clone().unwrap_or_default())),
                }
                send_socket(websocket, json!({
                    "action": "tagEditorBulkProgress",
                    "phase": "progress",
                    "path": path,
                    "ok": ok,
                    "error": err,
                    "index": i + 1,
                    "total": total
                })).await.ok();
            }

            let cancelled = TAG_EDITOR_CANCEL.load(Ordering::Relaxed);
            send_socket(websocket, json!({
                "action": "tagEditorSave",
                "report": report,
                "cancelled": cancelled
            })).await.ok();
        },
        // Filter — walk the folder, evaluate the query against each file's
        // tag map, return the matching paths (no tag dumps, the UI follows up
        // with TagEditorLoad to load the matched set).
        Action::TagEditorFilter { folder, recursive, query } => {
            let mut matches: Vec<PathBuf> = Vec::new();
            let mut scanned = 0usize;
            for entry in AudioFileInfo::load_files_iter(&folder, recursive, None, None) {
                scanned += 1;
                let info = match entry { Ok(i) => i, Err(_) => continue };
                let tag_wrap = match Tag::load_file(&info.path, true) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let tags = tag_wrap.tag().all_tags();
                if query.matches(&tags) {
                    matches.push(info.path);
                }
            }
            send_socket(websocket, json!({
                "action": "tagEditorFilter",
                "paths": matches,
                "scanned": scanned
            })).await.ok();
        },
        // Cancel an in-flight bulk save. Idempotent; the flag is reset at the
        // start of every TagEditorSave so a stale cancel can't poison the next
        // batch.
        Action::TagEditorCancel => {
            TAG_EDITOR_CANCEL.store(true, Ordering::Relaxed);
            send_socket(websocket, json!({
                "action": "tagEditorCancel"
            })).await.ok();
        },
        // Folder-walk variant of RenameFrame. Applies all 3 per-format
        // RenameFrame variants to every audio file under `folder` — files in
        // formats other than the matching one no-op cleanly because
        // RenameFrame is a no-op when the source key is absent.
        Action::MigrateTagFolder { folder, recursive, from, to } => {
            TAG_EDITOR_CANCEL.store(false, Ordering::Relaxed);
            let paths: Vec<PathBuf> = AudioFileInfo::load_files_iter(&folder, recursive, None, None)
                .filter_map(|r| r.ok().map(|i| i.path))
                .collect();
            let total = paths.len();
            let changes = TagChanges::new(vec![
                TagChange::RenameFrame { from: from.id3.clone(),    to: to.id3.clone()    },
                TagChange::RenameFrame { from: from.vorbis.clone(), to: to.vorbis.clone() },
                TagChange::RenameFrame { from: from.mp4.clone(),    to: to.mp4.clone()    },
            ]);

            send_socket(websocket, json!({
                "action": "tagEditorBulkProgress",
                "phase": "start",
                "total": total
            })).await.ok();

            let mut report = BatchReport { total, ..Default::default() };
            for (i, path) in paths.iter().enumerate() {
                if TAG_EDITOR_CANCEL.load(Ordering::Relaxed) {
                    break;
                }
                let result = changes.commit_one(path);
                let ok = result.is_ok();
                let err = result.as_ref().err().map(|e| format!("{e:#}"));
                match &result {
                    Ok(_) => report.succeeded += 1,
                    Err(_) => report.failed.push((path.clone(), err.clone().unwrap_or_default())),
                }
                send_socket(websocket, json!({
                    "action": "tagEditorBulkProgress",
                    "phase": "progress",
                    "path": path,
                    "ok": ok,
                    "error": err,
                    "index": i + 1,
                    "total": total
                })).await.ok();
            }

            let cancelled = TAG_EDITOR_CANCEL.load(Ordering::Relaxed);
            send_socket(websocket, json!({
                "action": "tagEditorMigrate",
                "report": report,
                "cancelled": cancelled,
                "scanned": total,
                "from": from,
                "to": to
            })).await.ok();
        },
        // Per-file value rename inside a frame. Loads each file, picks the
        // right frame key for its format, splits existing values via the
        // tag impl's get_raw (which handles ID3 separator semantics for
        // COMM/USLT), replaces matches, writes back. Files that don't carry
        // the value are reported as unmodified — they aren't write-touched.
        Action::MigrateValueFolder { folder, recursive, frame, old_value, new_value } => {
            TAG_EDITOR_CANCEL.store(false, Ordering::Relaxed);
            let paths: Vec<PathBuf> = AudioFileInfo::load_files_iter(&folder, recursive, None, None)
                .filter_map(|r| r.ok().map(|i| i.path))
                .collect();
            let total = paths.len();

            send_socket(websocket, json!({
                "action": "tagEditorBulkProgress",
                "phase": "start",
                "total": total
            })).await.ok();

            let mut report = BatchReport { total, ..Default::default() };
            let mut modified_count = 0usize;
            for (i, path) in paths.iter().enumerate() {
                if TAG_EDITOR_CANCEL.load(Ordering::Relaxed) {
                    break;
                }

                // Per-file mutation closure so we can `?` cleanly inside.
                let mutated: Result<bool, anyhow::Error> = (|| {
                    let mut tag_wrap = Tag::load_file(path, false)?;
                    let format = tag_wrap.format();
                    let key: &str = match format {
                        AudioFileFormat::MP3 | AudioFileFormat::AIFF | AudioFileFormat::WAV => &frame.id3,
                        AudioFileFormat::MP4 => &frame.mp4,
                        AudioFileFormat::FLAC | AudioFileFormat::OGG => &frame.vorbis,
                    };
                    if key.is_empty() { return Ok(false); }

                    let tag = tag_wrap.tag_mut();
                    let Some(values) = tag.get_raw(key) else { return Ok(false); };
                    let mut hit = false;
                    let new_values: Vec<String> = values.into_iter().map(|v| {
                        if v == old_value { hit = true; new_value.clone() } else { v }
                    }).collect();
                    if !hit { return Ok(false); }
                    tag.set_raw(key, new_values, true);
                    tag.save_file(path)?;
                    Ok(true)
                })();

                let (ok, err, modified) = match &mutated {
                    Ok(m) => (true, None, *m),
                    Err(e) => (false, Some(format!("{e:#}")), false),
                };
                match &mutated {
                    Ok(_) => report.succeeded += 1,
                    Err(_) => report.failed.push((path.clone(), err.clone().unwrap_or_default())),
                }
                if modified { modified_count += 1; }

                send_socket(websocket, json!({
                    "action": "tagEditorBulkProgress",
                    "phase": "progress",
                    "path": path,
                    "ok": ok,
                    "error": err,
                    "modified": modified,
                    "index": i + 1,
                    "total": total
                })).await.ok();
            }

            let cancelled = TAG_EDITOR_CANCEL.load(Ordering::Relaxed);
            send_socket(websocket, json!({
                "action": "tagEditorMigrateValue",
                "report": report,
                "cancelled": cancelled,
                "scanned": total,
                "modified": modified_count,
                "frame": frame,
                "oldValue": old_value,
                "newValue": new_value
            })).await.ok();
        },
        // Syntax highlight for renamer
        Action::RenamerSyntaxHighlight { template } => {
            let renamer = Renamer::new(TemplateParser::parse(&template));
            let html = renamer.generate_html(&template);
            send_socket(websocket, json!({
                "action": "renamerSyntaxHighlight",
                "html": html
            })).await.ok();
        },
        // Autocomplete data
        Action::RenamerAutocomplete { template } => {
            let ac = Autocomplete::parse(&template);
            let suggestions = ac.suggest_html();
            send_socket(websocket, json!({
                "action": "renamerAutocomplete",
                "suggestions": suggestions,
                "offset": ac.suggestion_offset()
            })).await.ok();
        },
        // Generate new names but don't rename
        Action::RenamerPreview { config } => {
            let mut renamer = Renamer::new(TemplateParser::parse(&config.template));
            let files = AudioFileInfo::load_files_iter(&config.path, config.subfolders, None, None);
            let files = renamer.generate(files.take(3), &config).unwrap_or(vec![]);
            send_socket(websocket, json!({
                "action": "renamerPreview",
                "files": files,
            })).await.ok();
        },
        // Start renamer
        Action::RenamerStart { config } => {
            let mut renamer = Renamer::new(TemplateParser::parse(&config.template));
            let files = AudioFileInfo::load_files_iter(&config.path, config.subfolders, None, None);
            let files = renamer.generate(files, &config)?;
            renamer.rename(&files, &config)?;
            send_socket(websocket, json!({
                "action": "renamerDone",
            })).await.ok();
        },
        // File browser list dir
        Action::FolderBrowser { path, child , base } => {
            // Windows root dir override
            let path = if cfg!(windows) && path.to_string_lossy() == "/" {
                if child.is_empty() {
                    PathBuf::from("/".to_string())
                } else {
                    PathBuf::from(format!("{}\\", child))
                }
            } else {
                canonicalize(PathBuf::from(path).join(child))?
            };

            let e = match base {
                true => FolderBrowser::generate_base(&path)?,
                false => FolderBrowser::list_dir(&path)?
            };

            send_socket(websocket, json!({
                "action": "folderBrowser",
                "entry": e,
                "base": base,
                "path": path
            })).await.ok();
        },

        // Manually tag a file
        Action::ManualTag { config, path } => {
            // Log config
            info!("Manual tag starting for path: {path:?}");
            TaggerConfigs::AutoTagger(config.clone()).debug_print();

            let rx = tokio::task::spawn_blocking(move || {
                digtrax_autotag::manual_tagger(path, &config)
            }).await.unwrap()?;

            for (platform, r) in rx {
                match r {
                    Ok(matches) => {
                        send_socket(websocket, json!({
                            "action": "manualTag",
                            "platform": platform,
                            "status": "ok",
                            "matches": matches
                        })).await.ok();
                    },
                    Err(e) => {
                        send_socket(websocket, json!({
                            "action": "manualTag",
                            "platform": platform,
                            "status": "error",
                            "error": e.to_string()
                        })).await.ok();
                    },
                }
            }

            // On done
            send_socket(websocket, json!({
                "action": "manualTagDone"
            })).await.ok();
        },
        // Apply the tags from manual tagger
        Action::ManualTagApply { matches, path, config } => {
            match digtrax_autotag::manual_tagger_apply(matches, path, &config) {
                Ok(_) => {
                    send_socket(websocket, json!({
                        "action": "manualTagApplied",
                        "status": "ok"
                    })).await.ok();
                },
                Err(e) => {
                    error!("Failed applying manual tag: {e}");
                    send_socket(websocket, json!({
                        "action": "manualTagApplied",
                        "status": "error",
                        "error": e.to_string()
                    })).await.ok();
                },
            }
        },


        Action::RepoManifest => {
            send_socket(websocket, json!({
                "action": "repoManifest",
                "manifest": digtrax_autotag::repo::fetch_manifest_async().await?
            })).await.ok();
        },
        Action::InstallPlatform { id, version, is_native } => {
            match digtrax_autotag::repo::install_platform(&id, &version, is_native) {
                Ok(_) => send_socket(websocket, json!({
                    "action": "installPlatform",
                    "status": "ok"
                })).await.ok(),
                Err(e) => {
                    error!("Failed installing platform {id}@{version}: {e}");
                    send_socket(websocket, json!({
                        "action": "installPlatform",
                        "status": "error",
                        "error": e.to_string()
                    })).await.ok()
                },
            };
        },

        
        
    }
   
    Ok(())
}