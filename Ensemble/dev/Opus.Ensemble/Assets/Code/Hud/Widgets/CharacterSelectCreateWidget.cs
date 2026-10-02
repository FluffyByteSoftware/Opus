// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectCreateWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// CREATE at character select: opens the card for making a character.
// Only shown while one of the account's three slots is empty.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectCreateWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_create",
            DisplayName = "Create",
            Description = "Opens the card for making a character.",
            Color = "#3A5A6A",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(224f, 75f),
            MinSize = new Vector2(160f, 64f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var button = new Button(CharacterSelectForm.OpenCreate);
            button.text = "CREATE";
            button.AddToClassList("character-select-button");
            box.Add(button);
            CharacterSelectForm.CreateBuilt(button);
        }
    }
}
