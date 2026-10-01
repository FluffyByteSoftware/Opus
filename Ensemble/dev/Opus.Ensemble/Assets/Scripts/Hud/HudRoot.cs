// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/Hud/HudRoot.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Sits on the GameObject beside the UI Document and builds the HUD into it:
// finds the layout, checks it, and hands it to HudBuilder.  Right-click it
// in the Inspector for Reset HUD To Default.

using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    [RequireComponent(typeof(UIDocument))]
    public class HudRoot : MonoBehaviour
    {
        [Tooltip("The HUD's default layout, Assets/Data/Layouts/hud_default.json.  Used whenever the player "
            + "has no layout of their own, or theirs can't be used.")]
        public TextAsset defaultLayout;

        [Tooltip("The HUD's look, Assets/Data/Styles/hud.uss.")]
        public StyleSheet styleSheet;

        UIDocument document;
        PanelSettings ownSettings;
        readonly HudBuilder builder = new HudBuilder();

        void OnEnable()
        {
            Build();
        }

        void OnDisable()
        {
            builder.Clear();
        }

        void OnDestroy()
        {
            if (ownSettings != null)
                Destroy(ownSettings);
        }

        void Build()
        {
            document = GetComponent<UIDocument>();
            LayoutFile layout = LayoutLoader.LoadHud(defaultLayout);
            if (layout == null)
                return;
            if (!ScaleTo(layout.reference))
                return;

            VisualElement root = document.rootVisualElement;
            if (root == null)
            {
                Debug.LogError("HUD: the UI Document has nothing to build into yet.");
                return;
            }

            // The document's root covers the whole screen and lets the mouse
            // through, so the HUD's layers under it are the screen's size.
            root.pickingMode = PickingMode.Ignore;
            root.style.position = Position.Absolute;
            root.style.left = 0;
            root.style.top = 0;
            root.style.right = 0;
            root.style.bottom = 0;

            if (styleSheet == null)
                Debug.LogWarning("HUD: no style sheet on HudRoot, so the widgets will be bare.  Drag hud.uss "
                    + "from Assets/Data/Styles onto its Style Sheet.");
            else if (!root.styleSheets.Contains(styleSheet))
                root.styleSheets.Add(styleSheet);

            builder.Build(root, LayoutChecker.Check(layout));
        }

        // The whole screen is scaled from the layout's reference to the real
        // one, the bigger of the two fitting (Expand), so a layout made on
        // any screen fits on any other and keeps its shape.  That's set on
        // the UI Document's Panel Settings, and we set it on a copy: a change
        // to the asset itself made in Play mode would stay in the asset after
        // Play stops.
        bool ScaleTo(PixelSize reference)
        {
            if (ownSettings == null)
            {
                if (document.panelSettings == null)
                {
                    Debug.LogError("HUD: the UI Document has no Panel Settings.  Give it one in the Inspector.");
                    return false;
                }
                ownSettings = Instantiate(document.panelSettings);
                ownSettings.name = document.panelSettings.name + " (HUD's copy)";
                document.panelSettings = ownSettings;
            }

            ownSettings.scaleMode = PanelScaleMode.ScaleWithScreenSize;
            ownSettings.screenMatchMode = PanelScreenMatchMode.Expand;
            ownSettings.referenceResolution = new Vector2Int(Mathf.RoundToInt(reference.width),
                                                             Mathf.RoundToInt(reference.height));
            return true;
        }

        // Back to the default layout: the player's file is deleted, and in
        // Play mode the HUD is built again straight away.  A settings menu
        // will call this one day; for now it's the right-click.
        [ContextMenu("Reset HUD To Default")]
        public void ResetToDefault()
        {
            LayoutLoader.ForgetPlayerLayout();
            if (Application.isPlaying && isActiveAndEnabled)
                Build();
        }
    }
}
