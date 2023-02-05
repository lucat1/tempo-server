use crate::get_database;
use entity::{ArtistCreditEntity, ArtistEntity, ReleaseEntity};

use eyre::{bail, eyre, Result};
use log::info;
use sea_orm::{EntityTrait, LoaderTrait};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;
use strfmt::strfmt;

static DEFAULT_FORMAT_ARTIST: &str = "{name} ({sort_name})";
static DEFAULT_FORMAT_ALBUM_ARTIST: &str = "{join_phrase}{name}";
static DEFAULT_FORMAT_TRACK: &str = "{album_artist} - {album} - {track_title}";
static DEFAULT_FORMAT_RELEASE: &str = "{album_artist} - {album} ({release_year}) ({release_type})";

pub async fn list(
    _filters: Vec<&String>,
    format: Option<&String>,
    object: Option<&String>,
) -> Result<()> {
    let start = Instant::now();
    let object = object.map_or("track", |s| s.as_str());
    let db = get_database()?;
    match object {
        "artist" | "artists" => {
            let artists = ArtistEntity::find().all(db).await?;
            let mut vars = HashMap::new();
            for artist in artists.into_iter() {
                vars.clear();
                vars.insert("name".to_string(), artist.name);
                vars.insert("sort_name".to_string(), artist.sort_name);
                println!(
                    "{}",
                    strfmt(format.map_or(DEFAULT_FORMAT_ARTIST, |s| s.as_str()), &vars)?
                );
            }
        }
        "album_artist" | "album_artists" => {
            let releases = ReleaseEntity::find()
                .find_with_related(ArtistCreditEntity)
                .all(db)
                .await?;
            let mut vars = HashMap::new();
            for (_, mut artist_credits) in releases.into_iter() {
                let artists = artist_credits.load_one(ArtistEntity, db).await?;
                let mut str = String::new();
                // TODO: consider the need for this
                artist_credits.sort_by(|a, b| -> Ordering {
                    a.join_phrase
                        .as_ref()
                        .map_or(0, |s| s.len())
                        .cmp(&b.join_phrase.as_ref().map_or(0, |s| s.len()))
                });
                for (i, artist_credit) in artist_credits.into_iter().enumerate() {
                    vars.clear();
                    let artist = artists[i]
                        .clone()
                        .ok_or(eyre!("Missing artist information"))?;
                    vars.insert("name".to_string(), artist.name);
                    vars.insert("sort_name".to_string(), artist.sort_name);
                    vars.insert(
                        "join_phrase".to_string(),
                        artist_credit.join_phrase.unwrap_or_default(),
                    );
                    str += strfmt(
                        format.map_or(DEFAULT_FORMAT_ALBUM_ARTIST, |s| s.as_str()),
                        &vars,
                    )?
                    .as_str();
                }
                println!("{}", str);
            }
            // let mut vars = HashMap::new();
            // for artist in artists.into_iter() {
            //     vars.clear();
            //     vars.insert("name".to_string(), artist.name);
            //     vars.insert("sort_name".to_string(), artist.sort_name);
            //     println!(
            //         "{}",
            //         strfmt(format.map_or(DEFAULT_FORMAT_ARTIST, |s| s.as_str()), &vars)?
            //     );
            // }
        }
        "track" | "tracks" => {}

        "release" | "releases" => (),
        _ => {
            bail!("Invalid object type {}", object)
        }
    };
    info!("Took {:?}", start.elapsed());
    Ok(())
}
