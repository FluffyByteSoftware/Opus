#!/usr/bin/env python3
# File:       Opus/Conductor/dev/networking/test_client.py
# Component:  Conductor
# Author:     Jacob Chacko
#
# A stand-in for Ensemble, for testing networking by hand until there is
# a real client.  It does what a client does and nothing more: TLS to the
# TCP port, a Login, then the Ticket to the UDP port, then keep-alives
# until Ctrl-C (which sends a Goodbye, wherever it lands after the login)
# or --leave-after runs out.  In
# between, at character select, it asks for the account's characters,
# makes, deletes or resets home the ones the flags name, and with --play
# brings one into the world and stays there.  PLAY fetches the simple
# overworld map first, the way Ensemble does (protocol version 11): the
# offer, the pieces 64 at a time, the SHA-256 checked, then PlayerReady.
# With --chunks it also pulls every chunk in the character's view before
# PlayerReady (protocol version 12), 64 at a time, nearest first, and
# squeezes them back out.  Once it's there, a line
# typed in the terminal and sent with Enter goes out the way the chat
# window would send it.  With --type it types lines
# there, the way a player types in the chat window (`/chat Yo yo yo!`,
# protocol version 8), and every chat the server sends is printed; a
# `/who` (version 9) is drawn the way Ensemble will draw it, at 79 wide,
# with the count in digits.  An answer in Spans is put back together.  Every
# packet in and out is printed, meaning first and raw bytes under it.  The
# bytes are the ones in Documentation/LLM/PROTOCOL.md; when this and the
# document disagree, the document wins.  To try "already logged in", leave
# one running and start a second in another terminal; it asks what to do.
#
# The password is typed as it is, and the script turns it into the
# password's key before the Login, the way Ensemble does (protocol version
# 7): PBKDF2 with HMAC-SHA256, 600,000 rounds, the name in the salt.  The
# key is printed in the Login's bytes like everything else; it's a test
# account's.  --no-key sends the password as typed instead, to see the
# server turn it away without a hash.
#
# From Conductor/dev, where the terminal sits.  The script finds the
# certificate from its own folder when --cert isn't given, and the account
# here is jacob_01 with the password 'Correct horse 1!' (the two last
# arguments every time):
#
#   python3 networking/test_client.py jacob_01 'Correct horse 1!'
#   python3 networking/test_client.py --host 127.0.0.1 --cert /opt/storage/Coding/Opus/Content/certs/conductor.crt ...
#   python3 networking/test_client.py --go-quiet ...   (stops the keep-alives, to see the 40 s drop)
#   python3 networking/test_client.py --pause-before-login 8 ...   (sits open, to KICK or ban it)
#   python3 networking/test_client.py --create Jacob ...   (makes a character, then lists again)
#   python3 networking/test_client.py --delete Jacob ...   (types DELETE; --delete-word to type another)
#   python3 networking/test_client.py --reset-home Jacob ...   (puts it back at 0, 0, 0)
#   python3 networking/test_client.py --play Jacob ...   (fetches the map, then brings Jacob into the world)
#   python3 networking/test_client.py --play Jacob --save-map /tmp/map ...   (keeps the map it fetched, for a cmp)
#   python3 networking/test_client.py --play Jacob --wrong-map-hash ...   (says the wrong hash: refused)
#   python3 networking/test_client.py --play Jacob --chunks ...   (pulls the chunks around Jacob before PlayerReady)
#   python3 networking/test_client.py --play Jacob --chunks --chunk-outside ...   (asks for one out of view too)
#   python3 networking/test_client.py --play Jacob --type '/chat Yo yo yo!' ...   (says it to everybody)
#   python3 networking/test_client.py --play Jacob --type '/who' --type '/who list' ...
#   python3 networking/test_client.py --play Jacob --type '/chat 1' --type '/chat 2' --type-gap 0 ...
#   python3 networking/test_client.py --no-key ...   (sends the password, not its key: refused)
#
# Standard library only.

import argparse
import hashlib
import os
import queue
import socket
import ssl
import struct
import sys
import threading
import time
from datetime import datetime, timedelta, timezone

PROTOCOL_VERSION = 12

# The password's key.  Changing any of these locks out every account; the
# server and Ensemble make it the same way.
KEY_SALT_PREFIX = "Opus login v1:"
KEY_ROUNDS = 600000
KEY_BYTES = 32

HELLO = 0x10
LOGIN = 0x11
IN_LINE = 0x12
LOGIN_RESULT = 0x13
SESSION_CHOICE = 0x14
TICKET = 0x15
CHARACTER_LIST_REQUEST = 0x20
CHARACTER_LIST_DELIVERY = 0x21
CREATE_CHARACTER = 0x22
CHARACTER_CREATE_RESULT = 0x23
DELETE_CHARACTER = 0x24
CHARACTER_DELETE_RESULT = 0x25
CHARACTER_REQUEST_RESET_HOME = 0x26
USER_PRESS_PLAY = 0x27
CHARACTER_ENTERED_WORLD = 0x28
PLAYER_READY = 0x29
CONNECT = 0x30
CONNECT_RESULT = 0x31
KEEP_ALIVE = 0x32
GOODBYE = 0x33
KICKED = 0x34
COMMAND_ACCEPTED = 0x35
COMMAND_REFUSED = 0x36
PLAYER_COMMAND = 0x37
CHAT_DELIVERY = 0x38
WHO_DELIVERY = 0x39
SPAN = 0x3A
PLEASE_WAIT = 0x3B
OVERWORLD_MAP_OFFER = 0x40
OVERWORLD_MAP_REQUEST = 0x41
OVERWORLD_MAP_PIECE = 0x42
CHUNK_REQUEST = 0x43
CHUNK_PIECE = 0x44
CHUNK_REFUSED = 0x45

NAMES = {HELLO: "Hello", LOGIN: "Login", IN_LINE: "InLine", LOGIN_RESULT: "LoginResult",
         SESSION_CHOICE: "SessionChoice", TICKET: "Ticket", CONNECT: "Connect", CONNECT_RESULT: "ConnectResult",
         KEEP_ALIVE: "KeepAlive", GOODBYE: "Goodbye", KICKED: "Kicked",
         CHARACTER_LIST_REQUEST: "CharacterListRequest", CHARACTER_LIST_DELIVERY: "CharacterListDelivery",
         CREATE_CHARACTER: "CreateCharacter", CHARACTER_CREATE_RESULT: "CharacterCreateResult",
         DELETE_CHARACTER: "DeleteCharacter", CHARACTER_DELETE_RESULT: "CharacterDeleteResult",
         CHARACTER_REQUEST_RESET_HOME: "CharacterRequestResetHome", COMMAND_ACCEPTED: "CommandAccepted",
         COMMAND_REFUSED: "CommandRefused", USER_PRESS_PLAY: "UserPressPlay",
         CHARACTER_ENTERED_WORLD: "CharacterEnteredWorld", PLAYER_COMMAND: "PlayerCommand",
         CHAT_DELIVERY: "ChatDelivery", WHO_DELIVERY: "WhoDelivery", SPAN: "Span", PLEASE_WAIT: "PleaseWait",
         PLAYER_READY: "PlayerReady", OVERWORLD_MAP_OFFER: "OverworldMapOffer",
         OVERWORLD_MAP_REQUEST: "OverworldMapRequest", OVERWORLD_MAP_PIECE: "OverworldMapPiece",
         CHUNK_REQUEST: "ChunkRequest", CHUNK_PIECE: "ChunkPiece", CHUNK_REFUSED: "ChunkRefused"}

LOGIN_ANSWERS = {1: "failed", 2: "already logged in", 3: "outdated client", 4: "unavailable"}
CREATE_ANSWERS = {0: "made", 1: "name not allowed", 2: "name taken", 3: "slots full", 4: "unavailable"}
DELETE_ANSWERS = {0: "approved", 1: "denied"}
CHUNK_REFUSALS = {1: "outside the view", 2: "not yet", 3: "unavailable"}
KICK_REASONS = {1: "logged in elsewhere", 2: "server stopping", 3: "banned", 4: "kicked by the admin",
                5: "ACCOUNT TERMINATED", 6: "character locked for a moment; log in again"}


def password_key(username, password):
    """The password's key, as Ensemble and the server make it: 64 lowercase
    hex.  The name's A to Z are made lowercase, nothing else."""
    lower = "".join(c.lower() if "A" <= c <= "Z" else c for c in username)
    salt = (KEY_SALT_PREFIX + lower).encode("utf-8")
    started = time.monotonic()
    key = hashlib.pbkdf2_hmac("sha256", password.encode("utf-8"), salt, KEY_ROUNDS, KEY_BYTES).hex()
    print("Made the password's key in %d ms." % int((time.monotonic() - started) * 1000))
    return key


def put_string(text):
    data = text.encode("utf-8")
    return struct.pack("<I", len(data)) + data


def take_string(payload, at):
    (length,) = struct.unpack_from("<I", payload, at)
    at += 4
    return payload[at:at + length].decode("utf-8"), at + length


def frame(kind, payload=b""):
    return struct.pack("<I", 1 + len(payload)) + bytes([kind]) + payload


def name_of(kind):
    return NAMES.get(kind, "0x%02X" % kind)


def say(direction, kind, detail="", payload=None):
    print("%s %-13s %s" % (direction, name_of(kind), detail), flush=True)
    if payload:
        # The raw bytes under the meaning, so a disagreement with
        # PROTOCOL.md shows up as bytes, not as a guess.
        print("   %s" % payload.hex(" "), flush=True)


def show_chat(data):
    """A ChatDelivery: a count, then that many finished lines."""
    count = data[1]
    at = 2
    lines = []
    for _ in range(count):
        line, at = take_string(data, at)
        lines.append(line)
    say("<-", CHAT_DELIVERY, "%d line(s)" % count, data[1:])
    for line in lines:
        print("   %s" % line, flush=True)


# How wide the /who box is drawn here.  Ensemble draws it to its chat
# box's width; 79 is Jacob's old MUD's.
WHO_WIDTH = 79

# How long to wait for every piece of an answer in Spans before giving up.
SPAN_WAIT = 2.0

# The map at PLAY: the most pieces one request asks for, how long to wait
# on them before asking again for what's missing, and how long without a
# new piece before giving up.  The same as Ensemble's.
MAP_PIECES_AT_ONCE = 64
MAP_ASK_AGAIN = 0.25
MAP_STALL = 10.0

# The chunks (protocol version 12): asked for 64 at a time, the same
# waits as the map.
CHUNKS_AT_ONCE = 64
CHUNK_SIDE = 32
CHUNK_BLOCKS = CHUNK_SIDE * CHUNK_SIDE * CHUNK_SIDE
CHUNK_ROWS = 11
BLOCK_NAMES = {0: "AIR", 1: "DIRT", 2: "STONE", 3: "WOOD", 4: "GOLD", 5: "BEDROCK"}


def unsqueeze(data):
    """A squeezed chunk (PROTOCOL.md, "The chunks around the player") back
    as its 32,768 block numbers, bottom layer first, the south row first
    in a layer, west to east in a row.  Raises ValueError if it's wrong."""
    if not data or data[0] != 1:
        raise ValueError("not squeezed as runs")
    (count,) = struct.unpack_from("<H", data, 1)
    if count < 1 or count > CHUNK_BLOCKS:
        raise ValueError("%d kinds" % count)
    at = 3
    kinds = list(struct.unpack_from("<%dH" % count, data, at))
    at += 2 * count
    wide = count > 256
    blocks = []
    while len(blocks) < CHUNK_BLOCKS:
        first = data[at]
        at += 1
        if first < 128:
            length = first + 1
        else:
            length = ((data[at] << 7) | (first & 0x7F)) + 1
            at += 1
        if wide:
            (place,) = struct.unpack_from("<H", data, at)
            at += 2
        else:
            place = data[at]
            at += 1
        if place >= count or len(blocks) + length > CHUNK_BLOCKS:
            raise ValueError("a bad run")
        blocks.extend([kinds[place]] * length)
    if at != len(data):
        raise ValueError("%d bytes left over" % (len(data) - at))
    return blocks


def chunk_place(data, at):
    """A chunk's x, z and row off the wire."""
    x, z, row = struct.unpack_from("<hhB", data, at)
    return (x, z, row)


def centred(text, fill):
    pad = WHO_WIDTH - len(text)
    left = pad // 2
    return fill * left + text + fill * (pad - left)


def show_who(data):
    """A WhoDelivery, drawn as the client will draw it: the banner, the
    time it ran in this computer's time zone, the names in columns (or a
    line each with where it stands, for /who list), and the count."""
    (ask, seconds) = struct.unpack_from("<II", data, 1)
    listed = data[9] == 1
    (count,) = struct.unpack_from("<H", data, 10)
    at = 12
    entries = []
    for _ in range(count):
        name, at = take_string(data, at)
        block = None
        if listed:
            block = struct.unpack_from("<iii", data, at)
            at += 12
        entries.append((name, block))
    say("<-", WHO_DELIVERY, "ask %d, %d character(s), %d seconds after midnight UTC%s"
        % (ask, count, seconds, ", listed" if listed else ""), data[1:])

    # The date is this computer's, the time the server's.
    midnight = datetime.now(timezone.utc).replace(hour=0, minute=0, second=0, microsecond=0)
    ran = (midnight + timedelta(seconds=seconds)).astimezone()
    stamp = ran.strftime("%a %b ") + "%2d" % ran.day + ran.strftime(" %H:%M:%S %Y")

    lines = [centred("======] Forgotten Legends [======", "-"), centred(stamp, " ").rstrip(),
             centred("] Players [", "-")]
    if listed:
        for name, (x, y, z) in entries:
            lines.append("[%s] is currently at [%d, %d, %d]" % (name, x, y, z))
    elif entries:
        width = max(len(name) for name, _ in entries) + 2
        across = max(1, (WHO_WIDTH + 2) // width)
        for start in range(0, len(entries), across):
            row = entries[start:start + across]
            lines.append("".join(name.ljust(width) for name, _ in row).rstrip())
    if count == 1:
        footer = "There is 1 legend currently online."
    else:
        footer = "There are %d legends currently online." % count
    lines.append(centred("> %s <" % footer, "-"))
    for line in lines:
        print("   " + line, flush=True)


class Tcp:
    """One packet at a time off a TLS stream."""

    def __init__(self, stream):
        self.stream = stream
        self.buffer = b""

    def read_packet(self):
        while True:
            if len(self.buffer) >= 4:
                (length,) = struct.unpack_from("<I", self.buffer, 0)
                if len(self.buffer) >= 4 + length:
                    kind = self.buffer[4]
                    payload = self.buffer[5:4 + length]
                    self.buffer = self.buffer[4 + length:]
                    return kind, payload
            chunk = self.stream.recv(8192)
            if not chunk:
                return None, None
            self.buffer += chunk

    def send(self, kind, payload=b""):
        self.stream.sendall(frame(kind, payload))
        say("->", kind, "", payload)


def log_in(args):
    """The TCP half.  Hands back (token, udp_port), or None."""
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    context.minimum_version = ssl.TLSVersion.TLSv1_3
    if args.cert:
        # Trust this one certificate and nothing else, the way Ensemble
        # will.  The name on it isn't checked; the file is the check.
        context.check_hostname = False
        context.verify_mode = ssl.CERT_REQUIRED
        context.load_verify_locations(args.cert)
    else:
        print("No --cert given, so the server's certificate isn't checked at all.  Fine on this machine.")
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE

    started = time.monotonic()
    raw = socket.create_connection((args.host, args.tcp_port), timeout=10)
    stream = context.wrap_socket(raw, server_hostname=args.host)
    print("TLS up in %d ms: %s" % ((time.monotonic() - started) * 1000, stream.version()))
    stream.settimeout(60)
    tcp = Tcp(stream)

    kind, payload = tcp.read_packet()
    if kind != HELLO:
        print("Expected a Hello, got %s." % name_of(kind))
        return None
    say("<-", kind, "protocol version %d" % payload[0], payload)
    if payload[0] != PROTOCOL_VERSION:
        print("This script speaks protocol version %d.  Stopping." % PROTOCOL_VERSION)
        return None

    if args.pause_before_login:
        # Holds the connection open after TLS with nothing sent, so the
        # Connections tab shows it waiting for its Login and the admin can
        # KICK or ban it.  Past the server's login deadline (10 s by
        # default) the server hangs up first.
        print("Pausing %d s before the Login." % args.pause_before_login, flush=True)
        time.sleep(args.pause_before_login)

    if args.no_key:
        key = args.password
        print("Sending the password as typed, not its key (--no-key).")
    else:
        key = password_key(args.username, args.password)
    tcp.send(LOGIN, put_string(args.version) + put_string(args.secret) + put_string(args.username)
             + put_string(key))
    sent_at = time.monotonic()

    while True:
        kind, payload = tcp.read_packet()
        if kind is None:
            print("The server hung up.")
            return None
        took = int((time.monotonic() - sent_at) * 1000)
        if kind == IN_LINE:
            ahead, wait = struct.unpack("<II", payload)
            say("<-", kind, "%d ahead, about %d ms" % (ahead, wait), payload)
        elif kind == LOGIN_RESULT:
            message, _ = take_string(payload, 1)
            say("<-", kind, "%s: %r  (%d ms)" % (LOGIN_ANSWERS.get(payload[0], payload[0]), message, took), payload)
            if payload[0] == 2:
                if args.leave_other_alone:
                    choice = 1
                else:
                    answer = input("Already logged in.  Log the other session out? (y/n) ").strip().lower()
                    choice = 0 if answer == "y" else 1
                print("Answering: %s." % ("leave the other session alone" if choice else "log the other session out"))
                tcp.send(SESSION_CHOICE, bytes([choice]))
                if choice == 1:
                    return None
                continue
            return None
        elif kind == TICKET:
            token, at = take_string(payload, 0)
            (udp_port,) = struct.unpack_from("<H", payload, at)
            say("<-", kind, "token %s... for UDP port %d  (%d ms)" % (token[:8], udp_port, took), payload)
            return token, udp_port
        else:
            say("<-", kind, "unexpected", payload)
            return None


class Kicked(Exception):
    """The server sent a Kicked in the middle of character select."""


class CharacterSelect:
    """The asks at character select, each with an ask number one higher
    than the last.  An ask not answered in half a second is sent again with
    the same number, so the server hands back the answer it kept instead of
    doing it twice."""

    def __init__(self, udp, server):
        self.udp = udp
        self.server = server
        self.last_ask = 0
        self.characters = []
        # --save-map, --wrong-map-hash, --chunks and --chunk-outside.
        self.save_map = None
        self.wrong_map_hash = False
        self.chunks = False
        self.chunk_outside = False

    def ask(self, kind, rest, detail):
        """Sends one ask and hands back (answer type, the answer after its
        ask number), or (None, None) if nothing came."""
        self.last_ask += 1
        ask = self.last_ask
        packet = bytes([kind]) + struct.pack("<I", ask) + rest
        # The pieces of an answer in Spans, by number, kept across resends:
        # the server sends every piece again, and the gaps fill in.
        pieces = {}
        first_piece = None
        for attempt in range(20):
            if first_piece is not None and time.monotonic() - first_piece > SPAN_WAIT:
                print("Gave up on ask %d: %d piece(s) in %g seconds." % (ask, len(pieces), SPAN_WAIT))
                return None, None
            self.udp.sendto(packet, self.server)
            say("->", kind, "ask %d, %s%s" % (ask, detail, "" if attempt == 0 else " (again)"), packet[1:])
            deadline = time.monotonic() + 0.5
            while time.monotonic() < deadline:
                try:
                    data, _ = self.udp.recvfrom(2048)
                except socket.timeout:
                    continue
                if not data:
                    continue
                if data[0] == KICKED:
                    (reason,) = struct.unpack("<I", data[1:5])
                    say("<-", KICKED, KICK_REASONS.get(reason, reason), data[1:])
                    raise Kicked()
                if data[0] == PLEASE_WAIT and len(data) >= 5:
                    (answered,) = struct.unpack_from("<I", data, 1)
                    words, _ = take_string(data, 5)
                    if answered == ask:
                        # Not the answer: the server is working on it and
                        # says so.  The answer follows under the same number.
                        say("<-", PLEASE_WAIT, "ask %d: %r" % (ask, words), data[1:])
                    else:
                        say("<-", PLEASE_WAIT, "for ask %d, an old one; ignored" % answered)
                    continue
                if data[0] == SPAN and len(data) >= 7:
                    (answered,) = struct.unpack_from("<I", data, 1)
                    if answered != ask:
                        say("<-", SPAN, "for ask %d, an old one; ignored" % answered)
                        continue
                    part, parts = data[5], data[6]
                    pieces[part] = data[7:]
                    if first_piece is None:
                        first_piece = time.monotonic()
                    say("<-", SPAN, "ask %d, piece %d of %d, %d bytes" % (ask, part, parts, len(data) - 7))
                    if all(number in pieces for number in range(1, parts + 1)):
                        data = b"".join(pieces[number] for number in range(1, parts + 1))
                    else:
                        continue
                if data[0] in (CHARACTER_LIST_DELIVERY, CHARACTER_CREATE_RESULT, CHARACTER_DELETE_RESULT,
                               COMMAND_ACCEPTED, COMMAND_REFUSED, CHARACTER_ENTERED_WORLD,
                               WHO_DELIVERY, OVERWORLD_MAP_OFFER) and len(data) >= 5:
                    (answered,) = struct.unpack_from("<I", data, 1)
                    if answered == ask:
                        return data[0], data
                    say("<-", data[0], "for ask %d, an old one; ignored" % answered, data[1:])
                elif data[0] == CHAT_DELIVERY:
                    show_chat(data)
                elif data[0] != KEEP_ALIVE:
                    say("<-", data[0], "", data[1:])
        print("No answer to ask %d in 10 seconds." % ask)
        return None, None

    def list(self):
        kind, data = self.ask(CHARACTER_LIST_REQUEST, b"", "the account's characters")
        if kind == COMMAND_REFUSED:
            message, _ = take_string(data, 5)
            say("<-", kind, repr(message), data[1:])
            return
        if kind != CHARACTER_LIST_DELIVERY:
            return
        count = data[5]
        at = 6
        self.characters = []
        for _ in range(count):
            uuid, at = take_string(data, at)
            name, at = take_string(data, at)
            slot, playable = data[at], data[at + 1]
            at += 2
            self.characters.append((uuid, name, slot, playable))
        say("<-", kind, "%d character(s)" % count, data[1:])
        for uuid, name, slot, playable in self.characters:
            print("   slot %d: %-20s %s  %s" % (slot, name, uuid, "playable" if playable else "UNPLAYABLE (greyed)"))
        if count < 3:
            print("   %d slot(s) free." % (3 - count))
        else:
            print("   All three slots are full: a client wouldn't offer CREATE.")

    def uuid_of(self, name):
        for uuid, listed, _, _ in self.characters:
            if listed.lower() == name.lower():
                return uuid
        print("No character called %s on this account." % name)
        return None

    def create(self, name):
        kind, data = self.ask(CREATE_CHARACTER, put_string(name), repr(name))
        if kind == CHARACTER_CREATE_RESULT:
            message, _ = take_string(data, 6)
            say("<-", kind, "%s: %r" % (CREATE_ANSWERS.get(data[5], data[5]), message), data[1:])

    def delete(self, name, word):
        uuid = self.uuid_of(name)
        if uuid is None:
            return
        kind, data = self.ask(DELETE_CHARACTER, put_string(uuid) + put_string(word), "%s, typed %r" % (name, word))
        if kind == CHARACTER_DELETE_RESULT:
            message, _ = take_string(data, 6)
            say("<-", kind, "%s: %r" % (DELETE_ANSWERS.get(data[5], data[5]), message), data[1:])

    def reset_home(self, name):
        uuid = self.uuid_of(name)
        if uuid is None:
            return
        kind, data = self.ask(CHARACTER_REQUEST_RESET_HOME, put_string(uuid), name)
        if kind == COMMAND_ACCEPTED:
            say("<-", kind, "", data[1:])
        elif kind == COMMAND_REFUSED:
            message, _ = take_string(data, 5)
            say("<-", kind, repr(message), data[1:])

    def play(self, name):
        """Fetches the map and brings the character into the world.  Its
        name if it's in, None if it was refused."""
        uuid = self.uuid_of(name)
        if uuid is None:
            return None
        kind, data = self.ask(USER_PRESS_PLAY, put_string(uuid), name)
        if kind == COMMAND_REFUSED:
            message, _ = take_string(data, 5)
            say("<-", kind, repr(message), data[1:])
            return None
        if kind != OVERWORLD_MAP_OFFER:
            return None
        size, piece_bytes, count = struct.unpack_from("<IHI", data, 5)
        expected, at = take_string(data, 15)
        x, y, z = struct.unpack_from("<fff", data, at)
        view = data[at + 12]
        say("<-", kind, "%d bytes in %d pieces of %d, SHA-256 %s; %s will stand at %g, %g, %g and see %d chunks "
            "each way" % (size, count, piece_bytes, expected, name, x, y, z, view), data[1:])

        got = self.fetch_map(size, count)
        if got is None:
            print("Couldn't get the world's map.  Ensemble sends the player back to the launcher here.")
            return None
        hash_got = hashlib.sha256(got).hexdigest()
        print("   SHA-256 of what came: %s, %s." % (hash_got, "the same" if hash_got == expected else "DIFFERENT"))
        if self.save_map:
            with open(self.save_map, "wb") as out:
                out.write(got)
            print("   Saved to %s." % self.save_map)
        if self.wrong_map_hash:
            hash_got = "0" * 64
            print("   --wrong-map-hash: saying %s instead." % hash_got)

        if self.chunks:
            self.fetch_chunks((x, y, z), view)

        kind, data = self.ask(PLAYER_READY, put_string(hash_got), "the map is in")
        if kind == CHARACTER_ENTERED_WORLD:
            entered_uuid, at = take_string(data, 5)
            entered_name, at = take_string(data, at)
            x, y, z = struct.unpack_from("<fff", data, at)
            say("<-", kind, "%s (%s) is in the world at %g, %g, %g" % (entered_name, entered_uuid, x, y, z),
                data[1:])
            return entered_name
        if kind == COMMAND_REFUSED:
            message, _ = take_string(data, 5)
            say("<-", kind, repr(message), data[1:])
        return None

    def fetch_map(self, size, count):
        """The map's pieces, asked for 64 at a time from the lowest one
        missing, and asked again after a quarter second for what didn't
        come.  The whole map's bytes, or None if no new piece came in
        MAP_STALL seconds.  Only the first request and piece are printed
        with their bytes; after that, a line every tenth of the way."""
        pieces = {}
        lowest_missing = 0
        started = time.monotonic()
        last_new = started
        next_tenth = 1
        requests = 0
        self.udp.settimeout(0.05)
        try:
            while lowest_missing < count:
                if time.monotonic() - last_new > MAP_STALL:
                    print("No new piece in %g seconds: %d of %d in." % (MAP_STALL, len(pieces), count))
                    return None
                ask_for = min(MAP_PIECES_AT_ONCE, count - lowest_missing)
                request = bytes([OVERWORLD_MAP_REQUEST]) + struct.pack("<IB", lowest_missing, ask_for)
                self.udp.sendto(request, self.server)
                requests += 1
                if requests == 1:
                    say("->", OVERWORLD_MAP_REQUEST, "pieces %d to %d" % (lowest_missing, lowest_missing + ask_for - 1),
                        request[1:])
                wanted = range(lowest_missing, lowest_missing + ask_for)
                deadline = time.monotonic() + MAP_ASK_AGAIN
                while time.monotonic() < deadline and not all(number in pieces for number in wanted):
                    try:
                        data, _ = self.udp.recvfrom(2048)
                    except socket.timeout:
                        continue
                    if not data:
                        continue
                    if data[0] == OVERWORLD_MAP_PIECE and len(data) >= 5:
                        (number,) = struct.unpack_from("<I", data, 1)
                        if number < count and number not in pieces:
                            if not pieces:
                                say("<-", OVERWORLD_MAP_PIECE, "piece %d, %d bytes" % (number, len(data) - 5),
                                    data[1:17])
                            pieces[number] = data[5:]
                            last_new = time.monotonic()
                    elif data[0] == KICKED:
                        (reason,) = struct.unpack("<I", data[1:5])
                        say("<-", KICKED, KICK_REASONS.get(reason, reason), data[1:])
                        raise Kicked()
                    elif data[0] == CHAT_DELIVERY:
                        show_chat(data)
                    elif data[0] != KEEP_ALIVE:
                        say("<-", data[0], "", data[1:])
                while lowest_missing < count and lowest_missing in pieces:
                    lowest_missing += 1
                while next_tenth <= 10 and len(pieces) * 10 >= count * next_tenth:
                    print("   %d%%: %d of %d pieces." % (next_tenth * 10, len(pieces), count), flush=True)
                    next_tenth += 1
        finally:
            self.udp.settimeout(0.5)
        took = time.monotonic() - started
        got = b"".join(pieces[number] for number in range(count))
        print("   The map: %d bytes in %.2f s (%.1f MB/s), %d requests.  The offer said %d bytes: %s."
              % (len(got), took, len(got) / max(took, 0.001) / 1e6, requests, size,
                 "the same" if len(got) == size else "DIFFERENT"))
        return got

    def fetch_chunks(self, position, view):
        """Every chunk within `view` chunks of the character's column, every
        row, nearest first, the way Ensemble will: 64 at a time from the
        first not yet in, and asked again after a quarter second for what
        didn't come.  A "not yet" is asked again; "outside the view" and
        "unavailable" are the end of that chunk.  Gives up with no new
        chunk in MAP_STALL seconds.  Prints what came, how big and how
        long, and checks the GOLD at 0,0,0 if that chunk came."""
        column = (int(position[0] // CHUNK_SIDE), int(position[2] // CHUNK_SIDE))
        stand_row = int((position[1] + 32) // CHUNK_SIDE)
        wanted = [(column[0] + dx, column[1] + dz, row)
                  for dz in range(-view, view + 1) for dx in range(-view, view + 1) for row in range(CHUNK_ROWS)]
        wanted.sort(key=lambda place: (max(abs(place[0] - column[0]), abs(place[1] - column[1])),
                                       abs(place[2] - stand_row)))
        if self.chunk_outside:
            # One past the view's east edge, at the end, to see it refused.
            wanted.append((column[0] + view + 1, column[1], 1))
        print("   The chunks: %d around column %d, %d (%d each way, every row)."
              % (len(wanted), column[0], column[1], view), flush=True)

        squeezed = {}
        refused = {}
        pieces = {}
        not_yet = 0
        packets = 0
        requests = 0
        started = time.monotonic()
        last_new = started
        next_tenth = 1
        self.udp.settimeout(0.05)
        try:
            while True:
                left = [place for place in wanted if place not in squeezed and place not in refused]
                if not left:
                    break
                if time.monotonic() - last_new > MAP_STALL:
                    print("No new chunk in %g seconds: %d of %d in." % (MAP_STALL, len(squeezed), len(wanted)))
                    return
                asked = left[:CHUNKS_AT_ONCE]
                request = bytes([CHUNK_REQUEST, len(asked)]) + b"".join(struct.pack("<hhB", *place)
                                                                         for place in asked)
                self.udp.sendto(request, self.server)
                requests += 1
                if requests == 1:
                    say("->", CHUNK_REQUEST, "%d chunks, the first %d,%d row %d" % ((len(asked),) + asked[0]),
                        request[1:17])
                deadline = time.monotonic() + MAP_ASK_AGAIN
                while time.monotonic() < deadline and any(p not in squeezed and p not in refused for p in asked):
                    try:
                        data, _ = self.udp.recvfrom(2048)
                    except socket.timeout:
                        continue
                    if not data:
                        continue
                    if data[0] == CHUNK_PIECE and len(data) >= 8:
                        place = chunk_place(data, 1)
                        part, parts = data[6], data[7]
                        packets += 1
                        if place in squeezed:
                            continue
                        if not squeezed and not pieces:
                            say("<-", CHUNK_PIECE, "%d,%d row %d, piece %d of %d, %d bytes"
                                % (place + (part, parts, len(data) - 8)), data[1:24])
                        got = pieces.setdefault(place, {})
                        got[part] = data[8:]
                        if all(number in got for number in range(1, parts + 1)):
                            squeezed[place] = b"".join(got[number] for number in range(1, parts + 1))
                            del pieces[place]
                            last_new = time.monotonic()
                    elif data[0] == CHUNK_REFUSED and len(data) == 7:
                        place = chunk_place(data, 1)
                        why = data[6]
                        if why == 2:
                            not_yet += 1
                        elif place not in refused:
                            refused[place] = why
                            last_new = time.monotonic()
                            say("<-", CHUNK_REFUSED, "%d,%d row %d: %s" % (place + (CHUNK_REFUSALS.get(why, why),)),
                                data[1:])
                    elif data[0] == KICKED:
                        (reason,) = struct.unpack("<I", data[1:5])
                        say("<-", KICKED, KICK_REASONS.get(reason, reason), data[1:])
                        raise Kicked()
                    elif data[0] == CHAT_DELIVERY:
                        show_chat(data)
                    elif data[0] != KEEP_ALIVE:
                        say("<-", data[0], "", data[1:])
                while next_tenth <= 10 and (len(squeezed) + len(refused)) * 10 >= len(wanted) * next_tenth:
                    print("   %d%%: %d of %d chunks." % (next_tenth * 10, len(squeezed) + len(refused), len(wanted)),
                          flush=True)
                    next_tenth += 1
        finally:
            self.udp.settimeout(0.5)
        took = time.monotonic() - started

        total = sum(len(data) for data in squeezed.values())
        kinds = {}
        bad = 0
        gold = None
        for place, data in squeezed.items():
            try:
                blocks = unsqueeze(data)
            except (ValueError, IndexError, struct.error) as e:
                print("   %d,%d row %d doesn't unsqueeze: %s" % (place + (e,)))
                bad += 1
                continue
            for block in blocks:
                kinds[block] = kinds.get(block, 0) + 1
            if place == (0, 0, 1):
                gold = BLOCK_NAMES.get(blocks[0], blocks[0])
        biggest = max(squeezed.items(), key=lambda item: len(item[1]), default=None)
        print("   The chunks: %d in, %d refused, in %.2f s; %d bytes squeezed (%.1f KB, %.0f MB as they are), "
              "%d packets, %d requests, %d \"not yet\"s."
              % (len(squeezed), len(refused), took, total, total / 1024, len(squeezed) * CHUNK_BLOCKS * 2 / 1048576,
                 packets, requests, not_yet))
        if biggest:
            print("   The biggest: %d,%d row %d, %d bytes." % (biggest[0] + (len(biggest[1]),)))
        print("   Blocks: %s.  %d didn't unsqueeze." % (", ".join("%s %d" % (BLOCK_NAMES.get(kind, kind), count)
                                                             for kind, count in sorted(kinds.items())), bad))
        if gold is not None:
            print("   The block at 0,0,0: %s." % gold)
        for why in sorted(set(refused.values())):
            print("   Refused, %s: %d." % (CHUNK_REFUSALS.get(why, why),
                                          sum(1 for reason in refused.values() if reason == why)))

    def type_line(self, line):
        """A line typed in the chat window, sent as it was typed."""
        kind, data = self.ask(PLAYER_COMMAND, put_string(line), repr(line))
        if kind == WHO_DELIVERY:
            show_who(data)
        elif kind == COMMAND_ACCEPTED:
            say("<-", kind, "", data[1:])
        elif kind == COMMAND_REFUSED:
            message, _ = take_string(data, 5)
            say("<-", kind, repr(message), data[1:])


def character_select(args, udp, server):
    """The list, then whatever the flags ask for, each followed by the list
    again, then --play, then the --type lines.  Hands back (still connected, the name of the
    character in the world or None, and the asks, for lines typed after)."""
    select = CharacterSelect(udp, server)
    select.save_map = args.save_map
    select.wrong_map_hash = args.wrong_map_hash
    select.chunks = args.chunks
    select.chunk_outside = args.chunk_outside
    playing = None
    try:
        select.list()
        if args.create:
            select.create(args.create)
            select.list()
        if args.delete:
            select.delete(args.delete, args.delete_word)
            select.list()
        if args.reset_home:
            select.reset_home(args.reset_home)
            select.list()
        if args.play:
            playing = select.play(args.play)
            if playing:
                # Character select is behind us now; the server says so.
                select.list()
        # Typed in the world, or at character select to see it refused,
        # --type-gap apart, so the server's anti-flood lets each through.
        for number, line in enumerate(args.type or []):
            if number > 0:
                time.sleep(args.type_gap)
            select.type_line(line)
    except Kicked:
        print("Back to the login screen.")
        return False, None, select
    return True, playing, select


def read_typed(typed):
    """Every line typed in the terminal, into `typed`, on a thread of its
    own so the keep-alives go on while it waits.  Started only once the
    login's "Already logged in" question is behind us, so it never takes
    that answer.  A daemon thread, so it never holds the script open."""
    for line in sys.stdin:
        typed.put(line.rstrip("\r\n"))


def play(args, token, udp_port):
    """The UDP half, with Ctrl-C caught wherever it lands: at character
    select, in the middle of an ask, or in the keep-alives, a Goodbye goes
    out, so the server lets the account go at once instead of waiting out
    its UDP timeout."""
    udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    udp.settimeout(0.5)
    server = (args.host, udp_port)
    still_in = True
    try:
        still_in = session(args, token, udp, server)
    except KeyboardInterrupt:
        print()
        print("Ctrl-C.")
    if still_in:
        udp.sendto(bytes([GOODBYE]), server)
        say("->", GOODBYE)


def session(args, token, udp, server):
    """Connect, character select, then keep-alives until it's time to go.
    True if the session is still open at the end (so a Goodbye is owed),
    False if it ended here: refused, kicked, or never answered."""
    for attempt in range(20):
        udp.sendto(bytes([CONNECT]) + put_string(token), server)
        say("->", CONNECT, "try %d" % (attempt + 1))
        try:
            data, _ = udp.recvfrom(2048)
        except socket.timeout:
            continue
        if data and data[0] == CONNECT_RESULT:
            message, _ = take_string(data, 2)
            say("<-", CONNECT_RESULT, "%s: %r" % ("accepted" if data[1] == 0 else "refused", message), data[1:])
            if data[1] != 0:
                return False
            break
    else:
        print("No answer to the Connect in 10 seconds.")
        return False

    connected, playing, select = character_select(args, udp, server)
    if not connected:
        return False

    if args.go_quiet:
        print("Going quiet.  The server should drop this session after its UDP timeout; watch its log.  "
              "Ctrl-C says Goodbye.")
        while True:
            try:
                data, _ = udp.recvfrom(2048)
                say("<-", data[0], "while quiet")
            except socket.timeout:
                pass

    if playing:
        print("In the world as %s." % playing)
    else:
        print("Still at character select.")
    print("Type a line and press Enter to send it, as the chat window would (/chat Yo yo yo!, /who, /who list).")
    print("Keep-alives go once a second, printed only when one isn't answered (--show-keepalives prints them "
          "all).  Ctrl-C says Goodbye.", flush=True)

    # Read often, so a typed line goes out at once rather than up to half a
    # second later.
    udp.settimeout(0.1)
    typed = queue.Queue()
    threading.Thread(target=read_typed, args=(typed,), daemon=True).start()

    started = time.monotonic()
    while True:
        udp.sendto(bytes([KEEP_ALIVE]), server)
        deadline = time.monotonic() + 1.0
        answered = False
        while time.monotonic() < deadline:
            while not typed.empty():
                line = typed.get()
                if line.strip():
                    try:
                        select.type_line(line)
                    except Kicked:
                        print("Back to the login screen.")
                        return False
            try:
                data, _ = udp.recvfrom(2048)
            except socket.timeout:
                continue
            if not data:
                continue
            if data[0] == KEEP_ALIVE:
                answered = True
            elif data[0] == KICKED:
                (reason,) = struct.unpack("<I", data[1:5])
                say("<-", KICKED, KICK_REASONS.get(reason, reason), data[1:])
                print("Back to the login screen.")
                return False
            elif data[0] == CHAT_DELIVERY:
                show_chat(data)
            else:
                say("<-", data[0])
        if args.show_keepalives or not answered:
            print("-> KeepAlive %s" % ("answered" if answered else "NOT answered"), flush=True)
        if args.leave_after and time.monotonic() - started >= args.leave_after:
            return True


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    default_cert = os.path.normpath(os.path.join(here, "..", "..", "..", "Content", "certs", "conductor.crt"))

    parser = argparse.ArgumentParser(description="A stand-in client for Conductor's networking.")
    parser.add_argument("username")
    parser.add_argument("password")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--tcp-port", type=int, default=9997)
    parser.add_argument("--cert", default=default_cert if os.path.exists(default_cert) else None,
                        help="the server's certificate to trust (default: Content/certs/conductor.crt if it's there)")
    parser.add_argument("--version", default="0.0.1", help="the client version to claim")
    parser.add_argument("--secret", default="potato", help="the secret word")
    parser.add_argument("--no-key", action="store_true",
                        help="send the password as typed instead of its key, to see it refused")
    parser.add_argument("--leave-other-alone", action="store_true",
                        help="when the account is already logged in, hang up without asking")
    parser.add_argument("--leave-after", type=float, default=0,
                        help="say Goodbye after this many seconds in the world (default: wait for Ctrl-C)")
    parser.add_argument("--go-quiet", action="store_true",
                        help="connect over UDP and then send nothing, to see the timeout drop the session")
    parser.add_argument("--pause-before-login", type=int, default=0,
                        help="wait this many seconds after TLS before sending the Login, to catch it open")
    parser.add_argument("--create", metavar="NAME", help="make a character with this name at character select")
    parser.add_argument("--delete", metavar="NAME", help="delete the account's character with this name")
    parser.add_argument("--delete-word", default="DELETE",
                        help="the word typed to confirm a --delete (default: DELETE; anything else is denied)")
    parser.add_argument("--reset-home", metavar="NAME", help="put the account's character with this name at 0, 0, 0")
    parser.add_argument("--play", metavar="NAME",
                        help="bring the account's character with this name into the world, after the other flags")
    parser.add_argument("--save-map", metavar="PATH",
                        help="with --play, write the map fetched at PLAY to this file, to compare with the server's")
    parser.add_argument("--wrong-map-hash", action="store_true",
                        help="with --play, say the wrong hash in PlayerReady, to see the server refuse it")
    parser.add_argument("--chunks", action="store_true",
                        help="with --play, pull every chunk in the character's view before PlayerReady")
    parser.add_argument("--chunk-outside", action="store_true",
                        help="with --chunks, ask for one chunk just past the view too, to see it refused")
    parser.add_argument("--type", metavar="LINE", action="append",
                        help="type this line in the chat window once at character select is done, after --play "
                             "('/chat Yo yo yo!'); give it more than once for more lines")
    parser.add_argument("--type-gap", type=float, default=1.1,
                        help="seconds between --type lines (default 1.1, past /who's wait of 1 second); 0 floods, "
                             "to see the server refuse the lines that come too soon")
    parser.add_argument("--show-keepalives", action="store_true",
                        help="print every keep-alive, not just the ones that go unanswered")
    args = parser.parse_args()

    try:
        ticket = log_in(args)
    except OSError as e:
        # A hang-up mid-read or mid-write (a KICK, a ban, the login
        # deadline) lands here rather than as a traceback.
        print("The connection broke: %s" % e)
        sys.exit(1)
    except KeyboardInterrupt:
        # Before there's a ticket there's nothing to say Goodbye to: the
        # TLS connection just closes.
        print()
        print("Ctrl-C before the login finished.")
        sys.exit(130)
    if ticket is None:
        sys.exit(1)
    play(args, *ticket)


if __name__ == "__main__":
    main()
