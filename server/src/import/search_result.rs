use base::util::{dedup, maybe_date};
use std::cmp::Ordering;
use uuid::Uuid;

use crate::fetch::musicbrainz;

pub struct SearchResult {
    pub artists: Vec<entity::Artist>,
    pub artist_credits: Vec<entity::ArtistCredit>,
    pub release: entity::Release,
    pub mediums: Vec<entity::Medium>,
    pub tracks: Vec<entity::Track>,
    pub artist_track_relations: Vec<entity::ArtistTrackRelation>,
    pub artist_credit_releases: Vec<entity::ArtistCreditRelease>,
    pub artist_credit_tracks: Vec<entity::ArtistCreditTrack>,
}

impl From<musicbrainz::Artist> for entity::Artist {
    fn from(artist: musicbrainz::Artist) -> entity::Artist {
        entity::Artist {
            id: artist.id,
            name: artist.name,
            sort_name: artist.sort_name,

            description: None,
        }
    }
}

trait ArtistCreditId {
    fn id(&self) -> String;
}

impl ArtistCreditId for musicbrainz::ArtistCredit {
    fn id(&self) -> String {
        self.artist.id.to_string() + "-" + self.joinphrase.as_ref().map_or("", |s| s.as_str())
    }
}

impl From<musicbrainz::ArtistCredit> for entity::ArtistCredit {
    fn from(artist_credit: musicbrainz::ArtistCredit) -> Self {
        entity::ArtistCredit {
            id: artist_credit.id(),
            join_phrase: artist_credit.joinphrase,
            artist_id: artist_credit.artist.id,
        }
    }
}

struct TrackWithMediumId(musicbrainz::Track, Uuid);

impl From<TrackWithMediumId> for entity::Track {
    fn from(TrackWithMediumId(track, medium_id): TrackWithMediumId) -> Self {
        let mut sorted_genres = track.recording.genres.unwrap_or_default();
        sorted_genres.sort_by(|a, b| a.count.partial_cmp(&b.count).unwrap_or(Ordering::Equal));

        entity::Track {
            id: track.id,
            medium_id,
            title: track.title,
            length: track.length.or(track.recording.length).unwrap_or_default(),
            number: track.position,
            genres: entity::Genres(
                sorted_genres
                    .into_iter()
                    .map(|g| g.name)
                    .collect::<Vec<_>>(),
            ),
            recording_id: track.recording.id,
            format: None,
            path: None,
        }
    }
}

impl From<musicbrainz::Release> for SearchResult {
    fn from(release: musicbrainz::Release) -> Self {
        let original_date = maybe_date(
            release
                .release_group
                .as_ref()
                .and_then(|r| r.first_release_date.clone()),
        );
        let date = maybe_date(release.date);
        let label = release.label_info.first();

        // artists for the release, to be extended by credits for the tracks
        let mut artists: Vec<entity::Artist> = release
            .artist_credit
            .iter()
            .map(|a| a.artist.clone().into())
            .collect();
        // artist credits for the release, to be extended by credits for the tracks
        let mut artist_credits: Vec<entity::ArtistCredit> = release
            .artist_credit
            .iter()
            .map(|ac| ac.clone().into())
            .collect();
        let mut artist_credit_tracks = Vec::new();
        let mut artist_track_relations = Vec::new();
        let mut tracks = Vec::new();

        let mediums: Vec<musicbrainz::Medium> = release
            .media
            .unwrap_or_default()
            .into_iter()
            .map(|medium| musicbrainz::Medium {
                id: medium.id.or(Some(Uuid::new_v4())),
                ..medium
            })
            .collect();

        for medium in mediums.iter() {
            for track in medium.tracks.as_ref().unwrap_or(&vec![]).iter() {
                let mut other_relations = track
                    .recording
                    .relations
                    .iter()
                    .filter_map(|rel| {
                        if <String as Into<entity::ArtistTrackRelationType>>::into(
                            rel.type_field.clone(),
                        ) == entity::ArtistTrackRelationType::Performance
                        {
                            rel.work.clone()
                        } else {
                            None
                        }
                    })
                    .filter_map(|work| work.relations)
                    .flatten()
                    .collect::<Vec<_>>();
                let mut all_relations = track.recording.relations.clone();
                all_relations.append(&mut other_relations);

                artists.extend(
                    track
                        .recording
                        .artist_credit
                        .as_ref()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|ac| ac.artist.clone().into()),
                );
                // Append artists for all other relations
                artists.extend(
                    all_relations
                        .iter()
                        .filter_map(|r| r.artist.as_ref())
                        .map(|a| a.clone().into()),
                );

                artist_credits.extend(
                    track
                        .recording
                        .artist_credit
                        .as_ref()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|ac| ac.clone().into()),
                );

                artist_credit_tracks.extend(
                    track
                        .recording
                        .artist_credit
                        .as_ref()
                        .unwrap_or(&vec![])
                        .into_iter()
                        .map(|ac| entity::ArtistCreditTrack {
                            artist_credit_id: ac.id(),
                            track_id: track.id,
                        }),
                );

                artist_track_relations.extend(
                    all_relations
                        .iter()
                        .filter_map(|r| {
                            r.artist
                                .as_ref()
                                .map(|a| (r.type_field.to_owned(), a.to_owned()))
                        })
                        .map(|(t, artist)| entity::ArtistTrackRelation {
                            artist_id: artist.id,
                            track_id: track.id,
                            relation_type: t.clone().into(),
                            relation_value: t,
                        }),
                );

                tracks.push(TrackWithMediumId(track.to_owned(), medium.id.unwrap()).into());
            }
        }

        let genres = tracks
            .iter()
            .flat_map(|track: &entity::Track| track.genres.0.to_owned())
            .collect();

        Self {
            artist_credits,
            artists,
            release: entity::Release {
                id: release.id,
                title: release.title,
                release_group_id: release.release_group.as_ref().map(|r| r.id),
                release_type: release
                    .release_group
                    .as_ref()
                    .and_then(|r| r.primary_type.as_ref())
                    .map(|s| s.to_lowercase()),
                genres: entity::Genres(dedup(genres)),
                asin: release.asin,
                country: release.country,
                label: label
                    .as_ref()
                    .and_then(|li| li.label.as_ref())
                    .map(|l| l.name.to_string()),
                catalog_no: label.as_ref().and_then(|l| l.catalog_number.clone()),
                status: release.status,
                year: date.year,
                month: date.month.map(|m| m as i16),
                day: date.day.map(|d| d as i16),
                original_year: original_date.year,
                original_month: original_date.month.map(|m| m as i16),
                original_day: original_date.day.map(|d| d as i16),
                script: release.text_representation.and_then(|t| t.script),
                path: None,
            },
            mediums: mediums
                .into_iter()
                .map(|m| entity::Medium {
                    id: m.id.unwrap(),
                    release_id: release.id,
                    position: m.position.unwrap_or_default(),
                    tracks: m.track_count,
                    track_offset: m.track_offset.unwrap_or_default(),
                    format: m.format.clone(),
                })
                .collect(),
            tracks,
            artist_credit_releases: release
                .artist_credit
                .iter()
                .map(|ac| entity::ArtistCreditRelease {
                    artist_credit_id: ac.id(),
                    release_id: release.id,
                })
                .collect(),
            artist_credit_tracks,
            artist_track_relations,
        }
    }
}

pub struct CombinedSearchResults {
    pub artists: Vec<entity::Artist>,
    pub artist_credits: Vec<entity::ArtistCredit>,
    pub releases: Vec<entity::Release>,
    pub mediums: Vec<entity::Medium>,
    pub tracks: Vec<entity::Track>,
    pub artist_track_relations: Vec<entity::ArtistTrackRelation>,
    pub artist_credit_releases: Vec<entity::ArtistCreditRelease>,
    pub artist_credit_tracks: Vec<entity::ArtistCreditTrack>,
}

impl From<Vec<musicbrainz::Release>> for CombinedSearchResults {
    fn from(musicbrainz_releases: Vec<musicbrainz::Release>) -> Self {
        let mut artists = Vec::new();
        let mut artist_credits = Vec::new();
        let mut releases = Vec::new();
        let mut mediums = Vec::new();
        let mut tracks = Vec::new();
        let mut artist_track_relations = Vec::new();
        let mut artist_credit_releases = Vec::new();
        let mut artist_credit_tracks = Vec::new();

        for release in musicbrainz_releases.into_iter() {
            let SearchResult {
                artists: partial_artists,
                artist_credits: partial_artist_credits,
                release,
                mediums: partial_mediums,
                tracks: partial_tracks,
                artist_track_relations: partial_artist_track_relations,
                artist_credit_tracks: partial_artist_credit_tracks,
                artist_credit_releases: partial_artist_credit_releases,
            } = release.into();
            artists.extend(partial_artists.into_iter());
            artist_credits.extend(partial_artist_credits.into_iter());
            releases.push(release);
            mediums.extend(partial_mediums.into_iter());
            tracks.extend(partial_tracks.into_iter());
            artist_track_relations.extend(partial_artist_track_relations.into_iter());
            artist_credit_releases.extend(partial_artist_credit_releases.into_iter());
            artist_credit_tracks.extend(partial_artist_credit_tracks.into_iter());
        }

        Self {
            artists: dedup(artists),
            artist_credits: dedup(artist_credits),
            releases: dedup(releases),
            mediums: dedup(mediums),
            tracks: dedup(tracks),
            artist_track_relations: dedup(artist_track_relations),
            artist_credit_releases: dedup(artist_credit_releases),
            artist_credit_tracks: dedup(artist_credit_tracks),
        }
    }
}
