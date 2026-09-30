-- One row per player's character.  Which account it belongs to, its name,
-- where it last stood, and the rest of it as Lua text.
--
-- The save column is the whole GameObject as primlib writes it: its
-- templates and every component's saved fields.  The name and the position
-- are columns of their own too, so character select can list names and the
-- spawn can read where to put a character without running any Lua.
--
-- conductor-accounts is the only thing that writes this table, and it
-- keeps the name and position columns in step with the save.

CREATE TABLE IF NOT EXISTS player_characters (
    -- Handed out by Postgres.  The account's slots point at a character by
    -- id.
    id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,

    -- The game's name for the row, unique across the whole server.
    uuid              UUID NOT NULL UNIQUE DEFAULT uuidv7(),

    -- The account it belongs to.  When the account goes, its characters go
    -- with it.
    account_id        BIGINT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,

    -- The character's name: 4 to 20 letters, a to z only, and only the
    -- first can be a capital.  Jacob's rule.
    character_name    TEXT NOT NULL
                      CHECK (character_name ~ '^[A-Za-z][a-z]{3,19}$'),

    -- Where it last stood, in the world's own terms (REAL is the same size
    -- as the f32 in primlib's Transform).  0, 0, 0 until it's first saved.
    position_x        REAL NOT NULL DEFAULT 0,
    position_y        REAL NOT NULL DEFAULT 0,
    position_z        REAL NOT NULL DEFAULT 0,

    -- The whole character as Lua text: return { templates = ..., components = ... }.
    save_lua          TEXT NOT NULL,

    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    saved_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Every name is unique across the whole server, whatever the capital:
-- Jacob and jacob are the same name.
CREATE UNIQUE INDEX IF NOT EXISTS player_characters_name_key ON player_characters (lower(character_name));

-- Character select asks for one account's characters.
CREATE INDEX IF NOT EXISTS player_characters_account_id ON player_characters (account_id);
