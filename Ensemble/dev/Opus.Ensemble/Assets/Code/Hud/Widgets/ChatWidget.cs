// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/ChatWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The chat.  For now a placeholder: what's typed and sent with Enter shows
// up in the box and goes nowhere else, which is enough to prove typing and
// clicking reach the HUD.  The server's side of chat isn't designed yet.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class ChatWidget : Widget
    {
        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "chat",
            DisplayName = "Chat",
            Description = "Talk to other players.",
            Color = "#3A6EA5",
            Screens = new[] { "hud" },
            DefaultSize = new Vector2(640f, 320f),
            MinSize = new Vector2(320f, 160f),
            Resizable = true,
            DefaultAnchor = Anchor.BottomLeft,
        };

        ScrollView lines;
        TextField input;

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            lines = new ScrollView(ScrollViewMode.Vertical);
            lines.AddToClassList("chat-lines");
            AddLine("Chat stays on this screen for now. Nothing is sent anywhere.");

            input = new TextField();
            input.AddToClassList("chat-input");

            // Caught on the way down (TrickleDown), before the text field
            // does anything of its own with Enter.
            input.RegisterCallback<KeyDownEvent>(KeyDown, TrickleDown.TrickleDown);

            box.Add(lines);
            box.Add(input);
        }

        void KeyDown(KeyDownEvent e)
        {
            if (e.keyCode != KeyCode.Return && e.keyCode != KeyCode.KeypadEnter)
                return;

            string text = input.value.Trim();
            if (text != "")
                AddLine(text);
            input.value = "";
            e.StopPropagation();
        }

        void AddLine(string text)
        {
            var line = new Label(text);
            line.AddToClassList("chat-line");
            lines.Add(line);

            // The new line has no size until the next layout pass, so the
            // scroll down waits for it.
            lines.schedule.Execute(() => lines.ScrollTo(line));
        }
    }
}
