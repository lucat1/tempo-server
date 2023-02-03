use chrono::NaiveDate;

pub const UNKNOWN_ARTIST: &str = "(unkown artist)";
pub const UNKNOWN_TITLE: &str = "(unkown title)";

pub struct Track {
    title: String,
    artists: Vec<String>,
    length: Option<String>,
    disc: Option<u64>,
    number: Option<u64>,
}

pub struct Release {
    title: String,
    artists: Vec<String>,
    media: Option<String>,
    discs: Option<u64>,
    tracks: Option<u64>,
    country: Option<String>,
    label: Option<String>,
    release_type: Option<String>,
    date: Option<NaiveDate>,
    original_date: Option<NaiveDate>,
}

impl From<entity::FullRelease> for Release {
    fn from(full_release: entity::FullRelease) -> Self {
        let (release, mediums, _, artists) = full_release;
        Release {
            title: release
                .title
                .into_value()
                .unwrap_or_else(|| UNKNOWN_TITLE.to_string()),
            artists: artists
                .into_iter()
                .filter_map(|a| a.name.into_value())
                .collect(),
            discs: Some(mediums.len()),
            media: mediums.first().and_then(|m| m.format),
            tracks: None, // TODO: consider adding a track count in the media structure
            country: release.country.into_value(),
            label: release.label.into_value(),
            release_type: release.release_type.into_value(),
            date: release.date.into_value(),
            original_date: release.original_date.into_value(),
        }
    }
}

impl From<entity::FullTrack> for Track {
    fn from(full_track: entity::FullTrack) -> Self {
        let (track, _, _, artists) = full_track;
        Track {
            title: track
                .title
                .into_value()
                .unwrap_or_else(|| UNKNOWN_TITLE.to_string()),
            artists: artists
                .into_iter()
                .filter_map(|a| a.name.into_value())
                .collect(),
            length: track.length.into_value(),
            disc: None, // TODO: see above
            number: track.number.into_value(),
        }
    }
}
