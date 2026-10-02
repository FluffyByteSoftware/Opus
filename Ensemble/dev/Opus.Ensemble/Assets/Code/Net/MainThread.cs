// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/MainThread.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// How the network's threads hand things to Unity's main thread.  Only the
// main thread may touch the screen, so a thread that hears something from
// the server posts the work here, and ScreenRoot runs whatever is waiting
// once a frame (its Update()).

using System;
using System.Collections.Concurrent;
using UnityEngine;

namespace Opus.Net
{
    public static class MainThread
    {
        static readonly ConcurrentQueue<Action> waiting = new ConcurrentQueue<Action>();

        // From any thread.
        public static void Post(Action work)
        {
            waiting.Enqueue(work);
        }

        // From the main thread only, once a frame.  One piece of work that
        // throws doesn't stop the rest.
        public static void Run()
        {
            Action work;
            while (waiting.TryDequeue(out work))
            {
                try
                {
                    work();
                }
                catch (Exception e)
                {
                    Debug.LogException(e);
                }
            }
        }
    }
}
