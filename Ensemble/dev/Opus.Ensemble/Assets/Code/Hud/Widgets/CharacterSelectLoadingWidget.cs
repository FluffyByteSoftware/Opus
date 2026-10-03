// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectLoadingWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The loading bar over character select's list, while the nearest chunks
// of the ground come in after PLAY and are drawn: an
// enemy's health bar going backwards, red, filling from empty as the
// pieces arrive (Jacob, 2026-10-03: "look like an enemy healthbar going
// backwards lol"; "we'll make it sexy later and look like you're fighting
// a boss").  Hidden the rest of the time.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class CharacterSelectLoadingWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "character_select_loading",
            DisplayName = "Loading Bar",
            Description = "The ground coming in after PLAY, as a bar filling up.",
            Color = "#8C2020",
            Screens = new[] { "character_select" },
            DefaultSize = new Vector2(853f, 213f),
            MinSize = new Vector2(533f, 160f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            box.AddToClassList("character-loading");

            var title = new Label("ENTERING THE WORLD");
            title.AddToClassList("character-loading-title");

            var track = new VisualElement();
            track.AddToClassList("character-loading-track");
            var fill = new VisualElement();
            fill.AddToClassList("character-loading-fill");
            track.Add(fill);

            // How much is in, under the bar.
            var line = new Label();
            line.AddToClassList("character-loading-line");

            box.Add(title);
            box.Add(track);
            box.Add(line);
            CharacterSelectForm.LoadingBuilt(box, fill, line);
        }
    }
}
