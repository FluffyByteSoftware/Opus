<!--
File:       Opus/Documentation/HowTo/RELEASE.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Releasing a version

How a version of Opus goes from `testing` to a package somebody can download.  Written for 0.0.1
(2026-10-02), the first one, so this is the plain way: a GitHub Release on the repo with two zips on it, one
for Conductor and one for Ensemble, made by hand.  Soundcheck (the patcher) will take over the client's half
one day; until then a release is a zip.

The repo is private, so a Release on it is private too: only people with access to the repo see it.  That's
where it stays.  The purchased art is in the Ensemble build, and a build can't be handed out past the people
who may have it.

The commands are one line each, from a terminal sitting in `Conductor/dev`, the same as always.

## 1. The branches

A release is `main`, and `main` only ever moves from `testing`, as a fast-forward (so `main`'s history is a
plain prefix of `testing`'s).  The session does this when I say release, or I do it myself:

```
git fetch origin && git push origin origin/testing:main
```

If the push is refused, `main` has a commit `testing` doesn't, and the fix is to merge `main` into `unstable`
first (CLAUDE.md, "Git rules").  Then check it landed:

```
git log --oneline -1 origin/main
```

That commit is the release.  Everything below is built from it.

## 2. The version number

The tag is `v` and the version: `v0.0.1`.  Three places carry a number of their own, and they all say the
same thing before the tag is made:

- **The tag**, made on GitHub in step 5.
- **Unity's Version** (Player Settings > Version, `bundleVersion` in `ProjectSettings.asset`).  It's what
  the client sends with its login, so `networking.cfg`'s `client_versions` has to list it.  Unity only
  writes `ProjectSettings.asset` on File > Save Project or on closing, so a change made in the editor is
  saved before it's committed, with the usual two-folder `git add`.
- **The crates' `version`** in each `Cargo.toml` under `Conductor/dev/`, and their lines in `Cargo.lock`.
  Nothing reads them yet.

All three went to `0.0.1` on 2026-10-02.  A number changed after the tag is a lie in the package, so the
next version's bump comes before step 3.

## 3. Conductor's package

Build it optimized, from `testing` checked out at the release commit:

```
git fetch origin && git checkout testing && git pull && cargo build --release && cargo test --release
```

The program is `Conductor/dev/target/release/conductor-launcher` (`conductor-launcher.exe` on Windows).
The package is that one file beside a `Content/` folder, since Conductor finds `Content/` by walking up from
where it's run:

```
Opus-Conductor-0.0.1/
├── conductor-launcher
└── Content/
    ├── cfg/          conductor_globals, wgui, postgres, networking, game, whitelist, blacklist
    ├── certs/        conductor.crt only.  The key never goes in a package.
    ├── scripts/      the Lua scripts
    └── psql/         defaults/schemas/ and migrations/
```

What's left out on purpose: `Content/logs/`, `Content/world/` and `Content/Assets/` (the game's own
files, and the purchased art), and `conductor.key`.  Whoever runs the package makes a key of their own with
the openssl line in README.md, or is handed ours by hand, never through a download.  The cfg files go in as
they are in git: `postgres.cfg`'s password is the placeholder, `networking.cfg`'s `bind_address` is my
machine's (10.0.0.84), and whoever runs it edits both.

Made in one line, from `Conductor/dev`, into `Conductor/build/` (gitignored):

```
rm -rf /opt/storage/Coding/Opus/Conductor/build/Opus-Conductor-0.0.1 && mkdir -p /opt/storage/Coding/Opus/Conductor/build/Opus-Conductor-0.0.1/Content && cp target/release/conductor-launcher /opt/storage/Coding/Opus/Conductor/build/Opus-Conductor-0.0.1/ && cp -r /opt/storage/Coding/Opus/Content/cfg /opt/storage/Coding/Opus/Content/scripts /opt/storage/Coding/Opus/Content/psql /opt/storage/Coding/Opus/Conductor/build/Opus-Conductor-0.0.1/Content/ && mkdir -p /opt/storage/Coding/Opus/Conductor/build/Opus-Conductor-0.0.1/Content/certs && cp /opt/storage/Coding/Opus/Content/certs/conductor.crt /opt/storage/Coding/Opus/Conductor/build/Opus-Conductor-0.0.1/Content/certs/ && rm -f /opt/storage/Coding/Opus/Conductor/build/Opus-Conductor-0.0.1/Content/cfg/*.wait4server && cd /opt/storage/Coding/Opus/Conductor/build && tar czf Opus-Conductor-0.0.1-linux-x86_64.tar.gz Opus-Conductor-0.0.1 && cd /opt/storage/Coding/Opus/Conductor/dev
```

Then look inside before it goes anywhere, and make sure there's no `.key` in it:

```
tar tzf /opt/storage/Coding/Opus/Conductor/build/Opus-Conductor-0.0.1-linux-x86_64.tar.gz
```

A Windows package is the same thing built on the laptop (`Documentation/HowTo/WINDOWS_INSTALL.md`), zipped
as `Opus-Conductor-0.0.1-windows-x86_64.zip`.  It hasn't had a database there yet (GitHub issue #10), so
for 0.0.1 it's optional.

## 4. Ensemble's package

Unity does this half.  In Unity 6 it's File > Build Profiles: pick the platform (Linux, Windows), and set
the build folder to `Ensemble/build/Opus-Ensemble-0.0.1-linux-x86_64/` (gitignored, like Conductor's).
Build, then zip the folder:

```
cd /opt/storage/Coding/Opus/Ensemble/build && zip -r Opus-Ensemble-0.0.1-linux-x86_64.zip Opus-Ensemble-0.0.1-linux-x86_64 && cd /opt/storage/Coding/Opus/Conductor/dev
```

The client trusts one certificate, the copy it carries in `Assets/Data/Certs/conductor_crt.txt`
(`ServerCertificate.cs`), and refuses any server that shows another.  So the build only talks to a
Conductor running on the same `conductor.crt` that's in step 3's package.  A new certificate means a new
client build.

The server address the client connects to is whatever it was built with (10.0.0.84 today).  Anybody
outside this house gets a build that can't reach a server, which is right for 0.0.1.

## 5. The Release on GitHub

On the repo: Releases, then "Draft a new release".

1. **Choose a tag**: type `v0.0.1`, and "Create new tag: v0.0.1 on publish".  **Target**: `main`.  (GitHub
   makes the tag at `main`'s tip when the release is published, so `main` has to be at the release commit
   from step 1 first.)
2. **Title**: `0.0.1`.
3. **Notes**: what the version does, in plain words.  For 0.0.1: a player logs in, picks a character, and
   stands in the world chatting with whoever else is there.  "Generate release notes" writes a list of the
   commits since the last tag, which is nothing for the first one and too much after that; write it by hand.
4. **Attach binaries**: drop the tar.gz from step 3 and the zip from step 4 on it.  GitHub takes files up
   to 2 GB each; the Unity build is well under that.
5. **Set as a pre-release**: yes, for anything under 1.0.  It's a label, nothing more.
6. **Publish release**.

The same from the terminal, if `gh` (GitHub's command line tool) is installed and logged in:

```
gh release create v0.0.1 --repo FluffyByteSoftware/Opus --target main --title "0.0.1" --prerelease --notes "A player logs in, picks a character, and stands in the world chatting." /opt/storage/Coding/Opus/Conductor/build/Opus-Conductor-0.0.1-linux-x86_64.tar.gz /opt/storage/Coding/Opus/Ensemble/build/Opus-Ensemble-0.0.1-linux-x86_64.zip
```

Publishing makes the tag, so afterwards `git fetch origin --tags` brings it down:

```
git fetch origin --tags && git tag -l
```

## 6. After

STATUS.md's "The branches" line says where `main` is and what version it was released as; the session
updates it at the next hand-off, or this one.  A fix that has to go into a released version goes the usual
way, `unstable`, `testing`, `main`, and is a new tag (`v0.0.2`), never the old tag moved.  A tag that
moves is a package nobody can trust.
