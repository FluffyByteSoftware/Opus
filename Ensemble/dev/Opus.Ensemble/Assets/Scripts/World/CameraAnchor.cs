// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/World/CameraAnchor.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// What the Cinemachine camera follows (Jacob, 2026-10-03: "a cinemachine
// following camera in the style of zomboid"): kept on the player's own
// character, the object WorldObjectsView draws for Session.OwnObject, so
// the camera goes wherever the server puts it.  Until that object comes it
// waits where CharacterEnteredWorld said the character stands.  It's never
// drawn itself: whatever it's on (the box that stood in for the character
// before there were objects, CharacterStandIn until session 9) has its
// renderers turned off, and the character is drawn by WorldObjectsView.
// Its middle sits as high over the character's feet as the box is half
// tall, so the camera's framing is what it was.

using Opus.Net;
using UnityEngine;

namespace Opus.World
{
    public class CameraAnchor : MonoBehaviour
    {
        bool inWorld;

        // Awake and OnDestroy, not OnEnable: it hears the session the whole
        // time.
        void Awake()
        {
            foreach (Renderer renderer in GetComponentsInChildren<Renderer>())
                renderer.enabled = false;
            Session.ReachedWorld += InWorld;
            Session.SessionOver += Gone;
        }

        void OnDestroy()
        {
            Session.ReachedWorld -= InWorld;
            Session.SessionOver -= Gone;
        }

        void InWorld()
        {
            inWorld = true;
            StandAt(Session.Standing);
        }

        void Gone(string why, bool trouble)
        {
            inWorld = false;
        }

        // After the objects have moved this frame.
        void LateUpdate()
        {
            if (!inWorld)
                return;
            Transform own = WorldObjectsView.Find(Session.OwnObject);
            if (own != null)
                StandAt(own.position);
        }

        void StandAt(Vector3 feet)
        {
            transform.position = new Vector3(feet.x, feet.y + transform.lossyScale.y / 2f, feet.z);
        }
    }
}
