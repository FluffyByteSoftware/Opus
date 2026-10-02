// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectLogOutWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// LOG OUT at character select: a Goodbye to the server, and back to the
// login screen.

using Opus.Net;
using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectLogOutWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_log_out",
            DisplayName = "Log Out",
            Description = "Back to the login screen.",
            Color = "#6A4A2A",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(320f, 75f),
            MinSize = new Vector2(160f, 64f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var button = new Button(Session.LogOut);
            button.text = "LOG OUT";
            button.AddToClassList("character-select-log-out");
            box.Add(button);
        }
    }
}
