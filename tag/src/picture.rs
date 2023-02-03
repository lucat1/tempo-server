#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PictureType {
    Other,
    Icon,
    OtherIcon,
    CoverFront,
    CoverBack,
    Leaflet,
    Media,
    LeadArtist,
    Artist,
    Conductor,
    Band,
    Composer,
    Lyricist,
    RecordingLocation,
    DuringRecording,
    DuringPerformance,
    ScreenCapture,
    BrightFish,
    Illustration,
    BandLogo,
    PublisherLogo,
}

#[derive(Clone, Debug)]
pub struct Picture {
    pub mime_type: mime::Mime,
    pub picture_type: PictureType,
    pub description: String,
    pub data: Vec<u8>,
}
