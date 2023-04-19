use super::documents::{Artist, ArtistCredit};

pub fn artist_to_artist(artist: &entity::Artist) -> Artist {
    Artist {
        id: artist.id,
        name: artist.name.clone(),
        sort_name: artist.sort_name.clone(),
    }
}

pub fn artist_credit_to_artist_credit(
    artist_credit: &entity::ArtistCredit,
    artist: &entity::Artist,
) -> ArtistCredit {
    ArtistCredit {
        id: artist_credit.id.clone(),
        join_phrase: artist_credit.join_phrase.clone(),
        artist: artist_to_artist(artist),
    }
}
