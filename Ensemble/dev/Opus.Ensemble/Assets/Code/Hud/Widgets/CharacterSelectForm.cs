// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/CharacterSelectForm.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Where character select's widgets meet the session: the list widget hands
// its box over as it's built, and it's filled from Session.Characters, now
// and every time the list changes.  Look only for now: making, deleting and
// playing a character come next.

using System;
using Opus.Net;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public static class CharacterSelectForm
    {
        // An account has three slots.
        public const int Slots = 3;

        static VisualElement list;

        // Rung after the list is filled.  ScreenRoot puts its text colour
        // and font on the new rows, since they're made after the screen.
        public static event Action Filled;

        public static void ListBuilt(VisualElement box)
        {
            list = box;
            Fill();
        }

        // ScreenRoot starts and stops the listening.  Off before on, so a
        // second Listen() never fills the list twice.
        public static void Listen()
        {
            StopListening();
            Session.CharactersChanged += Fill;
        }

        public static void StopListening()
        {
            Session.CharactersChanged -= Fill;
        }

        // The list as it stands: one row a slot, the character in it or
        // "Empty", and a line instead while it's being asked for or if the
        // server wouldn't send it.
        static void Fill()
        {
            if (list == null)
                return;
            list.Clear();

            var title = new Label("CHARACTERS");
            title.AddToClassList("characters-title");
            list.Add(title);

            if (Session.Characters == null)
            {
                var note = new Label(Session.CharactersTrouble ?? "Asking the server for your characters...");
                note.AddToClassList("characters-note");
                list.Add(note);
            }
            else
            {
                for (int slot = 1; slot <= Slots; slot++)
                    list.Add(Row(slot, InSlot(slot)));
            }

            if (Filled != null)
                Filled();
        }

        static CharacterEntry InSlot(int slot)
        {
            foreach (CharacterEntry character in Session.Characters)
            {
                if (character.Slot == slot)
                    return character;
            }
            return null;
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
            else if (!character.Playable)
            {
                // Its save wouldn't load this run.  Greyed, and the server
                // won't let it in anyway.
                words = character.Name + " (can't be played right now)";
                row.AddToClassList("character-row--unplayable");
            }
            else
            {
                words = character.Name;
            }

            var name = new Label(words);
            name.AddToClassList("character-name");
            row.Add(name);
            return row;
        }
    }
}
