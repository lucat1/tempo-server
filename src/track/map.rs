use std::collections::HashMap;

use super::TrackFile;
use eyre::{eyre, Result};

// Taken from Music Brainz Picard as a reference:
// https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html
#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum TagKey {
    AcoustidID,
    AcoustidIDFingerprint,
    Album,
    AlbumArtist,
    AlbumArtistSortOrder,
    AlbumSortOrder,
    Arranger,
    Artist,
    ArtistSortOrder,
    Artists,
    ASIN,
    Barcode,
    BPM,
    CatalogNumber,
    Comment,
    Compilation,
    Composer,
    ComposerSortOrder,
    Conductor,
    Copyright,
    Director,
    DiscNumber,
    DiscSubtitle,
    EncodedBy,
    EncoderSettings,
    Engineer,
    GaplessPlayback,
    Genre,
    Grouping,
    InitialKey,
    ISRC,
    Language,
    License,
    Liricist,
    Lyrics,
    Media,
    MixDJ,
    Mixer,
    Mood,
    Movement,
    MovementCount,
    MovementNumber,
    MusicBrainzArtistID,
    MusicBrainzDiscID,
    MusicBrainzOriginalArtistID,
    MusicBrainzOriginalReleaseID,
    MusicBrainzRecordingID,
    MusicBrainzReleaseArtistID,
    MusicBrainzReleaseGroupID,
    MusicBrainzReleaseID,
    MusicBrainzTrackID,
    MusicBrainzTRMID,
    MusicBrainzWorkID,
    MusicIPFingerprint,
    MusicIPPUID,
    OriginalAlbum,
    OriginalArtist,
    OriginalFilename,
    OriginalReleaseDate,
    OriginalReleaseYear,
    Performer,
    Podcast,
    PodcastURL,
    Producer,
    Rating,
    RecordLabel,
    ReleaseCountry,
    ReleaseDate,
    ReleaseStatus,
    ReleaseType,
    Remixer,
    ReplayGainAlbumGain,
    ReplayGainAlbumPeak,
    ReplayGainAlbumRange,
    ReplayGainReferenceLoudness,
    ReplayGainTrackGain,
    ReplayGainTrackPeak,
    ReplayGainTrackRange,
    Script,
    ShowName,
    ShowNameSortOrder,
    ShowWorkAndMovement,
    Subtitle,
    TotalDiscs,
    TotalTracks,
    TrackTitle,
    TrackTitleSortOrder,
    Website,
    WorkTitle,
    Writer,
}

impl TrackFile {
    pub fn get_tag(&self, key: TagKey) -> Result<Vec<String>> {
        let keystr = self.tag.key_to_str(key).ok_or(eyre!(
            "The {:?} key is not supported in the output format {:?}",
            key,
            self.format
        ))?;
        self.tag
            .get_str(keystr)
            .ok_or(eyre!("Could not read tag {:?} as {}", key, keystr))
    }

    pub fn set_tag(&mut self, key: TagKey, values: Vec<String>) -> Result<()> {
        let keystr = self.tag.key_to_str(key).ok_or(eyre!(
            "The {:?} key is not supported in the output format {:?}",
            key,
            self.format
        ))?;
        self.tag.set_str(keystr, values)
    }

    pub fn tags(&self) -> HashMap<TagKey, Vec<String>> {
        let mut map = HashMap::new();
        for (key, value) in self.tag.get_all() {
            if let Some(k) = self.tag.str_to_key(key.as_str()) {
                map.insert(k, value);
            }
        }
        map
    }
}
