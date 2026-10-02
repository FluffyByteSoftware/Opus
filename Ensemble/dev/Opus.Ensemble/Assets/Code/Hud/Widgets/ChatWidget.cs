// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/ChatWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The chat window: a "Chat" header, the lines, and a field on the bottom
// row to type in.  Enter sends what's typed to the server as it was typed,
// and echoes it as ">/chat hello" in yellow; what the server says is white.
// The box stops taking keys at 300 characters.  The font is ScreenRoot's
// Chat Font slot, a monospaced one, so /who's box lines up.  EverQuest's
// keys (Jacob, 2026-10-02): with the field not focused, Enter or a "/"
// anywhere on the screen brings the keys to it, the "/" already typed;
// Escape, or a click away, drops them.

using Opus.Net;
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
            DefaultSize = new Vector2(700f, 300f),
            MinSize = new Vector2(320f, 160f),
            Resizable = true,
            DefaultAnchor = Anchor.BottomLeft,
        };

        // The server ignores anything past 300, and the client never
        // makes more (Jacob, 2026-10-02).
        const int LongestLine = 300;

        // The lines kept to scroll back through; older ones go.
        const int KeptLines = 200;

        // /who's box is drawn this wide when the chat box can't be measured
        // yet.
        const int UnmeasuredColumns = 40;

        // The chat box's font, from ScreenRoot's Chat Font slot.  Null is
        // Unity's own.
        public static Font Font;

        VisualElement box;
        ScrollView lines;
        TextField input;

        // The panel the box is on, for the screen-wide keys, kept so they
        // can be let go when the box leaves it.
        IPanel panel;

        // Ten letters, never shown, to measure how wide a letter is in
        // the chat's font and size.
        Label ruler;

        public override WidgetInfo Info { get { return info; } }

        public override void Build(VisualElement box)
        {
            this.box = box;

            var header = new Label("Chat");
            header.AddToClassList("chat-header");

            lines = new ScrollView(ScrollViewMode.Vertical);
            lines.AddToClassList("chat-lines");
            lines.horizontalScrollerVisibility = ScrollerVisibility.Hidden;

            input = new TextField();
            input.maxLength = LongestLine;
            // A half-typed line is kept as it is when the keys come back
            // to the field, not selected whole to be typed over.
            input.selectAllOnFocus = false;
            input.AddToClassList("chat-input");

            // Caught on the way down (TrickleDown), before the text field
            // does anything of its own with Enter.
            input.RegisterCallback<KeyDownEvent>(KeyDown, TrickleDown.TrickleDown);
            input.RegisterCallback<FocusOutEvent>(LostKeys);

            ruler = new Label("MMMMMMMMMM");
            ruler.AddToClassList("chat-line");
            ruler.AddToClassList("chat-ruler");
            ruler.pickingMode = PickingMode.Ignore;

            box.Add(header);
            box.Add(lines);
            box.Add(input);
            box.Add(ruler);
            UseFont(box);

            // The HUD is built again on every switch of screens, so this
            // copy stops listening when its box goes.
            Session.ChatLine += ServerSaid;
            Session.WhoAnswered += DrawWho;
            box.RegisterCallback<AttachToPanelEvent>(Arrived);
            box.RegisterCallback<DetachFromPanelEvent>(Gone);

            // Straight to typing once the HUD is up.
            input.schedule.Execute(() => input.Focus());
        }

        // ScreenRoot's Chat Font on every piece of text in here.  Set on
        // each one, since Unity's theme gives the text field a font of its
        // own, and only a style on the element itself beats it.
        public static void UseFont(VisualElement within)
        {
            StyleFontDefinition font = Font != null
                ? new StyleFontDefinition(Font)
                : new StyleFontDefinition(StyleKeyword.Null);
            within.Query<TextElement>().ForEach(text => text.style.unityFontDefinition = font);
        }

        // Keys pressed anywhere on the screen reach the panel's root first,
        // whatever has the focus, so that's where Enter and "/" are watched
        // for.
        void Arrived(AttachToPanelEvent e)
        {
            panel = e.destinationPanel;
            panel.visualTree.RegisterCallback<KeyDownEvent>(KeyAnywhere, TrickleDown.TrickleDown);
        }

        void Gone(DetachFromPanelEvent e)
        {
            Session.ChatLine -= ServerSaid;
            Session.WhoAnswered -= DrawWho;
            if (panel != null)
                panel.visualTree.UnregisterCallback<KeyDownEvent>(KeyAnywhere, TrickleDown.TrickleDown);
            panel = null;
            box.UnregisterCallback<AttachToPanelEvent>(Arrived);
            box.UnregisterCallback<DetachFromPanelEvent>(Gone);
        }

        // The field lost the keys to nothing (a click on the scene): the
        // game takes them, a frame later, since the focus is still changing
        // hands while this runs.  Left to nobody, Unity would hand them
        // back to the field on the next key (GameFocus.cs).
        void LostKeys(FocusOutEvent e)
        {
            if (e.relatedTarget != null)
                return;
            box.schedule.Execute(() =>
            {
                if (panel != null && panel.focusController.focusedElement == null)
                    GameFocus.Take();
            });
        }

        // With the field not focused, Enter brings the keys to it, and "/"
        // does the same and goes in as the first character (Jacob: "pressing
        // enter or typing / immediately brings focus up to the chat window
        // and starts typing that into the input").  The key is used up here
        // so nothing else on the screen sees it.  With the field focused,
        // its own KeyDown has Enter and the "/" is just typed.
        void KeyAnywhere(KeyDownEvent e)
        {
            if (e.target is VisualElement target && (target == input || input.Contains(target)))
                return;

            bool enter = e.keyCode == KeyCode.Return || e.keyCode == KeyCode.KeypadEnter;
            bool slash = e.character == '/';
            if (!enter && !slash)
                return;

            if (slash)
                input.value = input.value + "/";
            input.Focus();
            // The cursor goes to the end once the field has the keys.
            int end = input.value.Length;
            input.schedule.Execute(() => input.SelectRange(end, end));
            e.StopPropagation();
        }

        void KeyDown(KeyDownEvent e)
        {
            // Escape hands the keys to the game, the line left as it is for
            // Enter to come back to.
            if (e.keyCode == KeyCode.Escape)
            {
                GameFocus.Take();
                e.StopPropagation();
                return;
            }
            if (e.keyCode != KeyCode.Return && e.keyCode != KeyCode.KeypadEnter)
                return;

            string line = input.value;
            input.value = "";
            e.StopPropagation();
            if (line.Trim() == "")
                return;

            AddLine(">" + line, "chat-line-mine");
            Session.SendLine(line);
        }

        void ServerSaid(string line)
        {
            AddLine(line, "chat-line-server");
        }

        void DrawWho(WhoAnswer who)
        {
            foreach (string line in WhoBox.Draw(who, Columns()))
                AddLine(line, "chat-line-server");
        }

        // How many letters fit across the lines, in the chat's font: the
        // lines' width over one letter's.  One fewer than fit, so a line
        // drawn to the full width never wraps on a rounding.
        int Columns()
        {
            float across = lines.contentViewport.resolvedStyle.width;
            float ten = ruler.MeasureTextSize(ruler.text, 0f, VisualElement.MeasureMode.Undefined, 0f,
                                              VisualElement.MeasureMode.Undefined).x;
            if (float.IsNaN(across) || across <= 0f || float.IsNaN(ten) || ten <= 0f)
                return UnmeasuredColumns;
            return Mathf.Max(1, Mathf.FloorToInt(across / (ten / 10f)) - 1);
        }

        void AddLine(string text, string kind)
        {
            var line = new Label(text);
            // A player's words are shown as they are: a <b> in the chat is
            // two letters and a bracket, not bold.
            line.enableRichText = false;
            line.AddToClassList("chat-line");
            line.AddToClassList(kind);
            UseFont(line);
            lines.Add(line);

            while (lines.childCount > KeptLines)
                lines.RemoveAt(0);

            // The new line has no size until the next layout pass, so the
            // scroll down waits for it.
            lines.schedule.Execute(() => lines.ScrollTo(line));
        }
    }
}
