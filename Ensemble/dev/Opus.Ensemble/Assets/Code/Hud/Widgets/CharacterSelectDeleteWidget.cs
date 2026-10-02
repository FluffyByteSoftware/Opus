// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectDeleteWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// DELETE at character select: opens the card that asks for DELETE to be
// typed before the picked character is deleted.  Any character can be
// deleted, an unplayable one too.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectDeleteWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_delete",
            DisplayName = "Delete",
            Description = "Opens the card for deleting the picked character.",
            Color = "#6A3A3A",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(224f, 75f),
            MinSize = new Vector2(160f, 64f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            var button = new Button(CharacterSelectForm.OpenDelete);
            button.text = "DELETE";
            button.AddToClassList("character-select-button");
            box.Add(button);
            CharacterSelectForm.DeleteBuilt(button);
        }
    }
}
