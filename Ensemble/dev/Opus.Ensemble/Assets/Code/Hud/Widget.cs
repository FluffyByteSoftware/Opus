// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// What every widget is: a piece of a screen (the HUD's health bar, the
// start screen's card) that draws its own insides and knows nothing about
// where it sits.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    // A widget gets handed an empty box that's already been placed and sized
    // from the layout, and fills it.  It never moves or sizes the box, so it
    // works wherever it's dropped.
    //
    // A new widget is a class like this plus one line in WidgetRegistry.
    public abstract class Widget
    {
        // What the catalog says about it.  The same for every copy.
        public abstract WidgetInfo Info { get; }

        // Fill the box.  Called once, when the HUD is built.
        public abstract void Build(VisualElement box);

        // The size the player picked for the widget's text, in the layout's
        // pixels.  Only a widget whose Info has font sizes is ever handed
        // one, after Build() and again whenever it's picked.
        public virtual void UseFontSize(int pixels)
        {
        }
    }

    // A widget's entry in the catalog.  HUD_FORMATS.md has what each field
    // means; the sizes are pixels on a 2560 x 1440 reference.
    public class WidgetInfo
    {
        public string Id;              // "chat".  Never changes once it's out there.
        public string DisplayName;     // "Chat", in the web editor's palette
        public string Description;     // a line for the editor's tooltip
        public string Color;           // "#3A6EA5", the editor's placeholder box
        public string[] Screens;       // { "hud" }
        public Vector2 DefaultSize;
        public Vector2 MinSize;
        public bool Resizable = true;
        public Anchor DefaultAnchor = Anchor.TopLeft;
        public int MaxCount = 1;

        // The text sizes a player can pick from the widget's right-click
        // menu, smallest to biggest, and the one it has until they do.  All
        // 0 for a widget with no size to pick.  The game's own for now; not
        // in the catalog's file.
        public int SmallestFont;
        public int BiggestFont;
        public int DefaultFont;

        public bool HasFontSize
        {
            get { return BiggestFont > 0; }
        }

        // Covers the whole real screen, whatever the layout says about its
        // anchor, offset and size: a background, say.  The layout still
        // places it, for its layer and its place in the drawing order.
        public bool FillsScreen;

        public bool GoesOn(string screen)
        {
            foreach (string s in Screens)
            {
                if (s == screen)
                    return true;
            }
            return false;
        }
    }
}
