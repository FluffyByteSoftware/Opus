// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginField.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The one thing the login's four text widgets share: a name over a box to
// type in, filling the widget's box.

using UnityEngine.UIElements;

namespace Opus.Hud
{
    public static class LoginField
    {
        // The catalog's sizes for a field are pixels on a 2560 x 1440
        // reference; the login's layout is made on 1920 x 1080, which scales
        // them by 0.75.  96 tall here is 72 on the login.
        public const float Height = 96f;

        public static TextField Build(VisualElement box, string name, string startsWith)
        {
            box.AddToClassList("login-field");

            var label = new Label(name);
            label.AddToClassList("login-label");

            var field = new TextField();
            field.AddToClassList("login-box");
            field.value = startsWith;

            box.Add(label);
            box.Add(field);
            return field;
        }
    }
}
