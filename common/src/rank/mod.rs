mod cover;
mod diff;

pub use cover::{rank_covers, CoverRating};
pub use diff::Diff;
use pathfinding::kuhn_munkres::kuhn_munkres_min;
use pathfinding::matrix::Matrix;

use crate::fetch::SearchResult;
use entity::{InternalRelease, InternalTrack};
