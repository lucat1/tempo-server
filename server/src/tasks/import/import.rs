use eyre::{bail, eyre, Result, WrapErr};
use reqwest::{Method, Request};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use taskie_client::{InsertTask, Task as TaskieTask, TaskKey};
use time::Duration;
use uuid::Uuid;

use crate::{
    fetch::musicbrainz::{self, MB_BASE_URL},
    import::{CombinedSearchResults, UNKNOWN_ARTIST},
    tasks::{push, TaskName},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Data(pub Uuid);

// pub async fn get_cover(library: &Library, cover: &Cover) -> Result<(Vec<u8>, (u32, u32), Mime)> {
//     let start = Instant::now();
//     let res = CLIENT.get(cover.url.clone()).send().await?;
//     let req_time = start.elapsed();
//     tracing::trace! {?req_time, "Fetch request for cover art took"};
//     if !res.status().is_success() {
//         bail!(
//             "Fetch request for cover art returned non-success error code: {} {}",
//             res.status(),
//             res.text().await?
//         );
//     }
//     let bytes = res.bytes().await?;
//     let bytes_time = start.elapsed();
//     let img = ImageReader::new(Cursor::new(bytes))
//         .with_guessed_format()?
//         .decode()?;
//     tracing::trace! {prase_time = ?(bytes_time - req_time), "Parse of cover art took"};
//     let resized = if library.art.width < img.width() || library.art.height < img.height() {
//         let converted = resize(
//             &img,
//             library.art.width,
//             library.art.height,
//             FilterType::Gaussian,
//         );
//         let convert_time = start.elapsed();
//         tracing::trace! {
//             convert_time = ?(convert_time - bytes_time - req_time),
//             src_width = img.width(),
//             src_height = img.height(),
//             dst_width = converted.width(),
//             dst_height = converted.height(),
//             "Done scaling/converting image",
//         };
//         DynamicImage::ImageRgba8(converted)
//     } else {
//         img
//     };
//     let mut bytes: Vec<u8> = Vec::new();
//     let format: ImageOutputFormat = library.art.format.into();
//     resized.write_to(&mut Cursor::new(&mut bytes), format)?;
//     Ok((
//         bytes,
//         (resized.width(), resized.height()),
//         library.art.format.mime(),
//     ))
// }

#[async_trait::async_trait]
impl crate::tasks::TaskTrait for Data {
    async fn run<C>(&self, db: &C, task: TaskieTask<TaskName, TaskKey>) -> Result<()>
    where
        C: ConnectionTrait + TransactionTrait,
    {
        let tx = db.begin().await?;
        let import = entity::ImportEntity::find_by_id(self.0)
            .one(&tx)
            .await?
            .ok_or(eyre!("Import not found"))?;

        tracing::info!(id = %import.id, "Fetching covers for import");
        Ok(())
    }
}
