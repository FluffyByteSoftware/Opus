<!--
File:       Opus/Documentation/LLM/HUD_FORMATS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- HUD Formats

The two files the HUD's tools pass between them: the **catalog** (every widget there is, written by the game)
and the **layout** (where the widgets go on one screen, written by the web editor or by the game, read by the
game).  This is the contract, the same way PROTOCOL.md is the packets': the game, the web editor and any
in-game editor are all written from it, and when one disagrees with this document, the code is what gets
fixed.  A change to either format bumps its `version`, and this document gets a line in the version history
saying what changed.

The design behind it is `design/ensemble-hud.md`; Jacob's brief is `HUD_LAYOUT_SYSTEM.md`.

## Shared rules

- Both files are JSON, UTF-8.
- **Every size and offset is in reference pixels**: pixels on the screen the layout was made on (its
  `reference`).  The game scales the whole screen from the reference to the real one, keeping its shape, with
  the bigger of the two fitting (so a layout made on 2560 x 1440 shown on 1920 x 1080 is drawn at 75%, and on
  2560 x 1080 it's drawn at 75% with room left over at the sides).  Numbers may have decimals; the web editor
  writes whole ones.
- **The catalog's sizes are for a 2560 x 1440 reference** (Jacob's monitor, where the editor's canvas
  starts).  For a layout made on another reference they're scaled by the smaller of `width / 2560` and
  `height / 1440`, the same way the game scales a screen, so a widget's smallest size looks the same on
  screen whatever the layout was made on.  On 1920 x 1080 that's 0.75: the chat's 320 x 160 minimum is
  240 x 120 in that layout's pixels.
- **x runs right and y runs down**, everywhere.
- A widget's id is lower case, `a` to `z`, `0` to `9` and `_`, and never changes once it's out there, since
  layouts on players' disks name it.
- An unknown field is ignored, so an older game can read a file with a field it doesn't know about yet, as
  long as the version is one it reads.

## The anchors

Nine, written as these exact strings:

| Anchor        | The point on the screen     | The point on the widget  |
|---------------|-----------------------------|--------------------------|
| `TopLeft`     | top-left corner             | its top-left corner      |
| `Top`         | middle of the top edge      | middle of its top edge   |
| `TopRight`    | top-right corner            | its top-right corner     |
| `Left`        | middle of the left edge     | middle of its left edge  |
| `Center`      | middle of the screen        | its middle               |
| `Right`       | middle of the right edge    | middle of its right edge |
| `BottomLeft`  | bottom-left corner          | its bottom-left corner   |
| `Bottom`      | middle of the bottom edge   | middle of its bottom edge|
| `BottomRight` | bottom-right corner         | its bottom-right corner  |

The widget's point sits on the screen's point, moved by the offset.  So a widget 24 pixels in from the
bottom-right corner is `BottomRight` with an offset of `-24, -24`.  On a screen wider or taller than the
reference, an anchored widget stays with its corner or edge.

## The layout

```json
{
  "format": "opus-hud-layout",
  "version": 2,
  "screen": "hud",
  "name": "Default",
  "reference": { "width": 2560, "height": 1440 },
  "widgets": [
    {
      "id": "health", "anchor": "TopLeft",
      "offset": { "x": 24, "y": 24 }, "size": { "width": 400, "height": 48 }, "layer": 0
    },
    {
      "id": "chat", "anchor": "BottomLeft",
      "offset": { "x": 24, "y": -24 }, "size": { "width": 900, "height": 420 }, "layer": 1,
      "locked": true, "fontSize": 30
    }
  ]
}
```

| Field       | Kind   | What it is |
|-------------|--------|------------|
| `format`    | string | Always `"opus-hud-layout"`.  Anything else isn't a layout. |
| `version`   | number | `2`, or `1`, which the game still reads (it has no `locked` or `fontSize`). |
| `screen`    | string | The screen it's for: `"hud"`, `"start"` or `"character_select"`.  Only the HUD's layout is ever the player's; the other two ship with the game. |
| `name`      | string | A name for a person to read ("Default", "Combat").  The game doesn't use it yet. |
| `reference` | object | `width` and `height`, the screen the layout was made on, in pixels.  320 to 7680 wide, 240 to 4320 tall. |
| `widgets`   | array  | The widgets placed, in draw order within a layer.  A widget not in it isn't shown. |

Each widget:

| Field    | Kind   | What it is |
|----------|--------|------------|
| `id`     | string | A widget in the catalog. |
| `anchor` | string | One of the nine anchors. |
| `offset` | object | `x` and `y`, from the anchor's point, in reference pixels. |
| `size`   | object | `width` and `height`, in reference pixels. |
| `layer`  | number | A whole number.  Higher layers draw on top; widgets in the same layer draw in the file's order. |
| `locked` | boolean | Version 2.  `true` when the player locked it from its right-click menu; it can't be moved or resized until it's unlocked.  Missing is `false`, so everything starts unlocked. |
| `fontSize` | number | Version 2.  The size its text is shown at, in reference pixels, for a widget with a size to pick (chat: 18 to 42, its lines and its field).  Missing or `0` is the widget's own (chat's is 24). |

**The player's HUD layout is the game's to write too** (2026-10-03): once the player moves, resizes or locks
a widget, or picks chat's font size, the game writes the HUD as it is to `<character>_hud_layout.json` in the
player's folder (`tester_hud_layout.json`), the character's name in lower case, as version 2, with the
reference and name of the layout it was built from.  That file is read when the character enters the world;
a character without one gets the default.

### What the game does with a layout

The game is the last word, and never trusts the file.

**The whole file is turned away** (and the HUD falls back to the default layout that ships with the game,
saying why in Unity's log) when it's missing or unreadable, isn't JSON, its `format` is wrong, its `version`
is one the game doesn't read, its `screen` isn't the one being built, or its `reference` is missing or outside
the sizes above.

**A widget is fixed or skipped** (each with a warning in the log), in this order:

1. An id the game doesn't have: skipped.
2. A widget that doesn't belong on this screen (its catalog `screens`): skipped.
3. More copies of a widget than its `maxCount`: the later ones are skipped.
4. An anchor that isn't one of the nine: the widget's `defaultAnchor`.
5. A widget that isn't `resizable`: its `defaultSize` (scaled to the reference), whatever the file says.
6. A size under the widget's `minSize` (scaled to the reference): raised to it.  A size bigger than the reference: cut to it.
7. A widget off the reference screen, wholly or partly: moved back on, by changing its offset.
8. A `fontSize` on a widget with no size to pick: ignored.  One outside the widget's range: brought into it.

**A widget whose catalog entry says `fillsScreen`** (the start screen's background) covers the whole real screen,
whatever its shape, and its `anchor`, `offset` and `size` are ignored, so rules 4 to 7 don't apply to it.
Its `layer` and its place in the file still count.

## The catalog

```json
{
  "format": "opus-hud-catalog",
  "version": 2,
  "widgets": [
    {
      "id": "chat",
      "displayName": "Chat",
      "description": "Talk to other players.",
      "color": "#3A6EA5",
      "screens": ["hud"],
      "defaultSize": { "width": 700, "height": 300 },
      "minSize": { "width": 320, "height": 160 },
      "resizable": true,
      "defaultAnchor": "BottomLeft",
      "maxCount": 1,
      "fillsScreen": false
    }
  ]
}
```

| Field         | Kind   | What it is |
|---------------|--------|------------|
| `format`      | string | Always `"opus-hud-catalog"`. |
| `version`     | number | `2`. |
| `widgets`     | array  | Every widget the game has. |

Each widget:

| Field           | Kind    | What it is |
|-----------------|---------|------------|
| `id`            | string  | Its id, as a layout names it. |
| `displayName`   | string  | Its name in the editor's palette. |
| `description`   | string  | A line about it, for the editor's tooltip. |
| `color`         | string  | `#RRGGBB`, the colour of its placeholder box in the editor. |
| `screens`       | array   | The screens it can go on (`"hud"`, `"start"`, `"character_select"`). |
| `defaultSize`   | object  | `width` and `height` when it's first dropped, in pixels on a 2560 x 1440 reference (scaled for another, above). |
| `minSize`       | object  | The smallest it can be, the same way. |
| `resizable`     | boolean | `false` means it's always its `defaultSize`. |
| `defaultAnchor` | string  | The anchor it gets when it's first dropped. |
| `maxCount`      | number  | How many times it can be placed on one screen.  `1` for every widget so far. |
| `fillsScreen`   | boolean | `true` means it always covers the whole screen, behind or over the rest by its layer (a background); the layout's anchor, offset and size for it are ignored.  The web editor draws it over the whole canvas.  Missing is `false`. |

## Version history

- **Layout 1, catalog 1** (2026-10-01): the first.
- **Catalog 2** (2026-10-01): `fillsScreen`, for the login's background.  The layout is still version 1.
- **Layout 2** (2026-10-03, session 7): `locked` and `fontSize` on each widget, and rule 8; the game writes the
  player's HUD layout, one a character.  Version 1 is still read.  The catalog is still version 2: a widget's
  font sizes (chat's 18 to 42) are the game's own for now.
