use crate::*;
use eyre::{bail, eyre};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize, Debug, Clone)]
pub struct FullRelease {
    pub release: Uuid,
    pub import: Arc<Import>,
}

impl FullRelease {
    pub fn new(import: Arc<Import>, id: Uuid) -> Result<Self> {
        if let None = import.releases.0.iter().find(|rel| rel.id == id) {
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
        Ok(self
            .get_tracks()
            .iter()
            .map(|track| FullTrack::new(self.import.clone(), track.id))
            .collect::<Result<Vec<_>>>()?)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct FullTrack {
    pub track: Uuid,
    pub import: Arc<Import>,
}

impl FullTrack {
    pub fn new(import: Arc<Import>, id: Uuid) -> Result<Self> {
        if let None = import.tracks.0.iter().find(|track| track.id == id) {
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

// impl FullRelease {
//     pub fn dedup(mut self) -> Self {
//         self.artist_credit.sort_unstable_by_key(|a| a.id.clone());
//         self.artist_credit.dedup();
//         self.artist_credit_release.sort_unstable_by_key(|a| {
//             a.artist_credit_id.to_owned() + a.release_id.to_string().as_str()
//         });
//         self.artist_credit_release.dedup();
//         self.artist.sort_unstable_by_key(|a| a.id);
//         self.artist.dedup();
//
//         self
//     }
// }

// #[derive(Debug, Clone)]
// pub struct FullReleaseActive {
//     pub release: ReleaseActive,
//     pub medium: Vec<MediumActive>,
//     pub artist_credit_release: Vec<ArtistCreditReleaseActive>,
//     pub artist_credit: Vec<ArtistCreditActive>,
//     pub artist: Vec<ArtistActive>,
// }

// impl From<FullRelease> for FullReleaseActive {
//     fn from(full_release: FullRelease) -> Self {
//         FullReleaseActive {
//             release: full_release.release.into(),
//             medium: full_release.medium.into_iter().map(|m| m.into()).collect(),
//             artist_credit_release: full_release
//                 .artist_credit_release
//                 .into_iter()
//                 .map(|acr| acr.into())
//                 .collect(),
//             artist_credit: full_release
//                 .artist_credit
//                 .into_iter()
//                 .map(|ac| ac.into())
//                 .collect(),
//             artist: full_release.artist.into_iter().map(|a| a.into()).collect(),
//         }
//     }
// }
// impl FullTrack {
//     pub fn dedup(mut self) -> Self {
//         self.artist_credit.sort_unstable_by_key(|a| a.id.clone());
//         self.artist_credit.dedup();
//         self.artist_credit_track.sort_unstable_by_key(|a| {
//             a.artist_credit_id.to_owned() + a.track_id.to_string().as_str()
//         });
//         self.artist_credit_track.dedup();
//         self.artist.sort_unstable_by_key(|a| a.id);
//         self.artist.dedup();
//         self.artist_track_relation.sort_unstable_by_key(|a| {
//             a.artist_id.to_string()
//                 + a.track_id.to_string().as_str()
//                 + a.relation_type.to_string().as_str()
//                 + a.relation_value.as_str()
//         });
//         self.artist_track_relation.dedup();
//
//         self
//     }
// }
//
// #[derive(Debug, Clone)]
// pub struct FullTrackActive {
//     pub track: TrackActive,
//     pub artist_credit_track: Vec<ArtistCreditTrackActive>,
//     pub artist_credit: Vec<ArtistCreditActive>,
//     pub artist_track_relation: Vec<ArtistTrackRelationActive>,
//     pub artist: Vec<ArtistActive>,
// }

// impl From<FullTrack> for FullTrackActive {
//     fn from(full_track: FullTrack) -> Self {
//         FullTrackActive {
//             track: full_track.track.into(),
//             artist_credit_track: full_track
//                 .artist_credit_track
//                 .into_iter()
//                 .map(|act| act.into())
//                 .collect(),
//             artist_credit: full_track
//                 .artist_credit
//                 .into_iter()
//                 .map(|ac| ac.into())
//                 .collect(),
//             artist_track_relation: full_track
//                 .artist_track_relation
//                 .into_iter()
//                 .map(|atr| atr.into())
//                 .collect(),
//             artist: full_track.artist.into_iter().map(|a| a.into()).collect(),
//         }
//     }
// }

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
