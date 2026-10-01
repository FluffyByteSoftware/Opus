// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginServerIpWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The login's Server IP box.  It starts filled with Conductor's address,
// networking.cfg's bind_address.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class LoginServerIpWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "login_server_ip",
            DisplayName = "Server IP",
            Description = "The server's address.",
            Color = "#3A5A8A",
            Screens = new[] { "login" },
            DefaultSize = new Vector2(507f, LoginField.Height),
            MinSize = new Vector2(240f, LoginField.Height),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            LoginField.Build(box, "Server IP", "10.0.0.84");
        }
    }
}
