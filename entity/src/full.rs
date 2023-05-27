use crate::*;
use eyre::{bail, eyre};
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct FullRelease {
    pub release: Release,
    pub medium: Vec<Medium>,
    pub artist_credit_release: Vec<ArtistCreditRelease>,
    pub artist_credit: Vec<ArtistCredit>,
    pub artist: Vec<Artist>,
}

impl FullRelease {
    pub fn dedup(mut self) -> Self {
        self.artist_credit.sort_unstable_by_key(|a| a.id.clone());
        self.artist_credit.dedup();
        self.artist.sort_unstable_by_key(|a| a.id);
        self.artist.dedup();

        self
    }
}

#[derive(Debug, Clone)]
pub struct FullReleaseActive {
    pub release: ReleaseActive,
    pub medium: Vec<MediumActive>,
    pub artist_credit_release: Vec<ArtistCreditReleaseActive>,
    pub artist_credit: Vec<ArtistCreditActive>,
    pub artist: Vec<ArtistActive>,
}

impl From<FullRelease> for FullReleaseActive {
    fn from(full_release: FullRelease) -> Self {
        FullReleaseActive {
            release: full_release.release.into(),
            medium: full_release.medium.into_iter().map(|m| m.into()).collect(),
            artist_credit_release: full_release
                .artist_credit_release
                .into_iter()
                .map(|acr| acr.into())
                .collect(),
            artist_credit: full_release
                .artist_credit
                .into_iter()
                .map(|ac| ac.into())
                .collect(),
            artist: full_release.artist.into_iter().map(|a| a.into()).collect(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct FullTrack {
    pub track: Track,
    pub artist_credit_track: Vec<ArtistCreditTrack>,
    pub artist_credit: Vec<ArtistCredit>,
    pub artist_track_relation: Vec<ArtistTrackRelation>,
    pub artist: Vec<Artist>,
}

impl FullTrack {
    pub fn dedup(mut self) -> Self {
        self.artist_credit.sort_unstable_by_key(|a| a.id.clone());
        self.artist_credit.dedup();
        self.artist.sort_unstable_by_key(|a| a.id);
        self.artist.dedup();

        self
    }
}

#[derive(Debug, Clone)]
pub struct FullTrackActive {
    pub track: TrackActive,
    pub artist_credit_track: Vec<ArtistCreditTrackActive>,
    pub artist_credit: Vec<ArtistCreditActive>,
    pub artist_track_relation: Vec<ArtistTrackRelationActive>,
    pub artist: Vec<ArtistActive>,
}

impl From<FullTrack> for FullTrackActive {
    fn from(full_track: FullTrack) -> Self {
        FullTrackActive {
            track: full_track.track.into(),
            artist_credit_track: full_track
                .artist_credit_track
                .into_iter()
                .map(|act| act.into())
                .collect(),
            artist_credit: full_track
                .artist_credit
                .into_iter()
                .map(|ac| ac.into())
                .collect(),
            artist_track_relation: full_track
                .artist_track_relation
                .into_iter()
                .map(|atr| atr.into())
                .collect(),
            artist: full_track.artist.into_iter().map(|a| a.into()).collect(),
        }
    }
}

pub trait ArtistInfo {
    fn get_artist(&self, id: Uuid) -> Option<&Artist>;
    fn get_artists(&self) -> Result<Vec<&Artist>>;
    fn get_joined_artists(&self) -> Result<String>;
}

impl ArtistInfo for FullRelease {
    fn get_artist(&self, id: Uuid) -> Option<&Artist> {
        self.artist
            .iter()
            .position(|a| a.id == id)
            .map(|i| &self.artist[i])
    }

    fn get_artists(&self) -> Result<Vec<&Artist>> {
        let FullRelease { artist_credit, .. } = self;
        let mut res = vec![];
        for credit in artist_credit.iter() {
            if let Some(artist) = self.get_artist(credit.artist_id) {
                res.push(artist);
            } else {
                bail!("Artist credit referes to a missing artist id");
            }
        }
        Ok(res)
    }

    fn get_joined_artists(&self) -> Result<String> {
        let FullRelease { artist_credit, .. } = self;
        let mut s = String::new();
        for credit in artist_credit.iter() {
            if let Some(artist) = self.get_artist(credit.artist_id) {
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

impl FullRelease {
    pub fn get_medium(&self, id: Uuid) -> Option<&Medium> {
        self.medium
            .iter()
            .position(|m| m.id == id)
            .map(|i| &self.medium[i])
    }
}

impl ArtistInfo for FullTrack {
    fn get_artist(&self, id: Uuid) -> Option<&Artist> {
        self.artist
            .iter()
            .position(|a| a.id == id)
            .map(|i| &self.artist[i])
    }

    fn get_artists(&self) -> Result<Vec<&Artist>> {
        let FullTrack { artist_credit, .. } = self;
        let mut res = vec![];
        for credit in artist_credit.iter() {
            if let Some(artist) = self.get_artist(credit.artist_id) {
                res.push(artist);
            } else {
                bail!("Artist credit referes to a missing artist id");
            }
        }
        Ok(res)
    }

    fn get_joined_artists(&self) -> Result<String> {
        let FullTrack { artist_credit, .. } = self;
        let mut s = String::new();
        for credit in artist_credit.iter() {
            if let Some(artist) = self.get_artist(credit.artist_id) {
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
    pub fn get_related(&self, relation_type: ArtistTrackRelationType) -> Result<Vec<&Artist>> {
        self.artist_track_relation
            .iter()
            .filter(|atr| atr.relation_type == relation_type)
            .map(|atr| {
                self.get_artist(atr.artist_id)
                    .ok_or(eyre!("Track has a non existant related artist"))
            })
            .collect()
    }
}
