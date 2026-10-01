// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/LayoutChecker.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The rules a layout's widgets have to pass before anything is built.  The
// game is the last word and never trusts the file, whoever wrote it.

using System.Collections.Generic;
using UnityEngine;

namespace Opus.Hud
{
    // One widget that passed, with where it goes once the rules are done
    // with it.
    public class PlacedWidget
    {
        public Widget Widget;
        public Anchor Anchor;
        public Vector2 Offset;   // reference pixels
        public Vector2 Size;     // reference pixels
        public int Layer;
    }

    public static class LayoutChecker
    {
        // The catalog's sizes are for this reference (Jacob's monitor, where
        // the web editor's canvas starts).
        public static readonly Vector2 CatalogReference = new Vector2(2560f, 1440f);

        // The widgets of a layout that LayoutLoader already took, fixed or
        // skipped by the rules in HUD_FORMATS.md, in the same order.  Each fix
        // is a warning, since somebody's file said something it shouldn't.
        public static List<PlacedWidget> Check(LayoutFile layout)
        {
            var placed = new List<PlacedWidget>();
            var count = new Dictionary<string, int>();
            var screen = new Vector2(layout.reference.width, layout.reference.height);

            // The catalog's sizes, scaled to this layout's reference the same
            // way the game scales a screen, so a widget's smallest looks the
            // same on screen whatever the layout was made on.
            float scale = Mathf.Min(screen.x / CatalogReference.x, screen.y / CatalogReference.y);

            for (int i = 0; i < layout.widgets.Count; i++)
            {
                LayoutEntry entry = layout.widgets[i];
                if (entry == null)
                    continue;
                string which = "Layout \"" + layout.screen + "\": widget " + (i + 1) + " (\"" + entry.id + "\")";

                // 1. An id we don't have.
                Widget widget = WidgetRegistry.Make(entry.id);
                if (widget == null)
                {
                    Debug.LogWarning(which + " isn't a widget the game has.  Skipped.");
                    continue;
                }
                WidgetInfo info = widget.Info;

                // 2. A widget that doesn't go on this screen.
                if (!info.GoesOn(layout.screen))
                {
                    Debug.LogWarning(which + " doesn't go on the \"" + layout.screen + "\" screen.  Skipped.");
                    continue;
                }

                // 3. One copy too many.
                int already;
                count.TryGetValue(info.Id, out already);
                if (already >= info.MaxCount)
                {
                    Debug.LogWarning(which + " is placed more than " + info.MaxCount + " time(s).  Skipped.");
                    continue;
                }
                count[info.Id] = already + 1;

                // A widget that fills the screen takes no anchor, offset or
                // size from the file, so the rules below don't apply to it.
                // HudBuilder stretches it over the real screen.
                if (info.FillsScreen)
                {
                    placed.Add(new PlacedWidget
                    {
                        Widget = widget,
                        Anchor = Anchor.TopLeft,
                        Offset = Vector2.zero,
                        Size = screen,
                        Layer = entry.layer,
                    });
                    continue;
                }

                // 4. An anchor that isn't one of the nine.
                Anchor anchor;
                if (!Anchors.TryRead(entry.anchor, out anchor))
                {
                    anchor = info.DefaultAnchor;
                    Debug.LogWarning(which + " has an anchor of \"" + entry.anchor + "\", which isn't one.  "
                        + "Using " + anchor + ".");
                }

                // 5 and 6. The size.
                Vector2 size = new Vector2(entry.size.width, entry.size.height);
                Vector2 smallest = info.MinSize * scale;
                if (!info.Resizable)
                {
                    Vector2 fixedSize = info.DefaultSize * scale;
                    if (!Near(size, fixedSize))
                        Debug.LogWarning(which + " can't be resized.  Using its own size, " + Show(fixedSize) + ".");
                    size = fixedSize;
                }
                else if (size.x < smallest.x - 0.5f || size.y < smallest.y - 0.5f)
                {
                    Debug.LogWarning(which + " is " + Show(size) + ", smaller than its smallest, " + Show(smallest)
                        + ".  Made bigger.");
                    size = Vector2.Max(size, smallest);
                }
                if (size.x > screen.x || size.y > screen.y)
                {
                    Debug.LogWarning(which + " is " + Show(size) + ", bigger than the layout's screen, "
                        + Show(screen) + ".  Cut down.");
                    size = Vector2.Min(size, screen);
                }

                // 7. Off the screen, wholly or partly: moved back on.
                Vector2 offset = new Vector2(entry.offset.x, entry.offset.y);
                Vector2 topLeft = Anchors.TopLeftOf(anchor, offset, size, screen);
                Vector2 onScreen = KeepOnScreen(topLeft, size, screen);
                if (!Near(topLeft, onScreen))
                {
                    offset = Anchors.OffsetFor(anchor, onScreen, size, screen);
                    Debug.LogWarning(which + " is off the screen.  Moved back on.");
                }

                placed.Add(new PlacedWidget
                {
                    Widget = widget,
                    Anchor = anchor,
                    Offset = offset,
                    Size = size,
                    Layer = entry.layer,
                });
            }
            return placed;
        }

        // A top-left corner moved just far enough that the whole widget is on
        // a screen this big.  The widget has to fit already.
        public static Vector2 KeepOnScreen(Vector2 topLeft, Vector2 size, Vector2 screen)
        {
            return new Vector2(Mathf.Clamp(topLeft.x, 0f, Mathf.Max(0f, screen.x - size.x)),
                               Mathf.Clamp(topLeft.y, 0f, Mathf.Max(0f, screen.y - size.y)));
        }

        // Within half a pixel, so a number that went through a scale and came
        // back a hair off doesn't read as wrong.
        static bool Near(Vector2 a, Vector2 b)
        {
            return Mathf.Abs(a.x - b.x) < 0.5f && Mathf.Abs(a.y - b.y) < 0.5f;
        }

        static string Show(Vector2 size)
        {
            return Mathf.RoundToInt(size.x) + " x " + Mathf.RoundToInt(size.y);
        }
    }
}
