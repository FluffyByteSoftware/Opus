// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/Hud/ScreenRoot.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Sits on the GameObject beside the UI Document and owns every screen: the
// start screen, character select and the HUD, and which one is showing.
// It finds a screen's layout, checks it, and hands it to HudBuilder.  The
// game starts on the start screen and, with a ticket from the launcher in
// its environment, joins the world at once, so the first thing a player
// sees is character select; without one the start screen stays.  The
// session (Opus.Net's Session) switches between the start screen,
// character select and the HUD, drawn over the game scene once the
// character is in the world.  This is where the network's threads get
// their turn on the main thread, once a frame.  The HUD's layout is the
// character's own once the player has moved, resized or locked a widget,
// and this is where it's written.  Right-click it in the Inspector for Show
// Start Screen, Show HUD and Reset HUD To Default.

using Opus.Net;
using UnityEngine;
using UnityEngine.Serialization;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    [RequireComponent(typeof(UIDocument))]
    public class ScreenRoot : MonoBehaviour
    {
        [Header("The start screen")]
        [Tooltip("The start screen's layout, Assets/Data/Layouts/start_default.json: what the game shows when it "
            + "has no ticket from the launcher, and for the moment before character select.  It ships with the "
            + "game; there's never a player's own.")]
        public TextAsset startLayout;

        [Tooltip("The start screen's look, Assets/Data/Styles/start.uss.")]
        public StyleSheet startStyle;

        // The two were Login Text Color and Login Text Font;
        // FormerlySerializedAs keeps what was set.
        [Header("The screens' words")]
        [Tooltip("The colour of every word on the start screen and character select: the names, what's typed, "
            + "the buttons.  It can be changed in Play mode and shows at once.")]
        [FormerlySerializedAs("loginTextColor")]
        public Color screenTextColor = new Color(0.91f, 0.89f, 0.84f);

        [Tooltip("The font of every word on the start screen and character select.  Empty is Unity's own.  It "
            + "can be changed in Play mode and shows at once.")]
        [FormerlySerializedAs("loginTextFont")]
        public Font screenTextFont;

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

        // Jacob's pointer pack is purchased, so it's never committed: the
        // two pictures are dragged on by hand (2026-10-03).
        [Header("The HUD's pointers")]
        [Tooltip("The pointer while the mouse is where a widget can be dragged about: Move_PremiumCursor, the "
            + "64 x 64 one, from Assets/Purchased.  Its Texture Type has to be Cursor.  Empty keeps the normal "
            + "pointer.")]
        public Texture2D movePointer;

        [Tooltip("The pixel of the Move Pointer that does the clicking, from its top-left corner.  32, 32 is "
            + "the middle of a 64 x 64.")]
        public Vector2 movePointerHotspot = new Vector2(32f, 32f);

        [Tooltip("The pointer while the mouse is on an edge a widget can be resized by: Hand2_PremiumCursor, "
            + "the fist, the 64 x 64 one, from Assets/Purchased.  Its Texture Type has to be Cursor.  Empty keeps "
            + "the normal pointer.")]
        public Texture2D gripPointer;

        [Tooltip("The pixel of the Grip Pointer that does the clicking, from its top-left corner.  32, 32 is "
            + "the middle of a 64 x 64.")]
        public Vector2 gripPointerHotspot = new Vector2(32f, 32f);

        // The game starts on the start screen.
        string showing = LayoutLoader.StartScreen;

        // The launcher's ticket is looked for once per run, not on every
        // enable: a token is good once.
        static bool ticketLookedFor;

        UIDocument document;
        PanelSettings ownSettings;
        readonly HudBuilder builder = new HudBuilder();

        // What the HUD showing was built from, for saving it back.
        PixelSize hudReference;
        string hudName;

        void OnEnable()
        {
            Session.ReachedCharacterSelect += ShowCharacterSelect;
            Session.SessionOver += SessionOver;
            Session.ReachedWorld += ShowHudInWorld;
            CharacterSelectForm.Filled += ApplyText;
            ChatWidget.Font = chatFont;
            SetPointers();
            builder.Changed += SaveHud;
            CharacterSelectForm.Listen();
            Show(showing);
            TakeTicket();
        }

        void OnDisable()
        {
            Session.ReachedCharacterSelect -= ShowCharacterSelect;
            Session.SessionOver -= SessionOver;
            Session.ReachedWorld -= ShowHudInWorld;
            CharacterSelectForm.Filled -= ApplyText;
            CharacterSelectForm.StopListening();
            builder.Changed -= SaveHud;
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
            SetPointers();
            HudPointer.Refresh();
            if (showing == LayoutLoader.HudScreen)
                ApplyChatFont();
            else
                ApplyText();
        }

        // The ticket the launcher started the game with, if it did: the
        // game joins the world straight away, and the start screen's card
        // says so until character select comes.  A ticket that doesn't hold
        // up is a warning; none at all is the start screen, saying to start
        // the game from the launcher.
        void TakeTicket()
        {
            if (ticketLookedFor)
                return;
            ticketLookedFor = true;

            string why;
            Ticket ticket = Ticket.FromEnvironment(out why);
            if (ticket != null)
            {
                Debug.Log("Game: started by the launcher, with a ticket for " + ticket.Host + ":" + ticket.UdpPort
                          + ".");
                Session.Enter(ticket);
            }
            else if (why != null)
            {
                Debug.LogWarning("Game: " + why + ", so it's the start screen.");
            }
            else
            {
                Debug.Log("Game: no ticket from the launcher, so it's the start screen.");
            }
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

        // The session is over and the game is staying open: the start
        // screen, whose card says why (it reads Session.Notice).
        void SessionOver(string why, bool trouble)
        {
            if (showing != LayoutLoader.StartScreen)
                Show(LayoutLoader.StartScreen);
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
                layout = LayoutLoader.LoadHud(hudLayout, Session.InWorldAs);
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
                layout = LayoutLoader.LoadShipped(startLayout, LayoutLoader.StartScreen, "Start Layout");
                style = startStyle;
                styleFile = "start.uss onto ScreenRoot's Start Style";
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

            builder.Build(root, LayoutChecker.Check(layout), screen, style, screen == LayoutLoader.HudScreen);
            if (screen == LayoutLoader.HudScreen)
            {
                hudReference = layout.reference;
                hudName = layout.name;
            }

            // The game's place for the keys is on the HUD only; the start
            // screen and character select are all widgets.  Placed before
            // the widgets wake, so the chat's "straight to typing" still
            // wins.
            if (screen == LayoutLoader.HudScreen)
                GameFocus.Place(builder.Screen);
            else
                ApplyText();
        }

        // The screens' colour and font, on every piece of text on the start
        // screen and character select.  Set on each one rather than once on
        // the screen, since Unity's own theme gives the text in a box and on
        // a button colours of their own, and a style set on the element
        // itself is the only thing that beats it.  Not on the HUD, which has
        // its own look.
        void ApplyText()
        {
            VisualElement screen = builder.Screen;
            if (screen == null || showing == LayoutLoader.HudScreen)
                return;

            StyleFontDefinition font = screenTextFont != null
                ? new StyleFontDefinition(screenTextFont)
                : new StyleFontDefinition(StyleKeyword.Null);
            screen.Query<TextElement>().ForEach(text =>
            {
                text.style.color = screenTextColor;
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

        // The two pointer slots, to HudPointer.
        void SetPointers()
        {
            HudPointer.MovePicture = movePointer;
            HudPointer.MoveHotspot = movePointerHotspot;
            HudPointer.GripPicture = gripPointer;
            HudPointer.GripHotspot = gripPointerHotspot;
        }

        // The player moved, resized or locked a widget, or picked a text
        // size: the HUD as it is now goes to the character's own layout.
        void SaveHud()
        {
            if (showing != LayoutLoader.HudScreen || hudReference == null)
                return;
            LayoutFile layout = LayoutLoader.HudLayoutOf(builder.Placed, hudReference, hudName);
            LayoutLoader.SaveHud(Session.InWorldAs, layout);
        }

        // The whole screen is scaled from the layout's reference to the real
        // one, the bigger of the two fitting (Expand), so a layout made on
        // any screen fits on any other and keeps its shape.  Each screen has
        // its own reference (the start screen 1920 x 1080, the HUD 2560 x
        // 1440), so it's set again on every switch.  That's on the UI
        // Document's Panel Settings, and we set it on a copy: a change to
        // the asset itself made in Play mode would stay in the asset after
        // Play stops.
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

        // Switching screens by hand, to look at one.  Play mode only:
        // outside it there's no screen.
        [ContextMenu("Show Start Screen")]
        public void ShowStartScreen()
        {
            ShowInPlay(LayoutLoader.StartScreen);
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

        // Back to the HUD's default layout: the character's file is deleted,
        // and if the HUD is showing in Play mode it's built again straight
        // away.  A settings menu will call this one day; for now it's the
        // right-click, for the character in the world.
        [ContextMenu("Reset HUD To Default")]
        public void ResetHudToDefault()
        {
            LayoutLoader.ForgetPlayerLayout(Session.InWorldAs);
            if (Application.isPlaying && isActiveAndEnabled && showing == LayoutLoader.HudScreen)
                Show(LayoutLoader.HudScreen);
        }
    }
}
