CREATE TABLE IF NOT EXISTS artists (
  mbid BLOB PRIMARY KEY,
  name TEXT NOT NULL,
  sort_name TEXT,
  instruments TEXT
);
CREATE TABLE IF NOT EXISTS releases (
  mbid BLOB PRIMARY KEY,
  release_group_mbid BLOB,
  asin TEXT,
  title TEXT NOT NULL,
  discs NUMBER,
  media TEXT,
  tracks NUMBER,
  country TEXT,
  label TEXT,
  catalog_no TEXT,
  status TEXT,
  release_type TEXT,
  date DATE,
  original_date DATE,
  script TEXT,
  UNIQUE(mbid)
);

CREATE TABLE IF NOT EXISTS release_artists (
  ref BLOB,
  artist BLOB,
  FOREIGN KEY(ref) REFERENCES releases(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(ref,artist)
);

CREATE TABLE IF NOT EXISTS tracks (
	mbid BLOB PRIMARY KEY,
  title TEXT NOT NULL,
  length INTEGER,
  disc INTEGER,
  disc_mbid BLOB,
  number INTEGER,
  genres TEXT,
  release BLOB,
  format TEXT,
  path TEXT,
  FOREIGN KEY(release) REFERENCES releases(mbid)
);

CREATE TABLE IF NOT EXISTS track_artists (
  ref BLOB,
  artist BLOB,
  FOREIGN KEY(ref) REFERENCES tracks(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(ref,artist)
);

CREATE TABLE IF NOT EXISTS track_performers (
  ref BLOB,
  artist BLOB,
  FOREIGN KEY(ref) REFERENCES tracks(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(ref,artist)
);

CREATE TABLE IF NOT EXISTS track_engigneers (
  ref BLOB,
  artist BLOB,
  FOREIGN KEY(ref) REFERENCES tracks(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(ref,artist)
);

CREATE TABLE IF NOT EXISTS track_mixers (
  ref BLOB,
  artist BLOB,
  FOREIGN KEY(ref) REFERENCES tracks(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(ref,artist)
);

CREATE TABLE IF NOT EXISTS track_producers (
  ref BLOB,
  artist BLOB,
  FOREIGN KEY(ref) REFERENCES tracks(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(ref,artist)
);

CREATE TABLE IF NOT EXISTS track_lyricists (
  ref BLOB,
  artist BLOB,
  FOREIGN KEY(ref) REFERENCES tracks(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(ref,artist)
);

CREATE TABLE IF NOT EXISTS track_writers (
  ref BLOB,
  artist BLOB,
  FOREIGN KEY(ref) REFERENCES tracks(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(ref,artist)
);

CREATE TABLE IF NOT EXISTS track_composers (
  ref BLOB,
  artist BLOB,
  FOREIGN KEY(ref) REFERENCES tracks(mbid),
  FOREIGN KEY(artist) REFERENCES artists(mbid),
  UNIQUE(ref,artist)
);
