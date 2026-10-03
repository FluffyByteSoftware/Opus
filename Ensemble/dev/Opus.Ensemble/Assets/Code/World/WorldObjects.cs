// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/World/WorldObjects.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// What the client knows of the world's objects (protocol version 14): every
// object in the player's view, by its number, the player's own character
// included.  The server has the say on where each one is (Jacob,
// 2026-10-03: "The server will be the authority, always on where the
// object actually is in the world.  The client is just a dumb renderer."),
// so this only ever keeps what it was told: a Hydrate puts an object here
// whole, ObjectsMoved moves it, ObjectsGone takes it out, and the roll
// call once a second mends anything a lost packet left wrong.  The
// screen's half is WorldObjectsView, which hears the events below.  Main
// thread only, like the Ground.

using System;
using System.Collections.Generic;
using UnityEngine;

namespace Opus.World
{
    // One object, as the server last described it.
    public class WorldObject
    {
        public uint Number;
        public string Uuid;
        // Living: drawn as an Actor, with its short name over its head.
        public bool Living;
        public string ShortName;
        // In blocks, y up; a character's position is its feet.
        public Vector3 Position;
        // Degrees about x, y and z, as Unity has them.
        public Vector3 Rotation;
        // Blocks a second; the object is moved along it every frame until
        // the server says otherwise.  0 until there's movement.
        public Vector3 Velocity;
        public Vector3 Scale;
        // The model's uuid, empty for none: then the fallback shape is
        // drawn.  Nothing has one until the model draw.
        public string Model;
        // 0 cube, 1 sphere, 2 capsule, 3 cylinder, 4 plane, 5 quad.
        public byte Shape;
        // What the model is doing ("idle"), empty for nothing.
        public string Doing;
    }

    // Where an object is now, from ObjectsMoved or a roll call.
    public struct ObjectMotion
    {
        public uint Number;
        public Vector3 Position;
        public Vector3 Rotation;
        public Vector3 Velocity;
    }

    public static class WorldObjects
    {
        static readonly Dictionary<uint, WorldObject> known = new Dictionary<uint, WorldObject>();

        // The roll call being put together: its number, how many pieces it
        // has, and the ones in so far.
        static uint roll;
        static int rollPieces;
        static readonly Dictionary<int, ObjectMotion[]> rollIn = new Dictionary<int, ObjectMotion[]>();

        // An object came into view, whole (or came again, whole).
        public static event Action<WorldObject> Added;

        // A known object moved.
        public static event Action<WorldObject> Moved;

        // An object went: out of view, out of the world, or off a roll call.
        public static event Action<uint> Removed;

        // Everything went: the session ended.
        public static event Action Cleared;

        public static int Count
        {
            get { return known.Count; }
        }

        public static WorldObject Find(uint number)
        {
            WorldObject found;
            return known.TryGetValue(number, out found) ? found : null;
        }

        // A Hydrate.  One already here is replaced whole: the server sends
        // it again when the client asked about it.
        public static void Put(WorldObject whole)
        {
            if (known.ContainsKey(whole.Number))
                Remove(whole.Number);
            known[whole.Number] = whole;
            Debug.Log("Game: object " + whole.Number + " (" + Label(whole) + ") came into view at "
                      + whole.Position.x + ", " + whole.Position.y + ", " + whole.Position.z + ".");
            if (Added != null)
                Added(whole);
        }

        // ObjectsMoved, or a roll call saying where a known one is.  An
        // object nothing's known about is left for the roll call to ask.
        public static void Move(ObjectMotion motion)
        {
            WorldObject there = Find(motion.Number);
            if (there == null)
                return;
            if (there.Position == motion.Position && there.Rotation == motion.Rotation
                && there.Velocity == motion.Velocity)
                return;
            there.Position = motion.Position;
            there.Rotation = motion.Rotation;
            there.Velocity = motion.Velocity;
            if (Moved != null)
                Moved(there);
        }

        // ObjectsGone.
        public static void Remove(uint number)
        {
            WorldObject there = Find(number);
            if (there == null)
                return;
            known.Remove(number);
            Debug.Log("Game: object " + number + " (" + Label(there) + ") is gone.");
            if (Removed != null)
                Removed(number);
        }

        // The session is over.
        public static void Clear()
        {
            known.Clear();
            rollIn.Clear();
            rollPieces = 0;
            if (Cleared != null)
                Cleared();
        }

        // A piece of a roll call.  Once every piece of it is in, whatever
        // isn't on it is dropped, whatever's on it is moved to where it
        // says, and the numbers on it nothing's known about are handed
        // back, for the server to be asked about.  Null until then.  A
        // piece of a newer roll call gives up on an older one still
        // coming.
        public static List<uint> RollCallPiece(uint number, int piece, int pieces, ObjectMotion[] motions)
        {
            if (number != roll || rollPieces == 0)
            {
                roll = number;
                rollPieces = pieces;
                rollIn.Clear();
            }
            rollIn[piece] = motions;
            if (rollIn.Count < rollPieces)
                return null;

            var listed = new HashSet<uint>();
            var unknown = new List<uint>();
            foreach (ObjectMotion[] some in rollIn.Values)
            {
                foreach (ObjectMotion motion in some)
                {
                    listed.Add(motion.Number);
                    if (known.ContainsKey(motion.Number))
                        Move(motion);
                    else
                        unknown.Add(motion.Number);
                }
            }
            rollIn.Clear();
            rollPieces = 0;

            var missing = new List<uint>();
            foreach (uint there in known.Keys)
            {
                if (!listed.Contains(there))
                    missing.Add(there);
            }
            foreach (uint gone in missing)
            {
                Debug.Log("Game: roll call " + number + " doesn't have object " + gone + ".  Dropped.");
                Remove(gone);
            }
            if (unknown.Count > 0)
                Debug.Log("Game: roll call " + number + " has " + unknown.Count + " object(s) this client doesn't "
                          + "know.  Asking about them.");
            return unknown;
        }

        static string Label(WorldObject whole)
        {
            return string.IsNullOrEmpty(whole.ShortName) ? "no name" : whole.ShortName;
        }
    }
}
