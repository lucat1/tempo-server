use eyre::{eyre, Result};
use tantivy::{doc, schema::Field, Document};

use super::schema::{ARTISTS_SCHEMA, RELEASES_SCHEMA, TRACKS_SCHEMA};

#[derive(Clone, Copy)]
pub struct ArtistFields {
    id: Field,
    name: Field,
    sort_name: Field,
    description: Field,
}

fn artist_fields() -> Option<ArtistFields> {
    Some(ArtistFields {
        id: ARTISTS_SCHEMA.get_field("id")?,
        name: ARTISTS_SCHEMA.get_field("name")?,
        sort_name: ARTISTS_SCHEMA.get_field("sort_name")?,
        description: ARTISTS_SCHEMA.get_field("description")?,
    })
}

pub fn artist_to_document(data: entity::Artist) -> Result<Document> {
    let ArtistFields {
        id,
        name,
        sort_name,
        description,
    } = artist_fields().ok_or(eyre!("Could not get search index artist fields"))?;

    let mut document = doc!(
    id => data.id.to_string(),
    name => data.name,
    sort_name => data.sort_name,
    );
    if let Some(desc) = data.description {
        document.add_text(description, desc);
    }
    Ok(document)
}

fn artists_string(artists: Vec<(entity::ArtistCredit, entity::Artist)>) -> String {
    let mut res = String::new();
    for (credit, artist) in artists.into_iter() {
        res += (credit.join_phrase.unwrap_or_default() + artist.name.as_str()).as_str()
    }
    res
}

#[derive(Clone, Copy)]
pub struct TrackFields {
    id: Field,
    artists: Field,
    title: Field,
    genres: Field,
}

fn track_fields() -> Option<TrackFields> {
    Some(TrackFields {
        id: TRACKS_SCHEMA.get_field("id")?,
        artists: TRACKS_SCHEMA.get_field("artists")?,
        title: TRACKS_SCHEMA.get_field("title")?,
        genres: TRACKS_SCHEMA.get_field("genres")?,
    })
}

pub fn track_to_document(
    (track_data, artists_data): (entity::Track, Vec<(entity::ArtistCredit, entity::Artist)>),
) -> Result<Document> {
    let TrackFields {
        id,
        artists,
        title,
        genres,
    } = track_fields().ok_or(eyre!("Could not get search index track fields"))?;

    let mut document = doc!(
    id => track_data.id.to_string(),
    title => track_data.title,
    genres => track_data.genres.0.join(" "),
    );
    document.add_text(artists, artists_string(artists_data));
    Ok(document)
}

#[derive(Clone, Copy)]
pub struct ReleaseFields {
    id: Field,
    artists: Field,
    title: Field,
    release_type: Field,
    genres: Field,
    date: Field,
    original_date: Field,
}

fn release_fields() -> Option<ReleaseFields> {
    Some(ReleaseFields {
        id: RELEASES_SCHEMA.get_field("id")?,
        artists: RELEASES_SCHEMA.get_field("artists")?,
        title: RELEASES_SCHEMA.get_field("title")?,
        release_type: RELEASES_SCHEMA.get_field("release_type")?,
        genres: RELEASES_SCHEMA.get_field("genres")?,
        date: RELEASES_SCHEMA.get_field("date")?,
        original_date: RELEASES_SCHEMA.get_field("original_date")?,
    })
}

pub fn release_to_document(
    (release_data, artists_data): (entity::Release, Vec<(entity::ArtistCredit, entity::Artist)>),
) -> Result<Document> {
    let ReleaseFields {
        id,
        artists,
        title,
        release_type,
        genres,
        date,
        original_date,
    } = release_fields().ok_or(eyre!("Could not get search index release fields"))?;

    let mut document = doc!(
    id => release_data.id.to_string(),
    title => release_data.title,
    genres => release_data.genres.0.join(" "),
    );
    if let Some(rel_typ) = release_data.release_type {
        document.add_text(release_type, rel_typ);
    }
    if let Some(d) = release_data.date {
        document.add_text(date, d);
    }
    if let Some(od) = release_data.original_date {
        document.add_text(original_date, od);
    }
    document.add_text(artists, artists_string(artists_data));
    Ok(document)
}
