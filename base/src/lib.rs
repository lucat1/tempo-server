pub mod database;
pub mod image_format;
pub mod setting;
pub mod util;

pub const CLI_NAME: &str = "tagger";
pub const VERSION: &str = "0.1.0";
pub const GITHUB: &str = "github.com/lucat1/tagger";

// logging constants
pub const TAGGER_LOGLEVEL: &str = "TAGGER_LOGLEVEL";

pub use image_format::ImageFormat;
