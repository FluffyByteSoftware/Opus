<!--
File:       Opus/Documentation/LLM/design/ensemble-hud.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Ensemble -- the HUD and its layouts

Jacob's brief is `../HUD_LAYOUT_SYSTEM.md`.  This file is what's settled in answer to it.  The two file
formats are `../HUD_FORMATS.md`, the contract.

**Phase 1 is written** (2026-10-01): the HUD from a hand-written layout.  **It compiles and runs in Unity**
(Jacob: "holy shit it worked"), and every one of its checks passed.
Phases 2 and 3 are in TODO.md.

## Phase 1, as written

- `Assets/Code/Hud/`, namespace `Opus.Hud`: `Widget.cs` (the base class and `WidgetInfo`, a widget's
  catalog entry), `WidgetRegistry.cs`, `HudLayout.cs` (the layout's classes for JsonUtility, the nine
  anchors and the one sum that places all of them), `LayoutLoader.cs`, `LayoutChecker.cs`,
  `HudBuilder.cs`, and three placeholder widgets in `Widgets/`: health (a full bar; no health to show
  yet), minimap (a box that says so) and chat (what's typed shows in the box and goes nowhere).
- `Assets/Scripts/Hud/HudRoot.cs`, the MonoBehaviour beside the UI Document, with two slots, Default
  Layout and Style Sheet.  It sets Scale With Screen Size, Expand and the layout's reference on **a copy**
  of the UI Document's Panel Settings, since a change to the asset in Play mode would stay in it.
- `Assets/Data/Layouts/hud_default.json` (2560 x 1440: health top-left, minimap top-right, chat
  bottom-left on layer 1) and `Assets/Data/Styles/hud.uss`.
- **Where the off-screen rule runs**: LayoutChecker moves a widget back on against the layout's reference,
  once, with a warning; HudBuilder places the boxes against the real screen every time its size changes,
  and only guards quietly there, since the real screen is never smaller than the reference.
- **The catalog's sizes are for a 2560 x 1440 reference**, scaled by the smaller of `width / 2560` and
  `height / 1440` for a layout made on another (`HUD_FORMATS.md`).

## Settled (2026-10-01)

- **The design is OK'd** as proposed: a `Widget` base class (`Info`, `Build(box)`), a registry that's one
  list with a line per widget, `LayoutLoader` (file, format, version, the fall back to the default),
  `LayoutChecker` (the load rules), `HudBuilder` (the layers and the boxes) and `HudRoot`, the
  MonoBehaviour beside the UIDocument.  UI Toolkit, `JsonUtility` (built in, nothing to add).
  - A widget fills the box it's handed and never moves or sizes it.
  - Layers are full-screen containers, one per layer in use, lowest first, `pickingMode = Ignore`.
  - The load rules, in order: unknown id skipped, a second copy of an id skipped, an unknown anchor takes
    the catalog's default, size clamped to the minimum (and to the screen; a widget that isn't resizable
    gets its default size), then pushed back on screen.  Each fix is a warning in Unity's log.
  - Reset deletes the player's file and builds again; a right-click on HudRoot in the Inspector for now.
- **Where things live** (Jacob):
  - The C# in `Assets/Code/Hud/` (the plain classes, the widgets in `Widgets/`) and
    `Assets/Scripts/Hud/HudRoot.cs`; the catalog's export in `Assets/Editor/`.
  - **The data in `Assets/Data/Layouts/`**, a new folder of ours (Jacob first said `Assets/Data/Hud/`,
    then `Assets/Data/Layouts` once the login and character select were layouts too): the HUD's default
    layout, the login's and character select's when they come, dragged onto HudRoot in the Inspector.
    **The stylesheets in `Assets/Data/Styles/`** (Jacob), `hud.uss` the first.
  - **The player's layout in a per-user folder** ("per user folder yep"): `hud_layout.json` in
    `~/.config/unity3d/FluffyByte Studios/Opus.Ensemble/Unity/` on Linux,
    `AppData\LocalLow\FluffyByte Studios\Opus.Ensemble\Unity\` on Windows (`LayoutLoader.PlayerFolder`).
    Unity's own folder for a player is named after the Product Name, which is the game's name, Forgotten
    Legends, and the game's name isn't the directory path (Jacob: "its the games name but not the
    directory path", then "change it to /home/froggy/.config/unity3d/FluffyByte Studios/Opus.Ensemble/Unity").
    It's "the folder we're stuck with for now" (Jacob, asked whether the `Unity` at the end was meant).
    So the code takes the company's folder above Unity's (`persistentDataPath`'s parent, Company Name from
    Player Settings) and names its own under it.  Unity's own per-player files stay in the Product Name
    folder.
- **Every screen is built with the tool, and only the player's HUD is dynamic** (Jacob: "we are going to
  use the tool to build layouts but the only dynamic one is the player hud").  The login and character
  select are layouts too, made in the editor and shipped with the game, but never read from the player's
  folder.  Only the HUD's layout can be the player's.  Jacob, asked to confirm: "The login and character
  select screens will be premade in a shipped product yes".
- **A layout says which screen it's for, and a catalog entry where it can go**: `"screen": "hud"`,
  `"login"` or `"character_select"` in the layout, `"screens": ["hud"]` in the catalog, both in version 1.
  The editor's palette shows only what belongs on the screen being edited, and the game turns away a
  widget found on a screen it doesn't belong on.  Phase 1 builds the HUD only.
- **The two formats** (catalog and layout) are as proposed, `format` and `version` in each, `maxCount`
  and `defaultAnchor` in the catalog, `name` in the layout, and they'll be written down as a contract in
  `../HUD_FORMATS.md` the way PROTOCOL.md is.  Their units are reference pixels, below.
- **Reference pixels, not percent** (turning round the brief's "no pixel-based positions"): Jacob,
  "actually shouldn't most of these sizes be in pixels?"  So sizes, offsets and minimum sizes are pixels
  on a reference screen, and UI Toolkit scales the whole HUD to the real one (PanelSettings, Scale With
  Screen Size, Expand, so a layout made on the reference always fits).  His answer on the reference:
  "You should pick your resolution with the default being 1080 yes".  Asked which reading, Jacob: "the
  tool webpage defaults to a 1080p screen.  Player can pick their resolution and change it.  Then the
  webpage rescales the projection so that it's pixel right and scrollable in the window."  And: "need a
  default scaled off my monitor which is 1440".
  - **Each layout says the resolution it was made at** (`"reference"`, width and height).  The web
    editor's canvas is that many pixels, shown one for one and scrolled when it's bigger than the
    window, and the game scales from the file's reference to the real screen, the bigger of the two
    fitting (Expand), so nothing falls off.  Jacob, on that reading: "yes now you got the picture".
  - **The editor's canvas starts at 2560 x 1440**, Jacob's monitor, not 1080 (his "b"), and the maker
    can change it.
