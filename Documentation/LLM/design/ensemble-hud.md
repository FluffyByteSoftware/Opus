<!--
File:       Opus/Documentation/LLM/design/ensemble-hud.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Ensemble -- the HUD and its layouts

Jacob's brief is `../HUD_LAYOUT_SYSTEM.md`.  This file is what's settled in answer to it.  **Nothing is
built yet.**

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
  - **The data in `Assets/Data/Hud/`**, a new folder of ours ("Assets/Data/Hud/ (new folder)"):
    `default_layout.json` and `hud.uss`, dragged onto HudRoot in the Inspector.
  - **The player's layout in Unity's per-user folder** ("per user folder yep"):
    `persistentDataPath/hud_layout.json`, `~/.config/unity3d/<Company>/<Product>/` on Linux,
    `AppData\LocalLow\<Company>\<Product>\` on Windows.
- **Every screen is built with the tool, and only the player's HUD is dynamic** (Jacob: "we are going to
  use the tool to build layouts but the only dynamic one is the player hud").  The login and character
  select are layouts too, made in the editor and shipped with the game, but never read from the player's
  folder.  Only the HUD's layout can be the player's.
- **The two formats** (catalog and layout) are as proposed, `format` and `version` in each, `maxCount`
  and `defaultAnchor` in the catalog, `name` in the layout, and they'll be written down as a contract in
  `../HUD_FORMATS.md` the way PROTOCOL.md is.  **Their units are open**, below.

## Open

- **Percent or pixels.**  The brief says percent of the screen.  Jacob, 2026-10-01: "actually shouldn't
  most of these sizes be in pixels?"  Proposed: reference pixels on a 1920 x 1080 screen, with UI
  Toolkit scaling the whole HUD (PanelSettings, Scale With Screen Size, Expand).
- **Which screen a widget belongs on**, now that the login and character select are layouts too.
