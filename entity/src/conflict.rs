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
            ReleaseColumn::Date,
            ReleaseColumn::OriginalDate,
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
            TrackColumn::Genres,
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
    ])
    .update_column(ArtistTrackRelationColumn::RelationValue)
    .to_owned();
}
