#!/usr/bin/env python3
# File:       Opus/Conductor/dev/networking/test_client.py
# Component:  Conductor
# Author:     Jacob Chacko
#
# A stand-in for Ensemble, for testing networking by hand until there is
# a real client.  It does what a client does and nothing more: TLS to the
# TCP port, a Login, then the Ticket to the UDP port, then keep-alives
# until Ctrl-C (which sends a Goodbye) or --leave-after runs out.  In
# between, at character select, it asks for the account's characters, and
# makes, deletes or resets home the ones the flags name.  Every
# packet in and out is printed, meaning first and raw bytes under it.  The
# bytes are the ones in Documentation/LLM/PROTOCOL.md; when this and the
# document disagree, the document wins.  To try "already logged in", leave
# one running and start a second in another terminal; it asks what to do.
#
#   python3 test_client.py jacob_01 'Correct horse 1!'
#   python3 test_client.py --host 127.0.0.1 --cert ../../../Content/certs/conductor.crt jacob_01 'Correct horse 1!'
#   python3 test_client.py --go-quiet jacob_01 'Correct horse 1!'   (stops the keep-alives, to see the 40 s drop)
#   python3 test_client.py --pause-before-login 8 jacob_01 'Correct horse 1!'   (sits open, to KICK or ban it)
#   python3 test_client.py --create Jacob jacob_01 'Correct horse 1!'   (makes a character, then lists again)
#   python3 test_client.py --delete Jacob jacob_01 'Correct horse 1!'   (types DELETE; --delete-word to type another)
#   python3 test_client.py --reset-home Jacob jacob_01 'Correct horse 1!'   (puts it back at 0, 0, 0)
#
# Standard library only.

import argparse
import os
import socket
import ssl
import struct
import sys
import time

PROTOCOL_VERSION = 5

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
CONNECT = 0x30
CONNECT_RESULT = 0x31
KEEP_ALIVE = 0x32
GOODBYE = 0x33
KICKED = 0x34
COMMAND_ACCEPTED = 0x35
COMMAND_REFUSED = 0x36

NAMES = {HELLO: "Hello", LOGIN: "Login", IN_LINE: "InLine", LOGIN_RESULT: "LoginResult",
         SESSION_CHOICE: "SessionChoice", TICKET: "Ticket", CONNECT: "Connect", CONNECT_RESULT: "ConnectResult",
         KEEP_ALIVE: "KeepAlive", GOODBYE: "Goodbye", KICKED: "Kicked",
         CHARACTER_LIST_REQUEST: "CharacterListRequest", CHARACTER_LIST_DELIVERY: "CharacterListDelivery",
         CREATE_CHARACTER: "CreateCharacter", CHARACTER_CREATE_RESULT: "CharacterCreateResult",
         DELETE_CHARACTER: "DeleteCharacter", CHARACTER_DELETE_RESULT: "CharacterDeleteResult",
         CHARACTER_REQUEST_RESET_HOME: "CharacterRequestResetHome", COMMAND_ACCEPTED: "CommandAccepted",
         COMMAND_REFUSED: "CommandRefused"}

LOGIN_ANSWERS = {1: "failed", 2: "already logged in", 3: "outdated client", 4: "unavailable"}
CREATE_ANSWERS = {0: "made", 1: "name not allowed", 2: "name taken", 3: "slots full", 4: "unavailable"}
DELETE_ANSWERS = {0: "approved", 1: "denied"}
KICK_REASONS = {1: "logged in elsewhere", 2: "server stopping", 3: "banned", 4: "kicked by the admin",
                5: "ACCOUNT TERMINATED"}


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

    tcp.send(LOGIN, put_string(args.version) + put_string(args.secret) + put_string(args.username)
             + put_string(args.password))
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

    def ask(self, kind, rest, detail):
        """Sends one ask and hands back (answer type, the answer after its
        ask number), or (None, None) if nothing came."""
        self.last_ask += 1
        ask = self.last_ask
        packet = bytes([kind]) + struct.pack("<I", ask) + rest
        for attempt in range(20):
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
                if data[0] in (CHARACTER_LIST_DELIVERY, CHARACTER_CREATE_RESULT, CHARACTER_DELETE_RESULT,
                               COMMAND_ACCEPTED, COMMAND_REFUSED) and len(data) >= 5:
                    (answered,) = struct.unpack_from("<I", data, 1)
                    if answered == ask:
                        return data[0], data
                    say("<-", data[0], "for ask %d, an old one; ignored" % answered, data[1:])
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


def character_select(args, udp, server):
    """The list, then whatever the flags ask for, each followed by the list
    again.  False if the server kicked us on the way."""
    select = CharacterSelect(udp, server)
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
    except Kicked:
        print("Back to the login screen.")
        return False
    return True


def play(args, token, udp_port):
    """The UDP half: Connect, then keep-alives until it's time to go."""
    udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    udp.settimeout(0.5)
    server = (args.host, udp_port)

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
                return
            break
    else:
        print("No answer to the Connect in 10 seconds.")
        return

    if not character_select(args, udp, server):
        return

    if args.go_quiet:
        print("Going quiet.  The server should drop this session after its UDP timeout; watch its log.")
        try:
            while True:
                try:
                    data, _ = udp.recvfrom(2048)
                    say("<-", data[0], "while quiet")
                except socket.timeout:
                    pass
        except KeyboardInterrupt:
            return

    print("Still at character select: there's no world to step into yet.  Keep-alives once a second; Ctrl-C to "
          "say Goodbye.")
    started = time.monotonic()
    try:
        while True:
            udp.sendto(bytes([KEEP_ALIVE]), server)
            deadline = time.monotonic() + 1.0
            answered = False
            while time.monotonic() < deadline:
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
                    return
                else:
                    say("<-", data[0])
            print("-> KeepAlive %s" % ("answered" if answered else "NOT answered"), flush=True)
            if args.leave_after and time.monotonic() - started >= args.leave_after:
                break
    except KeyboardInterrupt:
        print()
    udp.sendto(bytes([GOODBYE]), server)
    say("->", GOODBYE)


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
    args = parser.parse_args()

    try:
        ticket = log_in(args)
    except OSError as e:
        # A hang-up mid-read or mid-write (a KICK, a ban, the login
        # deadline) lands here rather than as a traceback.
        print("The connection broke: %s" % e)
        sys.exit(1)
    if ticket is None:
        sys.exit(1)
    play(args, *ticket)


if __name__ == "__main__":
    main()
