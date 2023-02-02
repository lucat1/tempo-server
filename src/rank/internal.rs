use chrono::NaiveDate;

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
