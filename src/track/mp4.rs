extern crate mp4ameta;

use super::map::TagKey;
use super::picture::{Picture, PictureType};
use core::convert::AsRef;
use eyre::{bail, eyre, Result};
use mp4ameta::ident::DataIdent;
use mp4ameta::{Data, ImgFmt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MAGIC: u8 = 0xa9;

#[derive(Clone)]
pub struct Tag {
    tag: mp4ameta::Tag,
    separator: String,
}

impl crate::track::TagFrom for Tag {
    fn from_path<P>(path: P) -> Result<Box<dyn crate::track::Tag>>
    where
        P: AsRef<Path>,
    {
        Ok(Box::new(Tag {
            tag: mp4ameta::Tag::read_from_path(path)?,
            // TODO
            separator: ",".to_string(),
        }))
    }
}

fn ident_to_string(ident: &DataIdent) -> String {
    match ident {
        DataIdent::Fourcc(d) => format!("{}", d),
        DataIdent::Freeform { mean, name } => format!("{}:{}", mean, name),
    }
}

fn str_to_ident(ident: &str) -> DataIdent {
    let mut bytes = ident.as_bytes().to_owned();
    // Replace UTF-8 © with the proper character
    if bytes.len() == 5 && bytes[0..2] == [194, 169] {
        bytes = vec![MAGIC, bytes[2], bytes[3], bytes[4]];
    }
    // Fourcc
    if bytes.len() == 4 {
        return DataIdent::fourcc(bytes.try_into().unwrap());
    }
    // Convert string freeform
    let mut ident = ident.replacen("----:", "", 1);
    // iTunes:VALUE abstraction
    if ident.starts_with("iTunes:") {
        ident = format!("com.apple.{}", ident);
    }
    let mut mean = "com.apple.iTunes";
    let mut name = ident.to_string();
    let split: Vec<&str> = ident.split(":").collect();
    if split.len() > 1 {
        mean = split[0];
        name = split[1].to_owned();
    }
    DataIdent::freeform(mean, name)
}

impl crate::track::Tag for Tag {
    fn clear(&mut self) -> Result<()> {
        self.tag.clear();
        Ok(())
    }
    fn separator(&self) -> Option<String> {
        Some(self.separator.clone())
    }
    fn get_str(&self, key: &str) -> Option<Vec<String>> {
        let ident = str_to_ident(key);
        let data: Vec<String> = self
            .tag
            .data_of(&ident)
            .filter_map(|data| {
                // Save only text values
                match data {
                    Data::Utf8(d) => Some(d.to_owned()),
                    Data::Utf16(d) => Some(d.to_owned()),
                    _ => None,
                }
            })
            .collect();
        if data.is_empty() {
            return None;
        }
        // Convert multi tag to single with separator
        Some(
            data.join(&self.separator)
                .split(&self.separator)
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        )
    }

    fn set_str(&mut self, key: &str, values: Vec<String>) -> Result<()> {
        self.tag
            .set_data(str_to_ident(key), Data::Utf8(values.join(&self.separator)));
        Ok(())
    }

    fn get_all(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();

        for (ident, data) in self.tag.data() {
            let mut values = vec![];
            // Save only text values
            match data {
                Data::Utf8(d) => values = d.split(&self.separator).map(String::from).collect(),
                Data::Utf16(d) => values = d.split(&self.separator).map(String::from).collect(),
                _ => {}
            }
            if !values.is_empty() {
                out.insert(ident_to_string(ident), values);
            }
        }

        out
    }

    fn get_pictures(&self) -> Result<Vec<Picture>> {
        Ok(self
            .tag
            .images()
            .map(|img| Picture {
                mime_type: match img.1.fmt {
                    ImgFmt::Png => "image/png".to_string(),
                    ImgFmt::Jpeg => "image/jpeg".to_string(),
                    ImgFmt::Bmp => "image/bmp".to_string(),
                },
                picture_type: PictureType::CoverFront,
                description: ident_to_string(img.0),
                data: img.1.data.to_owned(),
            })
            .collect::<Vec<_>>())
    }

    fn set_pictures(&mut self, pictures: Vec<Picture>) -> Result<()> {
        // remove all the previous pictures
        let retag = self.tag.clone();
        for (ident, _) in retag.images() {
            self.tag.remove_data_of(ident);
        }
        for pic in pictures {
            if pic.picture_type != PictureType::CoverFront {
                bail!("mp4 only supports cover front art");
            }
            let data = match pic.mime_type.as_str() {
                "image/png" => Ok(Data::Png(pic.data)),
                "image/jpeg" => Ok(Data::Jpeg(pic.data)),
                "image/bmp" => Ok(Data::Bmp(pic.data)),
                mime => Err(eyre!("Invalid mime type for a picture in mp4: {}", mime)),
            }?;
            self.tag.set_data(str_to_ident("covr"), data);
        }
        Ok(())
    }

    fn str_to_key(&self, str: &str) -> Option<TagKey> {
        unimplemented!();
    }
    fn key_to_str(&self, key: TagKey) -> Option<&'static str> {
        match key {
            TagKey::AcoustidID => Some("Acoustid Id"),
            TagKey::AcoustidIDFingerprint => Some("Acoustid Fingerprint"),
            TagKey::Album => Some("©alb"),
            TagKey::AlbumArtist => Some("aART"),
            TagKey::AlbumArtistSortOrder => Some("soaa"),
            TagKey::AlbumSortOrder => Some("soal"),
            TagKey::Artist => Some("©ART"),
            TagKey::ArtistSortOrder => Some("soar"),
            TagKey::Artists => Some("ARTISTS"),
            TagKey::ASIN => Some("ASIN"),
            TagKey::Barcode => Some("BARCODE"),
            TagKey::BPM => Some("tmpo"),
            TagKey::CatalogNumber => Some("CATALOGNUMBER"),
            TagKey::Comment => Some("©cmt"),
            TagKey::Compilation => Some("cpil"),
            TagKey::Composer => Some("©wrt"),
            TagKey::ComposerSortOrder => Some("soco"),
            TagKey::Conductor => Some("CONDUCTOR"),
            TagKey::Copyright => Some("cprt"),
            TagKey::Director => Some("©dir"),
            TagKey::DiscNumber => Some("disk"),
            TagKey::DiscSubtitle => Some("DISCSUBTITLE"),
            TagKey::EncodedBy => Some("©too"),
            TagKey::Engineer => Some("ENGINEER"),
            TagKey::GaplessPlayback => Some("pgap"),
            TagKey::Genre => Some("©gen"),
            TagKey::Grouping => Some("©grp"),
            TagKey::InitialKey => Some("initialkey"),
            TagKey::ISRC => Some("ISRC"),
            TagKey::Language => Some("LANGUAGE"),
            TagKey::License => Some("LICENSE"),
            TagKey::Liricist => Some("LYRICIST"),
            TagKey::Lyrics => Some("©lyr"),
            TagKey::Media => Some("MEDIA"),
            TagKey::MixDJ => Some("DJMIXER"),
            TagKey::Mixer => Some("MIXER"),
            TagKey::Mood => Some("MOOD"),
            TagKey::Movement => Some("©mvn"),
            TagKey::MovementCount => Some("mvc"),
            TagKey::MovementNumber => Some("mvi"),
            TagKey::MusicBrainzArtistID => Some("MusicBrainz Artist Id"),
            TagKey::MusicBrainzDiscID => Some("MusicBrainz Disc Id"),
            TagKey::MusicBrainzOriginalArtistID => Some("MusicBrainz Original Artist Id"),
            TagKey::MusicBrainzOriginalReleaseID => Some("MusicBrainz Original Album Id"),
            TagKey::MusicBrainzRecordingID => Some("MusicBrainz Track Id"),
            TagKey::MusicBrainzReleaseArtistID => Some("MusicBrainz Album Artist Id"),
            TagKey::MusicBrainzReleaseGroupID => Some("MusicBrainz Release Group Id"),
            TagKey::MusicBrainzReleaseID => Some("MusicBrainz Album Id"),
            TagKey::MusicBrainzTrackID => Some("MusicBrainz Release Track Id"),
            TagKey::MusicBrainzTRMID => Some("MusicBrainz TRM Id"),
            TagKey::MusicBrainzWorkID => Some("MusicBrainz Work Id"),
            TagKey::MusicIPFingerprint => Some("fingerprint"),
            TagKey::MusicIPPUID => Some("MusicIP PUID"),
            TagKey::Podcast => Some("pcst"),
            TagKey::PodcastURL => Some("purl"),
            TagKey::Producer => Some("PRODUCER"),
            TagKey::RecordLabel => Some("LABEL"),
            TagKey::ReleaseCountry => Some("MusicBrainz Album Release Country"),
            TagKey::ReleaseDate => Some("©day"),
            TagKey::ReleaseStatus => Some("MusicBrainz Album Status"),
            TagKey::ReleaseType => Some("MusicBrainz Album Type"),
            TagKey::Remixer => Some("REMIXER"),
            TagKey::ReplayGainAlbumGain => Some("REPLAYGAIN_ALBUM_GAIN"),
            TagKey::ReplayGainAlbumPeak => Some("REPLAYGAIN_ALBUM_PEAK"),
            TagKey::ReplayGainAlbumRange => Some("REPLAYGAIN_ALBUM_RANGE"),
            TagKey::ReplayGainReferenceLoudness => Some("REPLAYGAIN_REFERENCE_LOUDNESS"),
            TagKey::ReplayGainTrackGain => Some("REPLAYGAIN_TRACK_GAIN"),
            TagKey::ReplayGainTrackPeak => Some("REPLAYGAIN_TRACK_PEAK"),
            TagKey::ReplayGainTrackRange => Some("REPLAYGAIN_TRACK_RANGE"),
            TagKey::Script => Some("SCRIPT"),
            TagKey::ShowName => Some("tvsh"),
            TagKey::ShowNameSortOrder => Some("sosn"),
            TagKey::ShowWorkAndMovement => Some("shwm"),
            TagKey::Subtitle => Some("SUBTITLE"),
            TagKey::TotalDiscs => Some("disk"),
            TagKey::TotalTracks => Some("trkn"),
            TagKey::TrackTitle => Some("©nam"),
            TagKey::TrackTitleSortOrder => Some("sonm"),
            TagKey::WorkTitle => Some("©wrk"),

            // Internal, not mapped from picard
            TagKey::Duration => Some("LENGTH"),
            _ => None,
        }
    }

    fn write_to_path(&mut self, path: &PathBuf) -> Result<()> {
        self.tag.write_to_path(path).map_err(|e| eyre!(e))
    }
}
