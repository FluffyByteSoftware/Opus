// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectBackgroundWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Character select's background.  A placeholder: black, from
// character_select.uss, until there's a picture.  It covers the whole
// screen whatever shape the screen is, behind the rest.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectBackgroundWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_background",
            DisplayName = "Background",
            Description = "Behind everything on character select, the whole screen.",
            Color = "#202020",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(2560f, 1440f),
            MinSize = new Vector2(2560f, 1440f),
            Resizable = false,
            DefaultAnchor = Anchor.TopLeft,
            FillsScreen = true,
        };

        public override WidgetInfo Info { get { return info; } }

        // Nothing inside it: the box itself is the background.
        public override void Build(VisualElement box)
        {
        }
    }
}
