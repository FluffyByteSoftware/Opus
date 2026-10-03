// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/HudBuilder.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Turns the checked widgets into a screen (the HUD, the start screen): a stack of
// layers, a box per widget in its layer, and each box placed from its
// anchor.  On the HUD each box gets a WidgetFrame, so the player can move,
// resize and lock it, and Changed says when they have.

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class HudBuilder
    {
        VisualElement screen;
        readonly List<PlacedWidget> placed = new List<PlacedWidget>();
        readonly List<VisualElement> boxes = new List<VisualElement>();
        readonly List<WidgetFrame> frames = new List<WidgetFrame>();

        // The player moved, resized or locked a widget, or picked its text
        // size.  Placed has where everything is now.
        public event Action Changed;

        // The screen this builder built last, or null.
        public VisualElement Screen { get { return screen; } }

        // The widgets on it, as the player has them now.
        public List<PlacedWidget> Placed { get { return placed; } }

        // Build a screen under root, with its own style sheet.  The sheet
        // goes on the screen, not the root, so the HUD's look and the
        // start screen's never meet.  Anything this builder built before
        // goes first.  The player can move the widgets on an editable
        // screen, which is only ever the HUD.
        public void Build(VisualElement root, List<PlacedWidget> widgets, string name, StyleSheet styleSheet,
                          bool editable)
        {
            Clear();

            screen = FullScreen("screen");
            screen.AddToClassList("screen-" + name);
            if (styleSheet != null)
                screen.styleSheets.Add(styleSheet);
            root.Add(screen);

            // UI Toolkit has no z-index: what's later in the tree draws on
            // top.  So each layer in use is a full-screen container, added
            // lowest first.  They let the mouse through (pickingMode Ignore),
            // so the empty space between widgets doesn't block clicks meant
            // for the game.
            var layers = new SortedDictionary<int, VisualElement>();
            foreach (PlacedWidget p in widgets)
            {
                if (!layers.ContainsKey(p.Layer))
                    layers[p.Layer] = FullScreen("screen-layer");
            }
            foreach (VisualElement layer in layers.Values)
                screen.Add(layer);

            // Within a layer, the file's order is the drawing order.
            foreach (PlacedWidget p in widgets)
            {
                var box = new VisualElement();
                box.AddToClassList("widget");
                box.AddToClassList("widget-" + p.Widget.Info.Id);
                box.style.position = Position.Absolute;
                layers[p.Layer].Add(box);
                p.Widget.Build(box);
                if (p.FontSize != 0)
                    p.Widget.UseFontSize(p.FontSize);
                if (editable && !p.Widget.Info.FillsScreen)
                    frames.Add(new WidgetFrame(box, p, screen, SayChanged));

                placed.Add(p);
                boxes.Add(box);
            }

            // The boxes are placed once UI Toolkit knows how big the screen
            // is, and again every time that changes (a window resized, a
            // monitor switched), so an anchored widget stays with its corner.
            screen.RegisterCallback<GeometryChangedEvent>(ScreenChanged);
        }

        public void Clear()
        {
            WidgetMenu.Close();
            HudPointer.Show(PointerKind.Normal);
            if (screen != null)
            {
                screen.UnregisterCallback<GeometryChangedEvent>(ScreenChanged);
                screen.RemoveFromHierarchy();
                screen = null;
            }
            placed.Clear();
            boxes.Clear();
            frames.Clear();
        }

        void SayChanged()
        {
            if (Changed != null)
                Changed();
        }

        void ScreenChanged(GeometryChangedEvent e)
        {
            // The screen in the layout's own pixels.  The panel's scaled to
            // the layout's reference, so this is at least the reference both
            // ways, and bigger on a screen of another shape.
            var size = new Vector2(e.newRect.width, e.newRect.height);
            if (size.x <= 0f || size.y <= 0f)
                return;

            for (int i = 0; i < boxes.Count; i++)
                Place(boxes[i], placed[i], size);
        }

        static void Place(VisualElement box, PlacedWidget p, Vector2 screen)
        {
            // A background and the like: the whole real screen, which on a
            // screen of another shape is more than the layout's reference.
            if (p.Widget.Info.FillsScreen)
            {
                box.style.left = 0f;
                box.style.top = 0f;
                box.style.width = screen.x;
                box.style.height = screen.y;
                return;
            }

            // LayoutChecker already made everything fit the reference, and
            // the screen is never smaller.  This is only a guard, so it's
            // quiet: it would run again on every resize.
            Vector2 size = Vector2.Min(p.Size, screen);
            Vector2 topLeft = Anchors.TopLeftOf(p.Anchor, p.Offset, size, screen);
            topLeft = LayoutChecker.KeepOnScreen(topLeft, size, screen);

            box.style.left = topLeft.x;
            box.style.top = topLeft.y;
            box.style.width = size.x;
            box.style.height = size.y;
        }

        static VisualElement FullScreen(string className)
        {
            var element = new VisualElement();
            element.AddToClassList(className);
            element.pickingMode = PickingMode.Ignore;
            element.style.position = Position.Absolute;
            element.style.left = 0;
            element.style.top = 0;
            element.style.right = 0;
            element.style.bottom = 0;
            return element;
        }
    }
}
