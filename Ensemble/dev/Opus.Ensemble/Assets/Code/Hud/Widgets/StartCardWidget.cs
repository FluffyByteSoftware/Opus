// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/StartCardWidget.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The card on the start screen: what the game has to say before character
// select, and QUIT.  A game started by hand (not by the launcher) is told
// to start from the launcher; a game the launcher started says "Joining
// the world..." for the moment before character select; a session that
// ended with the game still open says why.  Trouble is a dark red band
// behind the words.
//
// In the editor the card is also dev mode: there's no launcher to start
// the game, so once a second it looks for a fresh ticket in Soundcheck's
// debug file (Net/Ticket.cs) and joins the world the moment one lands.
// A built game never does this.

using Opus.Net;
using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public class StartCardWidget : Widget
    {
        public const string StartFromTheLauncher = "Start Forgotten Legends from the launcher.";

        static readonly WidgetInfo info = new WidgetInfo
        {
            Id = "start_card",
            DisplayName = "Start Card",
            Description = "What the game has to say before character select, and QUIT.",
            Color = "#3A4A6A",
            Screens = new[] { "start" },
            DefaultSize = new Vector2(933f, 347f),
            MinSize = new Vector2(533f, 213f),
            Resizable = true,
            DefaultAnchor = Anchor.Center,
        };

        public override WidgetInfo Info { get { return info; } }

        Label line;

        public override void Build(VisualElement box)
        {
            line = new Label();
            line.AddToClassList("start-line");

            var quit = new Button(Session.CloseTheGame);
            quit.text = "QUIT";
            quit.AddToClassList("start-quit");

            box.Add(line);
#if UNITY_EDITOR
            var dev = new Label("DEV MODE: waiting for a ticket from Soundcheck (--debug)...");
            dev.AddToClassList("start-dev");
            box.Add(dev);
            box.schedule.Execute(LookForDevTicket).Every(1000);
#endif
            box.Add(quit);

            // Filled now and on every change, for as long as the card is on
            // screen: a card from a screen built earlier lets go.
            Fill();
            Session.NoticeChanged += Fill;
            box.RegisterCallback<DetachFromPanelEvent>(e => Session.NoticeChanged -= Fill);
        }

        void Fill()
        {
            bool said = !string.IsNullOrEmpty(Session.Notice);
            line.text = said ? Session.Notice : StartFromTheLauncher;
            line.EnableInClassList("start-line--trouble", said && Session.NoticeTrouble);
        }

#if UNITY_EDITOR
        // What was last complained about, so a broken file is said once,
        // not once a second.
        static string lastWhy;

        void LookForDevTicket()
        {
            if (Session.Busy)
                return;
            string why;
            Ticket ticket = Ticket.FromDevFile(out why);
            if (ticket == null)
            {
                if (why != null && why != lastWhy)
                {
                    Debug.LogWarning("Dev mode: " + why + ".");
                    lastWhy = why;
                }
                return;
            }
            lastWhy = null;
            Debug.Log("Dev mode: a fresh ticket from Soundcheck, for " + ticket.Host + ":" + ticket.UdpPort + ".");
            Session.Enter(ticket);
        }
#endif
    }
}
