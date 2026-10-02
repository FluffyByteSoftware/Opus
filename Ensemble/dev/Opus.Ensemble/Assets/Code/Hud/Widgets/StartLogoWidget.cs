// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/StartLogoWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The game's logo on the start screen.  A placeholder: a box that says
// LOGO, until there's a logo.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class StartLogoWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "start_logo",
            DisplayName = "Logo",
            Description = "The game's logo.",
            Color = "#8A6A2A",
            Screens = new[] { "start" },
            DefaultSize = new Vector2(800f, 213f),
            MinSize = new Vector2(200f, 80f),
            Resizable = true,
            DefaultAnchor = Anchor.Top,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var label = new Label("LOGO");
            label.AddToClassList("start-logo");
            box.Add(label);
        }
    }
}
