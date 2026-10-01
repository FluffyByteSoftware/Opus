// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginUsernameWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The login's Username box.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class LoginUsernameWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "login_username",
            DisplayName = "Username",
            Description = "The account's name.",
            Color = "#3A5A8A",
            Screens = new[] { "login" },
            DefaultSize = new Vector2(747f, LoginField.Height),
            MinSize = new Vector2(320f, LoginField.Height),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            LoginField.Build(box, "Username", "");
        }
    }
}
