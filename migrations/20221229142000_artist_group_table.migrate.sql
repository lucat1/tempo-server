CREATE TABLE IF NOT EXISTS artist_credits (
  id int AUTOINCREMENT PRIMARY KEY,
  artist BLOB NOT NULL,
  join_phrase VARCHAR(256),

  UNIQUE(artist, join_phrase),
  FOREIGN KEY(artist) REFERENCES releases(artists)
);
