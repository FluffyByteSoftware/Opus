// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/World/FluffyGameObject.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// On every object the client draws (Jacob, 2026-10-03: "every single drawn
// object to the client is going to need a FluffyGameObject now.  It's going
// to define how to represent it in the client").  It says which of the
// server's objects this is, and puts it where the server says: its feet at
// the position, turned to the rotation, moved along its velocity every
// frame until the server says otherwise.  It never decides a place of its
// own.  WorldObjectsView makes one for each object; an Actor goes on top
// for anything Living.  The model's own number goes here too once there
// are models (the model draw).

using UnityEngine;

namespace Opus.World
{
    public class FluffyGameObject : MonoBehaviour
    {
        [Header("What the server says this is")]
        [Tooltip("Its number among the world's objects, for this session.")]
        public uint objectNumber;

        [Tooltip("The game's name for it.")]
        public string uuid;

        [Tooltip("What its model is doing (\"idle\"), empty for nothing.  Nothing plays it yet.")]
        public string doing;

        // Blocks a second, from the server.
        Vector3 velocity;

        // Where the server last said it is, how it's turned and where it's
        // going.  Its feet are at the position.
        public void Place(Vector3 position, Vector3 rotation, Vector3 heading)
        {
            transform.position = position;
            transform.rotation = Quaternion.Euler(rotation);
            velocity = heading;
        }

        // Along its velocity until it's told otherwise, so it moves smoothly
        // between the server's word and the next.  Still, while there's no
        // movement.
        void Update()
        {
            if (velocity != Vector3.zero)
                transform.position += velocity * Time.deltaTime;
        }
    }
}
