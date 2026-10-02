// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectDeleteCardWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The card DELETE opens, over the list: which character, a box to type
// DELETE in, and DELETE and CANCEL.  Enter is DELETE, Escape is CANCEL.
// Only DELETE (any capitals) is sent; the server checks the word again
// and deletes only on it.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectDeleteCardWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_delete_card",
            DisplayName = "Delete Card",
            Description = "The card for deleting a character: DELETE typed out, then DELETE and CANCEL.",
            Color = "#5A2A2A",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(747f, 400f),
            MinSize = new Vector2(640f, 373f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            box.AddToClassList("character-card");

            var title = new Label("DELETE CHARACTER");
            title.AddToClassList("character-card-title");

            // Says which character when the card opens.
            var line = new Label();
            line.AddToClassList("character-card-note");

            var word = new TextField();
            word.maxLength = 20;
            word.AddToClassList("character-card-box");

            var buttons = new VisualElement();
            buttons.AddToClassList("character-card-buttons");

            var confirm = new Button(CharacterSelectForm.ConfirmDelete);
            confirm.text = "DELETE";
            confirm.AddToClassList("character-card-button");
            confirm.AddToClassList("character-card-button--danger");

            var cancel = new Button(CharacterSelectForm.CloseCard);
            cancel.text = "CANCEL";
            cancel.AddToClassList("character-card-button");

            buttons.Add(confirm);
            buttons.Add(cancel);
            box.Add(title);
            box.Add(line);
            box.Add(word);
            box.Add(buttons);
            CharacterSelectForm.DeleteCardBuilt(box, line, word, confirm);
        }
    }
}
