// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/ChatWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The chat window: a "Chat" header, the lines, and a field on the bottom
// row to type in.  Enter sends what's typed to the server as it was typed,
// and echoes it as ">/chat hello" in yellow; what the server says is white.
// The box stops taking keys at 300 characters.  The font is ScreenRoot's
// Chat Font slot, a monospaced one, so /who's box lines up.  EverQuest's
// keys (Jacob, 2026-10-02): Enter or a "/" while the game has the keys
// (GameFocus) brings them to the field, the "/" already typed; Enter in
// the field sends the line and hands the keys back; so do Escape and a
// click away.  The size of its lines and its field is the player's to pick
// on its right-click menu, 18 to 42 (Jacob, 2026-10-03: "just the contents
// of the chat window not the header"); the header stays as hud.uss has it.

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
            SmallestFont = 18,
            BiggestFont = 42,
            // hud.uss's size for the lines and the field; the two change
            // together.
            DefaultFont = 24,
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

        // The size the player picked for the lines and the field, or 0 for
        // hud.uss's own.
        int fontSize;

        VisualElement box;
        ScrollView lines;
        TextField input;

        // Ten letters, never shown, to measure how wide a letter is in
        // the chat's font and size.
        Label ruler;

        // The frames the field last took the keys on, and last gave them
        // back on.  One Enter is two events to Unity (the key, then its
        // "submit"), and the focus moves between them: the Enter that
        // brought the keys mustn't also send the line, and the Enter that
        // sent the line mustn't also bring the keys back.
        int tookKeysFrame = -1;
        int gaveKeysFrame = -1;

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
            // The mouse in the field is for typing, not for dragging the
            // window about (WidgetFrame).
            input.AddToClassList(WidgetFrame.KeepsMouse);
            lines.verticalScroller.AddToClassList(WidgetFrame.KeepsMouse);

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
            GameFocus.EnterPressed += EnterFromGame;
            GameFocus.SlashPressed += TakeKeysWithSlash;
            box.RegisterCallback<DetachFromPanelEvent>(Gone);

            // Straight to typing once the HUD is up.
            input.schedule.Execute(() => input.Focus());
        }

        // The size the player picked, on every line, the field, and the
        // ruler /who's box is measured with.  Not on the header.
        public override void UseFontSize(int pixels)
        {
            fontSize = pixels;
            box.Query<Label>(className: "chat-line").ForEach(line => line.style.fontSize = pixels);
            input.style.fontSize = pixels;

            // The lines got taller or shorter: back down to the newest.
            lines.schedule.Execute(() =>
            {
                if (lines.childCount > 0)
                    lines.ScrollTo(lines[lines.childCount - 1]);
            });
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

        void Gone(DetachFromPanelEvent e)
        {
            Session.ChatLine -= ServerSaid;
            Session.WhoAnswered -= DrawWho;
            GameFocus.EnterPressed -= EnterFromGame;
            GameFocus.SlashPressed -= TakeKeysWithSlash;
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
                if (box.panel != null && box.panel.focusController.focusedElement == null)
                    GameFocus.Take();
            });
        }

        // Enter while the game had the keys, unless it's the one that just
        // gave them away (above).
        void EnterFromGame()
        {
            if (Time.frameCount == gaveKeysFrame)
                return;
            TakeKeys();
        }

        // "/" pressed while the game had the keys: it's the first character
        // typed (Jacob: "pressing enter or typing / immediately brings
        // focus up to the chat window and starts typing that into the
        // input").
        void TakeKeysWithSlash()
        {
            input.value = input.value + "/";
            TakeKeys();
        }

        // The field gets the keys, with the cursor at the end of whatever's
        // in it once it has them.
        void TakeKeys()
        {
            tookKeysFrame = Time.frameCount;
            input.Focus();
            int end = input.value.Length;
            input.schedule.Execute(() => input.SelectRange(end, end));
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
            if (Time.frameCount == tookKeysFrame)
            {
                // The Enter that brought the keys here, not one to send.
                e.StopPropagation();
                return;
            }

            // Enter sends the line and hands the keys back to the game, as
            // EverQuest does (Jacob, 2026-10-02: "if you press enter into
            // the text chat it needs to switch to the other input window");
            // an empty Enter only hands them back.
            string line = input.value;
            input.value = "";
            e.StopPropagation();
            gaveKeysFrame = Time.frameCount;
            GameFocus.Take();
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
            if (fontSize != 0)
                line.style.fontSize = fontSize;
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
