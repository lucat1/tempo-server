CREATE TABLE releases (
  mbid BLOB PRIMARY KEY,
  title TEXT NOT NULL
);

CREATE TABLE release_artists (
  release BLOB,
  artist BLOB,
  FOREIGN KEY(release) REFERENCES releases(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(release,artist)
);

CREATE TABLE release_tracks (
  release BLOB,
  track BLOB,
  FOREIGN KEY(release) REFERENCES releases(mbid),
  FOREIGN KEY(track) REFERENCES tracks(mbid),
  UNIQUE(release,track)
);

CREATE TABLE tracks (
	mbid BLOB PRIMARY KEY,
  title TEXT NOT NULL,
  length INTEGER,
  disc INTEGER,
  number INTEGER,
  path TEXT,
  UNIQUE(path)
);

CREATE TABLE track_artists (
  track BLOB,
  artist BLOB,
  FOREIGN KEY(track) REFERENCES tracks(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(track,artist)
);

CREATE TABLE artists (
  mbid BLOB PRIMARY KEY,
  name TEXT NOT NULL,
  sort_name TEXT
);
