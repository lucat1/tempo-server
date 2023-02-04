mod artist;
mod medium;
mod release;
mod track;
mod track_format;

mod artist_credit;
mod artist_credit_release;
mod artist_credit_track;
mod artist_track_relation;

mod key;
mod map;

use eyre::{bail, Result};
use uuid::Uuid;

pub use artist::ActiveModel as ArtistActive;
pub use artist::Column as ArtistColumn;
pub use artist::Entity as ArtistEntity;
pub use artist::Model as Artist;
pub use artist_credit::ActiveModel as ArtistCreditActive;
pub use artist_credit::Column as AritstCreditColumn;
pub use artist_credit::Entity as ArtistCreditEntity;
pub use artist_credit::Model as ArtistCredit;
pub use artist_track_relation::ActiveModel as ArtistTrackRelationActive;
pub use artist_track_relation::Column as ArtistTrackRelationColumn;
pub use artist_track_relation::Entity as ArtistTrackRelationEntity;
pub use artist_track_relation::Model as ArtistTrackRelation;
pub use artist_track_relation::RelationType;
pub use medium::ActiveModel as MediumActive;
pub use medium::Column as MediumColumn;
pub use medium::Entity as MediumEntity;
pub use medium::Model as Medium;
pub use release::ActiveModel as ReleaseActive;
pub use release::Column as ReleaseColumn;
pub use release::Entity as ReleaseEntity;
pub use release::Model as Release;
pub use track::ActiveModel as TrackActive;
pub use track::Column as TrackColumn;
pub use track::Entity as TrackEntity;
pub use track::Model as Track;
pub use track_format::TrackFormat;

pub use artist_credit_release::Column as ArtistReleaseColumn;
pub use artist_credit_release::Entity as ArtistReleaseEntity;
pub use artist_credit_release::Model as ArtistRelease;
pub use artist_credit_track::Column as ArtistTrackColumn;
pub use artist_credit_track::Entity as ArtistTrackEntity;
pub use artist_credit_track::Model as ArtistTrack;

pub use key::TagKey;
pub use map::KeyMap;

#[derive(Debug, Clone)]
pub struct FullRelease(
    pub Release,
    pub Vec<Medium>,
    pub Vec<ArtistCredit>,
    pub Vec<Artist>,
);

#[derive(Debug, Clone)]
pub struct FullReleaseActive(
    pub ReleaseActive,
    pub Vec<MediumActive>,
    pub Vec<ArtistCreditActive>,
    pub Vec<ArtistActive>,
);

impl From<FullRelease> for FullReleaseActive {
    fn from(FullRelease(release, mediums, artist_credits, artists): FullRelease) -> Self {
        FullReleaseActive(
            release.into(),
            mediums.into_iter().map(|m| m.into()).collect(),
            artist_credits.into_iter().map(|ac| ac.into()).collect(),
            artists.into_iter().map(|a| a.into()).collect(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct FullTrackActive(
    pub TrackActive,
    pub Vec<ArtistCreditActive>,
    pub Vec<ArtistTrackRelationActive>,
    pub Vec<ArtistActive>,
);

#[derive(Debug, Clone)]
pub struct FullTrack(
    pub Track,
    pub Vec<ArtistCredit>,
    pub Vec<ArtistTrackRelation>,
    pub Vec<Artist>,
);

impl From<FullTrack> for FullTrackActive {
    fn from(FullTrack(track, artist_credits, artist_track_relations, artists): FullTrack) -> Self {
        FullTrackActive(
            track.into(),
            artist_credits.into_iter().map(|ac| ac.into()).collect(),
            artist_track_relations
                .into_iter()
                .map(|atr| atr.into())
                .collect(),
            artists.into_iter().map(|a| a.into()).collect(),
        )
    }
}

impl FullRelease {
    fn artist(&self, id: Uuid) -> Option<&Artist> {
        let FullRelease(_, _, _, artists) = self;
        for artist in artists.iter() {
            if artist.id == id {
                return Some(artist);
            }
        }
        None
    }

    pub fn artists(&self) -> Result<Vec<Artist>> {
        let FullRelease(_, _, credits, _) = self;
        let res = vec![];
        for credit in credits.iter() {
            if let Some(artist) = self.artist(credit.artist_id) {
                res.push(artist.clone());
            } else {
                bail!("Artist credit referes to a missing artist id");
            }
        }
        Ok(res)
    }

    pub fn artist_names(&self) -> Result<Vec<String>> {
        let FullRelease(_, _, credits, _) = self;
        let res = vec![];
        for credit in credits.iter() {
            if let Some(artist) = self.artist(credit.artist_id) {
                res.push(artist.name.as_str());
            } else {
                bail!("Artist credit referes to a missing artist id");
            }
        }
        Ok(res)
    }

    pub fn joined_artists(&self) -> Result<String> {
        let FullRelease(_, _, credits, _) = self;
        let mut s = String::new();
        for credit in credits.iter() {
            if let Some(artist) = self.artist(credit.artist_id) {
                s += artist.name.as_str();
                if let Some(join) = credit.join_phrase.as_ref() {
                    s += join.as_str();
                }
            } else {
                bail!("Artist credit referes to a missing artist id");
            }
        }
        Ok(s)
    }
}

impl FullTrack {
    fn artist(&self, id: Uuid) -> Option<&Artist> {
        let FullTrack(_, _, _, artists) = self;
        for artist in artists.iter() {
            if artist.id == id {
                return Some(artist);
            }
        }
        None
    }

    pub fn artists(&self) -> Result<Vec<Artist>> {
        let FullTrack(_, _, credits, _) = self;
        let res = vec![];
        for credit in credits.iter() {
            if let Some(artist) = self.artist(credit.artist_id) {
                res.push(artist.clone());
            } else {
                bail!("Artist credit referes to a missing artist id");
            }
        }
        Ok(res)
    }

    pub fn joined_artists(&self) -> Result<String> {
        let FullTrack(_, _, credits, _) = self;
        let mut s = String::new();
        for credit in credits.iter() {
            if let Some(artist) = self.artist(credit.artist_id) {
                s += artist.name.as_str();
                if let Some(join) = credit.join_phrase.as_ref() {
                    s += join.as_str();
                }
            } else {
                bail!("Artist credit referes to a missing artist id");
            }
        }
        Ok(s)
    }
}
