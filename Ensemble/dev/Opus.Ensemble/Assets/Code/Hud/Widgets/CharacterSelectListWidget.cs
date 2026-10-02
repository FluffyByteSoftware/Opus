// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectListWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The account's characters at character select: a row for each of the
// three slots, the character's name or "Empty", greyed when its save won't
// load.  A click on a character picks it.  Filled by CharacterSelectForm.cs
// from what the server sent; once the character is in the world, it says
// so instead.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectListWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_list",
            DisplayName = "Characters",
            Description = "The account's characters, a row per slot.",
            Color = "#3A6A5A",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(853f, 560f),
            MinSize = new Vector2(480f, 400f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            box.AddToClassList("characters");
            CharacterSelectForm.ListBuilt(box);
        }
    }
}
