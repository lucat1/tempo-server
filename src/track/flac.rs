extern crate metaflac;

use super::key::TagKey;
use super::picture::{Picture, PictureType};
use core::convert::AsRef;
use eyre::{eyre, Result};
use metaflac::block::PictureType as FLACPictureType;
use std::collections::HashMap;
use std::path::Path;

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
    fn clear(&mut self) -> Result<()> {
        let map = self.get_all();
        for key in map.keys() {
            self.tag.remove_vorbis(key);
        }
        self.set_pictures(vec![])?;
        Ok(())
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
        self.tag
            .pictures()
            .map(|pic| {
                Ok(Picture {
                    mime_type: pic.mime_type.parse()?,
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
            })
            .collect::<Result<Vec<_>>>()
    }

    fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()> {
        // remove all the previous pictures
        for pic in self.tag.clone().pictures() {
            self.tag.remove_picture_type(pic.picture_type);
        }
        for pic in pictures {
            self.tag.add_picture(
                pic.mime_type.to_string(),
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

    fn key_to_str(&self, key: TagKey) -> Vec<&'static str> {
        match key {
            TagKey::AcoustidID => vec!["ACOUSTID_ID"],
            TagKey::AcoustidIDFingerprint => vec!["ACOUSTID_FINGERPRINT"],
            TagKey::Album => vec!["ALBUM"],
            TagKey::AlbumArtist => vec!["ALBUMARTIST", "ALBUM ARTIST", "ALBUMARTIST_CREDIT"],
            TagKey::AlbumArtistSortOrder => vec!["ALBUMARTISTSORT"],
            TagKey::AlbumSortOrder => vec!["ALBUMSORT"],
            TagKey::Arranger => vec!["ARRANGER"],
            TagKey::Artist => vec!["ARTIST", "ARTIST_CREDIT"],
            TagKey::ArtistSortOrder => vec!["ARTISTSORT"],
            TagKey::Artists => vec!["ARTISTS"],
            TagKey::ASIN => vec!["ASIN"],
            TagKey::Barcode => vec!["BARCODE"],
            TagKey::BPM => vec!["BPM"],
            TagKey::CatalogNumber => vec!["CATALOGNUMBER", "LABELNO"],
            TagKey::Comment => vec!["COMMENT"],
            TagKey::Compilation => vec!["COMPILATION"],
            TagKey::Composer => vec!["COMPOSER"],
            TagKey::ComposerSortOrder => vec!["COMPOSERSORT"],
            TagKey::Conductor => vec!["CONDUCTOR"],
            TagKey::Copyright => vec!["COPYRIGHT"],
            TagKey::Director => vec!["DIRECTOR"],
            TagKey::DiscNumber => vec!["DISCNUMBER"],
            TagKey::DiscSubtitle => vec!["DISCSUBTITLE"],
            TagKey::EncodedBy => vec!["ENCODEDBY"],
            TagKey::EncoderSettings => vec!["ENCODERSETTINGS"],
            TagKey::Engineer => vec!["ENGINEER"],
            TagKey::Genre => vec!["GENRE"],
            TagKey::Grouping => vec!["GROUPING"],
            TagKey::InitialKey => vec!["KEY"],
            TagKey::ISRC => vec!["ISRC"],
            TagKey::Language => vec!["LANGUAGE"],
            TagKey::License => vec!["LICENSE"],
            TagKey::Lyricist => vec!["LYRICIST"],
            TagKey::Lyrics => vec!["LYRICS"],
            TagKey::Media => vec!["MEDIA"],
            TagKey::MixDJ => vec!["DJMIXER"],
            TagKey::Mixer => vec!["MIXER"],
            TagKey::Mood => vec!["MOOD"],
            TagKey::Movement => vec!["MOVEMENTNAME"],
            TagKey::MovementCount => vec!["MOVEMENTTOTAL"],
            TagKey::MovementNumber => vec!["MOVEMENT"],
            TagKey::MusicBrainzArtistID => vec!["MUSICBRAINZ_ARTISTID"],
            TagKey::MusicBrainzDiscID => vec!["MUSICBRAINZ_DISCID"],
            TagKey::MusicBrainzRecordingID => vec!["MUSICBRAINZ_TRACKID"],
            TagKey::MusicBrainzReleaseArtistID => vec!["MUSICBRAINZ_ALBUMARTISTID"],
            TagKey::MusicBrainzReleaseGroupID => vec!["MUSICBRAINZ_RELEASEGROUPID"],
            TagKey::MusicBrainzReleaseID => vec!["MUSICBRAINZ_ALBUMID"],
            TagKey::MusicBrainzTrackID => vec!["MUSICBRAINZ_RELEASETRACKID"],
            TagKey::MusicBrainzTRMID => vec!["MUSICBRAINZ_TRMID"],
            TagKey::MusicBrainzWorkID => vec!["MUSICBRAINZ_WORKID"],
            TagKey::MusicIPPUID => vec!["MUSICIP_PUID"],
            TagKey::OriginalArtist => vec!["Original Artist"],
            TagKey::OriginalFilename => vec!["ORIGINALFILENAME"],
            TagKey::OriginalReleaseDate => vec!["ORIGINALDATE"],
            TagKey::OriginalReleaseYear => vec!["ORIGINALYEAR"],
            TagKey::Performer => vec!["PERFORMER"],
            TagKey::Producer => vec!["PRODUCER"],
            TagKey::RecordLabel => vec!["Label"],
            TagKey::ReleaseCountry => vec!["RELEASECOUNTRY"],
            TagKey::ReleaseDate => vec!["DATE"],
            TagKey::ReleaseYear => vec!["YEAR"],
            TagKey::ReleaseStatus => vec!["MUSICBRAINZ_ALBUMSTATUS"],
            TagKey::ReleaseType => vec!["MUSICBRAINZ_ALBUMTYPE"],
            TagKey::Remixer => vec!["MixArtist"],
            TagKey::ReplayGainAlbumGain => vec!["REPLAYGAIN_ALBUM_GAIN"],
            TagKey::ReplayGainAlbumPeak => vec!["REPLAYGAIN_ALBUM_PEAK"],
            TagKey::ReplayGainAlbumRange => vec!["REPLAYGAIN_ALBUM_RANGE"],
            TagKey::ReplayGainReferenceLoudness => vec!["REPLAYGAIN_REFERENCE_LOUDNESS"],
            TagKey::ReplayGainTrackGain => vec!["REPLAYGAIN_TRACK_GAIN"],
            TagKey::ReplayGainTrackPeak => vec!["REPLAYGAIN_TRACK_PEAK"],
            TagKey::ReplayGainTrackRange => vec!["REPLAYGAIN_TRACK_RANGE"],
            TagKey::Script => vec!["Script"],
            TagKey::ShowWorkAndMovement => vec!["SHOWMOVEMENT"],
            TagKey::Subtitle => vec!["Subtitle"],
            TagKey::TotalDiscs => vec!["TOTALDISCS", "DISCTOTAL"],
            TagKey::TotalTracks => vec!["TRACKTOTAL", "TOTALTRACKS"],
            TagKey::TrackNumber => vec!["TRACKNUMBER"],
            TagKey::TrackTitle => vec!["Title"],
            TagKey::TrackTitleSortOrder => vec!["TITLESORT"],
            TagKey::Website => vec!["Weblink"],
            TagKey::WorkTitle => vec!["WORK"],
            TagKey::Writer => vec!["Writer"],

            // Internal, not mapped from picard
            TagKey::Duration => vec!["LENGTH"],
            _ => vec![],
        }
    }

    fn write_to_path(&mut self, path: &Path) -> Result<()> {
        self.tag.write_to_path(path).map_err(|e| eyre!(e))
    }
}
