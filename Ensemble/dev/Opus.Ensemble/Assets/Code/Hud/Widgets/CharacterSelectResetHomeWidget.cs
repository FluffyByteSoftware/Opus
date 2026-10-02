// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectResetHomeWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// RESET HOME at character select: sends the picked character back to
// 0, 0, 0, for one that's stuck somewhere.  Grey for an unplayable one,
// which the server won't move.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectResetHomeWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_reset_home",
            DisplayName = "Reset Home",
            Description = "Sends the picked character back to 0, 0, 0.",
            Color = "#5A5A3A",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(224f, 75f),
            MinSize = new Vector2(160f, 64f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var button = new Button(CharacterSelectForm.ResetHome);
            button.text = "RESET HOME";
            button.AddToClassList("character-select-button");
            box.Add(button);
            CharacterSelectForm.ResetHomeBuilt(button);
        }
    }
}
