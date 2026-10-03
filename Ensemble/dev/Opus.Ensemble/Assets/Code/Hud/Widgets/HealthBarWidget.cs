// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/HealthBarWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The health bar.  For now a placeholder: there's no health to show until
// the server sends some, so the bar sits full.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class HealthBarWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "health",
            DisplayName = "Health",
            Description = "Your character's health.",
            Color = "#B03A3A",
            Screens = new[] { "hud" },
            DefaultSize = new Vector2(400f, 48f),
            MinSize = new Vector2(200f, 32f),
            // Movable, but only chat is resizable for now (Jacob,
            // 2026-10-03).
            Resizable = false,
            DefaultAnchor = Anchor.TopLeft,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var label = new Label("HEALTH");
            label.AddToClassList("health-label");

            var track = new VisualElement();
            track.AddToClassList("health-track");
            var fill = new VisualElement();
            fill.AddToClassList("health-fill");
            track.Add(fill);

            box.Add(label);
            box.Add(track);
        }
    }
}
