// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/World/WorldObjectsView.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Draws the world's objects (protocol version 14), the player's own
// character among them: a GameObject under this one for each object
// WorldObjects knows, with a FluffyGameObject on it saying which it is and
// putting it where the server says, an Actor on top for anything Living
// (the short name over its head), and the fallback shape the server named
// as a child, its bottom at the object's feet.  Models come with the model
// draw; until then everything is its fallback shape (a character is a
// capsule, 1 block wide and 2 tall).  Put it on an empty GameObject in the
// scene.  The camera's anchor (CameraAnchor) follows the player's own.

using System.Collections.Generic;
using Opus.Net;
using UnityEngine;

namespace Opus.World
{
    public class WorldObjectsView : MonoBehaviour
    {
        [Header("Names over heads")]
        [Tooltip("The font for an Actor's name over its head.  Empty: Unity's own.")]
        public Font nameFont;

        [Tooltip("How tall a capital in a name is, in blocks.")]
        public float nameSize = 0.3f;

        [Tooltip("How far over the top of an Actor's shape its name sits, in blocks.")]
        public float nameGap = 0.25f;

        // What's drawn, by object number.
        readonly Dictionary<uint, FluffyGameObject> drawn = new Dictionary<uint, FluffyGameObject>();

        // The one in the scene, for CameraAnchor to find the player's own.
        static WorldObjectsView current;

        // Said once a session, not once an object.
        bool saidNoModels;

        // The object drawn for `number`, or null.
        public static Transform Find(uint number)
        {
            if (current == null || number == 0)
                return null;
            FluffyGameObject found;
            return current.drawn.TryGetValue(number, out found) ? found.transform : null;
        }

        void Awake()
        {
            current = this;
            WorldObjects.Added += Add;
            WorldObjects.Moved += Move;
            WorldObjects.Removed += Remove;
            WorldObjects.Cleared += Clear;
        }

        void OnDestroy()
        {
            WorldObjects.Added -= Add;
            WorldObjects.Moved -= Move;
            WorldObjects.Removed -= Remove;
            WorldObjects.Cleared -= Clear;
            if (current == this)
                current = null;
        }

        void Add(WorldObject whole)
        {
            Remove(whole.Number);

            bool own = whole.Number == Session.OwnObject;
            var root = new GameObject("Object " + whole.Number + " (" + (whole.ShortName ?? "") + ")"
                                      + (own ? ", yours" : ""));
            root.transform.SetParent(transform, false);
            root.transform.localScale = whole.Scale;

            var fluffy = root.AddComponent<FluffyGameObject>();
            fluffy.objectNumber = whole.Number;
            fluffy.uuid = whole.Uuid;
            fluffy.doing = whole.Doing;
            fluffy.Place(whole.Position, whole.Rotation, whole.Velocity);

            if (!string.IsNullOrEmpty(whole.Model) && !saidNoModels)
            {
                saidNoModels = true;
                Debug.Log("World: object " + whole.Number + " has a model (" + whole.Model + "), and models come "
                          + "with the model draw.  Its fallback shape is drawn instead.");
            }
            float height = Fallback(root.transform, whole.Shape);

            if (whole.Living)
                root.AddComponent<Actor>().Name(whole.ShortName, nameFont, nameSize, height + nameGap);

            drawn[whole.Number] = fluffy;
        }

        void Move(WorldObject whole)
        {
            FluffyGameObject fluffy;
            if (drawn.TryGetValue(whole.Number, out fluffy))
                fluffy.Place(whole.Position, whole.Rotation, whole.Velocity);
        }

        void Remove(uint number)
        {
            FluffyGameObject fluffy;
            if (!drawn.TryGetValue(number, out fluffy))
                return;
            drawn.Remove(number);
            if (fluffy != null)
                Destroy(fluffy.gameObject);
        }

        void Clear()
        {
            foreach (FluffyGameObject fluffy in drawn.Values)
            {
                if (fluffy != null)
                    Destroy(fluffy.gameObject);
            }
            drawn.Clear();
            saidNoModels = false;
        }

        // The server's fallback shape as Unity's own, a child of `root` with
        // its bottom at root's feet.  No collider: the client only draws.
        // How tall it is, for the name over it.
        static float Fallback(Transform root, byte shape)
        {
            PrimitiveType type;
            float height;
            switch (shape)
            {
                case 1: type = PrimitiveType.Sphere; height = 1f; break;
                case 2: type = PrimitiveType.Capsule; height = 2f; break;
                case 3: type = PrimitiveType.Cylinder; height = 2f; break;
                case 4: type = PrimitiveType.Plane; height = 0f; break;
                case 5: type = PrimitiveType.Quad; height = 1f; break;
                default: type = PrimitiveType.Cube; height = 1f; break;
            }
            GameObject drawnShape = GameObject.CreatePrimitive(type);
            drawnShape.name = "Shape";
            Destroy(drawnShape.GetComponent<Collider>());
            drawnShape.transform.SetParent(root, false);
            drawnShape.transform.localPosition = new Vector3(0, height / 2f, 0);
            return height;
        }
    }
}
