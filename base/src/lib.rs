pub mod database;
pub mod image_format;
pub mod setting;
pub mod util;

pub const CLI_NAME: &str = "tempo";
pub const VERSION: &str = "0.1.0";
pub const GITHUB: &str = "codeberg.org/tempo/server";

// logging constants
pub const TEMPO_LOGLEVEL: &str = "TEMPO_LOGLEVEL";

pub use image_format::ImageFormat;
