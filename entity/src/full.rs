use crate::*;
use eyre::{bail, eyre};
use serde::Serialize;
use std::{collections::HashSet, sync::Arc};

#[derive(Serialize, Debug, Clone)]
pub struct FullRelease {
    pub release: Uuid,
    pub import: Arc<Import>,
}

impl FullRelease {
    pub fn new(import: Arc<Import>, id: Uuid) -> Result<Self> {
        if !import.releases.0.iter().any(|rel| rel.id == id) {
            Err(eyre!(
                "Cannot construct FullRelease from import with a missing release"
            ))
        } else {
            Ok(Self {
                release: id,
                import,
            })
        }
    }

    pub fn get_release_genres(&self) -> Vec<&GenreRelease> {
        self.import
            .release_genres
            .0
            .iter()
            .filter(|rel| rel.release_id == self.release)
            .collect()
    }

    pub fn get_artist_credits_release(&self) -> Vec<&ArtistCreditRelease> {
        self.import
            .artist_credit_releases
            .0
            .iter()
            .filter(|rel| rel.release_id == self.release)
            .collect()
    }

    pub fn get_release(&self) -> &Release {
        self.import
            .releases
            .0
            .iter()
            .find(|rel| rel.id == self.release)
            .unwrap()
    }

    pub fn get_mediums(&self) -> Vec<&Medium> {
        self.import
            .mediums
            .0
            .iter()
            .filter(|medium| medium.release_id == self.release)
            .collect()
    }

    pub fn get_medium(&self, id: Uuid) -> Option<&Medium> {
        self.get_mediums()
            .iter()
            .find(|medium| medium.release_id == self.release && medium.id == id)
            .copied()
    }

    pub fn get_tracks(&self) -> Vec<&Track> {
        self.get_mediums()
            .iter()
            .flat_map(|medium| {
                self.import
                    .tracks
                    .0
                    .iter()
                    .filter(|track| track.medium_id == medium.id)
            })
            .collect()
    }

    pub fn get_full_tracks(&self) -> Result<Vec<FullTrack>> {
        self.get_tracks()
            .iter()
            .map(|track| FullTrack::new(self.import.clone(), track.id))
            .collect::<Result<Vec<_>>>()
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct FullTrack {
    pub track: Uuid,
    pub import: Arc<Import>,
}

impl FullTrack {
    pub fn new(import: Arc<Import>, id: Uuid) -> Result<Self> {
        if !import.tracks.0.iter().any(|track| track.id == id) {
            Err(eyre!(
                "Cannot construct FullTrack from import with a missing track"
            ))
        } else {
            Ok(Self { track: id, import })
        }
    }

    pub fn get_track(&self) -> &Track {
        self.import
            .tracks
            .0
            .iter()
            .find(|track| track.id == self.track)
            .unwrap()
    }

    pub fn get_medium(&self) -> &Medium {
        let medium_id = self.get_track().medium_id;
        self.import
            .mediums
            .0
            .iter()
            .find(|med| med.id == medium_id)
            .unwrap()
    }

    pub fn get_artist_credits_track(&self) -> Vec<&ArtistCreditTrack> {
        self.import
            .artist_credit_tracks
            .0
            .iter()
            .filter(|rel| rel.track_id == self.track)
            .collect()
    }

    pub fn get_track_genres(&self) -> Vec<&GenreTrack> {
        self.import
            .track_genres
            .0
            .iter()
            .filter(|rel| rel.track_id == self.track)
            .collect()
    }

    pub fn get_relations(&self) -> Vec<&ArtistTrackRelation> {
        self.import
            .artist_track_relations
            .0
            .iter()
            .filter(|atr| atr.track_id == self.track)
            .collect()
    }

    pub fn get_related_artists(&self) -> Result<Vec<&Artist>> {
        self.get_relations()
            .iter()
            .filter(|atr| atr.track_id == self.track)
            .map(|atr| {
                self.get_artist(atr.artist_id)
                    .ok_or(eyre!("Track has a non existant related artist"))
            })
            .collect()
    }

    pub fn get_related(&self, relation_type: ArtistTrackRelationType) -> Result<Vec<&Artist>> {
        self.import
            .artist_track_relations
            .0
            .iter()
            .filter(|atr| atr.track_id == self.track && atr.relation_type == relation_type)
            .map(|atr| {
                self.get_artist(atr.artist_id)
                    .ok_or(eyre!("Track has a non existant related artist"))
            })
            .collect()
    }
}

pub trait GetArtistCredits {
    fn get_artist_credits(&self) -> Vec<&ArtistCredit>;
}

pub trait GetArtist {
    fn get_artist(&self, id: Uuid) -> Option<&Artist>;
}

impl GetArtist for FullRelease {
    fn get_artist(&self, id: Uuid) -> Option<&Artist> {
        self.import.artists.0.iter().find(|a| a.id == id)
    }
}

impl GetArtistCredits for FullRelease {
    fn get_artist_credits(&self) -> Vec<&ArtistCredit> {
        self.import
            .artist_credit_releases
            .0
            .iter()
            .filter(|acr| acr.release_id == self.release)
            .flat_map(|acr| {
                self.import
                    .artist_credits
                    .0
                    .iter()
                    .find(|ac| ac.id == acr.artist_credit_id)
            })
            .collect::<Vec<_>>()
    }
}

impl GetArtistCredits for FullTrack {
    fn get_artist_credits(&self) -> Vec<&ArtistCredit> {
        self.import
            .artist_credit_tracks
            .0
            .iter()
            .filter(|act| act.track_id == self.track)
            .flat_map(|act| {
                self.import
                    .artist_credits
                    .0
                    .iter()
                    .find(|ac| ac.id == act.artist_credit_id)
            })
            .collect::<Vec<_>>()
    }
}

impl GetArtist for FullTrack {
    fn get_artist(&self, id: Uuid) -> Option<&Artist> {
        self.import.artists.0.iter().find(|a| a.id == id)
    }
}

pub trait ArtistInfo {
    fn get_artists(&self) -> Result<Vec<&Artist>>;
    fn get_joined_artists(&self) -> Result<String>;
}

impl<T> ArtistInfo for T
where
    T: GetArtist + GetArtistCredits,
{
    fn get_artists(&self) -> Result<Vec<&Artist>> {
        let mut res = vec![];
        for credit in self.get_artist_credits().iter() {
            if let Some(artist) = self.get_artist(credit.artist_id) {
                res.push(artist);
            } else {
                bail!("Artist credit referes to a missing artist id");
            }
        }
        Ok(res)
    }

    fn get_joined_artists(&self) -> Result<String> {
        let mut s = String::new();
        for credit in self.get_artist_credits().iter() {
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

pub trait GenreInfo {
    fn get_genres(&self) -> Result<Vec<&Genre>>;
}

// TODO: maybe generalize like we've done for artists. Would
// avoid duplication of the final iterator
impl GenreInfo for FullTrack {
    fn get_genres(&self) -> Result<Vec<&Genre>> {
        let genre_ids: HashSet<String> = self
            .get_track_genres()
            .iter()
            .map(|tg| tg.genre_id.to_owned())
            .collect();
        Ok(self
            .import
            .genres
            .0
            .iter()
            .filter(|g| genre_ids.contains(&g.id))
            .collect())
    }
}

impl GenreInfo for FullRelease {
    fn get_genres(&self) -> Result<Vec<&Genre>> {
        let genre_ids: HashSet<String> = self
            .get_release_genres()
            .iter()
            .map(|tg| tg.genre_id.to_owned())
            .collect();
        Ok(self
            .import
            .genres
            .0
            .iter()
            .filter(|g| genre_ids.contains(&g.id))
            .collect())
    }
}
