// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/World/CharacterStandIn.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Stands in for the player's character until there's a real one: put it
// on a box in the scene (a Cube, scaled 1 by 2 by 1, 2 blocks tall like
// the character, with whatever material), and it's moved to where the
// server says the character stands when it comes into the world, and
// hidden the rest of the time.  The Cinemachine camera follows it (Jacob,
// 2026-10-03: "a cinemachine following camera in the style of zomboid").
// The character's x and z are its middle and its y is its feet, so the box
// sits with its bottom at y.

using Opus.Net;
using UnityEngine;

namespace Opus.World
{
    public class CharacterStandIn : MonoBehaviour
    {
        Renderer[] shown;

        // Awake and OnDestroy, not OnEnable: the box is hidden by turning
        // its renderers off, so it hears the session the whole time.
        void Awake()
        {
            shown = GetComponentsInChildren<Renderer>();
            Show(false);
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
            Vector3 feet = Session.Standing;
            transform.position = new Vector3(feet.x, feet.y + transform.lossyScale.y / 2f, feet.z);
            Show(true);
        }

        void Gone(string why, bool trouble)
        {
            Show(false);
        }

        void Show(bool on)
        {
            foreach (Renderer renderer in shown)
                renderer.enabled = on;
        }
    }
}
