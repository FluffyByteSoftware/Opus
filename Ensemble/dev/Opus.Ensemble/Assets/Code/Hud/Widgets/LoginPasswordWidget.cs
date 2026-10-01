// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginPasswordWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The login's Password box.  What's typed shows as dots, and so does a
// remembered login's key standing in for it (LoginForm.cs).

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class LoginPasswordWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "login_password",
            DisplayName = "Password",
            Description = "The account's password, shown as dots.",
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
            TextField field = LoginField.Build(box, "Password", "");
            field.isPasswordField = true;
            field.maskChar = '\u2022';   // a dot, not Unity's asterisk
            LoginForm.PasswordBuilt(field);
        }
    }
}
