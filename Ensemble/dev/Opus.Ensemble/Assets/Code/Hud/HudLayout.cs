// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/HudLayout.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// A layout file, shaped the way its JSON is, and the nine anchors.
// HUD_FORMATS.md is the contract; these classes are written from it.

using System;
using System.Collections.Generic;
using UnityEngine;

namespace Opus.Hud
{
    // The field names match the JSON's exactly, since that's how Unity's
    // JsonUtility reads them.  A field missing from the file keeps the value
    // it starts with here, which is why the strings start empty rather than
    // null.
    [Serializable]
    public class LayoutFile
    {
        public string format = "";
        public int version;
        public string screen = "";
        public string name = "";
        public PixelSize reference = new PixelSize();
        public List<LayoutEntry> widgets = new List<LayoutEntry>();
    }

    [Serializable]
    public class LayoutEntry
    {
        public string id = "";
        public string anchor = "";
        public Offset offset = new Offset();
        public PixelSize size = new PixelSize();
        public int layer;

        // Version 2.  Whether the player locked it where it is (false, the
        // start, lets it be moved), and the size its text is shown at in
        // the layout's pixels, for a widget that has one to pick (0 is the
        // widget's own).
        public bool locked;
        public int fontSize;
    }

    [Serializable]
    public class Offset
    {
        public float x;
        public float y;
    }

    [Serializable]
    public class PixelSize
    {
        public float width;
        public float height;
    }

    public enum Anchor
    {
        TopLeft, Top, TopRight,
        Left, Center, Right,
        BottomLeft, Bottom, BottomRight,
    }

    public static class Anchors
    {
        // The anchor named in a file.  JsonUtility reads an enum as a number,
        // and Enum.TryParse would take "3" as well as "Left", so the names are
        // read here, one by one, and nothing else gets in.
        public static bool TryRead(string text, out Anchor anchor)
        {
            switch (text)
            {
                case "TopLeft":     anchor = Anchor.TopLeft;     return true;
                case "Top":         anchor = Anchor.Top;         return true;
                case "TopRight":    anchor = Anchor.TopRight;    return true;
                case "Left":        anchor = Anchor.Left;        return true;
                case "Center":      anchor = Anchor.Center;      return true;
                case "Right":       anchor = Anchor.Right;       return true;
                case "BottomLeft":  anchor = Anchor.BottomLeft;  return true;
                case "Bottom":      anchor = Anchor.Bottom;      return true;
                case "BottomRight": anchor = Anchor.BottomRight; return true;
                default:            anchor = Anchor.TopLeft;     return false;
            }
        }

        // Where the anchor sits, as a fraction across and down: 0 is the left
        // (or top), 0.5 the middle, 1 the right (or bottom).  The same
        // fraction is the point on the screen and the point on the widget,
        // which is what makes one sum place all nine.
        public static Vector2 Fraction(Anchor anchor)
        {
            switch (anchor)
            {
                case Anchor.TopLeft:     return new Vector2(0f, 0f);
                case Anchor.Top:         return new Vector2(0.5f, 0f);
                case Anchor.TopRight:    return new Vector2(1f, 0f);
                case Anchor.Left:        return new Vector2(0f, 0.5f);
                case Anchor.Center:      return new Vector2(0.5f, 0.5f);
                case Anchor.Right:       return new Vector2(1f, 0.5f);
                case Anchor.BottomLeft:  return new Vector2(0f, 1f);
                case Anchor.Bottom:      return new Vector2(0.5f, 1f);
                default:                 return new Vector2(1f, 1f);
            }
        }

        // The widget's top-left corner on a screen this big.  The widget's
        // anchor point sits on the screen's, moved by the offset.
        public static Vector2 TopLeftOf(Anchor anchor, Vector2 offset, Vector2 size, Vector2 screen)
        {
            Vector2 f = Fraction(anchor);
            return new Vector2(f.x * screen.x + offset.x - f.x * size.x,
                               f.y * screen.y + offset.y - f.y * size.y);
        }

        // The offset that puts the widget's top-left corner here: the sum
        // above, the other way round.
        public static Vector2 OffsetFor(Anchor anchor, Vector2 topLeft, Vector2 size, Vector2 screen)
        {
            Vector2 f = Fraction(anchor);
            return new Vector2(topLeft.x - f.x * screen.x + f.x * size.x,
                               topLeft.y - f.y * screen.y + f.y * size.y);
        }
    }
}
