// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/WidgetRegistry.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Every widget the game has, in one list.  A new widget is one line here.

using System;
using System.Collections.Generic;

namespace Opus.Hud
{
    public static class WidgetRegistry
    {
        // Each line makes a fresh widget.  Fresh, so a HUD built again (a
        // reset, or the edit mode one day) never shares a widget with the
        // one before it.
        static readonly Func<Widget>[] Makers =
        {
            () => new HealthBarWidget(),
            () => new ChatWidget(),
            () => new MinimapWidget(),
            () => new LoginBackgroundWidget(),
            () => new LoginLogoWidget(),
            () => new LoginServerIpWidget(),
            () => new LoginServerPortWidget(),
            () => new LoginUsernameWidget(),
            () => new LoginPasswordWidget(),
            () => new LoginRememberMeWidget(),
            () => new LoginSubmitWidget(),
            () => new LoginStatusWidget(),
            () => new CharacterSelectBackgroundWidget(),
            () => new CharacterSelectListWidget(),
            () => new CharacterSelectLogOutWidget(),
            () => new CharacterSelectStatusWidget(),
            () => new CharacterSelectPlayWidget(),
            () => new CharacterSelectCreateWidget(),
            () => new CharacterSelectDeleteWidget(),
            () => new CharacterSelectResetHomeWidget(),
            () => new CharacterSelectCreateCardWidget(),
            () => new CharacterSelectDeleteCardWidget(),
        };

        // A new widget for this id, or null when there's no widget called
        // that.  Making one is cheap (Build() is where the work is), so we
        // make each and ask its id rather than keep a second list of ids.
        public static Widget Make(string id)
        {
            foreach (Func<Widget> make in Makers)
            {
                Widget widget = make();
                if (widget.Info.Id == id)
                    return widget;
            }
            return null;
        }

        // Every widget's catalog entry, in the list's order.
        public static List<WidgetInfo> All()
        {
            var all = new List<WidgetInfo>();
            foreach (Func<Widget> make in Makers)
                all.Add(make().Info);
            return all;
        }
    }
}
