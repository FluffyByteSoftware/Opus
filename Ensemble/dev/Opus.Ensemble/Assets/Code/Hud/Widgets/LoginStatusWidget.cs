// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginStatusWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The line under SUBMIT that says how the login is going: connecting, how
// many logins are ahead, why it didn't work, why the player is back here.
// When the account is already playing somewhere else it asks what to do,
// with two buttons, KICK OTHER SESSION and LOG OFF, for 30 seconds
// (LoginForm.cs).

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class LoginStatusWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "login_status",
            DisplayName = "Status",
            Description = "How the login is going, and the question when the account is already playing.",
            Color = "#6A3A3A",
            Screens = new[] { "login" },
            DefaultSize = new Vector2(747f, 147f),
            MinSize = new Vector2(400f, 107f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var line = new Label();
            line.AddToClassList("login-status");

            var choices = new VisualElement();
            choices.AddToClassList("login-choice");

            var kick = new Button(() => LoginForm.Choose(true));
            kick.text = "KICK OTHER SESSION";
            kick.AddToClassList("login-choice-button");

            var logOff = new Button(() => LoginForm.Choose(false));
            logOff.text = "LOG OFF";
            logOff.AddToClassList("login-choice-button");

            choices.Add(kick);
            choices.Add(logOff);
            box.Add(line);
            box.Add(choices);
            LoginForm.StatusBuilt(box, line, choices);
        }
    }
}
