// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Editor/CopyAnimsFromFbxPack.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Tools > Opus > Copy Anims From FBX Pack.  Copies the animation clips out
// of an animation pack's model files into .anim files of their own, keeping
// the pack's folders, so they can be moved and edited like anything else.

using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEngine;

// A clip inside an FBX can't be taken out: Unity makes it fresh from the FBX
// on every import and won't let anybody edit it.  So we copy it.  The FBX
// keeps its own clip, and the copy is a plain .anim that's ours.
//
// The copies are still the pack's art, bought, not made, so they never go in
// the folders git keeps (Assets/Editor, Assets/Code, Assets/Scripts,
// Assets/Data).  The window refuses a destination inside one of them.
public class CopyAnimsFromFbxPack : EditorWindow
{
    // The folders under Assets/ that are committed.  Everything else in
    // Assets/ stays on this machine; see the .gitignore at the repo root.
    static readonly string[] CommittedFolders = { "Assets/Editor", "Assets/Code", "Assets/Scripts", "Assets/Data" };

    // Unity hides a clip of its own in some model files for the preview
    // window.  It isn't part of the pack.
    const string PreviewClipStart = "__preview__";

    // One clip found in one model file, and whether it's ticked.
    class Found
    {
        public string ModelPath;   // Assets/Packs/Unarmed/RPG-Character@Unarmed-Attack-L1.fbx
        public string SubFolder;   // the model's folder under the source: "Unarmed", or "" at the top
        public string Prefix;      // "RPG-Character@", the file name up to and including the @, or ""
        public string Rest;        // "Unarmed-Attack-L1", the file name after the @
        public string ClipName;    // the clip's own name inside the model
        public bool OnlyClip;      // true when the model holds just this one clip
        public bool Ticked = true;
    }

    // Asset paths, Assets/Packs/RPG, as typed or picked with BROWSE.
    string source = "";
    string destination = "";
    string foundIn;                // the source folder the list was found in
    List<Found> found = new List<Found>();

    // Each prefix found, and what it becomes.  "HumanM@" -> "Player_".  A
    // blank one drops the prefix.
    Dictionary<string, string> prefixes = new Dictionary<string, string>();

    // Which list is showing: every clip, or only the ones that clash (in
    // red), so they can be found without scrolling through all 600.
    int view;
    const int EveryClip = 0;
    const int OnlyClashes = 1;

    Vector2 scroll;
    string report = "";

    [MenuItem("Tools/Opus/Copy Anims From FBX Pack")]
    static void Open()
    {
        GetWindow<CopyAnimsFromFbxPack>("Copy Anims From FBX Pack");
    }

    void OnGUI()
    {
        source = FolderField("Source folder", source);
        destination = FolderField("Destination folder", destination);

        // A list found in another folder isn't this folder's list.
        string sourcePath = FolderPath(source);
        if (found.Count > 0 && sourcePath != foundIn)
        {
            found.Clear();
            prefixes.Clear();
            report = "";
        }

        using (new EditorGUI.DisabledScope(sourcePath == null))
        {
            if (GUILayout.Button("FIND"))
                Find(sourcePath);
        }

        if (found.Count > 0)
        {
            DrawPrefixes();
            DrawList();
            DrawCopyButton();
        }

        if (report != "")
            EditorGUILayout.HelpBox(report, MessageType.Info);
    }

    void DrawPrefixes()
    {
        if (prefixes.Count == 0)
            return;

        EditorGUILayout.Space();
        EditorGUILayout.LabelField("Prefixes (blank drops it)", EditorStyles.boldLabel);
        foreach (string prefix in prefixes.Keys.ToList())
            prefixes[prefix] = EditorGUILayout.TextField(prefix, prefixes[prefix]);
    }

    void DrawList()
    {
        EditorGUILayout.Space();
        EditorGUILayout.BeginHorizontal();
        EditorGUILayout.LabelField(found.Count(f => f.Ticked) + " of " + found.Count + " clips ticked",
            EditorStyles.boldLabel);
        if (GUILayout.Button("ALL", GUILayout.Width(50)))
            found.ForEach(f => f.Ticked = true);
        if (GUILayout.Button("NONE", GUILayout.Width(50)))
            found.ForEach(f => f.Ticked = false);
        EditorGUILayout.EndHorizontal();

        // Two ticked clips landing on the same file would have the second
        // write over the first, so they're shown in red and COPY waits.
        HashSet<string> clashes = Clashes();

        int before = view;
        view = GUILayout.Toolbar(view, new[] { "EVERY CLIP", "CLASHES (" + ClashingClips(clashes) + ")" });
        if (view != before)
            scroll = Vector2.zero;

        scroll = EditorGUILayout.BeginScrollView(scroll);
        if (view == EveryClip)
            DrawEveryClip(clashes);
        else
            DrawClashes(clashes);
        EditorGUILayout.EndScrollView();
    }

    // Every clip found, under the folder it's in.
    void DrawEveryClip(HashSet<string> clashes)
    {
        string folder = null;
        foreach (Found f in found)
        {
            if (f.SubFolder != folder)
            {
                folder = f.SubFolder;
                EditorGUILayout.LabelField(folder == "" ? "(top of the source)" : folder, EditorStyles.miniBoldLabel);
            }
            DrawRow(f, clashes);
        }
    }

    // Only the clips that clash, under the file they'd all land on.  Once a
    // clash is sorted out (one of them unticked, or a prefix changed), its
    // clips drop off this list.
    void DrawClashes(HashSet<string> clashes)
    {
        if (clashes.Count == 0)
        {
            EditorGUILayout.LabelField("No clashes.");
            return;
        }

        string destinationPath = FolderPath(destination);
        foreach (IGrouping<string, Found> clash in found.Where(f => f.Ticked && clashes.Contains(OutputPath(f)))
                     .GroupBy(OutputPath))
        {
            string landsOn = destinationPath != null && clash.Key.StartsWith(destinationPath + "/")
                ? clash.Key.Substring(destinationPath.Length + 1)
                : clash.Key;
            EditorGUILayout.LabelField(landsOn, EditorStyles.miniBoldLabel);
            foreach (Found f in clash)
                DrawRow(f, clashes);
        }
    }

    void DrawRow(Found f, HashSet<string> clashes)
    {
        string output = OutputName(f) + ".anim";
        string from = Path.GetFileName(f.ModelPath) + (f.OnlyClip ? "" : "  [" + f.ClipName + "]");

        Color before = GUI.color;
        if (f.Ticked && clashes.Contains(OutputPath(f)))
            GUI.color = Color.red;
        f.Ticked = EditorGUILayout.ToggleLeft(from + "  ->  " + output, f.Ticked);
        GUI.color = before;
    }

    // How many ticked clips are in a clash, for the CLASHES button.
    int ClashingClips(HashSet<string> clashes)
    {
        return found.Count(f => f.Ticked && clashes.Contains(OutputPath(f)));
    }

    void DrawCopyButton()
    {
        string problem = DestinationProblem();
        if (problem == null && Clashes().Count > 0)
            problem = "The clips in red would land on the same file. Untick or rename until they don't. "
                + "CLASHES lists only those.";

        if (problem != null)
            EditorGUILayout.HelpBox(problem, MessageType.Warning);

        int ticked = found.Count(f => f.Ticked);
        using (new EditorGUI.DisabledScope(problem != null || ticked == 0))
        {
            if (GUILayout.Button("COPY " + ticked + " CLIPS"))
                Copy();
        }
    }

    // Every model file under the source folder and all its subfolders, and
    // every clip in each.
    void Find(string sourcePath)
    {
        found.Clear();
        prefixes.Clear();
        report = "";
        foundIn = sourcePath;

        // FindAssets looks through the subfolders on its own.  Asking for
        // everything and keeping what a ModelImporter imports is the plain
        // way to say "model files", whatever their extension.
        string[] paths = AssetDatabase.FindAssets("", new[] { sourcePath })
            .Select(AssetDatabase.GUIDToAssetPath)
            .Distinct()
            .Where(p => AssetImporter.GetAtPath(p) is ModelImporter)
            .OrderBy(p => p)
            .ToArray();

        foreach (string path in paths)
        {
            AnimationClip[] clips = AssetDatabase.LoadAllAssetsAtPath(path)
                .OfType<AnimationClip>()
                .Where(c => !c.name.StartsWith(PreviewClipStart))
                .ToArray();

            string fileName = Path.GetFileNameWithoutExtension(path);
            int at = fileName.IndexOf('@');
            string prefix = at >= 0 ? fileName.Substring(0, at + 1) : "";
            string rest = at >= 0 ? fileName.Substring(at + 1) : fileName;

            foreach (AnimationClip clip in clips)
            {
                found.Add(new Found
                {
                    ModelPath = path,
                    SubFolder = SubFolderOf(path, sourcePath),
                    Prefix = prefix,
                    Rest = rest,
                    ClipName = clip.name,
                    OnlyClip = clips.Length == 1,
                });

                if (prefix != "" && !prefixes.ContainsKey(prefix))
                    prefixes[prefix] = prefix;
            }
        }

        report = found.Count == 0
            ? "No animation clips in any model file under " + sourcePath + "."
            : "Found " + found.Count + " clips in " + paths.Length + " model files.";
    }

    void Copy()
    {
        string destinationPath = FolderPath(destination);
        List<Found> ticked = found.Where(f => f.Ticked).ToList();
        int made = 0, overwritten = 0;
        List<string> skipped = new List<string>();

        // The folders first, outside StartAssetEditing: a folder made while
        // the AssetDatabase is holding its imports isn't there yet for the
        // next one made inside it.
        foreach (string folder in ticked.Select(f => Path.GetDirectoryName(OutputPath(f)).Replace('\\', '/'))
                     .Distinct())
            MakeFolders(folder);

        AssetDatabase.StartAssetEditing();
        try
        {
            for (int i = 0; i < ticked.Count; i++)
            {
                Found f = ticked[i];
                string outPath = OutputPath(f);
                EditorUtility.DisplayProgressBar("Copying clips", outPath, (float)i / ticked.Count);

                AnimationClip clip = AssetDatabase.LoadAllAssetsAtPath(f.ModelPath)
                    .OfType<AnimationClip>()
                    .FirstOrDefault(c => c.name == f.ClipName);
                if (clip == null)
                {
                    skipped.Add(outPath + " (the clip isn't in its model file any more)");
                    continue;
                }

                // Overwriting copies into the .anim that's already there
                // rather than making a new one.  This way it keeps its GUID
                // (Unity's name for the file), and whatever already uses the
                // clip, an Animator say, still finds it.
                Object already = AssetDatabase.LoadMainAssetAtPath(outPath);
                string name = Path.GetFileNameWithoutExtension(outPath);
                if (already is AnimationClip old)
                {
                    EditorUtility.CopySerialized(clip, old);
                    old.name = name;
                    EditorUtility.SetDirty(old);
                    overwritten++;
                }
                else if (already != null)
                {
                    skipped.Add(outPath + " (something that isn't a clip is already there)");
                }
                else
                {
                    AnimationClip copy = new AnimationClip();
                    EditorUtility.CopySerialized(clip, copy);
                    copy.name = name;
                    AssetDatabase.CreateAsset(copy, outPath);
                    made++;
                }
            }
        }
        finally
        {
            AssetDatabase.StopAssetEditing();
            EditorUtility.ClearProgressBar();
        }

        AssetDatabase.SaveAssets();
        AssetDatabase.Refresh();

        report = "Copied " + (made + overwritten) + " clips into " + destinationPath + ": " + made + " new, "
            + overwritten + " overwritten.";
        if (skipped.Count > 0)
            report += "\nSkipped " + skipped.Count + ":\n" + string.Join("\n", skipped);
    }

    // The name the copy gets.  A model with one clip is named after its file,
    // with the prefix swapped: RPG-Character@Unarmed-Attack-L1.fbx becomes
    // Unarmed-Attack-L1.anim with a blank prefix.  A model holding several
    // clips can't name them all after itself, so each is named after the
    // clip, with the same prefix in front.
    string OutputName(Found f)
    {
        string prefix = f.Prefix == "" ? "" : prefixes[f.Prefix];
        string name = prefix + (f.OnlyClip && f.Rest != "" ? f.Rest : f.ClipName);

        // A clip from Blender is often "Armature|Run", and | can't be in a
        // file name on Windows.
        foreach (char bad in Path.GetInvalidFileNameChars())
            name = name.Replace(bad, '_');
        return name;
    }

    // Asset paths always use /, on every OS; it's the AssetDatabase's own
    // format, not the disk's.
    string OutputPath(Found f)
    {
        string folder = FolderPath(destination);
        if (f.SubFolder != "")
            folder += "/" + f.SubFolder;
        return folder + "/" + OutputName(f) + ".anim";
    }

    HashSet<string> Clashes()
    {
        return new HashSet<string>(found.Where(f => f.Ticked)
            .GroupBy(OutputPath)
            .Where(g => g.Count() > 1)
            .Select(g => g.Key));
    }

    string DestinationProblem()
    {
        string path = FolderPath(destination);
        if (path == null)
            return "Pick a destination folder.";

        foreach (string kept in CommittedFolders)
        {
            if (path == kept || path.StartsWith(kept + "/"))
                return path + " is committed to git, and the copies are the pack's art. Pick a folder outside "
                    + string.Join(", ", CommittedFolders) + ".";
        }
        return null;
    }

    // Assets/Packs/Unarmed/a.fbx under Assets/Packs is "Unarmed".
    static string SubFolderOf(string modelPath, string sourcePath)
    {
        string folder = Path.GetDirectoryName(modelPath).Replace('\\', '/');
        return folder.Length > sourcePath.Length ? folder.Substring(sourcePath.Length + 1) : "";
    }

    // Makes Assets/A/B/C one folder at a time, skipping the ones already there.
    static void MakeFolders(string path)
    {
        string[] parts = path.Split('/');
        string made = parts[0];
        for (int i = 1; i < parts.Length; i++)
        {
            string next = made + "/" + parts[i];
            if (!AssetDatabase.IsValidFolder(next))
                AssetDatabase.CreateFolder(made, parts[i]);
            made = next;
        }
    }

    // A folder's path, typed in or picked with BROWSE.  The folder picker
    // hands back the whole path on the disk, and the AssetDatabase only
    // knows paths starting at Assets/, so a folder outside the project's
    // Assets/ is turned away.
    string FolderField(string label, string path)
    {
        EditorGUILayout.BeginHorizontal();
        path = EditorGUILayout.TextField(label, path);
        if (GUILayout.Button("BROWSE", GUILayout.Width(70)))
        {
            string dataPath = Application.dataPath.Replace('\\', '/');
            string start = FolderPath(path) != null ? Path.GetDirectoryName(dataPath) + "/" + path : dataPath;
            string picked = EditorUtility.OpenFolderPanel(label, start, "").Replace('\\', '/');

            if (picked == dataPath)
                path = "Assets";
            else if (picked.StartsWith(dataPath + "/"))
                path = "Assets" + picked.Substring(dataPath.Length);
            else if (picked != "")
                report = picked + " isn't inside this project's Assets folder.";

            // The folder picker steals the mouse, and the text field would
            // otherwise keep showing what it had before.
            GUI.FocusControl(null);
        }
        EditorGUILayout.EndHorizontal();
        return path;
    }

    // The path tidied up, or null when it's empty or not a folder in the
    // project.
    static string FolderPath(string path)
    {
        path = path.Trim().Replace('\\', '/').TrimEnd('/');
        return AssetDatabase.IsValidFolder(path) ? path : null;
    }
}
