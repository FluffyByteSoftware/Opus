// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginRememberMeWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The login's Remember Me box.  It only ticks for now: nothing is
// remembered until there's a network client to log in with.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class LoginRememberMeWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "login_remember_me",
            DisplayName = "Remember Me",
            Description = "Keep the login for next time.",
            Color = "#3A5A8A",
            Screens = new[] { "login" },
            DefaultSize = new Vector2(747f, 53f),
            MinSize = new Vector2(240f, 53f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            // The words after the box, the way a checkbox reads.
            var toggle = new Toggle();
            toggle.text = "Remember Me";
            toggle.AddToClassList("login-remember");
            box.Add(toggle);
        }
    }
}
