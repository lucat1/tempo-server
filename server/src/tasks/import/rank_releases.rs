use eyre::{eyre, Result};
use levenshtein::levenshtein;
use pathfinding::kuhn_munkres::kuhn_munkres_min;
use pathfinding::matrix::Matrix;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Sub, sync::Arc};
use taskie_client::{Task as TaskieTask, TaskKey};
use uuid::Uuid;

use crate::tasks::TaskName;

static TRACK_TITLE_FACTOR: usize = 5000;
static TRACK_LENGTH_FACTOR: u32 = 300;
static TRACK_DISC_FACTOR: u32 = 100;
static TRACK_NUMBER_FACTOR: u32 = 200;

static RELEASE_TITLE_FACTOR: usize = 1000;
static RELEASE_MEDIA_FACTOR: usize = 10;
static RELEASE_DISCS_FACTOR: u32 = 100;
static RELEASE_TRACKS_FACTOR: u32 = 1000;
static RELEASE_COUNTRY_FACTOR: usize = 5;
static RELEASE_LABEL_FACTOR: usize = 5;
static RELEASE_RELEASE_TYPE_FACTOR: usize = 50;
static RELEASE_YEAR_FACTOR: i32 = 100;
static RELEASE_MONTH_FACTOR: i64 = 50;
static RELEASE_DAY_FACTOR: i64 = 10;
static RELEASE_ORIGINAL_YEAR_FACTOR: i32 = 100;
static RELEASE_ORIGINAL_MONTH_FACTOR: i64 = 50;
static RELEASE_ORIGINAL_DAY_FACTOR: i64 = 10;

pub trait Diff {
    fn diff(&self, other: &Self) -> i64;
}

fn if_both<T, R>(a: Option<T>, b: Option<T>, then: impl Fn(T, T) -> R) -> Option<R> {
    if let Some(a_val) = a {
        if let Some(b_val) = b {
            return Some(then(a_val, b_val));
        }
    }
    None
}

impl Diff for entity::InternalTrack {
    fn diff(&self, other: &Self) -> i64 {
        // TODO: diff artists
        ((levenshtein(self.title.as_str(), other.title.as_str()) * TRACK_TITLE_FACTOR) as i64)
            + if_both(self.length, other.length, |l1, l2| {
                (l1.abs_diff(l2) * TRACK_LENGTH_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.disc, other.disc, |n1, n2| {
                (n1.abs_diff(n2) * TRACK_DISC_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.number, other.number, |n1, n2| {
                (n1.abs_diff(n2) * TRACK_NUMBER_FACTOR) as i64
            })
            .unwrap_or_default()
    }
}

impl Diff for entity::InternalRelease {
    fn diff(&self, other: &Self) -> i64 {
        // TODO: diff artists
        ((levenshtein(self.title.as_str(), other.title.as_str()) * RELEASE_TITLE_FACTOR) as i64)
            + if_both(self.media.as_ref(), other.media.as_ref(), |l1, l2| {
                (levenshtein(l1.as_str(), l2.as_str()) * RELEASE_MEDIA_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.discs, other.discs, |n1, n2| {
                (n1.abs_diff(n2) * RELEASE_DISCS_FACTOR) as i64
            })
            .unwrap_or_default()
            + ((self.tracks.abs_diff(other.tracks) * RELEASE_TRACKS_FACTOR) as i64)
            + if_both(self.country.as_ref(), other.country.as_ref(), |c1, c2| {
                (levenshtein(c1.as_str(), c2.as_str()) * RELEASE_COUNTRY_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.label.as_ref(), other.label.as_ref(), |l1, l2| {
                (levenshtein(l1.as_str(), l2.as_str()) * RELEASE_LABEL_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(
                self.release_type.as_ref(),
                other.release_type.as_ref(),
                |r1, r2| {
                    (levenshtein(r1.as_str(), r2.as_str()) * RELEASE_RELEASE_TYPE_FACTOR) as i64
                },
            )
            .unwrap_or_default()
            + if_both(self.year, other.year, |d1, d2| {
                (d1.sub(d2).abs() * RELEASE_YEAR_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.month, other.month, |d1, d2| {
                (d1.abs_diff(d2) as i64) * RELEASE_MONTH_FACTOR
            })
            .unwrap_or_default()
            + if_both(self.day, other.day, |d1, d2| {
                (d1.abs_diff(d2) as i64) * RELEASE_DAY_FACTOR
            })
            .unwrap_or_default()
            + if_both(self.original_year, other.original_year, |d1, d2| {
                (d1.sub(d2).abs() * RELEASE_ORIGINAL_YEAR_FACTOR) as i64
            })
            .unwrap_or_default()
            + if_both(self.original_month, other.original_month, |d1, d2| {
                (d1.abs_diff(d2) as i64) * RELEASE_ORIGINAL_MONTH_FACTOR
            })
            .unwrap_or_default()
            + if_both(self.original_day, other.original_day, |d1, d2| {
                (d1.abs_diff(d2) as i64) * RELEASE_ORIGINAL_DAY_FACTOR
            })
            .unwrap_or_default()
    }
}

pub fn rate_and_match(
    (release, tracks): (&entity::InternalRelease, &[entity::InternalTrack]),
    (full_release, full_tracks): (&entity::full::FullRelease, &[entity::full::FullTrack]),
) -> entity::import::ReleaseRating {
    let candidate_release: entity::InternalRelease = full_release.clone().into();

    let rows = tracks.len();
    let mut columns = full_tracks.len();
    let mut matrix_vec = vec![];

    for original_track in tracks.iter() {
        for candidate_track in full_tracks.iter() {
            matrix_vec.push(original_track.diff(&candidate_track.clone().into()));
        }
    }
    if matrix_vec.is_empty() {
        return entity::import::ReleaseRating {
            score: 0,
            assignment: HashMap::new(),
        };
    }
    tracing::debug!(%rows, %columns, "The kuhn_munkers matrix has size");
    if rows > columns {
        let max = match matrix_vec.iter().max() {
            Some(v) => *v,
            None => i64::MAX / (rows as i64),
        } + 1;
        for _ in 0..((rows - columns) * rows) {
            matrix_vec.push(max);
        }
        columns = rows
    }
    let matrix = Matrix::from_vec(rows, columns, matrix_vec);
    tracing::info!(
        %matrix.rows, %matrix.columns,
        source = %tracks.len(), candidates = %full_tracks.len(),
        "Adjacency matrix (rows = source, columns = candidates)"
    );
    let (val, map) = kuhn_munkres_min(&matrix);
    let score = val + release.diff(&candidate_release);
    tracing::debug!(
        original = ?(release, tracks),
        candidate = ?(candidate_release.artists, candidate_release.title),
        %score,
        "Release/tracks match value"
    );
    entity::import::ReleaseRating {
        score,
        assignment: map
            .into_iter()
            .enumerate()
            .filter_map(|(src, candidate)| {
                full_tracks
                    .get(candidate)
                    .map(|ft| (src, ft.get_track().id))
            })
            .collect(),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Data(pub Uuid);

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, _task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let import = entity::ImportEntity::find_by_id(self.0)
            .one(&tx)
            .await?
            .ok_or(eyre!("Import not found"))?;
        let rc_import = Arc::new(import.clone());
        tracing::info!(id = %import.id, "Ranking releases for import");

        let mut release_matches = HashMap::new();
        let mut best_rate = None;
        let mut best_release = None;
        for release in rc_import.releases.0.iter() {
            let full_release = entity::full::FullRelease::new(rc_import.clone(), release.id)?;
            let full_tracks = full_release.get_full_tracks()?;
            let rating = rate_and_match(
                (&rc_import.source_release, &rc_import.source_tracks.0),
                (&full_release, &full_tracks),
            );
            let entity::import::ReleaseRating { score, .. } = rating;
            (best_rate, best_release) = match best_rate {
                Some(best) if score < best => (Some(score), Some(release.id)),
                None => (Some(score), Some(release.id)),
                _ => (best_rate, best_release),
            };
            release_matches.insert(release.id, rating);
        }

        let mut import_active = import.into_active_model();
        import_active.release_matches =
            ActiveValue::Set(entity::import::ReleaseMatches(release_matches));
        import_active.selected_release = ActiveValue::Set(best_release);
        import_active.update(&tx).await?;
        Ok(tx.commit().await?)
    }
}
