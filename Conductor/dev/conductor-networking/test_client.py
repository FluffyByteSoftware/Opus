#!/usr/bin/env python3
# File:       Opus/Conductor/dev/conductor-networking/test_client.py
# Component:  Conductor
# Author:     Jacob Chacko
#
# A stand-in for Ensemble, for testing networking by hand until there is
# a real client.  It does what a client does and nothing more: TLS to the
# TCP port, a Login, then the Ticket to the UDP port, then keep-alives
# until Ctrl-C (which sends a Goodbye) or --leave-after runs out.  Every
# packet in and out is printed, meaning first and raw bytes under it.  The
# bytes are the ones in Documentation/LLM/PROTOCOL.md; when this and the
# document disagree, the document wins.  To try "already logged in", leave
# one running and start a second in another terminal; it asks what to do.
#
#   python3 test_client.py jacob_01 'Correct horse 1!'
#   python3 test_client.py --host 127.0.0.1 --cert ../../../Content/certs/conductor.crt jacob_01 'Correct horse 1!'
#   python3 test_client.py --go-quiet jacob_01 'Correct horse 1!'   (stops the keep-alives, to see the 40 s drop)
#
# Standard library only.

import argparse
import os
import socket
import ssl
import struct
import sys
import time

PROTOCOL_VERSION = 2

HELLO = 0x10
LOGIN = 0x11
IN_LINE = 0x12
LOGIN_RESULT = 0x13
SESSION_CHOICE = 0x14
TICKET = 0x15
CONNECT = 0x30
CONNECT_RESULT = 0x31
KEEP_ALIVE = 0x32
GOODBYE = 0x33
KICKED = 0x34

NAMES = {HELLO: "Hello", LOGIN: "Login", IN_LINE: "InLine", LOGIN_RESULT: "LoginResult",
         SESSION_CHOICE: "SessionChoice", TICKET: "Ticket", CONNECT: "Connect", CONNECT_RESULT: "ConnectResult",
         KEEP_ALIVE: "KeepAlive", GOODBYE: "Goodbye", KICKED: "Kicked"}

LOGIN_ANSWERS = {1: "failed", 2: "already logged in", 3: "outdated client", 4: "unavailable"}
KICK_REASONS = {1: "logged in elsewhere", 2: "server stopping", 3: "banned"}


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

    print("In the world.  Keep-alives once a second; Ctrl-C to say Goodbye.")
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
    args = parser.parse_args()

    ticket = log_in(args)
    if ticket is None:
        sys.exit(1)
    play(args, *ticket)


if __name__ == "__main__":
    main()
