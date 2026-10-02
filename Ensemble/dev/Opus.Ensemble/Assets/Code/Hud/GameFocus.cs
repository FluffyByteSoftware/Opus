// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/GameFocus.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The game's place-holder for the keyboard's focus (Jacob, 2026-10-02: "can
// we build a focus place holder for the game?").  UI Toolkit's focus is
// either on a widget or on nothing, and on nothing Unity's runtime panel
// hands it back to the last widget on the next key, so a chat field that
// let go of the keys got them back at once.  This is an invisible element
// on the HUD that holds the focus whenever no widget does: the keys are the
// game's then.  Enter and "/" pressed on it are the way into the chat
// (Jacob: "add enter as a key on the other focus and when its played just
// make it shift"), said through EnterPressed and SlashPressed.  Movement,
// when it comes, is on while this has the focus and off while it doesn't
// (Taken, Lost), so typing in chat never moves you.

using System;
using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public static class GameFocus
    {
        static VisualElement element;

        // The game got the keys, or a widget took them.  For movement.
        public static event Action Taken;
        public static event Action Lost;

        // Enter, or "/", pressed while the game has the keys: the chat
        // takes them.
        public static event Action EnterPressed;
        public static event Action SlashPressed;

        // Whether the game has the keys right now.
        public static bool Has
        {
            get
            {
                return element != null && element.panel != null
                    && element.panel.focusController.focusedElement == element;
            }
        }

        // Puts the place-holder on a freshly built HUD, and gives the game
        // the keys to start with.  The old one goes with the old screen.
        public static void Place(VisualElement screen)
        {
            element = new VisualElement();
            element.name = "game-focus";
            element.focusable = true;
            element.pickingMode = PickingMode.Ignore;
            element.style.position = Position.Absolute;
            element.style.width = 0;
            element.style.height = 0;
            element.RegisterCallback<FocusInEvent>(e => { if (Taken != null) Taken(); });
            element.RegisterCallback<FocusOutEvent>(e => { if (Lost != null) Lost(); });
            element.RegisterCallback<KeyDownEvent>(KeyDown);
            // Enter off a text field reaches the panel as Unity's "submit"
            // navigation event, with or without a key event of its own
            // (the key event alone never came, 2026-10-02).
            element.RegisterCallback<NavigationSubmitEvent>(Submit);
            screen.Add(element);
            Take();
        }

        static void KeyDown(KeyDownEvent e)
        {
            if (e.keyCode == KeyCode.Return || e.keyCode == KeyCode.KeypadEnter
                || e.character == '\n' || e.character == '\r')
            {
                e.StopPropagation();
                if (EnterPressed != null)
                    EnterPressed();
            }
            else if (e.character == '/')
            {
                e.StopPropagation();
                if (SlashPressed != null)
                    SlashPressed();
            }
        }

        static void Submit(NavigationSubmitEvent e)
        {
            e.StopPropagation();
            if (EnterPressed != null)
                EnterPressed();
        }

        // The game takes the keys: a widget let go of them.  Nothing happens
        // when the HUD isn't up.
        public static void Take()
        {
            if (element != null && element.panel != null)
                element.Focus();
        }
    }
}
