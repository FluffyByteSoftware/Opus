-- One row per account.  The person who owns it, how they log in, and when.
--
-- Postgres checks the account name and the email itself, so even a bug in
-- Conductor can't store a bad one.  Conductor checks them first too, so a
-- player gets a clear message instead of a database error.

CREATE TABLE IF NOT EXISTS accounts (
    -- Handed out by Postgres.  Other tables point at an account by id,
    -- never by name.
    id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,

    -- 8 to 32 characters: lowercase letters, numbers and _ only.
    account_username    TEXT NOT NULL UNIQUE
                        CHECK (account_username ~ '^[a-z0-9_]{8,32}$'),

    -- Kept however the owner capitalizes their own name.
    owner_first_name    TEXT NOT NULL CHECK (owner_first_name <> ''),
    owner_last_name     TEXT NOT NULL CHECK (owner_last_name <> ''),

    -- Loose on purpose: something, an @, then something with a dot in it.
    -- A strict email pattern turns away real addresses.
    owner_email         TEXT NOT NULL
                        CHECK (owner_email ~ '^[^@\s]+@[^@\s]+\.[^@\s]+$'),

    -- The Argon2 string: the hash with its salt and settings built in.
    password_hash       TEXT NOT NULL,

    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Empty until the first login.
    last_login_datetime TIMESTAMPTZ
);

-- One account per email address, whatever the capitals.  Joe@Mail.com and
-- joe@mail.com are the same person.
CREATE UNIQUE INDEX IF NOT EXISTS accounts_owner_email_key ON accounts (lower(owner_email));
