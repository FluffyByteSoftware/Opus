// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectStatusWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The line under character select's list: what the server said to the
// last ask (made, deleted, refused and why), and why the client didn't
// send one (a name that breaks the rule).  Trouble is a dark red band, the
// same as the login's status line.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectStatusWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_status",
            DisplayName = "Status",
            Description = "What the server said to the last ask at character select.",
            Color = "#6A3A3A",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(960f, 75f),
            MinSize = new Vector2(533f, 64f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var line = new Label();
            line.AddToClassList("character-select-status");
            box.Add(line);
            CharacterSelectForm.StatusBuilt(box, line);
        }
    }
}
