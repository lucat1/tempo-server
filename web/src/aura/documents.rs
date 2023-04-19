use jsonapi::api::*;
use jsonapi::array::*;
use jsonapi::jsonapi_model;
use jsonapi::model::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default)]
pub struct Artist {
    pub id: Uuid,
    pub name: String,
    pub sort_name: String,
}

jsonapi_model!(Artist; "artist");

#[derive(Serialize, Deserialize, Default)]
pub struct ArtistCredit {
    pub id: String,
    pub join_phrase: Option<String>,
    pub artist: Artist,
}

jsonapi_model!(ArtistCredit; "artist_credit"; has one artist);

#[derive(Serialize, Deserialize, Default)]
pub struct Image {
    pub id: String,
    pub role: String,
    pub format: String,
    pub description: Option<String>,
    pub width: u32,
    pub height: u32,
    pub size: u32,
}

jsonapi_model!(Image; "image");

#[derive(Serialize, Deserialize, Default)]
pub struct Track {
    pub id: Uuid,
    pub title: String,
    pub artists: Vec<ArtistCredit>,
    pub album: String,
    // TODO: either make it non-optional or make this optional
    pub cover: Image,

    pub track: u32,
    pub tracktotal: u32,
    pub disc: u32,
    pub disctotal: u32,
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
    pub bpm: Option<u32>,
    pub genres: Vec<String>,
    #[serde(rename = "recording-mbid")]
    pub recording_mbid: Uuid,
    #[serde(rename = "track-mbid")]
    pub track_mbid: Uuid,
    pub albumartists: Vec<ArtistCredit>,
    pub comments: Option<String>,

    pub mimetype: String,
    pub duration: Option<f32>,
    pub framerate: Option<u32>,
    pub framecount: Option<u32>,
    pub channels: Option<u32>,
    pub bitrate: Option<u32>,
    pub bitdepth: Option<u32>,
    pub size: Option<u32>,

    pub engigneers: Vec<Artist>,
    pub instrumentalists: Vec<Artist>,
    pub performers: Vec<Artist>,
    pub mixers: Vec<Artist>,
    pub producers: Vec<Artist>,
    pub vocalists: Vec<Artist>,
    pub lyricists: Vec<Artist>,
    pub writers: Vec<Artist>,
    pub composers: Vec<Artist>,
    pub others: Vec<Artist>,
}

jsonapi_model!(Track; "track"; has one cover; has many artists, albumartists, engigneers, instrumentalists, performers, mixers, producers, vocalists, lyricists, writers, composers, others);

pub fn dedup_document(doc: &mut JsonApiDocument) {
    if let JsonApiDocument::Data(d) = doc {
        if let Some(ref mut included) = &mut d.included {
            included.sort_by_key(|e| e.id.to_owned());
            included.dedup_by_key(|e| e.id.to_owned());
        }
    }
}

pub fn filter_included(doc: &mut JsonApiDocument, include: Vec<String>) {
    if let JsonApiDocument::Data(d) = doc {
        if let Some(included) = &d.included {
            let filtered = included
                .into_iter()
                .filter(|r| include.contains(&r._type))
                .map(|r| r.to_owned())
                .collect();
            d.included = Some(filtered);
        }
    }
}
