extern crate id3;

use super::key::TagKey;
use super::picture::{Picture, PictureType};
use core::convert::AsRef;
use eyre::{eyre, Result};
use id3::frame::ExtendedText;
use id3::frame::Picture as ID3Picture;
use id3::frame::PictureType as ID3PictureType;
use id3::{Content, Frame, TagLike, Version};
use std::collections::HashMap;
use std::path::Path;

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
    fn clear(&mut self) -> Result<()> {
        let map = self.get_all();
        for key in map.keys() {
            self.tag.remove(key);
        }
        self.set_pictures(vec![])?;
        Ok(())
    }
    fn separator(&self) -> Option<String> {
        Some(self.separator.clone())
    }
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
            None
        } else {
            self.tag.get(key).and_then(|t| {
                t.content()
                    .text()
                    .map(|content| content.split(&self.separator).map(String::from).collect())
            })
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
        self.tag
            .pictures()
            .map(|pic| {
                Ok(Picture {
                    mime_type: pic.mime_type.parse()?,
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
            })
            .collect::<Result<Vec<_>>>()
    }

    fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()> {
        // remove all the previous pictures
        let retag = self.tag.clone();
        for pic in retag.pictures() {
            self.tag.remove_picture_by_type(pic.picture_type);
        }
        for pic in pictures {
            self.tag.add_frame(ID3Picture {
                mime_type: pic.mime_type.to_string(),
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
        match str {
            "TXXX:Acoustid Id" => Some(TagKey::AcoustidID),
            "TXXX:Acoustid Fingerprint" => Some(TagKey::AcoustidIDFingerprint),
            "TALB" => Some(TagKey::Album),
            "TPE2" => Some(TagKey::AlbumArtist),
            "TSO2" => Some(TagKey::AlbumArtistSortOrder),
            "TSOA" => Some(TagKey::AlbumSortOrder),
            "TIPL:arranger" => Some(TagKey::Arranger),
            "IPLS:arranger" => Some(TagKey::Arranger),
            "TPE1" => Some(TagKey::Artist),
            "TSOP" => Some(TagKey::ArtistSortOrder),
            "TXXX:ARTISTS" => Some(TagKey::Artists),
            "TXXX:ASIN" => Some(TagKey::ASIN),
            "TXXX:BARCODE" => Some(TagKey::Barcode),
            "TBPM" => Some(TagKey::BPM),
            "TXXX:CATALOGNUMBER" => Some(TagKey::CatalogNumber),
            "COMM:description" => Some(TagKey::Comment),
            "TCMP" => Some(TagKey::Compilation),
            "TCOM" => Some(TagKey::Composer),
            "TSOC" => Some(TagKey::ComposerSortOrder),
            "TXXX:COMPOSERSORT" => Some(TagKey::ComposerSortOrder),
            "TPE3" => Some(TagKey::Conductor),
            "TCOP" => Some(TagKey::Copyright),
            "TXXX:DIRECTOR" => Some(TagKey::Director),
            // NOTE: this hold both MovementNumber and MovementTotal in a "../.." string
            "TPOS" => Some(TagKey::DiscNumber),
            "TSST" => Some(TagKey::DiscSubtitle),
            "TENC" => Some(TagKey::EncodedBy),
            "TSSE" => Some(TagKey::EncoderSettings),
            "TIPL:engineer" => Some(TagKey::Engineer),
            "IPLS:engineer" => Some(TagKey::Engineer),
            "TCON" => Some(TagKey::Genre),
            "TIT1" => Some(TagKey::Grouping),
            "GRP1" => Some(TagKey::Grouping),
            "TKEY" => Some(TagKey::InitialKey),
            "TSRC" => Some(TagKey::ISRC),
            "TLAN" => Some(TagKey::Language),
            "WCOP" => Some(TagKey::License),
            "TXXX:LICENSE" => Some(TagKey::License),
            "TEXT" => Some(TagKey::Lyricist),
            "USLT:description" => Some(TagKey::Lyrics),
            "TMED" => Some(TagKey::Media),
            "TIPL:DJ-mix" => Some(TagKey::MixDJ),
            "IPLS:DJ-mix" => Some(TagKey::MixDJ),
            "TIPL:mix" => Some(TagKey::Mixer),
            "IPLS:mix" => Some(TagKey::Mixer),
            "TMOO" => Some(TagKey::Mood),
            "MVNM" => Some(TagKey::Movement),
            // NOTE: this hold both MovementNumber and MovementTotal in a "../.." string
            "MVIN" => Some(TagKey::MovementNumber),
            "TXXX:MusicBrainz Artist Id" => Some(TagKey::MusicBrainzArtistID),
            "TXXX:MusicBrainz Disc Id" => Some(TagKey::MusicBrainzDiscID),
            "TXXX:MusicBrainz Original Artist Id" => Some(TagKey::MusicBrainzOriginalArtistID),
            "TXXX:MusicBrainz Original Album Id" => Some(TagKey::MusicBrainzOriginalReleaseID),
            "UFID:http://musicbrainz.org" => Some(TagKey::MusicBrainzRecordingID),
            "TXXX:MusicBrainz Album Artist Id" => Some(TagKey::MusicBrainzReleaseArtistID),
            "TXXX:MusicBrainz Release Group Id" => Some(TagKey::MusicBrainzReleaseGroupID),
            "TXXX:MusicBrainz Album Id" => Some(TagKey::MusicBrainzReleaseID),
            "TXXX:MusicBrainz Release Track Id" => Some(TagKey::MusicBrainzTrackID),
            "TXXX:MusicBrainz TRM Id" => Some(TagKey::MusicBrainzTRMID),
            "TXXX:MusicBrainz Work Id" => Some(TagKey::MusicBrainzWorkID),
            "TXXX:MusicMagic Fingerprint" => Some(TagKey::MusicIPFingerprint),
            "TXXX:MusicIP PUID" => Some(TagKey::MusicIPPUID),
            "TOAL" => Some(TagKey::OriginalAlbum),
            "TOPE" => Some(TagKey::OriginalArtist),
            "TOFN" => Some(TagKey::OriginalFilename),
            "TDOR" => Some(TagKey::OriginalReleaseDate),
            "TORY" => Some(TagKey::OriginalReleaseDate),
            "TMCL:instrument" => Some(TagKey::Performer),
            "IPLS:instrument" => Some(TagKey::Performer),
            "TIPL:producer" => Some(TagKey::Producer),
            "IPLS:producer" => Some(TagKey::Producer),
            "POPM" => Some(TagKey::Rating),
            "TPUB" => Some(TagKey::RecordLabel),
            "TXX:MusicBrainz Album Release Country" => Some(TagKey::ReleaseCountry),
            "TDRC" => Some(TagKey::ReleaseDate),
            "TYER" => Some(TagKey::ReleaseDate),
            "TDAT" => Some(TagKey::ReleaseDate),
            "TXXX:MusicBrainz Album Status" => Some(TagKey::ReleaseStatus),
            "TXXX:MusicBrainz Album Type" => Some(TagKey::ReleaseType),
            "TPE4" => Some(TagKey::Remixer),
            "TXXX:REPLAYGAIN_ALBUM_GAIN" => Some(TagKey::ReplayGainAlbumGain),
            "TXXX:REPLAYGAIN_ALBUM_PEAK" => Some(TagKey::ReplayGainAlbumPeak),
            "TXXX:REPLAYGAIN_ALBUM_RANGE" => Some(TagKey::ReplayGainAlbumRange),
            "TXXX:REPLAYGAIN_REFERENCE_LOUDNESS" => Some(TagKey::ReplayGainReferenceLoudness),
            "TXXX:REPLAYGAIN_TRACK_GAIN" => Some(TagKey::ReplayGainTrackGain),
            "TXXX:REPLAYGAIN_TRACK_PEAK" => Some(TagKey::ReplayGainTrackPeak),
            "TXXX:REPLAYGAIN_TRACK_RANGE" => Some(TagKey::ReplayGainTrackRange),
            "TXXX:SCRIPT" => Some(TagKey::Script),
            "TXXX:SHOWMOVEMENT" => Some(TagKey::ShowWorkAndMovement),
            "TIT3" => Some(TagKey::Subtitle),
            // NOTE: this hold both MovementNumber and MovementTotal in a "../.." string
            "TRCK" => Some(TagKey::TrackNumber),
            "TIT2" => Some(TagKey::TrackTitle),
            "TSOT" => Some(TagKey::TrackTitleSortOrder),
            "WOAR" => Some(TagKey::Website),
            // NOTE: WorkTitle is also found as TIT1 but it's not included here
            // in order not to overwrite the Grouping tag
            "TXXX:WORK" => Some(TagKey::WorkTitle),
            "TXX:Writer" => Some(TagKey::Writer),

            // Internal, not mapped from picard
            "TLEN" => Some(TagKey::Duration),

            _ => None,
        }
    }
    fn key_to_str(&self, key: TagKey) -> Vec<&'static str> {
        match key {
            TagKey::AcoustidID => vec!["TXXX:Acoustid Id"],
            TagKey::AcoustidIDFingerprint => vec!["TXXX:Acoustid Fingerprint"],
            TagKey::Album => vec!["TALB"],
            TagKey::AlbumArtist => vec!["TPE2"],
            TagKey::AlbumArtistSortOrder => vec!["TSO2"],
            TagKey::AlbumSortOrder => vec!["TSOA"],
            TagKey::Arranger => vec!["TIPL:arranger", "IPLS:arranger"],
            TagKey::Artist => vec!["TPE1"],
            TagKey::ArtistSortOrder => vec!["TSOP"],
            TagKey::Artists => vec!["TXXX:ARTISTS"],
            TagKey::ASIN => vec!["TXXX:ASIN"],
            TagKey::Barcode => vec!["TXXX:BARCODE"],
            TagKey::BPM => vec!["TBPM"],
            TagKey::CatalogNumber => vec!["TXXX:CATALOGNUMBER"],
            TagKey::Comment => vec!["COMM:description"],
            TagKey::Compilation => vec!["TCMP"],
            TagKey::Composer => vec!["TCOM"],
            TagKey::ComposerSortOrder => vec!["TSOC", "TXXX:COMPOSERSORT"],
            TagKey::Conductor => vec!["TPE3"],
            TagKey::Copyright => vec!["TCOP"],
            TagKey::Director => vec!["TXXX:DIRECTOR"],
            TagKey::DiscNumber => vec!["TPOS"],
            TagKey::DiscSubtitle => vec!["TSST"],
            TagKey::EncodedBy => vec!["TENC"],
            TagKey::EncoderSettings => vec!["TSSE"],
            TagKey::Engineer => vec!["TIPL:engineer", "IPLS:engineer"],
            TagKey::Genre => vec!["TCON"],
            TagKey::Grouping => vec!["TIT1", "GRP1"],
            TagKey::InitialKey => vec!["TKEY"],
            TagKey::ISRC => vec!["TSRC"],
            TagKey::Language => vec!["TLAN"],
            TagKey::License => vec!["WCOP", "TXXX:LICENSE"],
            TagKey::Lyricist => vec!["TEXT"],
            TagKey::Lyrics => vec!["USLT:description"],
            TagKey::Media => vec!["TMED"],
            TagKey::MixDJ => vec!["TIPL:DJ-mix", "IPLS:DJ-mix"],
            TagKey::Mixer => vec!["TIPL:mix", "IPLS:mix"],
            TagKey::Mood => vec!["TMOO"],
            // NOTE: this hold both MovementNumber and MovementTotal in a "../.." string
            TagKey::Movement => vec!["MVNM"],
            TagKey::MusicBrainzArtistID => vec!["TXXX:MusicBrainz Artist Id"],
            TagKey::MusicBrainzDiscID => vec!["TXXX:MusicBrainz Disc Id"],
            TagKey::MusicBrainzOriginalArtistID => vec!["TXXX:MusicBrainz Original Artist Id"],
            TagKey::MusicBrainzOriginalReleaseID => vec!["TXXX:MusicBrainz Original Album Id"],
            TagKey::MusicBrainzRecordingID => vec!["UFID:http://musicbrainz.org"],
            TagKey::MusicBrainzReleaseArtistID => vec!["TXXX:MusicBrainz Album Artist Id"],
            TagKey::MusicBrainzReleaseGroupID => vec!["TXXX:MusicBrainz Release Group Id"],
            TagKey::MusicBrainzReleaseID => vec!["TXXX:MusicBrainz Album Id"],
            TagKey::MusicBrainzTrackID => vec!["TXXX:MusicBrainz Release Track Id"],
            TagKey::MusicBrainzTRMID => vec!["TXXX:MusicBrainz TRM Id"],
            TagKey::MusicBrainzWorkID => vec!["TXXX:MusicBrainz Work Id"],
            TagKey::MusicIPFingerprint => vec!["TXXX:MusicMagic Fingerprint"],
            TagKey::MusicIPPUID => vec!["TXXX:MusicIP PUID"],
            TagKey::OriginalAlbum => vec!["TOAL"],
            TagKey::OriginalArtist => vec!["TOPE"],
            TagKey::OriginalFilename => vec!["TOFN"],
            TagKey::OriginalReleaseDate => vec!["TDOR", "TORY"],
            TagKey::Performer => vec!["TMCL:instrument", "IPLS:instrument"],
            TagKey::Producer => vec!["TIPL:producer", "IPLS:producer"],
            TagKey::Rating => vec!["POPM"],
            TagKey::RecordLabel => vec!["TPUB"],
            TagKey::ReleaseCountry => vec!["MusicBrainz Album Release Country"],
            TagKey::ReleaseDate => vec!["TDRC", "TYER", "TDAT"],
            TagKey::ReleaseStatus => vec!["TXXX:MusicBrainz Album Status"],
            TagKey::ReleaseType => vec!["TXXX:MusicBrainz Album Type"],
            TagKey::Remixer => vec!["TPE4"],
            TagKey::ReplayGainAlbumGain => vec!["TXXX:REPLAYGAIN_ALBUM_GAIN"],
            TagKey::ReplayGainAlbumPeak => vec!["TXXX:REPLAYGAIN_ALBUM_PEAK"],
            TagKey::ReplayGainAlbumRange => vec!["TXXX:REPLAYGAIN_ALBUM_RANGE"],
            TagKey::ReplayGainReferenceLoudness => vec!["TXXX:REPLAYGAIN_REFERENCE_LOUDNESS"],
            TagKey::ReplayGainTrackGain => vec!["TXXX:REPLAYGAIN_TRACK_GAIN"],
            TagKey::ReplayGainTrackPeak => vec!["TXXX:REPLAYGAIN_TRACK_PEAK"],
            TagKey::ReplayGainTrackRange => vec!["TXXX:REPLAYGAIN_TRACK_RANGE"],
            TagKey::Script => vec!["TXXX:SCRIPT"],
            TagKey::ShowWorkAndMovement => vec!["TXXX:SHOWMOVEMENT"],
            TagKey::Subtitle => vec!["TIT3"],
            // NOTE: this hold both MovementNumber and MovementTotal in a "../.." string
            TagKey::TrackNumber => vec!["TRCK"],
            TagKey::TrackTitle => vec!["TIT2"],
            TagKey::TrackTitleSortOrder => vec!["TSOT"],
            TagKey::Website => vec!["WOAR"],
            // NOTE: WorkTitle is also found as TIT1 but it's not included here
            // in order not to overwrite the Grouping tag
            TagKey::WorkTitle => vec!["TXXX:WORK", "TIT1"],
            TagKey::Writer => vec!["TXXX:Writer"],

            // Internal, not mapped from picard
            TagKey::Duration => vec!["TLEN"],
            _ => vec![],
        }
    }

    fn write_to_path(&mut self, path: &Path) -> Result<()> {
        self.tag
            .write_to_path(path, Version::Id3v24)
            .map_err(|e| eyre!(e))
    }
}
