// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/HudPointer.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The mouse pointer over the HUD: the normal one, or one of Jacob's two from
// his pointer pack the moment the mouse is where a drag can start (Jacob,
// 2026-10-03), one for moving a widget and one for gripping its edge.  The
// pictures are ScreenRoot's Move Pointer and Grip Pointer slots, since the
// pack is purchased and never committed.  An empty slot keeps the normal
// pointer, and moving and resizing work the same.
//
// The picture goes to the operating system as its own pointer (Unity's
// CursorMode.Auto, the same call UI Toolkit makes), so it never trails the
// mouse.  Windows holds that to about 32 x 32 and Linux took 128 x 128 (the
// forums and Unity's issue tracker); the 64 x 64s are the ones to use.

using UnityEngine;

namespace Opus.Hud
{
    public enum PointerKind
    {
        Normal,
        Move,
        Grip,
    }

    public static class HudPointer
    {
        // Set by ScreenRoot from its slots.  Null is the normal pointer.
        public static Texture2D MovePicture;
        public static Vector2 MoveHotspot;
        public static Texture2D GripPicture;
        public static Vector2 GripHotspot;

        static PointerKind showing = PointerKind.Normal;

        // A picture that can't be a pointer is said once, not on every move
        // of the mouse.
        static Texture2D warnedAbout;

        // The pointer for this spot.  Nothing happens when it's the one
        // already showing.
        public static void Show(PointerKind kind)
        {
            if (kind == showing)
                return;
            showing = kind;
            Apply();
        }

        // The slots changed in the Inspector: the pointer showing is drawn
        // again with the new picture.
        public static void Refresh()
        {
            Apply();
        }

        static void Apply()
        {
            Texture2D picture = null;
            Vector2 hotspot = Vector2.zero;
            string slot = "";
            if (showing == PointerKind.Move)
            {
                picture = MovePicture;
                hotspot = MoveHotspot;
                slot = "Move Pointer";
            }
            else if (showing == PointerKind.Grip)
            {
                picture = GripPicture;
                hotspot = GripHotspot;
                slot = "Grip Pointer";
            }

            // Unity can only make a pointer out of a picture it can read,
            // which is what Texture Type: Cursor sets up.
            if (picture != null && !picture.isReadable)
            {
                if (warnedAbout != picture)
                {
                    warnedAbout = picture;
                    Debug.LogWarning("HUD: ScreenRoot's " + slot + " (" + picture.name + ") can't be a pointer.  "
                        + "Click it in the Project window, set Texture Type to Cursor in the Inspector, and Apply.");
                }
                picture = null;
            }

            if (picture == null)
            {
                UnityEngine.Cursor.SetCursor(null, Vector2.zero, CursorMode.Auto);
                return;
            }

            // The hotspot is the pixel that does the clicking, and it has to
            // be on the picture.
            hotspot = new Vector2(Mathf.Clamp(hotspot.x, 0f, picture.width - 1),
                                  Mathf.Clamp(hotspot.y, 0f, picture.height - 1));
            UnityEngine.Cursor.SetCursor(picture, hotspot, CursorMode.Auto);
        }
    }
}
