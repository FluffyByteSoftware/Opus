// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// What every widget is: a piece of the HUD (the health bar, the chat, the
// minimap) that draws its own insides and knows nothing about where it sits.

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
