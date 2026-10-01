// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/MinimapWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The minimap.  For now a placeholder box that says so: there's no map to
// draw until the server sends the world around the player.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class MinimapWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "minimap",
            DisplayName = "Minimap",
            Description = "The ground around you, from above.",
            Color = "#4A7A4A",
            Screens = new[] { "hud" },
            DefaultSize = new Vector2(320f, 320f),
            MinSize = new Vector2(160f, 160f),
            Resizable = true,
            DefaultAnchor = Anchor.TopRight,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var label = new Label("MINIMAP");
            label.AddToClassList("minimap-label");
            box.Add(label);
        }
    }
}
