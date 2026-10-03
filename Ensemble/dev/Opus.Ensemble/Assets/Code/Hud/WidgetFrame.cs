// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/WidgetFrame.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// What makes a widget on the HUD movable, resizable and lockable (Jacob,
// 2026-10-03).  A widget never moves or sizes its own box, so this sits
// around it: HudBuilder gives every box on the HUD one.  Unlocked, which is
// how everything starts, the border flashes orange, red and yellow, the
// inside drags the widget about, and a widget whose catalog entry says
// resizable (only chat, for now) has edges and corners that drag its size,
// "like windows window".  The pointer changes the moment the mouse is
// where a drag can start, and goes back to the normal one once it isn't.
// A right-click opens the widget's menu (WidgetMenu), locked or not.  Every
// change goes back through HudBuilder, which has the character's layout
// written.

using System;
using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class WidgetFrame
    {
        // The edges a drag holds.  None is a move.
        [Flags]
        enum Edge
        {
            None = 0,
            Left = 1,
            Right = 2,
            Top = 4,
            Bottom = 8,
        }

        // How far in from the border an edge can be gripped, and how far
        // along from a corner the corner is, in the layout's pixels.
        const float EdgeBand = 10f;
        const float CornerBand = 24f;

        // How often the unlocked border changes colour, in milliseconds.
        const long FlashEvery = 300;
        static readonly string[] FlashColours =
        {
            "widget-flash-orange", "widget-flash-red", "widget-flash-yellow",
        };

        // A text size picked on the menu's slider is saved this long after
        // the slider stops, in milliseconds, not on every step of it.
        const long SaveFontAfter = 500;

        // A widget marks a piece of itself with this class when the mouse
        // there is its own (chat's field to type in, its scroll bar), so
        // the inside doesn't drag from it.
        public const string KeepsMouse = "widget-keeps-mouse";

        readonly VisualElement box;
        readonly PlacedWidget placed;
        readonly VisualElement screen;
        readonly Action changed;

        IVisualElementScheduledItem flashing;
        int flash;
        IVisualElementScheduledItem fontSave;

        // The drag under way, if there is one: which pointer, which edges,
        // and where the pointer and the box were when it started.
        bool dragging;
        int dragPointer;
        Edge grip;
        Vector2 startPointer;
        Vector2 startTopLeft;
        Vector2 startSize;
        bool moved;

        // Where the drag has put the box so far.  Kept here, since Unity
        // works out the box's layout a frame later.
        Vector2 dragTopLeft;
        Vector2 dragSize;

        public WidgetFrame(VisualElement box, PlacedWidget placed, VisualElement screen, Action changed)
        {
            this.box = box;
            this.placed = placed;
            this.screen = screen;
            this.changed = changed;

            // Caught on the way down (TrickleDown), so a drag that starts on
            // a piece of the widget is the frame's before the piece does
            // anything with it.
            box.RegisterCallback<PointerDownEvent>(PointerDown, TrickleDown.TrickleDown);
            box.RegisterCallback<PointerMoveEvent>(PointerMove, TrickleDown.TrickleDown);
            box.RegisterCallback<PointerUpEvent>(PointerUp, TrickleDown.TrickleDown);
            box.RegisterCallback<PointerLeaveEvent>(PointerLeft);
            box.RegisterCallback<PointerCaptureOutEvent>(CaptureLost);

            ShowLock();
        }

        public PlacedWidget Placed
        {
            get { return placed; }
        }

        // From the menu.  Saved at once.
        public void SetLocked(bool locked)
        {
            placed.Locked = locked;
            ShowLock();
            changed();
        }

        // From the menu's slider: shown at once, saved once the slider has
        // been still for a moment.
        public void SetFontSize(int pixels)
        {
            placed.FontSize = pixels;
            placed.Widget.UseFontSize(pixels);
            if (fontSave != null)
                fontSave.Pause();
            fontSave = box.schedule.Execute(changed).StartingIn(SaveFontAfter);
        }

        // The flashing border while it's unlocked; none once it's locked.
        void ShowLock()
        {
            if (!placed.Locked)
            {
                if (flashing == null)
                    flashing = box.schedule.Execute(Flash).Every(FlashEvery);
                else
                    flashing.Resume();
                return;
            }

            if (flashing != null)
                flashing.Pause();
            foreach (string colour in FlashColours)
                box.RemoveFromClassList(colour);
        }

        void Flash()
        {
            foreach (string colour in FlashColours)
                box.RemoveFromClassList(colour);
            box.AddToClassList(FlashColours[flash]);
            flash = (flash + 1) % FlashColours.Length;
        }

        void PointerDown(PointerDownEvent e)
        {
            // The right button: the menu, locked or not.
            if (e.button == 1)
            {
                e.StopPropagation();
                WidgetMenu.Open(screen, this, e.position);
                return;
            }
            if (e.button != 0 || placed.Locked || dragging)
                return;

            Edge edge = EdgeAt(e.localPosition);
            if (edge == Edge.None && !CanMoveFrom(e.target as VisualElement))
                return;

            dragging = true;
            dragPointer = e.pointerId;
            grip = edge;
            moved = false;
            startPointer = e.position;
            startTopLeft = box.layout.position;
            startSize = box.layout.size;
            dragTopLeft = startTopLeft;
            dragSize = startSize;

            // The box keeps hearing the pointer while the button's down,
            // wherever it goes.
            box.CapturePointer(e.pointerId);
            HudPointer.Show(edge == Edge.None ? PointerKind.Move : PointerKind.Grip);
            e.StopPropagation();
        }

        void PointerMove(PointerMoveEvent e)
        {
            if (dragging)
            {
                if (e.pointerId != dragPointer)
                    return;
                Drag((Vector2)e.position - startPointer);
                e.StopPropagation();
                return;
            }
            HudPointer.Show(PointerAt(e.localPosition, e.target as VisualElement));
        }

        // Letting go: whatever was dragged is kept, and the pointer is the
        // one for where the mouse is now.
        void PointerUp(PointerUpEvent e)
        {
            if (!dragging || e.pointerId != dragPointer)
                return;
            Finish();
            e.StopPropagation();
            HudPointer.Show(PointerAt(e.localPosition, box));
        }

        void PointerLeft(PointerLeaveEvent e)
        {
            if (!dragging)
                HudPointer.Show(PointerKind.Normal);
        }

        // The pointer was taken away mid-drag (the window lost focus, say):
        // the drag ends where it got to.
        void CaptureLost(PointerCaptureOutEvent e)
        {
            if (!dragging)
                return;
            Finish();
            HudPointer.Show(PointerKind.Normal);
        }

        void Finish()
        {
            dragging = false;
            if (box.HasPointerCapture(dragPointer))
                box.ReleasePointer(dragPointer);
            if (!moved)
                return;

            // Kept as the layout keeps it: the same anchor, the offset that
            // puts the box where it was left.
            placed.Size = dragSize;
            placed.Offset = Anchors.OffsetFor(placed.Anchor, dragTopLeft, dragSize, ScreenSize());
            changed();
        }

        // The box where the drag puts it, never smaller than its smallest
        // and never off the screen.  A grip on the left or top edge keeps
        // the opposite edge where it was.
        void Drag(Vector2 delta)
        {
            Vector2 screenSize = ScreenSize();
            Vector2 topLeft = startTopLeft;
            Vector2 size = startSize;

            if (grip == Edge.None)
            {
                topLeft = LayoutChecker.KeepOnScreen(startTopLeft + delta, size, screenSize);
            }
            else
            {
                Vector2 smallest = placed.Smallest;
                if ((grip & Edge.Right) != 0)
                    size.x = Mathf.Clamp(startSize.x + delta.x, smallest.x,
                                         Mathf.Max(smallest.x, screenSize.x - startTopLeft.x));
                if ((grip & Edge.Left) != 0)
                {
                    float right = startTopLeft.x + startSize.x;
                    topLeft.x = Mathf.Clamp(startTopLeft.x + delta.x, 0f, Mathf.Max(0f, right - smallest.x));
                    size.x = right - topLeft.x;
                }
                if ((grip & Edge.Bottom) != 0)
                    size.y = Mathf.Clamp(startSize.y + delta.y, smallest.y,
                                         Mathf.Max(smallest.y, screenSize.y - startTopLeft.y));
                if ((grip & Edge.Top) != 0)
                {
                    float bottom = startTopLeft.y + startSize.y;
                    topLeft.y = Mathf.Clamp(startTopLeft.y + delta.y, 0f, Mathf.Max(0f, bottom - smallest.y));
                    size.y = bottom - topLeft.y;
                }
            }

            box.style.left = topLeft.x;
            box.style.top = topLeft.y;
            box.style.width = size.x;
            box.style.height = size.y;
            dragTopLeft = topLeft;
            dragSize = size;
            if (topLeft != startTopLeft || size != startSize)
                moved = true;
        }

        // The pointer for a spot inside the box: the grip on an edge it can
        // be resized by, the move anywhere else a drag can start, and the
        // normal one when it's locked or the spot is the widget's own.
        PointerKind PointerAt(Vector2 local, VisualElement under)
        {
            if (placed.Locked || !Inside(local))
                return PointerKind.Normal;
            if (EdgeAt(local) != Edge.None)
                return PointerKind.Grip;
            if (CanMoveFrom(under))
                return PointerKind.Move;
            return PointerKind.Normal;
        }

        // Which edges a spot inside the box grips, none for a widget that
        // can't be resized.  Near a corner along an edge counts as the
        // corner, so the corners aren't a ten-pixel square to hunt for.
        Edge EdgeAt(Vector2 local)
        {
            if (!placed.Widget.Info.Resizable || !Inside(local))
                return Edge.None;

            float width = box.layout.width;
            float height = box.layout.height;
            bool left = local.x < EdgeBand;
            bool right = local.x > width - EdgeBand;
            bool top = local.y < EdgeBand;
            bool bottom = local.y > height - EdgeBand;

            if (top || bottom)
            {
                if (local.x < CornerBand)
                    left = true;
                else if (local.x > width - CornerBand)
                    right = true;
            }
            if (left || right)
            {
                if (local.y < CornerBand)
                    top = true;
                else if (local.y > height - CornerBand)
                    bottom = true;
            }

            Edge edge = Edge.None;
            if (left)
                edge |= Edge.Left;
            if (right)
                edge |= Edge.Right;
            if (top)
                edge |= Edge.Top;
            if (bottom)
                edge |= Edge.Bottom;
            return edge;
        }

        bool Inside(Vector2 local)
        {
            return local.x >= 0f && local.y >= 0f && local.x <= box.layout.width && local.y <= box.layout.height;
        }

        // Whether the inside drags from here: anywhere in the box but a piece
        // the widget keeps the mouse for.
        bool CanMoveFrom(VisualElement under)
        {
            for (VisualElement e = under; e != null && e != box; e = e.parent)
            {
                if (e.ClassListContains(KeepsMouse))
                    return false;
            }
            return true;
        }

        // The screen in the layout's own pixels: the layer the box is in
        // covers all of it.
        Vector2 ScreenSize()
        {
            return box.parent != null ? box.parent.layout.size : box.layout.size;
        }
    }
}
