mod artist;
mod medium;
mod release;
mod track;

mod artist_release;
mod artist_track;

pub use artist::Column as ArtistColumn;
pub use artist::Entity as ArtistEntity;
pub use artist::Model as Artist;
pub use medium::Column as MediumColumn;
pub use medium::Entity as MediumEntity;
pub use medium::Model as Medium;
pub use release::Column as ReleaseColumn;
pub use release::Entity as ReleaseEntity;
pub use release::Model as Release;
pub use track::Column as TrackColumn;
pub use track::Entity as TrackEntity;
pub use track::Model as Track;

pub use artist_release::Column as ArtistReleaseColumn;
pub use artist_release::Entity as ArtistReleaseEntity;
pub use artist_release::Model as ArtistRelease;
pub use artist_track::Column as ArtistTrackColumn;
pub use artist_track::Entity as ArtistTrackEntity;
pub use artist_track::Model as ArtistTrack;
