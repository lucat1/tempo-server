use lazy_static::lazy_static;
use tantivy::schema::{Schema, STORED, STRING, TEXT};

lazy_static! {
    pub static ref ARTISTS_SCHEMA: Schema = make_artists_schema();
    pub static ref TRACKS_SCHEMA: Schema = make_tracks_schema();
    pub static ref RELEASES_SCHEMA: Schema = make_releases_schema();
}

fn make_artists_schema() -> Schema {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("id", STORED);
    schema_builder.add_text_field("name", TEXT);
    schema_builder.add_text_field("sort_name", TEXT);
    schema_builder.add_text_field("description", STRING);
    schema_builder.build()
}

fn make_tracks_schema() -> Schema {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("id", STORED);
    schema_builder.add_text_field("artists", TEXT);
    schema_builder.add_text_field("title", TEXT);
    schema_builder.add_text_field("genres", TEXT);
    schema_builder.build()
}

fn make_releases_schema() -> Schema {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("id", STORED);
    schema_builder.add_text_field("artists", TEXT);
    schema_builder.add_text_field("title", TEXT);
    schema_builder.add_text_field("release_type", STRING);
    schema_builder.add_text_field("genres", TEXT);
    schema_builder.build()
}
