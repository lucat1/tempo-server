use entity::IgnoreNone;
use eyre::{bail, eyre, Result, WrapErr};
use itertools::Itertools;
use reqwest::{Method, Request};
use sea_orm::{ConnectionTrait, DbConn, EntityTrait, IntoActiveModel};
use serde::{Deserialize, Serialize};
use serde_enum_str::Deserialize_enum_str;
use url::Url;
use uuid::Uuid;

use crate::fetch::musicbrainz::{send_request, MB_BASE_STRURL};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Task(Uuid);

pub async fn all_data<C>(db: &C) -> Result<Vec<Task>>
where
    C: ConnectionTrait,
{
    Ok(entity::ArtistEntity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|a| Task(a.id))
        .collect())
}

#[derive(Debug, Clone, Deserialize)]
struct Document {
    pub relations: Vec<Relation>,
}

#[derive(Debug, Clone, Deserialize_enum_str)]
enum MusicBrainzRelationType {
    #[serde(rename = "biography")]
    Biography,
    #[serde(rename = "discogs")]
    Discogs,
    #[serde(rename = "free streaming")]
    FreeStreaming,
    #[serde(rename = "streaming")]
    Streaming,
    #[serde(rename = "last.fm")]
    LastFM,
    #[serde(rename = "songkick")]
    SongKick,
    #[serde(rename = "soundcloud")]
    SoundCloud,
    #[serde(rename = "allmusic")]
    AllMusic,
    #[serde(rename = "official homepage")]
    Homepage,
    #[serde(rename = "social network")]
    SocialNetwork,
    #[serde(rename = "wikidata")]
    Wikidata,
    #[serde(rename = "youtube")]
    Youtube,
    #[serde(other)]
    Other(String),
}

#[derive(Debug, Clone, Deserialize)]
struct Relation {
    pub r#type: MusicBrainzRelationType,
    pub url: Option<RelationUrl>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RelationUrl {
    pub resource: Url,
}

fn parse(url: Url, t: MusicBrainzRelationType) -> Option<(String, entity::ArtistUrlType)> {
    match t {
        MusicBrainzRelationType::Biography => Some(entity::ArtistUrlType::Biography),
        MusicBrainzRelationType::Discogs => Some(entity::ArtistUrlType::Discogs),
        MusicBrainzRelationType::LastFM => Some(entity::ArtistUrlType::LastFM),
        MusicBrainzRelationType::AllMusic => Some(entity::ArtistUrlType::AllMusic),
        MusicBrainzRelationType::Youtube => Some(entity::ArtistUrlType::Youtube),
        MusicBrainzRelationType::Homepage => Some(entity::ArtistUrlType::Homepage),
        MusicBrainzRelationType::Wikidata => Some(entity::ArtistUrlType::Wikidata),
        MusicBrainzRelationType::SongKick => Some(entity::ArtistUrlType::SongKick),
        MusicBrainzRelationType::SoundCloud=> Some(entity::ArtistUrlType::SoundCloud),
        MusicBrainzRelationType::FreeStreaming => match url.domain() {
            Some("spotify.com") | Some("open.spotify.com") => Some(entity::ArtistUrlType::Spotify),
            Some("deezer.com") | Some("www.deezer.com") => Some(entity::ArtistUrlType::Deezer),
            Some(domain) => {
                tracing::trace!(%domain,"Ignoring free streaming service relation");
                None
            }
            None => None,
        },
        MusicBrainzRelationType::Streaming => match url.domain() {
            Some("tidal.com") => Some(entity::ArtistUrlType::Tidal),
            Some(domain) => {
                tracing::trace!(%domain,"Ignoring streaming service relation");
                None
            }
            None => None,
        },
        MusicBrainzRelationType::SocialNetwork => match url.domain() {
            Some("twitter.com") | Some("www.twitter.com") => Some(entity::ArtistUrlType::Twitter),
            Some("facebook.com") | Some("www.facebook.com") => Some(entity::ArtistUrlType::Facebook),
            Some("instagram.com") | Some("www.instagram.com") => Some(entity::ArtistUrlType::Instagram),
            Some(domain) => {
                tracing::trace!(%domain,"Ignoring social network service relation");
                None
            }
            None => None,
        },
        MusicBrainzRelationType::Other(relation_type) => {
            tracing::trace!(%relation_type,"Ignoring MusicBrainz artist relation with unhandled type");
            None
        }
    }.map(|r| (url.to_string(), r))
}

#[async_trait::async_trait]
impl super::TaskTrait for Task {
    async fn run<D>(&self, db: &D) -> Result<()>
    where
        D: ConnectionTrait,
    {
        let Task(data) = self;
        tracing::trace!(%data, "Fetching artist urls");
        let req = Request::new(
            Method::GET,
            format!("{}artist/{}?fmt=json&inc=url-rels", MB_BASE_STRURL, data).parse()?,
        );
        let res = send_request(req).await?;
        if !res.status().is_success() {
            bail!(
                "MusicBrainz request returned non-success error code: {} {}",
                res.status(),
                res.text().await?
            );
        }
        let text = res
            .text()
            .await
            .wrap_err(eyre!("Could not read response as text"))?;

        let document: Document = serde_path_to_error::deserialize(
            &mut serde_json::Deserializer::from_str(text.as_str()),
        )
        .map_err(|e| {
            eyre!(
                "Error wihle decoding: {} at path {}",
                e,
                e.path().to_string()
            )
        })?;
        let urls = document
            .relations
            .into_iter()
            .filter_map(|r| r.url.and_then(|url| parse(url.resource, r.r#type)))
            .map(|(url, t)| entity::ArtistUrl {
                artist_id: *data,
                r#type: t,
                url,
            })
            .sorted_by_key(|relation| (relation.r#type, relation.artist_id))
            .unique_by(|relation| (relation.r#type, relation.artist_id))
            .map(|r| r.into_active_model())
            .collect::<Vec<_>>();
        if !urls.is_empty() {
            entity::ArtistUrlEntity::insert_many(urls)
                .on_conflict(entity::conflict::ARTIST_RELATION_CONFLICT.to_owned())
                .exec(db)
                .await
                .ignore_none()?;
        }
        Ok(())
    }
}
