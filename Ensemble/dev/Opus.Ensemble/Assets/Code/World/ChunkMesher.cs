// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/World/ChunkMesher.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Turns a chunk into what its mesh is made of: a face wherever a block that
// isn't air meets air, and nowhere else, so the inside of the ground costs
// nothing.  Each face is four corners of its own (so it lights flat) and
// two triangles, kept by block kind, so each kind can have its own
// material.  At a chunk's edge the chunk beside it says whether a face
// shows, which is why a chunk is only meshed once all six round it are in
// (GroundView).  Plain C#, nothing of Unity's but Vector3, so it runs on a
// worker thread.  A chunk never changes once it's in, so reading one from
// another thread is safe.

using System.Collections.Generic;
using System.Diagnostics;
using UnityEngine;

namespace Opus.World
{
    // A chunk's mesh as plain lists, built off the main thread.  GroundView
    // makes Unity's Mesh from it on the main thread.  Corners are inside
    // the chunk, 0 to 32 each way; the chunk's GameObject sits at Origin.
    public sealed class ChunkMesh
    {
        public ChunkPlace Place;
        public Vector3 Origin;
        public readonly List<Vector3> Corners = new List<Vector3>();
        public readonly List<Vector3> Normals = new List<Vector3>();

        // A list of triangles for each block kind with faces, in the order
        // the kinds first turned up.
        public readonly List<ushort> Kinds = new List<ushort>();
        public readonly List<List<int>> Triangles = new List<List<int>>();

        public int Faces;
        public double TookMs;
    }

    public static class ChunkMesher
    {
        // The six chunks round a chunk, in this order, in the array Build()
        // takes.
        public const int East = 0;   // +x
        public const int West = 1;   // -x
        public const int Up = 2;     // +y
        public const int Down = 3;   // -y
        public const int North = 4;  // +z
        public const int South = 5;  // -z

        const int Side = Chunk.Side;

        // Whether there's a row the way given: none above row 10 or below
        // row 0.
        public static bool HasRow(ChunkPlace place, int way)
        {
            if (way == Up)
                return place.Row + 1 < Chunk.Rows;
            if (way == Down)
                return place.Row > 0;
            return true;
        }

        // The place of the chunk beside this one, the six ways.  Only for a
        // way HasRow() says there's a row.
        public static ChunkPlace Beside(ChunkPlace place, int way)
        {
            switch (way)
            {
                case East: return new ChunkPlace((short)(place.X + 1), place.Z, place.Row);
                case West: return new ChunkPlace((short)(place.X - 1), place.Z, place.Row);
                case Up: return new ChunkPlace(place.X, place.Z, (byte)(place.Row + 1));
                case Down: return new ChunkPlace(place.X, place.Z, (byte)(place.Row - 1));
                case North: return new ChunkPlace(place.X, (short)(place.Z + 1), place.Row);
                default: return new ChunkPlace(place.X, (short)(place.Z - 1), place.Row);
            }
        }

        // The mesh of a chunk, given the six round it (null above the top
        // row, which counts as air, and below the bottom, which counts as
        // solid: nobody sees the world's underside).
        public static ChunkMesh Build(Chunk chunk, Chunk[] around)
        {
            var clock = Stopwatch.StartNew();
            var mesh = new ChunkMesh();
            mesh.Place = chunk.Place;
            mesh.Origin = new Vector3(chunk.Place.X * Side, Chunk.LowestBlock + chunk.Place.Row * Side,
                                      chunk.Place.Z * Side);
            var listOf = new Dictionary<ushort, List<int>>();

            for (int y = 0; y < Side; y++)
            {
                for (int z = 0; z < Side; z++)
                {
                    for (int x = 0; x < Side; x++)
                    {
                        ushort kind = chunk.BlockAt(x, y, z);
                        if (kind == Blocks.Air)
                            continue;
                        if (!Solid(chunk, around, x, y + 1, z))
                            Face(mesh, listOf, kind, Vector3.up,
                                 x, y + 1, z, x, y + 1, z + 1, x + 1, y + 1, z + 1, x + 1, y + 1, z);
                        if (!Solid(chunk, around, x, y - 1, z))
                            Face(mesh, listOf, kind, Vector3.down,
                                 x, y, z, x + 1, y, z, x + 1, y, z + 1, x, y, z + 1);
                        if (!Solid(chunk, around, x + 1, y, z))
                            Face(mesh, listOf, kind, Vector3.right,
                                 x + 1, y, z, x + 1, y + 1, z, x + 1, y + 1, z + 1, x + 1, y, z + 1);
                        if (!Solid(chunk, around, x - 1, y, z))
                            Face(mesh, listOf, kind, Vector3.left,
                                 x, y, z + 1, x, y + 1, z + 1, x, y + 1, z, x, y, z);
                        if (!Solid(chunk, around, x, y, z + 1))
                            Face(mesh, listOf, kind, Vector3.forward,
                                 x + 1, y, z + 1, x + 1, y + 1, z + 1, x, y + 1, z + 1, x, y, z + 1);
                        if (!Solid(chunk, around, x, y, z - 1))
                            Face(mesh, listOf, kind, Vector3.back,
                                 x, y, z, x, y + 1, z, x + 1, y + 1, z, x + 1, y, z);
                    }
                }
            }
            mesh.TookMs = clock.Elapsed.TotalMilliseconds;
            return mesh;
        }

        // Whether the block at x, y, z is anything but air.  One of the
        // three may be one past the chunk's edge (-1 or 32), and then the
        // chunk beside it says.
        static bool Solid(Chunk chunk, Chunk[] around, int x, int y, int z)
        {
            Chunk other;
            if (x == Side)
            {
                other = around[East];
                x = 0;
            }
            else if (x < 0)
            {
                other = around[West];
                x = Side - 1;
            }
            else if (y == Side)
            {
                // Over the top row is air.
                other = around[Up];
                if (other == null)
                    return false;
                y = 0;
            }
            else if (y < 0)
            {
                other = around[Down];
                y = Side - 1;
            }
            else if (z == Side)
            {
                other = around[North];
                z = 0;
            }
            else if (z < 0)
            {
                other = around[South];
                z = Side - 1;
            }
            else
            {
                return chunk.BlockAt(x, y, z) != Blocks.Air;
            }
            // Under the bottom row, or a chunk that isn't there (which
            // GroundView never meshes beside), counts as solid: no face.
            if (other == null)
                return true;
            return other.BlockAt(x, y, z) != Blocks.Air;
        }

        // One face: four corners, clockwise as seen from outside (Unity's
        // front), and its two triangles in its kind's list.
        static void Face(ChunkMesh mesh, Dictionary<ushort, List<int>> listOf, ushort kind, Vector3 normal,
                         int x0, int y0, int z0, int x1, int y1, int z1,
                         int x2, int y2, int z2, int x3, int y3, int z3)
        {
            List<int> triangles;
            if (!listOf.TryGetValue(kind, out triangles))
            {
                triangles = new List<int>();
                listOf[kind] = triangles;
                mesh.Kinds.Add(kind);
                mesh.Triangles.Add(triangles);
            }

            int first = mesh.Corners.Count;
            mesh.Corners.Add(new Vector3(x0, y0, z0));
            mesh.Corners.Add(new Vector3(x1, y1, z1));
            mesh.Corners.Add(new Vector3(x2, y2, z2));
            mesh.Corners.Add(new Vector3(x3, y3, z3));
            for (int i = 0; i < 4; i++)
                mesh.Normals.Add(normal);
            triangles.Add(first);
            triangles.Add(first + 1);
            triangles.Add(first + 2);
            triangles.Add(first);
            triangles.Add(first + 2);
            triangles.Add(first + 3);
            mesh.Faces++;
        }
    }
}
