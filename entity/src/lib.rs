mod genres;
mod medium;
mod release;
mod track;
mod track_format;

mod artist;
mod artist_credit;
mod artist_credit_release;
mod artist_credit_track;
mod artist_track_relation;
mod artist_url;

mod image;
mod image_artist;
mod image_release;

mod scrobble;
mod user;
pub mod user_connection;

pub mod import;

pub mod conflict;
pub mod full;

mod update_artist;

use eyre::Result;
use sea_orm::DbErr;
use uuid::Uuid;

pub use artist::ActiveModel as ArtistActive;
pub use artist::Column as ArtistColumn;
pub use artist::Entity as ArtistEntity;
pub use artist::Model as Artist;
pub use artist::Relation as ArtistRelation;
pub use artist_credit::ActiveModel as ArtistCreditActive;
pub use artist_credit::Column as ArtistCreditColumn;
pub use artist_credit::Entity as ArtistCreditEntity;
pub use artist_credit::Model as ArtistCredit;
pub use artist_credit::Relation as ArtistCreditRelation;
pub use artist_track_relation::ActiveModel as ArtistTrackRelationActive;
pub use artist_track_relation::Column as ArtistTrackRelationColumn;
pub use artist_track_relation::Entity as ArtistTrackRelationEntity;
pub use artist_track_relation::Model as ArtistTrackRelation;
pub use artist_track_relation::Relation as ArtistTrackRelationRelation;
pub use artist_track_relation::RelationType as ArtistTrackRelationType;
pub use artist_url::ActiveModel as ArtistUrlActive;
pub use artist_url::Column as ArtistUrlColumn;
pub use artist_url::Entity as ArtistUrlEntity;
pub use artist_url::Model as ArtistUrl;
pub use artist_url::Relation as ArtistUrlRelation;
pub use artist_url::UrlType as ArtistUrlType;
pub use genres::Genres;
pub use medium::ActiveModel as MediumActive;
pub use medium::Column as MediumColumn;
pub use medium::Entity as MediumEntity;
pub use medium::Model as Medium;
pub use medium::Relation as MediumRelation;
pub use release::ActiveModel as ReleaseActive;
pub use release::Column as ReleaseColumn;
pub use release::Entity as ReleaseEntity;
pub use release::Model as Release;
pub use release::Relation as ReleaseRelation;
pub use track::ActiveModel as TrackActive;
pub use track::Column as TrackColumn;
pub use track::Entity as TrackEntity;
pub use track::Model as Track;
pub use track::Relation as TrackRelation;
pub use track::TrackToArtist;
pub use track::TrackToPerformer;
pub use track::TrackToRelease;
pub use track_format::TrackFormat;

pub use artist_credit_release::ActiveModel as ArtistCreditReleaseActive;
pub use artist_credit_release::Column as ArtistCreditReleaseColumn;
pub use artist_credit_release::Entity as ArtistCreditReleaseEntity;
pub use artist_credit_release::Model as ArtistCreditRelease;
pub use artist_credit_release::Relation as ArtistCreditReleaseRelation;
pub use artist_credit_track::ActiveModel as ArtistCreditTrackActive;
pub use artist_credit_track::Column as ArtistCreditTrackColumn;
pub use artist_credit_track::Entity as ArtistCreditTrackEntity;
pub use artist_credit_track::Model as ArtistCreditTrack;
pub use artist_credit_track::Relation as ArtistCreditTrackRelation;

pub use image::ActiveModel as ImageActive;
pub use image::Column as ImageColumn;
pub use image::Entity as ImageEntity;
pub use image::Model as Image;
pub use image::Relation as ImageRelation;
pub use image_artist::ActiveModel as ImageArtistActive;
pub use image_artist::Column as ImageArtistColumn;
pub use image_artist::Entity as ImageArtistEntity;
pub use image_artist::Model as ImageArtist;
pub use image_artist::Relation as ImageArtistRelation;
pub use image_release::ActiveModel as ImageReleaseActive;
pub use image_release::Column as ImageReleaseColumn;
pub use image_release::Entity as ImageReleaseEntity;
pub use image_release::Model as ImageRelease;
pub use image_release::Relation as ImageReleaseRelation;

pub use scrobble::ActiveModel as ScrobbleActive;
pub use scrobble::Column as ScrobbleColumn;
pub use scrobble::Entity as ScrobbleEntity;
pub use scrobble::Model as Scrobble;
pub use scrobble::Relation as ScrobbleRelation;
pub use user::ActiveModel as UserActive;
pub use user::Column as UserColumn;
pub use user::Entity as UserEntity;
pub use user::Model as User;
pub use user::Relation as UserRelation;
pub use user_connection::ActiveModel as UserConnectionActive;
pub use user_connection::Column as UserConnectionColumn;
pub use user_connection::ConnectionProvider;
pub use user_connection::Entity as UserConnectionEntity;
pub use user_connection::Model as UserConnection;
pub use user_connection::Relation as UserConnectionRelation;

pub use import::ActiveModel as ImportActive;
pub use import::Column as ImportColumn;
pub use import::Entity as ImportEntity;
pub use import::InternalRelease;
pub use import::InternalTrack;
pub use import::InternalTracks;
pub use import::Model as Import;
pub use import::Relation as ImportRelation;

pub use update_artist::filter as update_artist_filter;
pub use update_artist::ActiveModel as UpdateArtistActive;
pub use update_artist::Column as UpdateArtistColumn;
pub use update_artist::Entity as UpdateArtistEntity;
pub use update_artist::Model as UpdateArtist;
pub use update_artist::Relation as UpdateArtistRelation;
pub use update_artist::UpdateType as UpdateArtistType;

pub trait IgnoreNone {
    fn ignore_none(self) -> Result<(), DbErr>;
}

impl<T> IgnoreNone for Result<T, DbErr> {
    fn ignore_none(self) -> Result<(), DbErr> {
        match self {
            Err(DbErr::RecordNotInserted) => Ok(()),
            Err(v) => Err(v),
            Ok(_) => Ok(()),
        }
    }
}
