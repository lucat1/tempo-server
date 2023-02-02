use super::internal::Release;
use crate::fetch::Cover;
use crate::settings::ArtProvider;
use crate::SETTINGS;
use levenshtein::levenshtein;
use std::cmp::Ordering;

static MAX_COVER_SIZE: usize = 5000 * 5000;

#[derive(Debug, Clone)]
pub struct CoverRating(pub f64, pub Cover);

impl PartialEq for CoverRating {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}
impl Eq for CoverRating {}

impl PartialOrd for CoverRating {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl Ord for CoverRating {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

fn in_range(val: f64, min: f64, max: f64) -> f64 {
    val / (max - min)
}

fn valuate_cover(levenshtein: f64, cover: &Cover) -> f64 {
    let art_settings = &SETTINGS.get().unwrap().art;
    let provider_index = art_settings
        .providers
        .iter()
        .position(|p| *p == cover.provider)
        .unwrap();

    in_range(
        provider_index as f64,
        0.0,
        art_settings.providers.len() as f64,
    ) * art_settings.provider_relevance
        + levenshtein * art_settings.match_relevance
        + in_range(
            (cover.width * cover.height) as f64,
            0.0,
            MAX_COVER_SIZE as f64,
        ) * art_settings.size_relevance
}

pub fn rank_covers(covers_by_provider: Vec<Vec<Cover>>, release: &Release) -> Vec<CoverRating> {
    let mut vec: Vec<CoverRating> = covers_by_provider
        .into_iter()
        .flat_map(|covers| {
            covers.into_iter().map(|cover| {
                let mut distance = 1.0
                    - ((levenshtein(cover.title.as_str(), release.title.as_str())
                        + levenshtein(cover.artist.as_str(), release.artists.joined().as_str()))
                        as f64
                        / (cover.title.len().max(release.title.len())
                            + cover.artist.len().max(release.artists.joined().len()))
                            as f64);
                if cover.provider == ArtProvider::CoverArtArchive {
                    distance = 0.9; // TODO: better way? otherwise art from the CoverArtArchive always
                                    // achieves the best score
                }
                CoverRating(valuate_cover(distance, &cover), cover)
            })
        })
        .collect();
    vec.sort();
    vec.reverse();
    vec
}
