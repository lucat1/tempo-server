// use crate::library::Format;
use eyre::Result;
use log::info;
use std::time::Instant;

static DEFAULT_FORMAT_ARTIST: &str = "{name} {join_phrase}";
static DEFAULT_FORMAT_TRACK: &str = "{album_artist} - {album} - {track_title}";
static DEFAULT_FORMAT_RELEASE: &str = "{album_artist} - {album} ({release_year}) ({release_type})";

pub async fn list(
    _filters: Vec<&String>,
    format: Option<&String>,
    object: Option<&String>,
) -> Result<()> {
    let start = Instant::now();
    // let object = object.map_or("track", |s| s.as_str());
    // let (objects, format) = match object {
    //     "artist" | "artists" => (
    //         Artist::filter::<String, String>(vec![], vec![" ORDER BY sort_name".to_string()])
    //             .await?
    //             .into_iter()
    //             .map(|a| Box::new(a) as Box<dyn Format>)
    //             .collect::<Vec<_>>(),
    //         format.map_or(DEFAULT_FORMAT_ARTIST, |s| s.as_str()),
    //     ),
    //     "track" | "tracks" => {
    //         let track = Track::filter::<String, String>(
    //             vec![],
    //             vec![" ORDER BY tracks.release, tracks.disc, tracks.number".to_string()],
    //         )
    //         .await?
    //         .into_iter()
    //         .map(|t| Box::new(t) as Box<dyn Format>)
    //         .collect();
    //         (track, format.map_or(DEFAULT_FORMAT_TRACK, |s| s.as_str()))
    //     }
    //
    //     "release" | "releases" => (
    //         Release::filter::<String, String>(vec![], vec![" ORDER BY title".to_string()])
    //             .await?
    //             .into_iter()
    //             .map(|r| Box::new(r) as Box<dyn Format>)
    //             .collect(),
    //         format.map_or(DEFAULT_FORMAT_RELEASE, |s| s.as_str()),
    //     ),
    //     _ => {
    //         bail!("Invalid object type {}", object)
    //     }
    // };
    // for track in objects.into_iter() {
    //     println!("{}", track.fmt(format)?);
    // }
    info!("Took {:?}", start.elapsed());
    Ok(())
}
