-- Every row gets a uuid: the game's name for it, unique across the whole
-- server.  id stays as the table's own number.
--
-- Conductor hands over a UUID from Fingerprinter.  The default only catches
-- a row that comes in without one (typed into psql, say, or a bug), and
-- gives the rows already here one each.  Those carry the time this runs,
-- not the time the row was made.
--
-- uuidv7() is built into Postgres 18: the time first, so they sort in the
-- order they were made, the same as Fingerprinter's.

ALTER TABLE accounts
    ADD COLUMN uuid UUID NOT NULL UNIQUE DEFAULT uuidv7();

ALTER TABLE archivist_migrations
    ADD COLUMN uuid UUID NOT NULL UNIQUE DEFAULT uuidv7();
