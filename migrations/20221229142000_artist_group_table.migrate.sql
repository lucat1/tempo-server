CREATE TABLE IF NOT EXISTS artist_credits (
  id integer PRIMARY KEY,
  artist blob NOT NULL,
  join_phrase varchar(256),

  UNIQUE(artist, join_phrase),
  FOREIGN KEY(artist) REFERENCES releases(artists)
);
