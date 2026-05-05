// TODO: Use log & anyhow somewhere
#[cfg(feature = "tag")]
#[macro_use] extern crate log;
#[cfg(feature = "tag")]
#[macro_use] extern crate anyhow;

use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::Duration;
use anyhow::Error;

#[cfg(feature = "tag")]
use std::collections::HashMap;
#[cfg(feature = "tag")]
use std::path::Path;

#[cfg(feature = "tag")]
pub mod id3;
#[cfg(feature = "tag")]
pub mod flac;
#[cfg(feature = "tag")]
pub mod mp4;
#[cfg(feature = "tag")]
pub mod vorbis;
#[cfg(feature = "tag")]
mod wav;

// Supported extensions
pub static EXTENSIONS : [&'static str; 11] = ["mp3", "flac", "aif", "aiff", "m4a", 
    "mp4", "wav", "ogg", "opus", "spx", "oga"];

#[cfg(feature = "tag")]
pub enum Tag {
    FLAC(flac::FLACTag),
    ID3(id3::ID3Tag),
    MP4(mp4::MP4Tag),
    Vorbis(vorbis::VorbisTag)
}

#[cfg(feature = "tag")]
impl Tag {
    pub fn load_file(path: impl AsRef<Path>, allow_new: bool) -> Result<Tag, Error> {
        let ext = path.as_ref().extension().ok_or(anyhow!("Missing extension"))?.to_ascii_lowercase();
        // FLAC
        if ext == "flac" {
            return Ok(Tag::FLAC(flac::FLACTag::load_file(path)?));
        }
        // MP4
        if ext == "m4a" || ext == "mp4" {
            return Ok(Tag::MP4(mp4::MP4Tag::load_file(path)?));
        }

        // Vorbis
        if ext == "ogg" || ext == "opus" || ext == "oga" || ext == "spx" {
            return Ok(Tag::Vorbis(vorbis::VorbisTag::load_file(path)?));
        }

        // ID3
        let tag = if allow_new {
            id3::ID3Tag::load_or_new(path)
        } else {
            id3::ID3Tag::load_file(path)?
        };
        Ok(Tag::ID3(tag))
    }

    // Set proper separators for every format
    pub fn set_separators(&mut self, separators: &TagSeparators) {
        match self {
            Tag::FLAC(tag) => tag.set_separator(separators.vorbis.as_ref().unwrap_or(&String::new())),
            Tag::ID3(tag) => tag.set_separator(&separators.id3),
            Tag::MP4(tag) => tag.set_separator(&separators.mp4),
            Tag::Vorbis(tag) => tag.set_separator(separators.vorbis.as_ref().unwrap_or(&String::new())),
        }
    }

    // Get generic
    pub fn tag(&self) -> Box<&dyn TagImpl> {
        match self {
            Tag::FLAC(tag) => Box::new(tag),
            Tag::ID3(tag) => Box::new(tag),
            Tag::MP4(tag) => Box::new(tag),
            Tag::Vorbis(tag) => Box::new(tag),
        }
    }
    pub fn tag_mut(&mut self) -> Box<&mut dyn TagImpl> {
        match self {
            Tag::FLAC(tag) => Box::new(tag),
            Tag::ID3(tag) => Box::new(tag),
            Tag::MP4(tag) => Box::new(tag),
            Tag::Vorbis(tag) => Box::new(tag)
        }
    }

    // Get format
    pub fn format(&self) -> AudioFileFormat {
        match self {
            Tag::FLAC(_) => AudioFileFormat::FLAC,
            Tag::MP4(_) => AudioFileFormat::MP4,
            Tag::ID3(id3) => match id3.format {
                id3::ID3AudioFormat::MP3 => AudioFileFormat::MP3,
                id3::ID3AudioFormat::AIFF => AudioFileFormat::AIFF,
                id3::ID3AudioFormat::WAV => AudioFileFormat::WAV
            },
            Tag::Vorbis(_) => AudioFileFormat::OGG
        }
    }
}

#[cfg(feature = "tag")]
pub trait TagImpl {
    /// Write file to path, using Path because of object safety
    fn save_file(&mut self, path: &Path) -> Result<(), Error>;

    /// Since all formats right now support separators
    fn set_separator(&mut self, separator: &str);
    /// Get the separator
    fn get_separator(&self) -> Option<String>;

    /// Get all string tags
    fn all_tags(&self) -> HashMap<String, Vec<String>>;

    /// Set/Get dates
    fn get_date(&self) -> Option<TagDate>;
    fn set_date(&mut self, date: &TagDate, overwrite: bool);
    fn set_publish_date(&mut self, date: &TagDate, overwrite: bool);

    /// Get/Set rating as 1 - 5 stars value
    fn get_rating(&self) -> Option<u8>;
    fn set_rating(&mut self, rating: u8, overwrite: bool);

    /// Set/Get album art
    fn set_art(&mut self, kind: CoverType, mime: &str, description: Option<&str>, data: Vec<u8>);
    /// To not load all album arts
    fn has_art(&self) -> bool;
    fn get_art(&self) -> Vec<Picture>;
    fn remove_art(&mut self, kind: CoverType);

    /// Set/Get named field
    fn set_field(&mut self, field: Field, value: Vec<String>, overwrite: bool);
    fn get_field(&self, field: Field) -> Option<Vec<String>>;

    /// Set/Get by tag field name
    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool);
    fn get_raw(&self, tag: &str) -> Option<Vec<String>>;
    fn remove_raw(&mut self, tag: &str);

    /// Set lyrics
    fn set_lyrics(&mut self, lyrics: &Lyrics, synced: bool, overwrite: bool);

    /// Set track number (because formats like MP3 and M4A use custom format)
    /// Track number is string because of platforms like discogs
    fn set_track_number(&mut self, track_number: &str, track_total: Option<u16>, overwrite: bool);

    /// Set whether the track is explicit
    fn set_explicit(&mut self, explicit: bool);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSeparators {
    pub id3: String,
    pub vorbis: Option<String>,
    pub mp4: String
}

impl Default for TagSeparators {
    fn default() -> Self {
        TagSeparators {
            id3: ", ".to_string(),
            vorbis: None,
            mp4: ", ".to_string()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFileFormat {
    FLAC, AIFF, MP3, MP4, WAV, OGG
}

impl AudioFileFormat {
    // Recognize format from extension
    pub fn from_extension(ext: &str) -> Option<AudioFileFormat> {
        match &ext.to_lowercase()[..] {
            "flac" => Some(AudioFileFormat::FLAC),
            "aiff" | "aif" => Some(AudioFileFormat::AIFF),
            "mp3" => Some(AudioFileFormat::MP3),
            "m4a" | "mp4" => Some(AudioFileFormat::MP4),
            "wav" => Some(AudioFileFormat::WAV),
            "ogg" | "opus" | "spx" | "oga" => Some(AudioFileFormat::OGG),
            _ => None
        }
    }
}

/// Tag fields from UI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct FrameName {
    pub id3: String,
    pub vorbis: String,
    pub mp4: String
}

impl FrameName {
    /// All formats the same Frame name
    pub fn same(name: &str) -> FrameName {
        FrameName {
            id3: name.to_string(),
            vorbis: name.to_string(),
            mp4: name.to_string()
        }
    }

    /// Shorthand for creating
    pub fn new(id3: &str, vorbis: &str, mp4: &str) -> FrameName {
        FrameName { id3: id3.to_string(), vorbis: vorbis.to_string(), mp4: mp4.to_string() }
    }

    /// Get raw value by format
    pub fn by_format(&self, format: &AudioFileFormat) -> String {
        match format.to_owned() {
            AudioFileFormat::AIFF => self.id3.to_string(),
            AudioFileFormat::MP3 => self.id3.to_string(),
            AudioFileFormat::WAV => self.id3.to_string(),
            AudioFileFormat::FLAC => self.vorbis.to_string(),
            AudioFileFormat::MP4 => self.mp4.to_string(),
            AudioFileFormat::OGG => self.vorbis.to_string()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub kind: CoverType,
    pub data: Vec<u8>,
    pub description: String,
    pub mime: String
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CoverType {
    CoverFront,
    CoverBack,
    Other,
    Artist,
    Icon,
    OtherIcon,
    Leaflet,
    Media,
    LeadArtist,
    Conductor,
    Band,
    Composer,
    Lyricist,
    RecordingLocation,
    DuringRecording,
    DuringPerformance,
    ScreenCapture,
    BrightFish,
    Illustration,
    BandLogo,
    PublisherLogo,
    Undefined
}

impl CoverType {
    // Get all the types
    pub fn types() -> [CoverType; 22] {
        [CoverType::CoverFront, CoverType::CoverBack, CoverType::Other, CoverType::Artist,
        CoverType::Icon, CoverType::OtherIcon, CoverType::Leaflet, CoverType::Media, CoverType::LeadArtist,
        CoverType::Conductor, CoverType::Band, CoverType::Composer, CoverType::Lyricist,
        CoverType::RecordingLocation, CoverType::DuringRecording, CoverType::DuringPerformance,
        CoverType::ScreenCapture, CoverType::BrightFish, CoverType::Illustration, CoverType::BandLogo,
        CoverType::PublisherLogo, CoverType::Undefined]
    }
}

#[derive(Debug, Clone)]
pub struct TagDate {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>
}

impl TagDate {
    // If has day and month
    pub fn has_md(&self) -> bool {
        return self.month.is_some() && self.day.is_some();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Title,
    Artist,
    Album,
    AlbumArtist,
    Key,
    BPM,
    Genre,
    Style,
    Label,
    ISRC,
    CatalogNumber,
    Version,
    TrackNumber,
    Duration,
    Remixer,
    Mood,
    TrackTotal,
    DiscNumber,
}

impl Field {
    /// Get tag name by format
    pub fn by_format(&self, format: &AudioFileFormat) -> &'static str {
        match format {
            AudioFileFormat::FLAC => self.vorbis(),
            AudioFileFormat::AIFF => self.id3(),
            AudioFileFormat::WAV => self.id3(),
            AudioFileFormat::MP3 => self.id3(),
            AudioFileFormat::MP4 => self.mp4(),
            AudioFileFormat::OGG => self.vorbis(),
        }
    }

    /// Convert to ID3 frame name
    pub fn id3(&self) -> &'static str {
        match self {
            Field::Title => "TIT2",
            Field::Artist => "TPE1",
            Field::AlbumArtist => "TPE2",
            Field::Album => "TALB",
            Field::Key => "TKEY",
            Field::BPM => "TBPM",
            Field::Genre => "TCON",
            Field::Label => "TPUB",
            Field::Style => "STYLE",
            Field::ISRC => "TSRC",
            Field::CatalogNumber => "CATALOGNUMBER",
            Field::Version => "TIT3",
            Field::TrackNumber => "TRCK",
            Field::Duration => "TLEN",
            Field::Remixer => "TPE4",
            Field::Mood => "TMOO",
            Field::TrackTotal => "TRCK",
            Field::DiscNumber => "TPOS",
        }
    }

    /// Convert to VORBIS frame name
    pub fn vorbis(&self) -> &'static str {
        match self {
            Field::Title => "TITLE",
            Field::Artist => "ARTIST",
            Field::AlbumArtist => "ALBUMARTIST",
            Field::Album => "ALBUM",
            Field::Key => "INITIALKEY",
            Field::BPM => "BPM",
            Field::Genre => "GENRE",
            Field::Label => "PUBLISHER",
            Field::Style => "STYLE",
            Field::ISRC => "ISRC",
            Field::CatalogNumber => "CATALOGNUMBER",
            Field::Version => "SUBTITLE",
            Field::TrackNumber => "TRACKNUMBER",
            Field::Duration => "LENGTH",
            Field::Remixer => "REMIXER",
            Field::Mood => "MOOD",
            Field::TrackTotal => "TRACKTOTAL",
            Field::DiscNumber => "DISCNUMBER",
        }
    }

    /// Convert to MP4 frame name
    pub fn mp4(&self) -> &'static str {
        match self {
            Field::Title => "©nam",
            Field::Artist => "©ART",
            Field::AlbumArtist => "aART",
            Field::Album => "©alb",
            Field::BPM => "tmpo",
            Field::Genre => "©gen",
            Field::Label => "com.apple.iTunes:LABEL",
            Field::ISRC => "com.apple.iTunes:ISRC",
            Field::CatalogNumber => "com.apple.iTunes:CATALOGNUMBER",
            Field::Version => "desc",
            Field::TrackNumber => "trkn",
            Field::Remixer => "com.apple.iTunes:REMIXER",
            Field::Key => "com.apple.iTunes:initialkey",
            Field::Style => "com.apple.iTunes:STYLE",
            Field::Duration => "com.apple.iTunes:LENGTH",
            Field::Mood => "com.apple.iTunes:MOOD",
            Field::TrackTotal => "trkn",
            Field::DiscNumber => "disk",
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum TagChange {
    Raw { tag: String, value: Vec<String> },
    Rating { value: u8 },
    Genre { value: Vec<String> },
    Remove { tag: String },
    RemovePicture { kind: CoverType },

    // Rename a tag frame across formats. Reads the existing value(s) under `from`,
    // writes them under `to`, then removes `from`. No-op if `from` is absent.
    // For ID3, frame IDs (e.g. TXXX:OLD → TXXX:NEW); for Vorbis, lowercase keys;
    // for MP4, atom names.
    RenameFrame { from: String, to: String },

    // For adding from UI
    #[cfg(feature = "tag")]
    AddPictureBase64 { kind: CoverType, description: String, data: String, mime: String },

    #[cfg(feature = "tag")]
    #[serde(rename = "id3Comments")]
    ID3Comments { comments: Vec<id3::ID3Comment> },

    #[cfg(feature = "tag")]
    #[serde(rename = "id3UnsynchronizedLyrics")]
    ID3UnsynchronizedLyrics { lyrics: Vec<id3::ID3Comment> },

    #[cfg(feature = "tag")]
    #[serde(rename = "id3Popularimeter")]
    ID3Popularimeter { popm: id3::ID3Popularimeter }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagChanges {
    changes: Vec<TagChange>,
    separators: TagSeparators,
    id3v24: bool,
    id3_comm_lang: Option<String>
}

/// Aggregate result of a multi-file commit. `failed` carries the human-readable
/// error string per failed path so callers can surface it without re-deriving.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatchReport {
    pub total: usize,
    pub succeeded: usize,
    pub failed: Vec<(PathBuf, String)>,
}

#[cfg(feature = "tag")]
impl TagChanges {
    /// Construct a TagChanges with default separators / no ID3v2.4 forcing.
    /// Used by callers that build a one-off batch (e.g. the tag-migration
    /// handler) and don't need to negotiate per-file format options.
    pub fn new(changes: Vec<TagChange>) -> Self {
        TagChanges {
            changes,
            separators: TagSeparators::default(),
            id3v24: false,
            id3_comm_lang: None,
        }
    }

    /// Apply all changes to a single file. Returns the loaded `Tag` so callers
    /// (e.g. QuickTag) can extract post-write metadata without re-reading.
    pub fn commit_one(&self, path: &Path) -> Result<Tag, Error> {
        use base64::Engine;

        let mut tag_wrap = Tag::load_file(path, false)?;
        tag_wrap.set_separators(&self.separators);

        // Format specific changes
        if let Tag::ID3(id3) = &mut tag_wrap {
            id3.set_id3v24(self.id3v24);
            if let Some(lang) = self.id3_comm_lang.as_ref() {
                if !lang.is_empty() {
                    id3.set_comm_lang(lang.to_string());
                }
            }

            for change in self.changes.clone() {
                match change {
                    TagChange::ID3Comments {comments} => id3.set_comments(&comments),
                    TagChange::ID3UnsynchronizedLyrics {lyrics} => id3.set_unsync_lyrics(&lyrics),
                    TagChange::ID3Popularimeter {popm} => id3.set_popularimeter(&popm),
                    _ => {}
                }
            }
        }

        // MP4 doesn't have any way to distinguish between artwork types so abstraction to do that
        // Not very efficient, but rarely used and should work
        if let Tag::MP4(mp4) = &mut tag_wrap {
            // Get album art indexes
            let mut indicies: Vec<usize> = self.changes.iter().filter_map(|c| match c {
                TagChange::RemovePicture {kind} => CoverType::types().iter().position(|k| k == kind),
                _ => None
            }).collect();
            // Last to first
            indicies.sort();
            indicies.reverse();
            let types = CoverType::types();
            for i in indicies {
                mp4.remove_art(types[i].to_owned());
            };
        }

        let format = tag_wrap.format();
        let tag = tag_wrap.tag_mut();
        // Match changes
        for change in self.changes.clone() {
            match change {
                TagChange::Raw {tag: t, value} => tag.set_raw(&t, value, true),
                TagChange::Rating {value} => tag.set_rating(value, true),
                TagChange::Genre {value} => tag.set_field(Field::Genre, value, true),
                TagChange::Remove {tag: t} => tag.remove_raw(&t),
                TagChange::RemovePicture {kind} => if format != AudioFileFormat::MP4 { tag.remove_art(kind) },
                TagChange::AddPictureBase64 {kind, description, data, mime} => tag.set_art(kind, &mime, Some(&description), base64::engine::general_purpose::STANDARD.decode(&data)?),
                TagChange::RenameFrame { from, to } => {
                    if from != to {
                        if let Some(values) = tag.get_raw(&from) {
                            tag.set_raw(&to, values, true);
                            tag.remove_raw(&from);
                        }
                    }
                },
                _ => {}
            }
        }
        // Save
        tag.save_file(path)?;

        Ok(tag_wrap)
    }

    /// Apply the same change set to N files. Streams per-file results through
    /// `on_progress` so the caller can forward updates without buffering the
    /// whole batch. Continues on per-file errors and reports them in the
    /// returned `BatchReport`.
    pub fn commit_multi<F>(&self, paths: &[PathBuf], mut on_progress: F) -> BatchReport
    where
        F: FnMut(&Path, &Result<(), Error>),
    {
        let mut report = BatchReport { total: paths.len(), ..Default::default() };
        for path in paths {
            let result = self.commit_one(path).map(|_| ());
            on_progress(path, &result);
            match result {
                Ok(()) => report.succeeded += 1,
                Err(e) => report.failed.push((path.clone(), format!("{e:#}"))),
            }
        }
        report
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct Lyrics {
    /// Double vec for paragraph separation
    pub paragraphs: Vec<Vec<LyricsLine>>,
    pub language: String,
}

impl Lyrics {
    /// Join into text
    pub fn text(&self) -> String {
        self.paragraphs.iter().map(|p| 
            p.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n")
        ).collect::<Vec<_>>().join("\n\n")
    }

    /// Parse MM:SS.ms (optionally just SS.ms)
    pub fn parse_lrc_timestamp(input: &str) -> Result<Duration, Error> {
        let mut minutes = 0;
        let parts = input.split(":").collect::<Vec<_>>();
        let seconds = if parts.len() == 2 {
            minutes = parts[0].parse::<u32>()?;
            parts[1].parse::<f32>()?
        } else {
            parts[0].parse()?
        };
        return Ok(Duration::from_secs_f32(seconds + minutes as f32 * 60.0))
    }

    /// Are the lyrics synced?
    pub fn synced(&self) -> bool {
        self.paragraphs.first().map(|p| p.first().map(|l| l.start.is_some())).flatten().unwrap_or(false)
    }

    /// Iterate over lines
    pub fn iter_lines(&self) -> impl Iterator<Item = &LyricsLine> {
        self.paragraphs.iter().flatten()
    }

    /// Iterate over lines (owned)
    pub fn into_iter_lines(self) -> impl Iterator<Item = LyricsLine> {
        self.paragraphs.into_iter().flatten()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct LyricsLine {
    pub text: String,
    pub start: Option<Duration>,
    pub end: Option<Duration>,
    /// Optional
    pub parts: Vec<LyricsLinePart>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct LyricsLinePart {
    pub text: String,
    pub start: Option<Duration>,
    pub end: Option<Duration>
}

/// How a query value matches against a tag value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MatchMode {
    Exact,
    Contains,
    Regex,
}

impl Default for MatchMode {
    fn default() -> Self { MatchMode::Contains }
}

/// Composable predicate for filtering files by their tag content. Evaluated
/// against the result of `TagImpl::all_tags()` — tag keys are matched
/// case-insensitively to absorb format quirks (Vorbis lowercase, ID3 frame IDs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum TagFilterQuery {
    /// Tag key is present and has at least one non-empty value.
    HasTag { tag: String },
    /// Tag value matches under the given `mode`. Case-insensitive for
    /// `Exact` and `Contains`; the regex carries its own flags for `Regex`.
    Equals { tag: String, value: String, mode: MatchMode },
    /// Tag is absent or all of its values are empty strings.
    Missing { tag: String },
    /// All sub-queries must match.
    And { children: Vec<TagFilterQuery> },
    /// At least one sub-query must match.
    Or { children: Vec<TagFilterQuery> },
}

#[cfg(feature = "tag")]
impl TagFilterQuery {
    /// Evaluate the query against the supplied tag map. Returns `true` if the
    /// file matches. The map is `key -> Vec<value>` exactly as
    /// `TagImpl::all_tags` produces it.
    pub fn matches(&self, tags: &HashMap<String, Vec<String>>) -> bool {
        match self {
            TagFilterQuery::HasTag { tag } => Self::lookup(tags, tag)
                .map(|v| v.iter().any(|s| !s.is_empty()))
                .unwrap_or(false),
            TagFilterQuery::Missing { tag } => match Self::lookup(tags, tag) {
                None => true,
                Some(values) => values.iter().all(|s| s.is_empty()),
            },
            TagFilterQuery::Equals { tag, value, mode } => {
                let Some(values) = Self::lookup(tags, tag) else { return false };
                match mode {
                    MatchMode::Exact => {
                        let needle = value.to_lowercase();
                        values.iter().any(|v| v.to_lowercase() == needle)
                    }
                    MatchMode::Contains => {
                        let needle = value.to_lowercase();
                        values.iter().any(|v| v.to_lowercase().contains(&needle))
                    }
                    MatchMode::Regex => match regex::Regex::new(value) {
                        Ok(re) => values.iter().any(|v| re.is_match(v)),
                        // Bad regex -> no match. Surfacing a parse error for
                        // every file would be noise; the UI validates up front.
                        Err(_) => false,
                    },
                }
            }
            TagFilterQuery::And { children } => children.iter().all(|q| q.matches(tags)),
            TagFilterQuery::Or { children } => children.iter().any(|q| q.matches(tags)),
        }
    }

    fn lookup<'a>(tags: &'a HashMap<String, Vec<String>>, key: &str) -> Option<&'a Vec<String>> {
        // Fast path: exact key.
        if let Some(v) = tags.get(key) {
            return Some(v);
        }
        // Slow path: case-insensitive, since ID3/Vorbis/MP4 disagree on case.
        let lower = key.to_lowercase();
        tags.iter().find(|(k, _)| k.to_lowercase() == lower).map(|(_, v)| v)
    }
}

#[cfg(all(test, feature = "tag"))]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn empty_changes() -> TagChanges {
        TagChanges {
            changes: vec![],
            separators: TagSeparators::default(),
            id3v24: false,
            id3_comm_lang: None,
        }
    }

    #[test]
    fn commit_multi_empty_paths_produces_empty_report() {
        let changes = empty_changes();
        let report = changes.commit_multi(&[], |_, _| panic!("callback should not fire"));
        assert_eq!(report.total, 0);
        assert_eq!(report.succeeded, 0);
        assert!(report.failed.is_empty());
    }

    #[test]
    fn commit_multi_reports_per_file_failures_without_aborting() {
        let changes = empty_changes();
        let paths = vec![
            PathBuf::from("/nonexistent/dir/a.mp3"),
            PathBuf::from("/nonexistent/dir/b.flac"),
            PathBuf::from("/nonexistent/dir/c.m4a"),
        ];
        let calls = RefCell::new(0usize);
        let report = changes.commit_multi(&paths, |_, _| {
            *calls.borrow_mut() += 1;
        });

        assert_eq!(*calls.borrow(), 3, "callback fires once per path");
        assert_eq!(report.total, 3);
        assert_eq!(report.succeeded, 0);
        assert_eq!(report.failed.len(), 3);
        for (path, _) in &report.failed {
            assert!(paths.contains(path));
        }
    }

    #[test]
    fn rename_frame_serializes_with_camel_case_tag() {
        let change = TagChange::RenameFrame {
            from: "TXXX:OLD".into(),
            to: "TXXX:NEW".into(),
        };
        let json = serde_json::to_string(&change).expect("serialize");
        assert!(json.contains("\"type\":\"renameFrame\""), "got: {json}");
        assert!(json.contains("\"from\":\"TXXX:OLD\""));
        assert!(json.contains("\"to\":\"TXXX:NEW\""));
    }

    #[test]
    fn rename_frame_round_trips() {
        let change = TagChange::RenameFrame {
            from: "MOOD".into(),
            to: "MOOD_NEW".into(),
        };
        let json = serde_json::to_string(&change).unwrap();
        let parsed: TagChange = serde_json::from_str(&json).unwrap();
        match parsed {
            TagChange::RenameFrame { from, to } => {
                assert_eq!(from, "MOOD");
                assert_eq!(to, "MOOD_NEW");
            }
            other => panic!("expected RenameFrame, got {other:?}"),
        }
    }

    #[test]
    fn batch_report_default_is_zeroed() {
        let r = BatchReport::default();
        assert_eq!(r.total, 0);
        assert_eq!(r.succeeded, 0);
        assert!(r.failed.is_empty());
    }

    fn sample_tags() -> HashMap<String, Vec<String>> {
        let mut m = HashMap::new();
        m.insert("GENRE".into(), vec!["Deep House".into()]);
        m.insert("ARTIST".into(), vec!["Bicep".into(), "Hammer".into()]);
        m.insert("BPM".into(), vec!["122".into()]);
        m.insert("EMPTY".into(), vec!["".into()]);
        m
    }

    #[test]
    fn filter_has_tag_finds_present_non_empty() {
        let tags = sample_tags();
        assert!(TagFilterQuery::HasTag { tag: "GENRE".into() }.matches(&tags));
        assert!(TagFilterQuery::HasTag { tag: "genre".into() }.matches(&tags), "case-insensitive");
        assert!(!TagFilterQuery::HasTag { tag: "EMPTY".into() }.matches(&tags), "all-empty values count as missing");
        assert!(!TagFilterQuery::HasTag { tag: "MOOD".into() }.matches(&tags));
    }

    #[test]
    fn filter_missing_inverts_has_tag() {
        let tags = sample_tags();
        assert!(TagFilterQuery::Missing { tag: "MOOD".into() }.matches(&tags));
        assert!(TagFilterQuery::Missing { tag: "EMPTY".into() }.matches(&tags));
        assert!(!TagFilterQuery::Missing { tag: "GENRE".into() }.matches(&tags));
    }

    #[test]
    fn filter_equals_modes() {
        let tags = sample_tags();
        // Exact matches case-insensitively
        assert!(TagFilterQuery::Equals {
            tag: "GENRE".into(), value: "deep house".into(), mode: MatchMode::Exact
        }.matches(&tags));
        assert!(!TagFilterQuery::Equals {
            tag: "GENRE".into(), value: "deep".into(), mode: MatchMode::Exact
        }.matches(&tags));

        // Contains
        assert!(TagFilterQuery::Equals {
            tag: "GENRE".into(), value: "house".into(), mode: MatchMode::Contains
        }.matches(&tags));
        // Multi-valued tag — match if any value matches
        assert!(TagFilterQuery::Equals {
            tag: "ARTIST".into(), value: "hammer".into(), mode: MatchMode::Exact
        }.matches(&tags));

        // Regex
        assert!(TagFilterQuery::Equals {
            tag: "BPM".into(), value: r"^1[0-9]{2}$".into(), mode: MatchMode::Regex
        }.matches(&tags));
        // Bad regex -> no match (no panic)
        assert!(!TagFilterQuery::Equals {
            tag: "BPM".into(), value: "(unclosed".into(), mode: MatchMode::Regex
        }.matches(&tags));
    }

    #[test]
    fn filter_and_or_compose() {
        let tags = sample_tags();
        let has_genre = TagFilterQuery::HasTag { tag: "GENRE".into() };
        let has_bpm = TagFilterQuery::HasTag { tag: "BPM".into() };
        let has_mood = TagFilterQuery::HasTag { tag: "MOOD".into() };

        assert!(TagFilterQuery::And {
            children: vec![has_genre.clone(), has_bpm.clone()]
        }.matches(&tags));
        assert!(!TagFilterQuery::And {
            children: vec![has_genre.clone(), has_mood.clone()]
        }.matches(&tags));
        assert!(TagFilterQuery::Or {
            children: vec![has_genre.clone(), has_mood.clone()]
        }.matches(&tags));
        assert!(!TagFilterQuery::Or {
            children: vec![has_mood.clone(), TagFilterQuery::HasTag { tag: "EMPTY".into() }]
        }.matches(&tags));
    }
}
