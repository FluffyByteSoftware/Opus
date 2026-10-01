// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/Hud/ScreenRoot.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Sits on the GameObject beside the UI Document and owns every screen: the
// login and the HUD, and which one is showing.  It finds a screen's layout,
// checks it, and hands it to HudBuilder.  Right-click it in the Inspector
// for Show Login, Show HUD and Reset HUD To Default.

using UnityEngine;
using UnityEngine.Serialization;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    [RequireComponent(typeof(UIDocument))]
    public class ScreenRoot : MonoBehaviour
    {
        [Header("The login")]
        [Tooltip("The login's layout, Assets/Data/Layouts/login_default.json.  It ships with the game; there's "
            + "never a player's own.")]
        public TextAsset loginLayout;

        [Tooltip("The login's look, Assets/Data/Styles/login.uss.")]
        public StyleSheet loginStyle;

        [Tooltip("The colour of every word on the login: the names, what's typed, the button.  It can be "
            + "changed in Play mode and shows at once.")]
        public Color loginTextColor = new Color(0.91f, 0.89f, 0.84f);

        [Tooltip("The font of every word on the login.  Empty is Unity's own.  It can be changed in Play mode "
            + "and shows at once.")]
        public Font loginTextFont;

        // The HUD's two slots were HudRoot's Default Layout and Style Sheet;
        // FormerlySerializedAs keeps what was dragged into them.
        [Header("The HUD")]
        [Tooltip("The HUD's default layout, Assets/Data/Layouts/hud_default.json.  Used whenever the player "
            + "has no layout of their own, or theirs can't be used.")]
        [FormerlySerializedAs("defaultLayout")]
        public TextAsset hudLayout;

        [Tooltip("The HUD's look, Assets/Data/Styles/hud.uss.")]
        [FormerlySerializedAs("styleSheet")]
        public StyleSheet hudStyle;

        // The game starts on the login.
        string showing = LayoutLoader.LoginScreen;

        UIDocument document;
        PanelSettings ownSettings;
        readonly HudBuilder builder = new HudBuilder();

        void OnEnable()
        {
            Show(showing);
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

        // Unity calls this when a slot changes in the Inspector, so a new
        // colour or font shows on the login straight away in Play mode.
        void OnValidate()
        {
            if (Application.isPlaying && showing == LayoutLoader.LoginScreen)
                ApplyLoginText();
        }

        void Show(string screen)
        {
            showing = screen;
            builder.Clear();
            document = GetComponent<UIDocument>();

            LayoutFile layout;
            StyleSheet style;
            string styleFile;
            if (screen == LayoutLoader.HudScreen)
            {
                layout = LayoutLoader.LoadHud(hudLayout);
                style = hudStyle;
                styleFile = "hud.uss onto ScreenRoot's Hud Style";
            }
            else
            {
                layout = LayoutLoader.LoadShipped(loginLayout, LayoutLoader.LoginScreen, "Login Layout");
                style = loginStyle;
                styleFile = "login.uss onto ScreenRoot's Login Style";
            }
            if (layout == null)
                return;
            if (!ScaleTo(layout.reference))
                return;

            VisualElement root = document.rootVisualElement;
            if (root == null)
            {
                Debug.LogError("Screens: the UI Document has nothing to build into yet.");
                return;
            }

            // The document's root covers the whole screen and lets the mouse
            // through, so the screen's layers under it are the screen's size.
            root.pickingMode = PickingMode.Ignore;
            root.style.position = Position.Absolute;
            root.style.left = 0;
            root.style.top = 0;
            root.style.right = 0;
            root.style.bottom = 0;

            if (style == null)
                Debug.LogWarning("Screens: no style sheet for the \"" + screen + "\" screen, so its widgets will "
                    + "be bare.  Drag " + styleFile + ", from Assets/Data/Styles.");

            builder.Build(root, LayoutChecker.Check(layout), screen, style);

            if (screen == LayoutLoader.LoginScreen)
                ApplyLoginText();
        }

        // The login's colour and font, on every piece of text in it.  Set on
        // each one rather than once on the screen, since Unity's own theme
        // gives the text in a box and on a button colours of their own, and
        // a style set on the element itself is the only thing that beats it.
        void ApplyLoginText()
        {
            VisualElement screen = builder.Screen;
            if (screen == null)
                return;

            StyleFontDefinition font = loginTextFont != null
                ? new StyleFontDefinition(loginTextFont)
                : new StyleFontDefinition(StyleKeyword.Null);
            screen.Query<TextElement>().ForEach(text =>
            {
                text.style.color = loginTextColor;
                text.style.unityFontDefinition = font;
            });
        }

        // The whole screen is scaled from the layout's reference to the real
        // one, the bigger of the two fitting (Expand), so a layout made on
        // any screen fits on any other and keeps its shape.  Each screen has
        // its own reference (the login 1920 x 1080, the HUD 2560 x 1440), so
        // it's set again on every switch.  That's on the UI Document's Panel
        // Settings, and we set it on a copy: a change to the asset itself
        // made in Play mode would stay in the asset after Play stops.
        bool ScaleTo(PixelSize reference)
        {
            if (ownSettings == null)
            {
                if (document.panelSettings == null)
                {
                    Debug.LogError("Screens: the UI Document has no Panel Settings.  Give it one in the "
                        + "Inspector.");
                    return false;
                }
                ownSettings = Instantiate(document.panelSettings);
                ownSettings.name = document.panelSettings.name + " (ScreenRoot's copy)";
                document.panelSettings = ownSettings;
            }

            ownSettings.scaleMode = PanelScaleMode.ScaleWithScreenSize;
            ownSettings.screenMatchMode = PanelScreenMatchMode.Expand;
            ownSettings.referenceResolution = new Vector2Int(Mathf.RoundToInt(reference.width),
                                                             Mathf.RoundToInt(reference.height));
            return true;
        }

        // Switching screens by hand, until something in the game does it
        // (SUBMIT, one day).  Play mode only: outside it there's no screen.
        [ContextMenu("Show Login")]
        public void ShowLogin()
        {
            ShowInPlay(LayoutLoader.LoginScreen);
        }

        [ContextMenu("Show HUD")]
        public void ShowHud()
        {
            ShowInPlay(LayoutLoader.HudScreen);
        }

        void ShowInPlay(string screen)
        {
            if (Application.isPlaying && isActiveAndEnabled)
                Show(screen);
            else
                Debug.Log("Screens: switching screens only works in Play mode.");
        }

        // Back to the HUD's default layout: the player's file is deleted, and
        // if the HUD is showing in Play mode it's built again straight away.
        // A settings menu will call this one day; for now it's the
        // right-click.
        [ContextMenu("Reset HUD To Default")]
        public void ResetHudToDefault()
        {
            LayoutLoader.ForgetPlayerLayout();
            if (Application.isPlaying && isActiveAndEnabled && showing == LayoutLoader.HudScreen)
                Show(LayoutLoader.HudScreen);
        }
    }
}
