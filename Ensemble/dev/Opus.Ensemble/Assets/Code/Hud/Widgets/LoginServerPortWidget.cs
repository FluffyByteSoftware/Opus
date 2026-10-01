// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginServerPortWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The login's Server Port box.  It starts filled with Conductor's TCP port,
// networking.cfg's tcp_port, or the remembered login's.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class LoginServerPortWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "login_server_port",
            DisplayName = "Server Port",
            Description = "The server's port.",
            Color = "#3A5A8A",
            Screens = new[] { "login" },
            DefaultSize = new Vector2(213f, LoginField.Height),
            MinSize = new Vector2(120f, LoginField.Height),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            TextField field = LoginField.Build(box, "Server Port", "9997");
            field.maxLength = 5;
            LoginForm.ServerPortBuilt(field);
        }
    }
}
