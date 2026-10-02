// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectForm.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Where character select's widgets meet the session.  Each widget hands
// over what it built (the list, the status line, the four buttons, the
// two cards), and this fills them from Session, now and every time
// something there changes.  A click on a row picks that character; PLAY,
// DELETE and RESET HOME act on the one picked, CREATE and DELETE through a
// card.  Once the character is in the world the list says so, and LOG OUT
// is the only way on.

using System;
using Opus.Net;
using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public static class CharacterSelectForm
    {
        // An account has three slots.
        public const int Slots = 3;

        enum Card
        {
            None,
            Create,
            Delete,
        }

        static VisualElement list;
        static VisualElement statusBox;
        static Label statusLine;
        static Button play;
        static Button create;
        static Button delete;
        static Button resetHome;

        static VisualElement createCard;
        static TextField createName;
        static Button createMake;

        static VisualElement deleteCard;
        static Label deleteLine;
        static TextField deleteWord;
        static Button deleteConfirm;

        // The picked character's uuid, or null.
        static string picked;

        // The card that's open, and for DELETE the character it's for (the
        // pick can't move under an open card, but the uuid is kept anyway).
        static Card open = Card.None;
        static string deleting;

        // Rung after the screen is filled.  ScreenRoot puts its text colour
        // and font on the new rows, since they're made after the screen.
        public static event Action Filled;

        // ---------------------------------------------------------------
        // The widgets, as they're built
        // ---------------------------------------------------------------

        // The list comes first in the layout, so a fresh character select
        // starts here with nothing picked and no card open.
        public static void ListBuilt(VisualElement box)
        {
            list = box;
            picked = null;
            open = Card.None;
            deleting = null;
            Fill();
        }

        public static void StatusBuilt(VisualElement box, Label line)
        {
            statusBox = box;
            statusLine = line;
            Fill();
        }

        public static void PlayBuilt(Button button)
        {
            play = button;
            Fill();
        }

        public static void CreateBuilt(Button button)
        {
            create = button;
            Fill();
        }

        public static void DeleteBuilt(Button button)
        {
            delete = button;
            Fill();
        }

        public static void ResetHomeBuilt(Button button)
        {
            resetHome = button;
            Fill();
        }

        public static void CreateCardBuilt(VisualElement box, TextField name, Button make)
        {
            createCard = box;
            createName = name;
            createMake = make;
            name.RegisterCallback<KeyDownEvent>(e => CardKeys(e, Make), TrickleDown.TrickleDown);
            Fill();
        }

        public static void DeleteCardBuilt(VisualElement box, Label line, TextField word, Button confirm)
        {
            deleteCard = box;
            deleteLine = line;
            deleteWord = word;
            deleteConfirm = confirm;
            word.RegisterCallback<KeyDownEvent>(e => CardKeys(e, ConfirmDelete), TrickleDown.TrickleDown);
            Fill();
        }

        // ScreenRoot starts and stops the listening.  Off before on, so a
        // second Listen() never fills the screen twice.
        public static void Listen()
        {
            StopListening();
            Session.CharacterSelectChanged += Fill;
            Session.AskAnswered += Answered;
        }

        public static void StopListening()
        {
            Session.CharacterSelectChanged -= Fill;
            Session.AskAnswered -= Answered;
        }

        // ---------------------------------------------------------------
        // The buttons
        // ---------------------------------------------------------------

        public static void Play()
        {
            if (picked != null)
                Session.Play(picked);
        }

        public static void ResetHome()
        {
            if (picked != null)
                Session.ResetHome(picked);
        }

        public static void OpenCreate()
        {
            if (createCard == null)
                return;
            open = Card.Create;
            createName.value = "";
            Fill();
            FocusSoon(createName);
        }

        public static void OpenDelete()
        {
            CharacterEntry character = Session.Find(picked);
            if (character == null || deleteCard == null)
                return;
            open = Card.Delete;
            deleting = character.Uuid;
            deleteLine.text = "Type " + Session.DeleteWord + " to delete " + character.Name + ".  It can't be "
                              + "undone.";
            deleteWord.value = "";
            Fill();
            FocusSoon(deleteWord);
        }

        // MAKE on the create card, or Enter in its box.  The name rule is
        // checked before anything's sent (Session.CreateCharacter).
        public static void Make()
        {
            if (open == Card.Create && createName != null)
                Session.CreateCharacter(createName.value.Trim());
        }

        // DELETE on the delete card, or Enter in its box.
        public static void ConfirmDelete()
        {
            if (open == Card.Delete && deleting != null && deleteWord != null)
                Session.DeleteCharacter(deleting, deleteWord.value);
        }

        public static void CloseCard()
        {
            open = Card.None;
            deleting = null;
            Fill();
        }

        static void Pick(string uuid)
        {
            // Not while a card is open: the card is about what was picked.
            if (open != Card.None)
                return;
            picked = uuid;
            Fill();
        }

        // Enter does the card's button, Escape closes it.  Caught on the way
        // down (TrickleDown), before the text box does anything of its own.
        static void CardKeys(KeyDownEvent e, Action confirm)
        {
            if (e.keyCode == KeyCode.Return || e.keyCode == KeyCode.KeypadEnter)
            {
                confirm();
                e.StopPropagation();
            }
            else if (e.keyCode == KeyCode.Escape)
            {
                CloseCard();
                e.StopPropagation();
            }
        }

        // The box can only take the keyboard once it's showing, which is
        // after the next layout pass.
        static void FocusSoon(TextField field)
        {
            field.schedule.Execute(() => field.Focus());
        }

        // A card closes on a yes from the server.  On a no it stays open, the
        // why on the status line, so the player can fix it and try again.
        static void Answered(byte kind, bool done)
        {
            if (!done)
                return;
            if (kind == Protocol.CreateCharacter && open == Card.Create)
            {
                open = Card.None;
            }
            else if (kind == Protocol.DeleteCharacter && open == Card.Delete)
            {
                open = Card.None;
                deleting = null;
                picked = null;
            }
        }

        // ---------------------------------------------------------------
        // Filling it in
        // ---------------------------------------------------------------

        static void Fill()
        {
            bool atSelect = Session.Stage == SessionStage.AtCharacterSelect;
            if (!atSelect)
                open = Card.None;
            // A pick that's gone from the list (deleted) is let go.
            if (picked != null && Session.Find(picked) == null)
                picked = null;

            FillList();
            FillStatus();
            FillButtons(atSelect);
            FillCards();

            if (Filled != null)
                Filled();
        }

        // The list as it stands: one row a slot, the character in it or
        // "Empty", and a line instead while it's being asked for, if the
        // server wouldn't send it, or once the character is in the world.
        static void FillList()
        {
            if (list == null)
                return;
            list.Clear();

            if (Session.Stage == SessionStage.InWorld)
            {
                var inWorld = new Label("In the world as " + Session.InWorldAs);
                inWorld.AddToClassList("characters-in-world");
                list.Add(inWorld);
                return;
            }

            var title = new Label("CHARACTERS");
            title.AddToClassList("characters-title");
            list.Add(title);

            if (Session.Characters == null)
            {
                var note = new Label(Session.CharactersTrouble ?? "Asking the server for your characters...");
                note.AddToClassList("characters-note");
                list.Add(note);
                return;
            }

            for (int slot = 1; slot <= Slots; slot++)
                list.Add(Row(slot, Session.InSlot(slot)));
        }

        static VisualElement Row(int slot, CharacterEntry character)
        {
            var row = new VisualElement();
            row.AddToClassList("character-row");

            var number = new Label(slot.ToString());
            number.AddToClassList("character-slot");
            row.Add(number);

            string words;
            if (character == null)
            {
                words = "Empty";
                row.AddToClassList("character-row--empty");
            }
            else
            {
                // An unplayable one can still be picked, to DELETE it; PLAY
                // and RESET HOME stay grey for it, and the server won't let it
                // in anyway.
                words = character.Playable ? character.Name : character.Name + " (can't be played right now)";
                if (!character.Playable)
                    row.AddToClassList("character-row--unplayable");
                if (character.Uuid == picked)
                    row.AddToClassList("character-row--picked");
                row.AddToClassList("character-row--pickable");
                string uuid = character.Uuid;
                row.RegisterCallback<ClickEvent>(e => Pick(uuid));
            }

            var name = new Label(words);
            name.AddToClassList("character-name");
            row.Add(name);
            return row;
        }

        static void FillStatus()
        {
            if (statusLine == null)
                return;
            string said = Session.Said ?? "";
            statusLine.text = said;
            statusBox.EnableInClassList("character-select-status--trouble", Session.SaidTrouble && said != "");
        }

        // The buttons only show at character select.  They're grey while an
        // ask is out (the server takes one at a time) and while a card is
        // open; PLAY and RESET HOME need a playable character picked, DELETE
        // any character, and CREATE only shows while there's an empty slot.
        static void FillButtons(bool atSelect)
        {
            bool free = atSelect && !Session.Waiting && open == Card.None && Session.Characters != null;
            CharacterEntry character = Session.Find(picked);
            bool playable = character != null && character.Playable;
            bool room = Session.Characters != null && Session.Characters.Length < Slots;

            Set(play, atSelect, free && playable);
            Set(resetHome, atSelect, free && playable);
            Set(delete, atSelect, free && character != null);
            Set(create, atSelect && room, free);
        }

        static void Set(Button button, bool shown, bool enabled)
        {
            if (button == null)
                return;
            button.style.display = shown ? DisplayStyle.Flex : DisplayStyle.None;
            button.SetEnabled(enabled);
        }

        static void FillCards()
        {
            if (createCard != null)
            {
                createCard.style.display = open == Card.Create ? DisplayStyle.Flex : DisplayStyle.None;
                createMake.SetEnabled(!Session.Waiting);
            }
            if (deleteCard != null)
            {
                deleteCard.style.display = open == Card.Delete ? DisplayStyle.Flex : DisplayStyle.None;
                deleteConfirm.SetEnabled(!Session.Waiting);
            }
        }
    }
}
