<!--
File:       Opus/Documentation/LLM/PATCH_MANIFEST.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- the manifests: manifest_lin.json and manifest_win.json

What a manifest holds, field by field.  It's the stamp of the shipped client: every file in it, with its
size and its hash, written once per release by Soundcheck's admin mode and read by Soundcheck's user mode
(and by Conductor, when its half comes).  There's one a platform, since a Linux build and a Windows build
are different files: `manifest_lin.json` and `manifest_win.json`, the same two names on the disk and at
the web address.  This is the contract, the way PROTOCOL.md is the packets' and REGION_MAP.md is
region.map's: `Soundcheck/dev/Patch/Manifest.cs` is written from it, and so will Conductor's reader be.  A
change to the shape bumps `format` and gets a line in the history at the bottom, and the code and this file
change together.

Started 2026-10-02 with Soundcheck (`design/soundcheck.md`).  Jacob: "we are gonna have to produce a file
manifest that specifies the expected shape of the client files.  All of them."  Two of them, one a
platform, from the same day: "admin mode builds a working manifest for Windows and Linux".

## Where

- **Written** by Soundcheck in admin mode (`--admin`), from the correct client folder for the platform
  picked, on the server's machine, into whatever folder the admin says; it can't go inside the folder it
  describes, or it would have to list itself.
- **Served** from the web address, plain HTTP (Jacob, 2026-10-02):
  `http://opusensemble.com:8553/manifest_lin.json` and `http://opusensemble.com:8553/manifest_win.json`.
  Putting the two files there is a step of the release.  The address is a constant in
  `Patch/ManifestSource.cs`; `--manifest <url>` on Soundcheck's command line points at another copy for a
  test.
- **Fetched** by Soundcheck in user mode after SUBMIT's login: the one for the OS it's running on, and it
  checks that the file says the same platform.  Nothing in a manifest is secret, and a tampered one can
  only make the check fail, never pass something the server would turn away.

## The shape

```json
{
  "format": 2,
  "platform": "linux",
  "client_version": "0.0.1",
  "written": "2026-10-02T21:14:09Z",
  "files": [
    {
      "path": "Ensemble.x86_64",
      "size": 14656,
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    },
    {
      "path": "Ensemble_Data/StreamingAssets/World/region.map",
      "size": 2883626,
      "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
    }
  ]
}
```

| Field            | Type    | What it is |
|------------------|---------|------------|
| `format`         | number  | The shape of this file.  `2` today.  A reader that finds another number stops and says so, rather than guessing. |
| `platform`       | string  | Who the client is built for: `linux` or `windows`, nothing else.  A reader checks it's the one it asked for, so a Windows manifest put up under the Linux name is caught, not checked against. |
| `client_version` | string  | The client's version, as the Login carries it (`0.0.1`).  `networking.cfg`'s `client_versions` has to list it.  Typed by the admin when the manifest is written; a launcher whose own version is another number stops there and tells the player to download the launcher again. |
| `written`        | string  | When it was written: UTC, `YYYY-MM-DDTHH:MM:SSZ`, like every time we show. |
| `files`          | array   | One entry per file, sorted by `path`, byte by byte (`StringComparer.Ordinal`), so two manifests of the same folder are the same text. |
| `files[].path`   | string  | The file's path relative to the install folder, with forward slashes whatever the OS, so the same file has the same name on Linux and Windows.  Never starts with `/` or `./`. |
| `files[].size`   | number  | Its size in bytes. |
| `files[].sha256` | string  | Its SHA-256, as 64 lowercase hex characters.  SHA-256 over MD5 because Conductor already has `sha2` for the password's key, .NET has it built in, and MD5 buys nothing here. |

The example hashes above are made up for the shape; the sizes are region.map's at `world_size` 16 and a
guess.  The world's files aren't in a build yet (TODO.md).

## The rules

- **Every file in the folder goes in**, all of them, Soundcheck's own files included.  No pattern of files
  is left out: a Unity build is what it is, and the point is to know that every byte of it is ours.
- **Two exceptions**, and only these:
  - A manifest sitting at the folder's root (`manifest_lin.json`, `manifest_win.json`, or the old
    `patch_manifest.json`) is skipped, since a manifest can't list itself.  It shouldn't be there anyway
    (above).
  - Unity's `Ensemble_BackUpThisFolder_ButDontShipItWithYourGame/` beside a build is skipped whole: it's
    the IL2CPP symbols, hundreds of megabytes a player never needs, and the package leaves it out.
- **Folders aren't listed**, only files; an empty folder isn't in the manifest and isn't checked.
- **The order is the sort**, not the disk's: the relative paths sorted byte by byte.  So a manifest is the
  same whichever machine wrote it.
- **Pretty-printed**, two spaces, one field a line, as .NET's `WriteIndented` writes it.  A reader doesn't
  care; a person diffing two manifests does.
- **The check** (user mode) walks the install with the same rules, so the two sides skip the same things.
  A listed file that's missing, or whose size or hash is off, fails it.  A file that's there and not listed
  is said and left alone: a patcher mends files, it doesn't delete them.

## Checking one by hand

From the `Opus` folder, what a manifest is for, how many files it lists and how big they come to:

```
python3 -c "import json,sys; m=json.load(open(sys.argv[1])); print(m['platform'], m['client_version'], len(m['files']), 'files', sum(f['size'] for f in m['files']), 'bytes')" /opt/storage/Coding/Opus/Content/patch/manifest_lin.json
```

The one that's up at the web address, the same way:

```
curl -s http://opusensemble.com:8553/manifest_lin.json | python3 -c "import json,sys; m=json.load(sys.stdin); print(m['platform'], m['client_version'], len(m['files']), 'files', sum(f['size'] for f in m['files']), 'bytes')"
```

And one file's hash, to compare with its line:

```
sha256sum /path/to/the/client/Ensemble.x86_64
```

## The version history

- **2** (2026-10-02): `platform` added, and the file is `manifest_lin.json` or `manifest_win.json` by it,
  served from the web address.  Unity's backup folder is skipped.
- **1** (2026-10-02): the first shape, `patch_manifest.json`.  `format`, `client_version`, `written`, and
  `files` of `path`, `size` and `sha256`.
