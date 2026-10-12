-- Your SQL goes here
CREATE TABLE files (
  hash TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  deleted BOOLEAN NOT NULL DEFAULT 0
);

CREATE TABLE file_blobs (
  file_hash TEXT NOT NULL,
  hash TEXT NOT NULL,
  PRIMARY KEY (file_hash, hash),
  FOREIGN KEY(file_hash) REFERENCES files(hash)
);
