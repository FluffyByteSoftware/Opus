// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectCreateCardWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The card CREATE opens, over the list: a box for the new character's
// name, the rule under it, and MAKE and CANCEL.  Enter is MAKE, Escape is
// CANCEL.  Hidden until CREATE is pressed, and closed again once the
// server has made the character.

using Opus.Net;
using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectCreateCardWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_create_card",
            DisplayName = "Create Card",
            Description = "The card for making a character: its name, MAKE and CANCEL.",
            Color = "#2A4A5A",
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

            var title = new Label("NEW CHARACTER");
            title.AddToClassList("character-card-title");

            var label = new Label("Name");
            label.AddToClassList("character-card-label");

            var name = new TextField();
            name.maxLength = 20;
            name.AddToClassList("character-card-box");

            var rule = new Label(Session.NameRule);
            rule.AddToClassList("character-card-note");

            var buttons = new VisualElement();
            buttons.AddToClassList("character-card-buttons");

            var make = new Button(CharacterSelectForm.Make);
            make.text = "MAKE";
            make.AddToClassList("character-card-button");

            var cancel = new Button(CharacterSelectForm.CloseCard);
            cancel.text = "CANCEL";
            cancel.AddToClassList("character-card-button");

            buttons.Add(make);
            buttons.Add(cancel);
            box.Add(title);
            box.Add(label);
            box.Add(name);
            box.Add(rule);
            box.Add(buttons);
            CharacterSelectForm.CreateCardBuilt(box, name, make);
        }
    }
}
