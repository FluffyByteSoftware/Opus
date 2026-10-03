// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/World/GroundView.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Draws the Ground: a GameObject a chunk, under this one, with a mesh of
// the faces between blocks and air and a material for each block kind
// from the slots below (Jacob's materials, "for now they're just colors").
// A chunk is drawn once it and all six chunks round it are in (Minecraft's
// and 7 Days to Die's way), so the outermost ring of the view is never
// drawn.  An all-air chunk has nothing to draw.  The meshes are worked out
// on a thread of its own (ChunkMesher) and made into Unity's Mesh here, a
// few a frame.  Everything drawn goes when the Ground is emptied.  The
// Console says how many were drawn and how long it took each time the
// worker runs out of chunks.

using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading;
using Opus.Net;
using UnityEngine;
using UnityEngine.Rendering;
using Debug = UnityEngine.Debug;

namespace Opus.World
{
    public class GroundView : MonoBehaviour
    {
        [Header("The blocks' materials")]
        [Tooltip("DIRT's material.  An empty slot's faces aren't drawn.")]
        public Material dirt;

        [Tooltip("STONE's material.")]
        public Material stone;

        [Tooltip("WOOD's material.")]
        public Material wood;

        [Tooltip("GOLD's material.")]
        public Material gold;

        [Tooltip("BEDROCK's material.")]
        public Material bedrock;

        [Header("Making the meshes")]
        [Tooltip("The most chunks' meshes made into Unity's Mesh in one frame, so a view coming in doesn't stop "
            + "the screen.  The working out is on a thread of its own either way.")]
        public int meshesPerFrame = 32;

        // A chunk to mesh, with the six round it, from the Ground as it
        // was.  Generation says which Ground it's from.
        class Job
        {
            public int Generation;
            public Chunk Chunk;
            public Chunk[] Around;
        }

        class Result
        {
            public int Generation;
            public ChunkMesh Mesh;
        }

        // The worker thread and what goes back and forth.
        BlockingCollection<Job> jobs;
        readonly ConcurrentQueue<Result> results = new ConcurrentQueue<Result>();
        Thread worker;

        // Bumped every time the Ground is emptied, so a mesh worked out
        // for the old one is thrown away.  Read by the worker only through
        // the job it carries.
        int generation;

        // Chunks sent to the worker, so none goes twice, and what's drawn.
        readonly HashSet<ChunkPlace> sent = new HashSet<ChunkPlace>();
        readonly List<GameObject> drawn = new List<GameObject>();
        readonly List<Mesh> meshes = new List<Mesh>();

        // For the Console's line: since the worker last ran out.
        int waiting;
        int madeSince;
        int facesSince;
        double workerMsSince;
        readonly Stopwatch since = new Stopwatch();

        // The block kinds with no material, said once each.
        readonly HashSet<ushort> saidMissing = new HashSet<ushort>();

        void OnEnable()
        {
            jobs = new BlockingCollection<Job>();
            worker = new Thread(Work);
            worker.Name = "Ground mesher";
            worker.IsBackground = true;
            worker.Start(jobs);

            Ground.Added += ChunkAdded;
            Ground.Cleared += Forget;
        }

        void OnDisable()
        {
            Ground.Added -= ChunkAdded;
            Ground.Cleared -= Forget;
            jobs.CompleteAdding();
            worker = null;
            Forget();
        }

        // ---------------------------------------------------------------
        // The worker
        // ---------------------------------------------------------------

        // Its own collection is handed over, so a worker from before an
        // OnDisable finishes with the old one and stops.
        void Work(object handed)
        {
            var mine = (BlockingCollection<Job>)handed;
            foreach (Job job in mine.GetConsumingEnumerable())
            {
                ChunkMesh mesh = ChunkMesher.Build(job.Chunk, job.Around);
                results.Enqueue(new Result { Generation = job.Generation, Mesh = mesh });
            }
        }

        // ---------------------------------------------------------------
        // Main thread
        // ---------------------------------------------------------------

        // A chunk came in: it, and the six round it, may be drawable now.
        void ChunkAdded(Chunk chunk)
        {
            Send(chunk.Place);
            for (int way = 0; way < 6; way++)
            {
                if (ChunkMesher.HasRow(chunk.Place, way))
                    Send(ChunkMesher.Beside(chunk.Place, way));
            }
        }

        // To the worker, if it's in, has something but air, hasn't gone
        // already, and all six round it are in.  An all-air chunk has
        // nothing to draw, so it's done as soon as it's in.
        void Send(ChunkPlace place)
        {
            if (sent.Contains(place))
                return;
            Chunk chunk = Ground.Get(place);
            if (chunk == null)
                return;
            if (chunk.AllOneKind && chunk.OnlyKind == Blocks.Air)
            {
                Session.ChunkShown(place);
                return;
            }

            var around = new Chunk[6];
            for (int way = 0; way < 6; way++)
            {
                if (!ChunkMesher.HasRow(place, way))
                    continue;
                around[way] = Ground.Get(ChunkMesher.Beside(place, way));
                if (around[way] == null)
                    return;
            }

            sent.Add(place);
            if (waiting == 0 && madeSince == 0)
                since.Restart();
            waiting++;
            jobs.Add(new Job { Generation = generation, Chunk = chunk, Around = around });
        }

        void Update()
        {
            int made = 0;
            Result result;
            while (made < meshesPerFrame && results.TryDequeue(out result))
            {
                if (result.Generation != generation)
                    continue;
                waiting--;
                madeSince++;
                facesSince += result.Mesh.Faces;
                workerMsSince += result.Mesh.TookMs;
                Draw(result.Mesh);
                // Drawn, or nothing in it to draw: either way it's done, and
                // the nearest being done is what PlayerReady waits on.
                Session.ChunkShown(result.Mesh.Place);
                made++;
            }

            if (made > 0 && waiting == 0)
            {
                Debug.Log("World: " + madeSince + " chunks meshed, " + facesSince + " faces, in "
                          + since.Elapsed.TotalSeconds.ToString("0.00") + " s (the worker's share "
                          + (workerMsSince / 1000.0).ToString("0.00") + " s, " + (workerMsSince / madeSince)
                          .ToString("0.00") + " ms a chunk); " + drawn.Count + " chunks drawn in all.");
                madeSince = 0;
                facesSince = 0;
                workerMsSince = 0;
            }
        }

        // The worker's lists into Unity's Mesh, and a GameObject for it.
        // Only the kinds with a material are kept; a chunk whose faces are
        // all of kinds without one, or that has no faces (all of it inside
        // the ground), gets nothing.
        void Draw(ChunkMesh worked)
        {
            var materials = new List<Material>();
            var triangles = new List<List<int>>();
            for (int i = 0; i < worked.Kinds.Count; i++)
            {
                Material material = MaterialOf(worked.Kinds[i]);
                if (material == null)
                    continue;
                materials.Add(material);
                triangles.Add(worked.Triangles[i]);
            }
            if (materials.Count == 0)
                return;

            var mesh = new Mesh();
            mesh.name = "Chunk " + worked.Place;
            if (worked.Corners.Count > 65535)
                mesh.indexFormat = IndexFormat.UInt32;
            mesh.SetVertices(worked.Corners);
            mesh.SetNormals(worked.Normals);
            mesh.subMeshCount = materials.Count;
            for (int i = 0; i < triangles.Count; i++)
                mesh.SetTriangles(triangles[i], i);
            mesh.RecalculateBounds();

            var shown = new GameObject("Chunk " + worked.Place);
            shown.transform.SetParent(transform, false);
            shown.transform.localPosition = worked.Origin;
            shown.AddComponent<MeshFilter>().sharedMesh = mesh;
            shown.AddComponent<MeshRenderer>().sharedMaterials = materials.ToArray();
            drawn.Add(shown);
            meshes.Add(mesh);
        }

        Material MaterialOf(ushort kind)
        {
            Material material;
            switch (kind)
            {
                case Blocks.Dirt: material = dirt; break;
                case Blocks.Stone: material = stone; break;
                case Blocks.Wood: material = wood; break;
                case Blocks.Gold: material = gold; break;
                case Blocks.Bedrock: material = bedrock; break;
                default: material = null; break;
            }
            if (material == null && saidMissing.Add(kind))
                Debug.LogWarning("World: no material for " + Blocks.NameOf(kind) + " on GroundView, so its faces "
                                 + "aren't drawn.");
            return material;
        }

        // The Ground was emptied (or this is going away): everything drawn
        // goes, and whatever the worker is still on is thrown away when it
        // comes back.
        void Forget()
        {
            generation++;
            foreach (GameObject shown in drawn)
                Destroy(shown);
            foreach (Mesh mesh in meshes)
                Destroy(mesh);
            drawn.Clear();
            meshes.Clear();
            sent.Clear();
            waiting = 0;
            madeSince = 0;
            facesSince = 0;
            workerMsSince = 0;
        }
    }
}
