-- Three character slots on every account, each the id of one of its rows in
-- player_characters, or empty.
--
-- When a character goes, its slot empties on its own.  Postgres can't check
-- that a slot points at one of this account's own characters (that would
-- take a look at another table), so conductor-accounts makes sure of it,
-- setting the slot in the same transaction that makes the character.

ALTER TABLE accounts
    ADD COLUMN character_slot_1 BIGINT REFERENCES player_characters (id) ON DELETE SET NULL,
    ADD COLUMN character_slot_2 BIGINT REFERENCES player_characters (id) ON DELETE SET NULL,
    ADD COLUMN character_slot_3 BIGINT REFERENCES player_characters (id) ON DELETE SET NULL;
