// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectPlayWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// PLAY at character select: brings the picked character into the world.
// Grey until a playable character is picked.  Only this button plays one;
// there's no double-click (Jacob: "you need to explicitly hit play").

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectPlayWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_play",
            DisplayName = "Play",
            Description = "Brings the picked character into the world.",
            Color = "#3A6A3A",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(224f, 75f),
            MinSize = new Vector2(160f, 64f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var button = new Button(CharacterSelectForm.Play);
            button.text = "PLAY";
            button.AddToClassList("character-select-button");
            box.Add(button);
            CharacterSelectForm.PlayBuilt(button);
        }
    }
}
