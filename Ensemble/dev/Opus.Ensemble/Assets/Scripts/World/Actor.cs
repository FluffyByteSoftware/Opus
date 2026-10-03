// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/World/Actor.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// On top of a FluffyGameObject for anything Living: a player's character,
// and the NPCs once there are some.  For now it's the short name over its
// head (Jacob, 2026-10-03), turned to face the camera every frame so it
// reads square on, whichever way the Actor faces.  Unity's own TextMesh, so
// no package.

using UnityEngine;

namespace Opus.World
{
    public class Actor : MonoBehaviour
    {
        TextMesh label;

        // The short name, `above` blocks over the Actor's feet, in `font`
        // (Unity's own when it's empty), with a capital `size` blocks tall.
        public void Name(string shortName, Font font, float size, float above)
        {
            if (font == null)
                font = Resources.GetBuiltinResource<Font>("LegacyRuntime.ttf");

            var holder = new GameObject("Name");
            holder.transform.SetParent(transform, false);
            holder.transform.localPosition = new Vector3(0, above, 0);

            label = holder.AddComponent<TextMesh>();
            label.text = shortName ?? "";
            label.font = font;
            label.fontSize = 64;
            // A guess at TextMesh's scale, not yet seen on screen: a line
            // is about fontSize x characterSize / 10 blocks tall, and a
            // capital about 0.7 of that.  WorldObjectsView's Name Size
            // slot puts it right if it's off.
            label.characterSize = size / (label.fontSize * 0.7f) * 10f;
            label.anchor = TextAnchor.LowerCenter;
            label.alignment = TextAlignment.Center;
            label.color = Color.white;
            // A TextMesh draws with its font's own material; without it the
            // letters come out as boxes.
            holder.GetComponent<MeshRenderer>().sharedMaterial = font.material;
        }

        // Square on to the camera, after everything has moved this frame.
        void LateUpdate()
        {
            Camera camera = Camera.main;
            if (label != null && camera != null)
                label.transform.rotation = camera.transform.rotation;
        }
    }
}
