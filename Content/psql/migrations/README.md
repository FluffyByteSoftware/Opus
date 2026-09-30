# Migrations

Every change to a table after it was first made goes here, one `.sql` file per change.  Archivist runs the ones
that haven't run yet every time it connects, right after the schemas in `../defaults/schemas/`.

- Name each file with a number from 1 up and an `_`: `0001_add_last_played_character.sql`.  They run lowest
  number first (by the number, so `10_` comes after `0002_`), and each one runs exactly once.  Postgres keeps
  the list of which ones have run in `archivist_migrations`.  Anything here that isn't a `.sql` file, like this
  README, is skipped.
- A migration runs in one transaction along with its line in that list.  If it fails, nothing in it is kept,
  it's an Error in the log and on the bell, and it's tried again next connect.  The ones after it wait.
- A file that doesn't start with a number and an `_`, a number 0, or two files with the same number, stops all
  of them until it's fixed.
- Never edit a migration that has already run somewhere.  Write a new one.
- Never edit a schema file once its table exists.  Change it with a migration instead.  A fresh database runs
  the schemas and then every migration here, and ends up the same shape as one that's been around the whole
  time.
- The query time limit from `postgres.cfg` doesn't apply to migrations.
