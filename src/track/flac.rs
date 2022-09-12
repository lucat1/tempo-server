extern crate metaflac;

use super::map::TagKey;
use super::picture::{Picture, PictureType};
use core::convert::AsRef;
use eyre::{eyre, Result};
use metaflac::block::PictureType as FLACPictureType;
use phf::phf_map;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

static TAG_MAPPING: phf::Map<&'static str, TagKey> = phf_map! {
    "ACOUSTID_ID"=>               TagKey::AcoustidID,
    "ACOUSTID_FINGERPRINT"=>               TagKey::AcoustidIDFingerprint,
    "ALBUM"=>               TagKey::Album,
    "ALBUMARTIST"=>               TagKey::AlbumArtist,
    "ALBUMARTISTSORT"=>               TagKey::AlbumArtistSortOrder,
    "ALBUMSORT"=>               TagKey::AlbumSortOrder,
    "ARRANGER"=>               TagKey::Arranger,
    "ARTIST"=>               TagKey::Artist,
    "ARTISTSORT"=>               TagKey::ArtistSortOrder,
    "ARTISTS"=>               TagKey::Artists,
    "ASIN"=>               TagKey::ASIN,
    "BARCODE"=>               TagKey::Barcode,
    "BPM"=>               TagKey::BPM,
    "CATALOGNUMBER"=>               TagKey::CatalogNumber,
    "COMMENT"=>               TagKey::Comment,
    "COMPILATION"=>               TagKey::Compilation,
    "COMPOSER"=>               TagKey::Composer,
    "COMPOSERSORT"=>               TagKey::ComposerSortOrder,
    "CONDUCTOR"=>               TagKey::Conductor,
    "COPYRIGHT"=>               TagKey::Copyright,
    "DIRECTOR"=>               TagKey::Director,
    "DISCNUMBER"=>               TagKey::DiscNumber,
    "DISCSUBTITLE"=>               TagKey::DiscSubtitle,
    "ENCODEDBY"=>               TagKey::EncodedBy,
    "ENCODERSETTINGS"=>               TagKey::EncoderSettings,
    "ENGINEER"=>               TagKey::Engineer,
    "GENRE"=>               TagKey::Genre,
    "GROUPING"=>               TagKey::Grouping,
    "KEY"=>               TagKey::InitialKey,
    "ISRC"=>               TagKey::ISRC,
    "LANGUAGE"=>               TagKey::Language,
    "LICENSE"=>               TagKey::License,
    "LYRICIST"=>               TagKey::Liricist,
    "LYRICS"=>               TagKey::Lyrics,
    "MEDIA"=>               TagKey::Media,
    "DJMIXER"=>               TagKey::MixDJ,
    "MIXER"=>               TagKey::Mixer,
    "MOOD"=>               TagKey::Mood,
    "MOVEMENTNAME"=>               TagKey::Movement,
    "MOVEMENTTOTAL"=>               TagKey::MovementCount,
    "MOVEMENT"=>               TagKey::MovementNumber,
    "MUSICBRAINZ_ARTISTID"=>               TagKey::MusicBrainzArtistID,
    "MUSICBRAINZ_DISCID"=>               TagKey::MusicBrainzDiscID,
    "MUSICBRAINZ_TRACKID"=>               TagKey::MusicBrainzRecordingID,
    "MUSICBRAINZ_ALBUMARTISTID"=>               TagKey::MusicBrainzReleaseArtistID,
    "MUSICBRAINZ_RELEASEGROUPID"=>               TagKey::MusicBrainzReleaseGroupID,
    "MUSICBRAINZ_ALBUMID"=>               TagKey::MusicBrainzReleaseID,
    "MUSICBRAINZ_RELEASETRACKID"=>               TagKey::MusicBrainzTrackID,
    "MUSICBRAINZ_TRMID"=>               TagKey::MusicBrainzTRMID,
    "MUSICBRAINZ_WORKID"=>               TagKey::MusicBrainzWorkID,
    "MUSICIP_PUID"=>               TagKey::MusicIPPUID,
    "Original Artist"=>               TagKey::OriginalArtist,
    "ORIGINALFILENAME"=>               TagKey::OriginalFilename,
    "ORIGINALYEAR"=>               TagKey::OriginalReleaseYear,
    "Producer"=>               TagKey::Producer,
    "Label"=>               TagKey::RecordLabel,
    "RELEASECOUNTRY"=>               TagKey::ReleaseCountry,
    "Year"=>               TagKey::ReleaseDate,
    "MUSICBRAINZ_ALBUMSTATUS"=>               TagKey::ReleaseStatus,
    "MUSICBRAINZ_ALBUMTYPE"=>               TagKey::ReleaseType,
    "MixArtist"=>               TagKey::Remixer,
    "REPLAYGAIN_ALBUM_GAIN"=>               TagKey::ReplayGainAlbumGain,
    "REPLAYGAIN_ALBUM_PEAK"=>               TagKey::ReplayGainAlbumPeak,
    "REPLAYGAIN_ALBUM_RANGE"=>               TagKey::ReplayGainAlbumRange,
    "REPLAYGAIN_REFERENCE_LOUDNESS"=>               TagKey::ReplayGainReferenceLoudness,
    "REPLAYGAIN_TRACK_GAIN"=>               TagKey::ReplayGainTrackGain,
    "REPLAYGAIN_TRACK_PEAK"=>               TagKey::ReplayGainTrackPeak,
    "REPLAYGAIN_TRACK_RANGE"=>               TagKey::ReplayGainTrackRange,
    "Script"=>               TagKey::Script,
    "SHOWMOVEMENT"=>               TagKey::ShowWorkAndMovement,
    "Subtitle"=>               TagKey::Subtitle,
    "Disc"=>               TagKey::TotalDiscs,
    "TRACKTOTAL"=>               TagKey::TotalTracks,
    "TOTALTRACKS"=>               TagKey::TotalTracks,
    "TRACKNUMBER"=>               TagKey::TrackNumber,
    "Title"=>               TagKey::TrackTitle,
    "TITLESORT"=>               TagKey::TrackTitleSortOrder,
    "Weblink"=>               TagKey::Website,
    "WORK"=>               TagKey::WorkTitle,
    "Writer"=>               TagKey::Writer,

    // Internal, not mapped from picard
    "LENGTH"=>               TagKey::Duration,
};

#[derive(Clone)]
pub struct Tag {
    tag: metaflac::Tag,
}

impl crate::track::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::track::Tag>>
    where
        P: AsRef<Path>,
    {
        Ok(Box::new(Tag {
            tag: metaflac::Tag::read_from_path(path)?,
        }))
    }
}

impl crate::track::Tag for Tag {
    fn clear(&mut self) {
        let map = self.get_all();
        for key in map.keys().into_iter() {
            self.tag.remove_vorbis(key);
        }
        self.set_pictures(vec![]);
    }
    fn separator(&self) -> Option<String> {
        None
    }
    fn get_str(&self, key: &str) -> Option<Vec<String>> {
        if let Some(values) = self.tag.get_vorbis(key) {
            let v: Vec<&str> = values.collect();
            if v.is_empty() {
                return None;
            }

            return Some(v.into_iter().map(|v| v.to_string()).collect());
        }
        None
    }

    fn set_str(&mut self, key: &str, values: Vec<String>) -> Result<()> {
        self.tag.set_vorbis(key, values);
        Ok(())
    }

    fn get_all(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        if let Some(vorbis) = self.tag.vorbis_comments() {
            // Get value of tag with proper separators
            vorbis.comments.iter().for_each(|(k, _)| {
                if let Some(v) = self.get_str(k) {
                    out.insert(k.to_string(), v);
                }
            });
        }
        out
    }

    fn get_pictures(&self) -> Result<Vec<Picture>> {
        Ok(self
            .tag
            .pictures()
            .map(|pic| Picture {
                mime_type: pic.mime_type.clone(),
                picture_type: match pic.picture_type {
                    FLACPictureType::Other => PictureType::Other,
                    FLACPictureType::Icon => PictureType::Icon,
                    FLACPictureType::OtherIcon => PictureType::OtherIcon,
                    FLACPictureType::CoverFront => PictureType::CoverFront,
                    FLACPictureType::CoverBack => PictureType::CoverBack,
                    FLACPictureType::Leaflet => PictureType::Leaflet,
                    FLACPictureType::Media => PictureType::Media,
                    FLACPictureType::LeadArtist => PictureType::LeadArtist,
                    FLACPictureType::Artist => PictureType::Artist,
                    FLACPictureType::Conductor => PictureType::Conductor,
                    FLACPictureType::Band => PictureType::Band,
                    FLACPictureType::Composer => PictureType::Composer,
                    FLACPictureType::Lyricist => PictureType::Lyricist,
                    FLACPictureType::RecordingLocation => PictureType::RecordingLocation,
                    FLACPictureType::DuringRecording => PictureType::DuringRecording,
                    FLACPictureType::DuringPerformance => PictureType::DuringPerformance,
                    FLACPictureType::ScreenCapture => PictureType::ScreenCapture,
                    FLACPictureType::BrightFish => PictureType::BrightFish,
                    FLACPictureType::Illustration => PictureType::Illustration,
                    FLACPictureType::BandLogo => PictureType::BandLogo,
                    FLACPictureType::PublisherLogo => PictureType::PublisherLogo,
                },
                description: pic.description.clone(),
                data: pic.data.clone(),
            })
            .collect::<Vec<_>>())
    }

    fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()> {
        // remove all the previous pictures
        for pic in self.tag.clone().pictures() {
            self.tag.remove_picture_type(pic.picture_type);
        }
        for pic in pictures {
            self.tag.add_picture(
                pic.mime_type,
                match pic.picture_type {
                    PictureType::Other => FLACPictureType::Other,
                    PictureType::Icon => FLACPictureType::Icon,
                    PictureType::OtherIcon => FLACPictureType::OtherIcon,
                    PictureType::CoverFront => FLACPictureType::CoverFront,
                    PictureType::CoverBack => FLACPictureType::CoverBack,
                    PictureType::Leaflet => FLACPictureType::Leaflet,
                    PictureType::Media => FLACPictureType::Media,
                    PictureType::LeadArtist => FLACPictureType::LeadArtist,
                    PictureType::Artist => FLACPictureType::Artist,
                    PictureType::Conductor => FLACPictureType::Conductor,
                    PictureType::Band => FLACPictureType::Band,
                    PictureType::Composer => FLACPictureType::Composer,
                    PictureType::Lyricist => FLACPictureType::Lyricist,
                    PictureType::RecordingLocation => FLACPictureType::RecordingLocation,
                    PictureType::DuringRecording => FLACPictureType::DuringRecording,
                    PictureType::DuringPerformance => FLACPictureType::DuringPerformance,
                    PictureType::ScreenCapture => FLACPictureType::ScreenCapture,
                    PictureType::BrightFish => FLACPictureType::BrightFish,
                    PictureType::Illustration => FLACPictureType::Illustration,
                    PictureType::BandLogo => FLACPictureType::BandLogo,
                    PictureType::PublisherLogo => FLACPictureType::PublisherLogo,
                },
                pic.data,
            );
        }
        Ok(())
    }

    fn str_to_key(&self, str: &str) -> Option<TagKey> {
        TAG_MAPPING.get(str).cloned()
    }
    fn key_to_str(&self, key: TagKey) -> Option<&'static str> {
        match key {
            TagKey::AcoustidID => Some("ACOUSTID_ID"),
            TagKey::AcoustidIDFingerprint => Some("ACOUSTID_FINGERPRINT"),
            TagKey::Album => Some("ALBUM"),
            TagKey::AlbumArtist => Some("ALBUMARTIST"),
            TagKey::AlbumArtistSortOrder => Some("ALBUMARTISTSORT"),
            TagKey::AlbumSortOrder => Some("ALBUMSORT"),
            TagKey::Arranger => Some("ARRANGER"),
            TagKey::Artist => Some("ARTIST"),
            TagKey::ArtistSortOrder => Some("ARTISTSORT"),
            TagKey::Artists => Some("ARTISTS"),
            TagKey::ASIN => Some("ASIN"),
            TagKey::Barcode => Some("BARCODE"),
            TagKey::BPM => Some("BPM"),
            TagKey::CatalogNumber => Some("CATALOGNUMBER"),
            TagKey::Comment => Some("COMMENT"),
            TagKey::Compilation => Some("COMPILATION"),
            TagKey::Composer => Some("COMPOSER"),
            TagKey::ComposerSortOrder => Some("COMPOSERSORT"),
            TagKey::Conductor => Some("CONDUCTOR"),
            TagKey::Copyright => Some("COPYRIGHT"),
            TagKey::Director => Some("DIRECTOR"),
            TagKey::DiscNumber => Some("DISCNUMBER"),
            TagKey::DiscSubtitle => Some("DISCSUBTITLE"),
            TagKey::EncodedBy => Some("ENCODEDBY"),
            TagKey::EncoderSettings => Some("ENCODERSETTINGS"),
            TagKey::Engineer => Some("ENGINEER"),
            TagKey::Genre => Some("GENRE"),
            TagKey::Grouping => Some("GROUPING"),
            TagKey::InitialKey => Some("KEY"),
            TagKey::ISRC => Some("ISRC"),
            TagKey::Language => Some("LANGUAGE"),
            TagKey::License => Some("LICENSE"),
            TagKey::Liricist => Some("LYRICIST"),
            TagKey::Lyrics => Some("LYRICS"),
            TagKey::Media => Some("MEDIA"),
            TagKey::MixDJ => Some("DJMIXER"),
            TagKey::Mixer => Some("MIXER"),
            TagKey::Mood => Some("MOOD"),
            TagKey::Movement => Some("MOVEMENTNAME"),
            TagKey::MovementCount => Some("MOVEMENTTOTAL"),
            TagKey::MovementNumber => Some("MOVEMENT"),
            TagKey::MusicBrainzArtistID => Some("MUSICBRAINZ_ARTISTID"),
            TagKey::MusicBrainzDiscID => Some("MUSICBRAINZ_DISCID"),
            TagKey::MusicBrainzRecordingID => Some("MUSICBRAINZ_TRACKID"),
            TagKey::MusicBrainzReleaseArtistID => Some("MUSICBRAINZ_ALBUMARTISTID"),
            TagKey::MusicBrainzReleaseGroupID => Some("MUSICBRAINZ_RELEASEGROUPID"),
            TagKey::MusicBrainzReleaseID => Some("MUSICBRAINZ_ALBUMID"),
            TagKey::MusicBrainzTrackID => Some("MUSICBRAINZ_RELEASETRACKID"),
            TagKey::MusicBrainzTRMID => Some("MUSICBRAINZ_TRMID"),
            TagKey::MusicBrainzWorkID => Some("MUSICBRAINZ_WORKID"),
            TagKey::MusicIPPUID => Some("MUSICIP_PUID"),
            TagKey::OriginalArtist => Some("Original Artist"),
            TagKey::OriginalFilename => Some("ORIGINALFILENAME"),
            TagKey::OriginalReleaseYear => Some("ORIGINALYEAR"),
            // TODO: ?
            // TagKey::Performer => Some(""),
            TagKey::Producer => Some("Producer"),
            TagKey::RecordLabel => Some("Label"),
            TagKey::ReleaseCountry => Some("RELEASECOUNTRY"),
            TagKey::ReleaseDate => Some("Year"),
            TagKey::ReleaseStatus => Some("MUSICBRAINZ_ALBUMSTATUS"),
            TagKey::ReleaseType => Some("MUSICBRAINZ_ALBUMTYPE"),
            TagKey::Remixer => Some("MixArtist"),
            TagKey::ReplayGainAlbumGain => Some("REPLAYGAIN_ALBUM_GAIN"),
            TagKey::ReplayGainAlbumPeak => Some("REPLAYGAIN_ALBUM_PEAK"),
            TagKey::ReplayGainAlbumRange => Some("REPLAYGAIN_ALBUM_RANGE"),
            TagKey::ReplayGainReferenceLoudness => Some("REPLAYGAIN_REFERENCE_LOUDNESS"),
            TagKey::ReplayGainTrackGain => Some("REPLAYGAIN_TRACK_GAIN"),
            TagKey::ReplayGainTrackPeak => Some("REPLAYGAIN_TRACK_PEAK"),
            TagKey::ReplayGainTrackRange => Some("REPLAYGAIN_TRACK_RANGE"),
            TagKey::Script => Some("Script"),
            TagKey::ShowWorkAndMovement => Some("SHOWMOVEMENT"),
            TagKey::Subtitle => Some("Subtitle"),
            TagKey::TotalDiscs => Some("Disc"),
            TagKey::TotalTracks => Some("TRACKTOTAL"),
            TagKey::TrackNumber => Some("TRACKNUMBER"),
            TagKey::TrackTitle => Some("Title"),
            TagKey::TrackTitleSortOrder => Some("TITLESORT"),
            TagKey::Website => Some("Weblink"),
            TagKey::WorkTitle => Some("WORK"),
            TagKey::Writer => Some("Writer"),

            // Internal, not mapped from picard
            TagKey::Duration => Some("LENGTH"),
            _ => None,
        }
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        self.tag.write_to_path(path).map_err(|e| eyre!(e))
    }
}
