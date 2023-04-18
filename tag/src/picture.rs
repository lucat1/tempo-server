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

impl ToString for PictureType {
    fn to_string(&self) -> String {
        match self {
            PictureType::PublisherLogo => "publisher_logo",
            PictureType::Other => "other",
            PictureType::OtherIcon => "other_icon",
            PictureType::Icon => "icon",
            PictureType::CoverFront => "cover_front",
            PictureType::CoverBack => "cover_back",
            PictureType::Leaflet => "leaflet",
            PictureType::Media => "media",
            PictureType::LeadArtist => "lead_artist",
            PictureType::Artist => "artist",
            PictureType::Conductor => "conductor",
            PictureType::Band => "band",
            PictureType::BandLogo => "band_logo",
            PictureType::Composer => "composer",
            PictureType::Lyricist => "lyricist",
            PictureType::RecordingLocation => "recording_location",
            PictureType::DuringRecording => "during_recording",
            PictureType::DuringPerformance => "during_performance",
            PictureType::ScreenCapture => "screen_capture",
            PictureType::BrightFish => "bright_fish",
            PictureType::Illustration => "illustration",
        }
        .to_string()
    }
}

#[derive(Clone, Debug)]
pub struct Picture {
    pub mime_type: mime::Mime,
    pub picture_type: PictureType,
    pub description: String,
    pub data: Vec<u8>,
}
