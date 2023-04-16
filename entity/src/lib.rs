mod artist;
mod medium;
mod release;
mod track;
mod track_format;

mod artist_credit;
mod artist_credit_release;
mod artist_credit_track;
mod artist_track_relation;

pub mod conflict;
pub mod full;

use eyre::Result;
use uuid::Uuid;

pub use artist::ActiveModel as ArtistActive;
pub use artist::Column as ArtistColumn;
pub use artist::Entity as ArtistEntity;
pub use artist::Model as Artist;
pub use artist_credit::ActiveModel as ArtistCreditActive;
pub use artist_credit::Column as ArtistCreditColumn;
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
pub use track::Genres;
pub use track::Model as Track;
pub use track::TrackToArtist;
pub use track::TrackToPerformer;
pub use track::TrackToRelease;
pub use track_format::TrackFormat;

pub use artist_credit_release::ActiveModel as ArtistCreditReleaseActive;
pub use artist_credit_release::Column as ArtistCreditReleaseColumn;
pub use artist_credit_release::Entity as ArtistCreditReleaseEntity;
pub use artist_credit_release::Model as ArtistCreditRelease;
pub use artist_credit_track::ActiveModel as ArtistCreditTrackActive;
pub use artist_credit_track::Column as ArtistCreditTrackColumn;
pub use artist_credit_track::Entity as ArtistCreditTrackEntity;
pub use artist_credit_track::Model as ArtistCreditTrack;
