// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginSubmitWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The login's SUBMIT button.  For now it only presses down, so we get a
// feel for the screen; logging in comes with the network client.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class LoginSubmitWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "login_submit",
            DisplayName = "Submit",
            Description = "Log in.",
            Color = "#2A6A4A",
            Screens = new[] { "login" },
            DefaultSize = new Vector2(320f, 75f),
            MinSize = new Vector2(160f, 64f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var button = new Button();
            button.text = "SUBMIT";
            button.AddToClassList("login-submit");
            button.clicked += () => Debug.Log("Login: SUBMIT pressed.  It doesn't log in yet.");
            box.Add(button);
        }
    }
}
