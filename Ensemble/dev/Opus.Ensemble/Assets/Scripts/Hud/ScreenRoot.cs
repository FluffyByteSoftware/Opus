// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/Hud/ScreenRoot.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Sits on the GameObject beside the UI Document and owns every screen: the
// login, character select and the HUD, and which one is showing.  It finds
// a screen's layout, checks it, and hands it to HudBuilder.  The session
// (Opus.Net's Session) switches between the login, character select and
// the HUD, drawn over the game scene once the character is in the world.
// This is where the network's threads get their turn on the main thread,
// once a frame.  Right-click it in the Inspector for Show Login, Show HUD
// and Reset HUD To Default.

using Opus.Net;
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

        [Tooltip("The colour of every word on the login and character select: the names, what's typed, the "
            + "buttons.  It can be changed in Play mode and shows at once.")]
        public Color loginTextColor = new Color(0.91f, 0.89f, 0.84f);

        [Tooltip("The font of every word on the login and character select.  Empty is Unity's own.  It can be "
            + "changed in Play mode and shows at once.")]
        public Font loginTextFont;

        [Tooltip("The server's certificate, Assets/Data/Certs/conductor_crt.txt, a copy of "
            + "Content/certs/conductor.crt.  The client trusts that server and no other.")]
        public TextAsset serverCertificate;

        [Header("Character select")]
        [Tooltip("Character select's layout, Assets/Data/Layouts/character_select_default.json.  It ships "
            + "with the game; there's never a player's own.")]
        public TextAsset characterSelectLayout;

        [Tooltip("Character select's look, Assets/Data/Styles/character_select.uss.")]
        public StyleSheet characterSelectStyle;

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

        [Tooltip("The chat box's font, on its header, its lines and the field to type in: Retro, from "
            + "Assets/Purchased/Font Nation/TTF Fonts.  It has to be monospaced, or /who's box won't line up.  "
            + "Empty is Unity's own.  It can be changed in Play mode and shows at once.")]
        public Font chatFont;

        // The game starts on the login.
        string showing = LayoutLoader.LoginScreen;

        UIDocument document;
        PanelSettings ownSettings;
        readonly HudBuilder builder = new HudBuilder();

        void OnEnable()
        {
            Session.Certificate = serverCertificate != null ? serverCertificate.text : null;
            Session.ReachedCharacterSelect += ShowCharacterSelect;
            Session.BackAtLogin += BackAtLogin;
            Session.ReachedWorld += ShowHudInWorld;
            CharacterSelectForm.Filled += ApplyText;
            ChatWidget.Font = chatFont;
            LoginForm.Listen();
            CharacterSelectForm.Listen();
            Show(showing);
        }

        void OnDisable()
        {
            Session.ReachedCharacterSelect -= ShowCharacterSelect;
            Session.BackAtLogin -= BackAtLogin;
            Session.ReachedWorld -= ShowHudInWorld;
            CharacterSelectForm.Filled -= ApplyText;
            LoginForm.StopListening();
            CharacterSelectForm.StopListening();
            builder.Clear();
        }

        void OnDestroy()
        {
            Session.Quit();
            if (ownSettings != null)
                Destroy(ownSettings);
        }

        // Whatever the network's threads have heard since the last frame.
        void Update()
        {
            MainThread.Run();
        }

        // The game closing, or Play mode stopping: a Goodbye to the server,
        // and the network's threads let go.
        void OnApplicationQuit()
        {
            Session.Quit();
        }

        // Unity calls this when a slot changes in the Inspector, so a new
        // colour or font shows straight away in Play mode.
        void OnValidate()
        {
            if (!Application.isPlaying)
                return;
            ChatWidget.Font = chatFont;
            if (showing == LayoutLoader.HudScreen)
                ApplyChatFont();
            else
                ApplyText();
        }

        void ShowCharacterSelect()
        {
            Show(LayoutLoader.CharacterSelectScreen);
        }

        // PLAY's answer: the HUD, over the game scene.
        void ShowHudInWorld()
        {
            Show(LayoutLoader.HudScreen);
        }

        // The session is over.  The login says why (LoginForm hears it too);
        // it's only built again if it isn't already showing, so a failed
        // login keeps what was typed.
        void BackAtLogin(string why, bool trouble)
        {
            if (showing != LayoutLoader.LoginScreen)
                Show(LayoutLoader.LoginScreen);
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
            else if (screen == LayoutLoader.CharacterSelectScreen)
            {
                layout = LayoutLoader.LoadShipped(characterSelectLayout, LayoutLoader.CharacterSelectScreen,
                                                  "Character Select Layout");
                style = characterSelectStyle;
                styleFile = "character_select.uss onto ScreenRoot's Character Select Style";
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

            if (screen != LayoutLoader.HudScreen)
                ApplyText();
        }

        // The login's colour and font, on every piece of text in it, and in
        // character select.  Set on each one rather than once on the screen,
        // since Unity's own theme gives the text in a box and on a button
        // colours of their own, and a style set on the element itself is the
        // only thing that beats it.  Not on the HUD, which has its own look.
        void ApplyText()
        {
            VisualElement screen = builder.Screen;
            if (screen == null || showing == LayoutLoader.HudScreen)
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

        // The Chat Font slot on the chat box as it stands; a chat box built
        // later takes it by itself.
        void ApplyChatFont()
        {
            VisualElement screen = builder.Screen;
            if (screen != null)
                screen.Query<VisualElement>(className: "widget-chat").ForEach(ChatWidget.UseFont);
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
