extern crate id3;

use super::map::TagKey;
use super::picture::{Picture, PictureType};
use core::convert::AsRef;
use eyre::{eyre, Result};
use id3::frame::ExtendedText;
use id3::frame::Picture as ID3Picture;
use id3::frame::PictureType as ID3PictureType;
use id3::{Content, Frame, TagLike, Version};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Tag {
    tag: id3::Tag,
    separator: String,
}

impl crate::track::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::track::Tag>>
    where
        P: AsRef<Path>,
    {
        Ok(Box::new(Tag {
            tag: id3::Tag::read_from_path(path)?,
            // TODO
            separator: ",".to_string(),
        }))
    }
}

impl crate::track::Tag for Tag {
    fn get_str(&self, key: &str) -> Option<Vec<String>> {
        if key.len() != 4 {
            if let Some(t) = self.tag.extended_texts().find(|t| t.description == key) {
                return Some(
                    t.value
                        .split(&self.separator)
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>(),
                );
            }
            return None;
        } else if let Some(t) = self.tag.get(key) {
            if let Some(content) = t.content().text() {
                Some(content.split(&self.separator).map(String::from).collect())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn set_str(&mut self, key: &str, values: Vec<String>) -> Result<()> {
        let val = values.join(&self.separator);
        let frame = if key.len() != 4 {
            ExtendedText {
                description: key.to_string(),
                value: val,
            }
            .into()
        } else {
            Frame::text(key, val)
        };
        self.tag.add_frame(frame);
        Ok(())
    }

    fn get_all(&self) -> HashMap<String, Vec<String>> {
        let mut tags = HashMap::new();
        for frame in self.tag.frames() {
            if let Content::Text(v) = frame.content() {
                tags.insert(
                    frame.id().to_owned(),
                    v.split(&self.separator).map(String::from).collect(),
                );
            }
        }
        // Add TXXX
        for extended in self.tag.extended_texts() {
            tags.insert(
                extended.description.to_string(),
                extended
                    .value
                    .split(&self.separator)
                    .map(String::from)
                    .collect(),
            );
        }
        tags
    }

    fn get_pictures(&self) -> Result<Vec<Picture>> {
        Ok(self
            .tag
            .pictures()
            .map(|pic| Picture {
                mime_type: pic.mime_type.clone(),
                picture_type: match pic.picture_type {
                    ID3PictureType::Other => PictureType::Other,
                    ID3PictureType::Icon => PictureType::Icon,
                    ID3PictureType::OtherIcon => PictureType::OtherIcon,
                    ID3PictureType::CoverFront => PictureType::CoverFront,
                    ID3PictureType::CoverBack => PictureType::CoverBack,
                    ID3PictureType::Leaflet => PictureType::Leaflet,
                    ID3PictureType::Media => PictureType::Media,
                    ID3PictureType::LeadArtist => PictureType::LeadArtist,
                    ID3PictureType::Artist => PictureType::Artist,
                    ID3PictureType::Conductor => PictureType::Conductor,
                    ID3PictureType::Band => PictureType::Band,
                    ID3PictureType::Composer => PictureType::Composer,
                    ID3PictureType::Lyricist => PictureType::Lyricist,
                    ID3PictureType::RecordingLocation => PictureType::RecordingLocation,
                    ID3PictureType::DuringRecording => PictureType::DuringRecording,
                    ID3PictureType::DuringPerformance => PictureType::DuringPerformance,
                    ID3PictureType::ScreenCapture => PictureType::ScreenCapture,
                    ID3PictureType::BrightFish => PictureType::BrightFish,
                    ID3PictureType::Illustration => PictureType::Illustration,
                    ID3PictureType::BandLogo => PictureType::BandLogo,
                    ID3PictureType::PublisherLogo => PictureType::PublisherLogo,
                    ID3PictureType::Undefined(_) => PictureType::Other,
                },
                description: pic.description.clone(),
                data: pic.data.clone(),
            })
            .collect::<Vec<_>>())
    }

    fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()> {
        // remove all the previous pictures
        let retag = self.tag.clone();
        for pic in retag.pictures() {
            self.tag.remove_picture_by_type(pic.picture_type);
        }
        for pic in pictures {
            self.tag.add_frame(ID3Picture {
                mime_type: pic.mime_type,
                picture_type: match pic.picture_type {
                    PictureType::Other => ID3PictureType::Other,
                    PictureType::Icon => ID3PictureType::Icon,
                    PictureType::OtherIcon => ID3PictureType::OtherIcon,
                    PictureType::CoverFront => ID3PictureType::CoverFront,
                    PictureType::CoverBack => ID3PictureType::CoverBack,
                    PictureType::Leaflet => ID3PictureType::Leaflet,
                    PictureType::Media => ID3PictureType::Media,
                    PictureType::LeadArtist => ID3PictureType::LeadArtist,
                    PictureType::Artist => ID3PictureType::Artist,
                    PictureType::Conductor => ID3PictureType::Conductor,
                    PictureType::Band => ID3PictureType::Band,
                    PictureType::Composer => ID3PictureType::Composer,
                    PictureType::Lyricist => ID3PictureType::Lyricist,
                    PictureType::RecordingLocation => ID3PictureType::RecordingLocation,
                    PictureType::DuringRecording => ID3PictureType::DuringRecording,
                    PictureType::DuringPerformance => ID3PictureType::DuringPerformance,
                    PictureType::ScreenCapture => ID3PictureType::ScreenCapture,
                    PictureType::BrightFish => ID3PictureType::BrightFish,
                    PictureType::Illustration => ID3PictureType::Illustration,
                    PictureType::BandLogo => ID3PictureType::BandLogo,
                    PictureType::PublisherLogo => ID3PictureType::PublisherLogo,
                },
                description: pic.description,
                data: pic.data,
            });
        }
        Ok(())
    }

    fn str_to_key(&self, str: &str) -> Option<TagKey> {
        unimplemented!()
    }
    fn key_to_str(&self, key: TagKey) -> Option<&'static str> {
        match key {
            TagKey::AcoustidID => Some("Acoustid Id"),
            TagKey::AcoustidIDFingerprint => Some("Acoustid Fingerprint"),
            TagKey::Album => Some("TALB"),
            TagKey::AlbumArtist => Some("TPE2"),
            TagKey::AlbumArtistSortOrder => Some("TSO2"),
            TagKey::AlbumSortOrder => Some("TSOA"),
            TagKey::Arranger => Some("TIPL:arranger"), // TODO: or IPLS:
            TagKey::Artist => Some("TPE1"),
            TagKey::ArtistSortOrder => Some("TSOP"),
            TagKey::Artists => Some("ARTISTS"),
            TagKey::ASIN => Some("ASIN"),
            TagKey::Barcode => Some("BARCODE"),
            TagKey::BPM => Some("TBPM"),
            TagKey::CatalogNumber => Some("CATALOGNUMBER"),
            TagKey::Comment => Some("description"),
            TagKey::Compilation => Some("TCMP"),
            TagKey::Composer => Some("TCOM"),
            TagKey::ComposerSortOrder => Some("TSOC"),
            TagKey::Conductor => Some("TPE3"),
            TagKey::Copyright => Some("TCOP"),
            TagKey::Director => Some("DIRECTOR"),
            TagKey::DiscNumber => Some("TPOS"),
            TagKey::DiscSubtitle => Some("TSST"),
            TagKey::EncodedBy => Some("TENC"),
            TagKey::EncoderSettings => Some("TSSE"),
            TagKey::Engineer => Some("TIPL:engineer"), // TODO: or IPLS:
            TagKey::Genre => Some("TCON"),
            TagKey::Grouping => Some("TIT1"), // TODO: or GRP1
            TagKey::InitialKey => Some("TKEY"),
            TagKey::ISRC => Some("TSRC"),
            TagKey::Language => Some("TLAN"),
            TagKey::License => Some("WCOP"), // TODO: or LICENSE
            TagKey::Liricist => Some("TEXT"),
            TagKey::Lyrics => Some("USLT:description"),
            TagKey::Media => Some("TMED"),
            TagKey::MixDJ => Some("TIPL:DJ-mix"), // TODO: or IPLS
            TagKey::Mixer => Some("TIPL:mix"),    // TODO: or IPLS
            TagKey::Mood => Some("TMOO"),
            TagKey::Movement => Some("MVNM"),
            TagKey::MovementCount => Some("MVIN"),
            TagKey::MovementNumber => Some("MVIN"),
            TagKey::MusicBrainzArtistID => Some("MusicBrainz Artist Id"),
            TagKey::MusicBrainzDiscID => Some("MusicBrainz Disc Id"),
            TagKey::MusicBrainzOriginalArtistID => Some("MusicBrainz Original Artist Id"),
            TagKey::MusicBrainzOriginalReleaseID => Some("MusicBrainz Original Album Id"),
            TagKey::MusicBrainzRecordingID => Some("UFID:http://musicbrainz.org"),
            TagKey::MusicBrainzReleaseArtistID => Some("MusicBrainz Album Artist Id"),
            TagKey::MusicBrainzReleaseGroupID => Some("MusicBrainz Release Group Id"),
            TagKey::MusicBrainzReleaseID => Some("MusicBrainz Album Id"),
            TagKey::MusicBrainzTrackID => Some("MusicBrainz Release Track Id"),
            TagKey::MusicBrainzTRMID => Some("MusicBrainz TRM Id"),
            TagKey::MusicBrainzWorkID => Some("MusicBrainz Work Id"),
            TagKey::MusicIPFingerprint => Some("MusicMagic Fingerprint"),
            TagKey::MusicIPPUID => Some("MusicIP PUID"),
            TagKey::OriginalAlbum => Some("TOAL"),
            TagKey::OriginalArtist => Some("TOPE"),
            TagKey::OriginalFilename => Some("TOFN"),
            TagKey::OriginalReleaseDate => Some("TDOR"),
            TagKey::Performer => Some("TMCL:instrument"),
            TagKey::Producer => Some("TIPL:producer"),
            TagKey::Rating => Some("POPM"),
            TagKey::RecordLabel => Some("TPUB"),
            TagKey::ReleaseCountry => Some("MusicBrainz Album Release Country"),
            TagKey::ReleaseDate => Some("TDRC"),
            TagKey::ReleaseStatus => Some("MusicBrainz Album Status"),
            TagKey::ReleaseType => Some("MusicBrainz Album Type"),
            TagKey::Remixer => Some("TPE4"),
            TagKey::ReplayGainAlbumGain => Some("REPLAYGAIN_ALBUM_GAIN"),
            TagKey::ReplayGainAlbumPeak => Some("REPLAYGAIN_ALBUM_PEAK"),
            TagKey::ReplayGainAlbumRange => Some("REPLAYGAIN_ALBUM_RANGE"),
            TagKey::ReplayGainReferenceLoudness => Some("REPLAYGAIN_REFERENCE_LOUDNESS"),
            TagKey::ReplayGainTrackGain => Some("REPLAYGAIN_TRACK_GAIN"),
            TagKey::ReplayGainTrackPeak => Some("REPLAYGAIN_TRACK_PEAK"),
            TagKey::ReplayGainTrackRange => Some("REPLAYGAIN_TRACK_RANGE"),
            TagKey::Script => Some("SCRIPT"),
            TagKey::ShowWorkAndMovement => Some("SHOWMOVEMENT"),
            TagKey::Subtitle => Some("TIT3"),
            TagKey::TotalDiscs => Some("TPOS"),
            TagKey::TotalTracks => Some("TRCK"),
            TagKey::TrackTitle => Some("TRCK"),
            TagKey::TrackTitleSortOrder => Some("TIT2"),
            TagKey::Website => Some("TSOT"),
            TagKey::WorkTitle => Some("WORK"), // or TIT1
            TagKey::Writer => Some("Writer"),
            _ => None,
        }
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        self.tag
            .write_to_path(path, Version::Id3v24)
            .map_err(|e| eyre!(e))
    }
}
