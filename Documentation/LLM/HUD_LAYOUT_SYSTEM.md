<!--
File:       Opus/Documentation/LLM/HUD_LAYOUT_SYSTEM.md
Component:  Documentation
Author:     Jacob Chacko
-->

# HUD Layout System -- Design Brief

Jacob's brief, 2026-10-01, kept as he wrote it.  The design that answers it goes in
`design/ensemble-hud.md` once he's OKed it.

## Your job

Design and build a data-driven HUD system for a Unity game client. The HUD is assembled at runtime from a
layout file instead of being hardcoded. Players will eventually customize that layout with an external
web-based editor, and possibly an in-game editor later.

Before writing any code, read this whole brief and reply with:

1. Your proposed design, including folder layout, main classes, and how data flows from file to screen.
2. Your proposed JSON schemas for the catalog and layout files, with a short example of each.
3. Any questions or concerns about what is written here.

Wait for Jacob to approve the design before implementing anything.

## About the developer

- Jacob is a hobbyist developer. He is comfortable in C# and C.
- Keep code simple and readable over clever. Prefer plain classes and clear names to heavy abstraction,
  reflection tricks, or deep inheritance.
- Jacob builds and runs everything himself in Unity. After each step, tell him exactly what to set up in the
  editor and what he should see when he presses Play, so he can test it and report back.
- Every source file starts with a header comment giving its path relative to the repo root, starting with
  `Opus/`. Follow the existing convention in `CLAUDE.md`.
- Work in small, testable steps. Don't build all phases at once.

## Hard requirements

- Use Unity UI Toolkit (`UIDocument`, `VisualElement`, USS). Do not use the Canvas / uGUI system.
- Terminology: call the HUD pieces **widgets**, never "UI elements." This avoids confusion with UI Toolkit's
  `VisualElement`. Use this naming consistently in code, files, and docs.
- The file format is the contract. The game, the web editor, and any future in-game editor all read and
  write the same formats. Design the formats carefully; they matter more than any single implementation.
- Use JSON for all files. Don't use binary. Every file includes a `version` field so older files can still
  be loaded after the format changes.

## Core concepts

### 1. Widget

A self-contained HUD piece: health bar, chat window, combat log, minimap, hotbar, and so on.

- Each widget type has a unique string ID, such as `"chat"` or `"minimap"`.
- A widget draws its own contents but knows nothing about where it is placed. Placement comes only from the
  layout.
- Widgets should follow one simple common contract, such as a base class or interface, so that adding a new
  widget type means writing the widget plus one registration line. Propose what this contract looks like.

### 2. Catalog (exported by the game)

The catalog lists every widget type that exists. The game generates it so the web editor knows what is
available. Each entry includes at least:

- `id`
- `displayName`
- `defaultSize`
- `minSize`
- `resizable` (bool)
- Something visual for the editor's placeholder box, such as a color or short description.

Propose anything else that is useful, such as whether a widget can be placed more than once. For now,
assume each widget appears at most once.

### 3. Layout (written by the editor, read by the game)

The layout lists only the widgets the player chose to place. A widget that is not in the layout is not
shown. Each entry includes:

- `id` (refers to a catalog entry)
- `anchor`: which corner or edge, or the center, the widget is attached to
- `offset` and `size`, stored as percentages of the screen, not pixels, so layouts survive resolution and
  aspect-ratio changes
- `layer`: draw order; higher layers draw on top

UI Toolkit has no z-index. Draw order is hierarchy order. Implement layers as stacked full-screen container
elements whose `pickingMode` is set to `Ignore`, so empty space doesn't block clicks to the game.

## Rules the game must enforce when loading a layout

The game is the final authority. Never trust the file.

- Unknown widget ID → skip it and log a warning.
- Size below the catalog minimum → clamp it up to the minimum.
- Widget positioned off-screen → clamp it back on screen.
- Missing, unreadable, or unsupported-version layout file → fall back to the built-in default layout and log
  why.
- The default layout ships with the game as its own JSON file. Provide a way to reset to it.

## Phases

Build in this order. Each phase should end in something Jacob can run and check.

### Phase 1: Runtime from a hand-written layout

This is the immediate goal: a functional HUD.

- Widget contract plus a registry.
- Two or three simple placeholder widgets to prove the system, such as a health bar, a chat box, and a
  minimap box. They can be visually crude.
- The layer stack and a HUD builder that reads the layout JSON and places widgets.
- A hand-written default layout JSON.
- The load-time validation rules above.

### Phase 2: Catalog export

- A way, such as an editor menu item, to write the catalog JSON from the registered widgets.

### Phase 3: Web layout editor

This is a separate small project.

- A static web page that loads a catalog JSON and an optional existing layout JSON.
- A palette of widgets on one side and a screen canvas with a selectable aspect ratio (16:9, 21:9, 16:10).
- Drag widgets onto the canvas, move them, resize them while respecting minimum sizes, set their anchor, and
  remove them.
- Show widgets as labeled placeholder boxes. The editor does not render the real widgets.
- Optional snap-to-grid.
- Save the result as a layout JSON download. Jacob copies it into the game's settings folder.

### Future (design for it, don't build it)

- An in-game edit mode that edits the same layout data.
- Multiple saved layout profiles, such as combat and crafting.
- Widgets that can appear more than once, or player-built custom panels.

## Things to avoid

- Hardcoding widget positions anywhere except the default layout file.
- Pixel-based positions in saved files.
- Coupling widgets to the layout system. A widget should work if dropped anywhere.
- Over-engineering Phase 1 for the future features. Leave room for them, but don't build them.
