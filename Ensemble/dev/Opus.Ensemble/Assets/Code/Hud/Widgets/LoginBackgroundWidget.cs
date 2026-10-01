// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginBackgroundWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The login's background.  A placeholder: black, from login.uss, until
// there's a picture to put there.  It covers the whole screen whatever
// shape the screen is, and sits on the lowest layer, behind the rest.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class LoginBackgroundWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "login_background",
            DisplayName = "Background",
            Description = "Behind everything on the login screen, the whole screen.",
            Color = "#202020",
            Screens = new[] { "login" },
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
