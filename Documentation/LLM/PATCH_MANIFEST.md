<!--
File:       Opus/Documentation/LLM/PATCH_MANIFEST.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- patch_manifest.json

What `patch_manifest.json` holds, field by field.  It's the stamp of the shipped client: every file in it,
with its size and its hash, written once per release by Soundcheck's admin mode and read by Soundcheck's
user mode and by Conductor.  Three readers, so this is the contract, the way PROTOCOL.md is the packets'
and REGION_MAP.md is region.map's: `Soundcheck/dev/Patch/Manifest.cs` is written from it, and so will
Conductor's reader be.  A change to the shape bumps `format` and gets a line in the history at the bottom,
and the code and this file change together.

Started 2026-10-02 with Soundcheck (`design/soundcheck.md`).  Jacob: "we are gonna have to produce a file
manifest that specifies the expected shape of the client files.  All of them."

## Where

- **Written** by Soundcheck in admin mode (`--admin`), from the correct client folder on the server's
  machine, to wherever the admin says; it can't go inside the folder it describes, or it would have to
  list itself.
- **Kept** at `Content/patch/patch_manifest.json` on the server.  `Content/patch/` is gitignored: the
  manifest is this machine's, for the client it ships.
- **Sent** to every client after its login, in the protocol's own bytes (a count, then each file's path,
  size and hash), not as this JSON.  The JSON is the file on disk.

## The shape

```json
{
  "format": 1,
  "client_version": "0.0.1",
  "written": "2026-10-02T21:14:09Z",
  "files": [
    {
      "path": "Opus.Ensemble_Data/StreamingAssets/World/region.map",
      "size": 2883626,
      "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
    },
    {
      "path": "Opus.Ensemble.x86_64",
      "size": 14656,
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    }
  ]
}
```

| Field            | Type    | What it is |
|------------------|---------|------------|
| `format`         | number  | The shape of this file.  `1` today.  A reader that finds another number stops and says so, rather than guessing. |
| `client_version` | string  | The client's version, as the Login carries it (`0.0.1`).  `networking.cfg`'s `client_versions` has to list it.  Typed by the admin when the manifest is written. |
| `written`        | string  | When it was written: UTC, `YYYY-MM-DDTHH:MM:SSZ`, like every time we show. |
| `files`          | array   | One entry per file, sorted by `path`, byte by byte (`StringComparer.Ordinal`), so two manifests of the same folder are the same text. |
| `files[].path`   | string  | The file's path relative to the install folder, with forward slashes whatever the OS, so the same file has the same name on Linux and Windows.  Never starts with `/` or `./`. |
| `files[].size`   | number  | Its size in bytes. |
| `files[].sha256` | string  | Its SHA-256, as 64 lowercase hex characters.  SHA-256 over MD5 because Conductor already has `sha2` for the password's key, .NET has it built in, and MD5 buys nothing here. |

The example hashes above are made up for the shape; the sizes are region.map's at `world_size` 16 and a
guess.

## The rules

- **Every file in the folder goes in**, all of them, Soundcheck's own files included.  No pattern of files
  is left out: a Unity build is what it is, and the point is to know that every byte of it is ours.
- **The one exception**: a `patch_manifest.json` sitting at the folder's root is skipped, since a manifest
  can't list itself.  It shouldn't be there anyway (above).
- **Folders aren't listed**, only files; an empty folder isn't in the manifest and isn't checked.
- **The order is the sort**, not the disk's: the relative paths sorted byte by byte.  So a manifest is the
  same whichever machine wrote it.
- **Pretty-printed**, two spaces, one field a line, as .NET's `WriteIndented` writes it.  A reader doesn't
  care; a person diffing two manifests does.

## Checking one by hand

From the `Opus` folder, how many files a manifest lists and how big they come to:

```
python3 -c "import json,sys; m=json.load(open(sys.argv[1])); print(m['client_version'], len(m['files']), 'files', sum(f['size'] for f in m['files']), 'bytes')" /opt/storage/Coding/Opus/Content/patch/patch_manifest.json
```

And one file's hash, to compare with its line:

```
sha256sum /path/to/the/client/Opus.Ensemble.x86_64
```

## The version history

- **1** (2026-10-02): the first shape.  `format`, `client_version`, `written`, and `files` of `path`, `size`
  and `sha256`.
