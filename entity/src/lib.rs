mod artist;
mod artist_release;
mod release;

pub use artist::Column as ArtistColumn;
pub use artist::Entity as ArtistEntity;
pub use artist_release::Column as ArtistReleaseColumn;
pub use artist_release::Entity as ArtistReleaseEntity;
pub use release::Column as ReleaseColumn;
pub use release::Entity as ReleaseEntity;
