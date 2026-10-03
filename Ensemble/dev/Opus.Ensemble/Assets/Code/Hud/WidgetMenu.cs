// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/WidgetMenu.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The right-click menu on a widget on the HUD (Jacob, 2026-10-03): LOCK or
// UNLOCK, and for a widget with a text size to pick (chat) a slider from its
// smallest to its biggest ("on a sliding scale ... for now we'll make this a
// px measurement but it might be an algorithm later").  Unity's own
// right-click menu only works in the editor, so this one is ours: a box at
// the mouse over a see-through cover across the whole screen, so a click
// anywhere else closes it and goes no further.  Escape closes it too.  One
// menu at a time.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public static class WidgetMenu
    {
        static VisualElement cover;
        static VisualElement menu;

        // The screen it's on, which hears Escape for it.
        static VisualElement on;

        public static void Open(VisualElement screen, WidgetFrame frame, Vector2 at)
        {
            Close();
            WidgetInfo info = frame.Placed.Widget.Info;

            cover = new VisualElement();
            cover.AddToClassList("widget-menu-cover");
            cover.style.position = Position.Absolute;
            cover.style.left = 0;
            cover.style.top = 0;
            cover.style.right = 0;
            cover.style.bottom = 0;
            cover.RegisterCallback<PointerDownEvent>(e =>
            {
                // Only a click on the cover itself, not one on the menu that
                // came up through it.
                if (e.target == cover)
                {
                    e.StopPropagation();
                    Close();
                }
            });

            menu = new VisualElement();
            menu.AddToClassList("widget-menu");
            menu.style.position = Position.Absolute;
            menu.style.left = at.x;
            menu.style.top = at.y;

            var title = new Label(info.DisplayName.ToUpperInvariant());
            title.AddToClassList("widget-menu-title");
            menu.Add(title);

            var lockButton = new Button(() =>
            {
                frame.SetLocked(!frame.Placed.Locked);
                Close();
            });
            lockButton.text = frame.Placed.Locked ? "UNLOCK" : "LOCK";
            lockButton.AddToClassList("widget-menu-button");
            menu.Add(lockButton);

            if (info.HasFontSize)
            {
                int now = frame.Placed.FontSize != 0 ? frame.Placed.FontSize : info.DefaultFont;

                var row = new VisualElement();
                row.AddToClassList("widget-menu-row");
                var name = new Label("FONT SIZE");
                name.AddToClassList("widget-menu-label");
                var value = new Label(now + " px");
                value.AddToClassList("widget-menu-label");
                row.Add(name);
                row.Add(value);
                menu.Add(row);

                var slider = new SliderInt(info.SmallestFont, info.BiggestFont);
                slider.value = now;
                slider.AddToClassList("widget-menu-slider");
                slider.RegisterValueChangedCallback(e =>
                {
                    value.text = e.newValue + " px";
                    frame.SetFontSize(e.newValue);
                });
                menu.Add(slider);
            }

            // Kept on the screen once it knows its own size.
            menu.RegisterCallback<GeometryChangedEvent>(KeepOnScreen);

            ChatWidget.UseFont(menu);
            cover.Add(menu);
            screen.Add(cover);

            on = screen;
            on.RegisterCallback<KeyDownEvent>(EscapeCloses, TrickleDown.TrickleDown);
        }

        // Closes the menu if it's open, and the game takes the keys back
        // (the button clicked had them; GameFocus.cs says why they mustn't
        // be left to nothing).
        public static void Close()
        {
            if (cover == null)
                return;
            on.UnregisterCallback<KeyDownEvent>(EscapeCloses, TrickleDown.TrickleDown);
            cover.RemoveFromHierarchy();
            cover = null;
            menu = null;
            on = null;
            GameFocus.Take();
        }

        static void EscapeCloses(KeyDownEvent e)
        {
            if (e.keyCode != KeyCode.Escape)
                return;
            e.StopPropagation();
            Close();
        }

        // Moved back on when it was opened too near the right or the bottom.
        // Setting it changes its geometry once more, and then it's on.
        static void KeepOnScreen(GeometryChangedEvent e)
        {
            if (menu == null || cover == null)
                return;
            Vector2 topLeft = menu.layout.position;
            Vector2 onScreen = LayoutChecker.KeepOnScreen(topLeft, menu.layout.size, cover.layout.size);
            if (onScreen == topLeft)
                return;
            menu.style.left = onScreen.x;
            menu.style.top = onScreen.y;
        }
    }
}
