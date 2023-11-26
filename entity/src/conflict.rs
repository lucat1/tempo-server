use lazy_static::lazy_static;
use sea_orm::sea_query::OnConflict;

use crate::*;

lazy_static! {
    pub static ref ARTIST_CONFLICT: OnConflict = OnConflict::column(ArtistColumn::Id)
        .update_columns([ArtistColumn::Name, ArtistColumn::SortName])
        .to_owned();
    pub static ref ARTIST_CREDIT_CONFLICT: OnConflict = OnConflict::column(ArtistCreditColumn::Id)
        .do_nothing()
        .to_owned();
    pub static ref IMAGE_CONFLICT_1: OnConflict = OnConflict::column(ImageColumn::Id)
        .update_columns([
            ImageColumn::Role,
            ImageColumn::Format,
            ImageColumn::Description,
            ImageColumn::Width,
            ImageColumn::Height,
            ImageColumn::Path
        ])
        .to_owned();
    pub static ref IMAGE_CONFLICT_2: OnConflict = OnConflict::column(ImageColumn::Path)
        .update_columns([
            ImageColumn::Role,
            ImageColumn::Format,
            ImageColumn::Description,
            ImageColumn::Width,
            ImageColumn::Height,
        ])
        .to_owned();
    pub static ref IMAGE_RELEASE_CONFLICT: OnConflict =
        OnConflict::columns([ImageReleaseColumn::ImageId, ImageReleaseColumn::ReleaseId])
            .do_nothing()
            .to_owned();
    pub static ref IMAGE_ARTIST_CONFLICT: OnConflict =
        OnConflict::columns([ImageArtistColumn::ImageId, ImageArtistColumn::ArtistId])
            .do_nothing()
            .to_owned();
    pub static ref RELEASE_CONFLICT: OnConflict = OnConflict::column(ReleaseColumn::Id)
        .update_columns([
            ReleaseColumn::Title,
            ReleaseColumn::ReleaseGroupId,
            ReleaseColumn::ReleaseType,
            ReleaseColumn::Asin,
            ReleaseColumn::Country,
            ReleaseColumn::Label,
            ReleaseColumn::CatalogNo,
            ReleaseColumn::Status,
            ReleaseColumn::Year,
            ReleaseColumn::Month,
            ReleaseColumn::Day,
            ReleaseColumn::OriginalYear,
            ReleaseColumn::OriginalMonth,
            ReleaseColumn::OriginalDay,
            ReleaseColumn::Script,
        ])
        .to_owned();
    pub static ref ARTIST_CREDIT_RELEASE_CONFLICT: OnConflict = OnConflict::columns([
        ArtistCreditReleaseColumn::ArtistCreditId,
        ArtistCreditReleaseColumn::ReleaseId,
    ])
    .do_nothing()
    .to_owned();
    pub static ref MEDIUM_CONFLICT: OnConflict = OnConflict::column(MediumColumn::Id)
        .update_columns([
            MediumColumn::ReleaseId,
            MediumColumn::Position,
            MediumColumn::Tracks,
            MediumColumn::TrackOffset,
            MediumColumn::Format,
        ])
        .to_owned();
    pub static ref TRACK_CONFLICT: OnConflict = OnConflict::column(TrackColumn::Id)
        .update_columns([
            TrackColumn::Title,
            TrackColumn::Length,
            TrackColumn::Number,
            TrackColumn::Format,
            TrackColumn::Path,
        ])
        .to_owned();
    pub static ref ARTIST_CREDIT_TRACK_CONFLICT: OnConflict = OnConflict::columns([
        ArtistCreditTrackColumn::ArtistCreditId,
        ArtistCreditTrackColumn::TrackId,
    ])
    .do_nothing()
    .to_owned();
    pub static ref ARTIST_TRACK_RELATION_CONFLICT: OnConflict = OnConflict::columns([
        ArtistTrackRelationColumn::ArtistId,
        ArtistTrackRelationColumn::TrackId,
        ArtistTrackRelationColumn::RelationType,
        ArtistTrackRelationColumn::RelationValue,
    ])
    .update_column(ArtistTrackRelationColumn::RelationValue)
    .to_owned();
    pub static ref ARTIST_RELATION_CONFLICT: OnConflict =
        OnConflict::columns([ArtistUrlColumn::ArtistId, ArtistUrlColumn::Type,])
            .update_column(ArtistUrlColumn::Url)
            .to_owned();
    pub static ref USER_CONFLICT: OnConflict = OnConflict::column(UserColumn::Username)
        .update_columns([UserColumn::FirstName, UserColumn::LastName])
        .to_owned();
    pub static ref UPDATE_ARTIST_CONFLICT: OnConflict =
        OnConflict::columns([UpdateArtistColumn::Type, UpdateArtistColumn::Id])
            .update_column(UpdateArtistColumn::Time)
            .to_owned();
    pub static ref ARTIST_PICTURE_CONFLICT: OnConflict = OnConflict::columns([
        ArtistPictureColumn::ImageId,
        ArtistPictureColumn::ArtistId,
        ArtistPictureColumn::Type
    ])
    .update_column(ArtistPictureColumn::ImageId)
    .to_owned();
    pub static ref GENRE_CONFLICT: OnConflict = OnConflict::column(GenreColumn::Id)
        .update_columns([GenreColumn::Name, GenreColumn::Disambiguation])
        .to_owned();
    pub static ref GENRE_TRACK_CONFLICT: OnConflict =
        OnConflict::columns([GenreTrackColumn::GenreId, GenreTrackColumn::TrackId])
            .do_nothing()
            .to_owned();
    pub static ref GENRE_RELEASE_CONFLICT: OnConflict =
        OnConflict::columns([GenreReleaseColumn::GenreId, GenreReleaseColumn::ReleaseId])
            .do_nothing()
            .to_owned();
}
