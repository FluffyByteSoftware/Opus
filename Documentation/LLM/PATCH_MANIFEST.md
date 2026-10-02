<!--
File:       Opus/Documentation/LLM/PATCH_MANIFEST.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- the manifests: linux_manifest.json and windows_manifest.json, and the web folder

What a manifest holds, field by field, and how the web folder it sits in is laid out.  A manifest is the
stamp of the shipped client: every file in it, with its size and its hash, written once per release by
Soundcheck's admin mode and read by Soundcheck's user mode.  There's one a platform, since a Linux build
and a Windows build are different files.  This is the contract, the way PROTOCOL.md is the packets' and
REGION_MAP.md is region.map's: `Soundcheck/dev/Patch/Manifest.cs` is written from it.  A change to the
shape bumps `format` and gets a line in the history at the bottom, and the code and this file change
together.

Started 2026-10-02 with Soundcheck (`design/soundcheck.md`).  Jacob: "we are gonna have to produce a file
manifest that specifies the expected shape of the client files.  All of them."  Two of them, one a
platform, and the files beside them, from the same day: "the patcher knows if they're on linux or not and
looks for linux_manifest.json or windows_manifest.json... Then we'll reach to the
opusensemble.duckdns.org:8553/download/windows/<this will mimic the client directory so you find the
file> and the same for Linux".

## The web folder

`/opt/storage/WWW` on Jacob's machine, served as it is at `http://opusensemble.duckdns.org:8553/`:

```
WWW/
├── linux_manifest.json
├── windows_manifest.json
└── download/
    ├── linux/        an exact copy of the Linux client folder
    │   ├── Ensemble.x86_64
    │   ├── Ensemble_Data/...
    │   └── ...
    └── windows/      an exact copy of the Windows client folder
        ├── Ensemble.exe
        └── ...
```

- **Admin mode writes all of it** (`--admin`, PUBLISH): the build folder for the platform ticked is mirrored
  into `download/<platform>/` (what's new or changed copied, what the build no longer has taken out), and
  the manifest is written from the mirror at the folder's root.  So the copy and the manifest always agree.
- **User mode reads it**: at start, Soundcheck fetches the manifest for the OS it's on, and then any file
  that's missing or changed from `download/<platform>/<the file's path>`, each piece of the path
  URL-escaped (a space is `%20`).  Plain HTTP: nothing in the folder is secret, and a tampered file fails
  its hash.
- `--www <url>` on Soundcheck's command line points at another web folder for a test
  (`python3 -m http.server 8553` on a folder laid out the same).

## The shape

```json
{
  "format": 3,
  "platform": "linux",
  "client_version": "0.0.1",
  "written": "2026-10-02T21:14:09Z",
  "files": [
    {
      "path": "Ensemble.x86_64",
      "size": 14656,
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "executable": true
    },
    {
      "path": "Ensemble_Data/StreamingAssets/World/region.map",
      "size": 2883626,
      "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
    }
  ]
}
```

| Field                | Type    | What it is |
|----------------------|---------|------------|
| `format`             | number  | The shape of this file.  `3` today.  A reader that finds another number stops and says so, rather than guessing. |
| `platform`           | string  | Who the client is built for: `linux` or `windows`, nothing else.  A reader checks it's the one it asked for, so a Windows manifest put up under the Linux name is caught, not checked against. |
| `client_version`     | string  | The client's version, as the Login carries it (`0.0.1`).  `networking.cfg`'s `client_versions` has to list it.  Typed by the admin when the manifest is written.  A note for a person: the file list is what decides a patch. |
| `written`            | string  | When it was written: UTC, `YYYY-MM-DDTHH:MM:SSZ`, like every time we show. |
| `files`              | array   | One entry per file, sorted by `path`, byte by byte (`StringComparer.Ordinal`), so two manifests of the same folder are the same text. |
| `files[].path`       | string  | The file's path relative to the install folder, with forward slashes whatever the OS, so the same file has the same name on Linux and Windows, and the same path under `download/<platform>/`.  Never starts with `/` or `./`. |
| `files[].size`       | number  | Its size in bytes. |
| `files[].sha256`     | string  | Its SHA-256, as 64 lowercase hex characters.  SHA-256 over MD5 because Conductor already has `sha2` for the password's key, .NET has it built in, and MD5 buys nothing here. |
| `files[].executable` | boolean | Only when true: the file is a program on Linux (its owner's execute bit was set where the manifest was written).  A download comes with no permissions, so the patcher sets the bit back on a fetched copy.  Never written by a manifest made on Windows, which has no such bit. |

The example hashes above are made up for the shape; the sizes are region.map's at `world_size` 16 and a
guess.  The world's files aren't in a build yet (TODO.md).

## The rules

- **Every file in the folder goes in**, all of them, Soundcheck's own files included if the launcher sits in
  the build folder.  No pattern of files is left out: a Unity build is what it is, and the point is to
  know that every byte of it is ours.
- **The exceptions**, and only these:
  - A manifest sitting at the folder's root (`linux_manifest.json`, `windows_manifest.json`, or an older
    name) is skipped, since a manifest can't list itself.  It shouldn't be there anyway.
  - Unity's `Ensemble_BackUpThisFolder_ButDontShipItWithYourGame/` beside a build is skipped whole: it's
    the IL2CPP symbols, hundreds of megabytes a player never needs, and the package leaves it out.
  - The patcher's own leftovers, a `.patch` (a download on its way in) or a `.old` (a launcher file renamed
    aside on Windows), are skipped; the launcher cleans them up at its next start.
- **Folders aren't listed**, only files; an empty folder isn't in the manifest and isn't checked.
- **The order is the sort**, not the disk's: the relative paths sorted byte by byte.  So a manifest is the
  same whichever machine wrote it.
- **Pretty-printed**, two spaces, one field a line, as .NET's `WriteIndented` writes it.  A reader doesn't
  care; a person diffing two manifests does.
- **The check** (user mode) walks the install with the same rules, so the two sides skip the same things.
  A listed file that's missing, or whose size, hash or execute bit is off, is fetched and swapped in.  A
  file that's there and not listed is said and left alone: a patcher mends files, it doesn't delete them.

## Checking one by hand

From the `Opus` folder, what a manifest is for, how many files it lists and how big they come to:

```
python3 -c "import json,sys; m=json.load(open(sys.argv[1])); print(m['platform'], m['client_version'], len(m['files']), 'files', sum(f['size'] for f in m['files']), 'bytes')" /opt/storage/WWW/linux_manifest.json
```

The one that's up at the web address, the same way:

```
curl -s http://opusensemble.duckdns.org:8553/linux_manifest.json | python3 -c "import json,sys; m=json.load(sys.stdin); print(m['platform'], m['client_version'], len(m['files']), 'files', sum(f['size'] for f in m['files']), 'bytes')"
```

And one file's hash, to compare with its line:

```
sha256sum /opt/storage/WWW/download/linux/Ensemble.x86_64
```

## The version history

- **3** (2026-10-02): the files are `linux_manifest.json` and `windows_manifest.json` at the web folder's
  root, with the clients copied under `download/<platform>/` beside them, a file at a time to fetch;
  `executable` added for Linux programs; the patcher's leftovers skipped.
- **2** (2026-10-02): `platform` added, and the file was `manifest_lin.json` or `manifest_win.json` by it.
  Unity's backup folder skipped.
- **1** (2026-10-02): the first shape, `patch_manifest.json`.  `format`, `client_version`, `written`, and
  `files` of `path`, `size` and `sha256`.
