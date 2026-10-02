// File:       Opus/Soundcheck/dev/ClientVersion.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// The client's version, read off the program itself (the <Version> in
// Opus.Soundcheck.csproj), so it's written in one place.  It's what the
// Login carries, and networking.cfg's client_versions has to list it.

using System.Reflection;

namespace Opus.Soundcheck
{
    public static class ClientVersion
    {
        public static readonly string Text = Read();

        static string Read()
        {
            var attribute = Assembly.GetExecutingAssembly().GetCustomAttribute<AssemblyInformationalVersionAttribute>();
            if (attribute != null && !string.IsNullOrEmpty(attribute.InformationalVersion))
                return attribute.InformationalVersion;
            return Assembly.GetExecutingAssembly().GetName().Version.ToString(3);
        }
    }
}
