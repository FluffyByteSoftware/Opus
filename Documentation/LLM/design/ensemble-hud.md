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
  It's ScreenRoot now (below, "The login, as written").
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

## The login and character select screens (2026-10-01; the login written)

Jacob: "we need to build our HUD up for login and char select".  Settled so far:

- **Layouts, made for 1080p**: first "I think we're gonna build it as fixed docs at first assuming a 1080p
  resolution", then, turned round: "honestly actually let's just build a default layout for these using
  our UI builder".  So the login and character select are shipped layouts like the HUD's
  (`"screen": "login"` and `"character_select"`), built by our own HudBuilder from widgets, on a
  1920 x 1080 reference.
- **"Our UI builder" is the layout system we built for the HUD** (Jacob: "The one we built earlier").
- **A widget per piece** (Jacob: "A widget per piece honestly... its more work up front but may make it
  more mutatable later"): every box, checkbox and button on the screen is a widget of its own, placed on
  its own in the layout.
- **SUBMIT does nothing yet but press down** ("Right now it just presses down we want to get a feel for
  the HUD").  No networking, no switch to another screen.
- **Effects aren't decided** ("I don't know yet, don't worry about it").  Switching between screens
  "with cool ass effects if we can" waits for that.
- **HudRoot becomes ScreenRoot** (Jacob: "Yes"), the one component that owns every screen and which
  one is showing.
- **What's on the login screen, "just for now"**: Server IP and Server Port side by side, Username,
  Password, a Remember Me checkbox, and SUBMIT.
- **The plan is OK'd** (2026-10-01), with three things added (Jacob): "add in a "logo" placeholder widget
  and a background image placeholder as well (for now just a black background)", and "make it so that we
  can easily change the color and font for the text displayed as well".  The plan: six widgets,
  `login_server_ip`, `login_server_port`, `login_username`, `login_password` (dots), `login_remember_me`
  and `login_submit`, each its label and its box together, `"screens": ["login"]`;
  `Assets/Data/Layouts/login_default.json`; `Assets/Scripts/Hud/ScreenRoot.cs` in place of `HudRoot.cs`,
  starting on the login, with Show Login, Show HUD and Reset HUD To Default on its right-click menu;
  `LayoutLoader` loading any screen; `Assets/Data/Styles/login.uss`.
- **The boxes start filled** ("Prefilled with those numbers please"): Server IP `10.0.0.84`, Server Port
  `9997`, Conductor's TCP port, as `networking.cfg` has them.
- **Remember Me doesn't work yet.**  Once it does, Jacob: "when we write our hash in it will hash the
  password and I think we may rewrite the server to accept a hash instead of plaintext".  TODO.md has it.
- **The background is a widget that fills the screen** (Jacob, asked whether it was the screen's own
  colour or a widget: "fills the screen background always is a widget too?  placed behind the other in
  layer order?").  `login_background`, on layer 0 with everything else on layer 1.  Its catalog entry says
  `FillsScreen` (`fillsScreen` in the catalog, its version 2, `../HUD_FORMATS.md`), so HudBuilder stretches
  it over the whole real screen, whatever shape, and the layout's anchor, offset and size for it are
  ignored.  Black from `login.uss` for now; a picture later, as `background-image` there.
- **The text's colour and font are slots on ScreenRoot** (Jacob: "oooo slots on ScreenRoot"): Login Text
  Color and Login Text Font, in the Inspector, on every word of the login, changeable in Play mode.
  Told with it: those values live in the scene, and the scene isn't committed, so git never sees them.

### The login, as written (2026-10-01, built and tested in Unity)

Every check passed (Jacob: "That was smooth!").


- **Eight widgets** in `Assets/Code/Hud/Widgets/`, all `"screens": ["login"]`: `login_background`,
  `login_logo` (a box that says LOGO), `login_server_ip` (`10.0.0.84`), `login_server_port` (`9997`, five
  characters at most), `login_username`, `login_password` (dots), `login_remember_me` (only ticks) and
  `login_submit` (presses down, and says so in the Console).  The four text ones share `LoginField.cs`, a
  name over a box.  Their catalog sizes are the login's sizes over 0.75, since the catalog's are for
  2560 x 1440.
- **`login_default.json`**: 1920 x 1080, everything anchored to the middle; the logo, then IP and port
  side by side, Username, Password, Remember Me and SUBMIT down a column 560 wide.
- **`ScreenRoot.cs`** is `HudRoot.cs` moved, its `.meta` with it, so the GUID is the same and the
  component in Jacob's scene turns into ScreenRoot by itself; the HUD's two slots keep what was in them
  (`FormerlySerializedAs`).  Its slots: Login Layout, Login Style, Login Text Color, Login Text Font, Hud
  Layout, Hud Style.  It starts on the login; the right-click has Show Login, Show HUD (Play mode only)
  and Reset HUD To Default.  The scale is set again on every switch, since the two layouts' references
  differ.
- **Each screen carries its own style sheet** (HudBuilder puts it on the screen, not the root), so
  `hud.uss`'s `.widget` never touches the login and `login.uss`'s never touches the HUD.
- **SUBMIT and Remember Me do something now** (2026-10-01, security in the client, built and tested): SUBMIT turns the password into its key and keeps or forgets the login; a remembered login fills the
  boxes in.  `LoginForm.cs` beside the widgets is where they meet.  `design/client-security.md` has it.
- **The colour and font are set on each piece of text**, not once on the screen: Unity's theme gives the
  text in a box and on a button colours of their own, and only a style on the element itself beats it.
