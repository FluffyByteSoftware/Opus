// File:       Opus/Soundcheck/dev/Net/ServerCertificate.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// The one certificate this client trusts.  Conductor's is self-signed, so
// no authority vouches for it; instead Soundcheck carries a copy of it
// (conductor.crt beside the program, the same file as
// Content/certs/conductor.crt) and refuses any server that shows it
// anything else.  The name on the certificate isn't checked: the bytes are
// the check, the same as test_client.py --cert and Ensemble before it.

using System;
using System.IO;
using System.Text;

namespace Opus.Net
{
    public static class ServerCertificate
    {
        public const string FileName = "conductor.crt";

        const string Begin = "-----BEGIN CERTIFICATE-----";
        const string End = "-----END CERTIFICATE-----";

        // Where the copy sits: beside the program, wherever that is.
        public static string FilePath
        {
            get { return Path.Combine(AppContext.BaseDirectory, FileName); }
        }

        // The carried certificate's bytes (DER), read from the file beside
        // the program.  Null when the file is missing or isn't a
        // certificate; the caller says so to the player.
        public static byte[] Load()
        {
            string path = FilePath;
            if (!File.Exists(path))
                return null;
            try
            {
                return FromPem(File.ReadAllText(path));
            }
            catch (Exception)
            {
                return null;
            }
        }

        // The certificate's bytes (DER) out of its PEM text, the first one
        // in it.  Null when there's no certificate in the text.
        public static byte[] FromPem(string pem)
        {
            if (string.IsNullOrEmpty(pem))
                return null;
            int begin = pem.IndexOf(Begin, StringComparison.Ordinal);
            if (begin < 0)
                return null;
            begin += Begin.Length;
            int end = pem.IndexOf(End, begin, StringComparison.Ordinal);
            if (end < 0)
                return null;

            // Base64 over several lines; the line breaks go first.
            var base64 = new StringBuilder();
            for (int i = begin; i < end; i++)
            {
                char c = pem[i];
                if (!char.IsWhiteSpace(c))
                    base64.Append(c);
            }
            try
            {
                return Convert.FromBase64String(base64.ToString());
            }
            catch (FormatException)
            {
                return null;
            }
        }

        // Whether the server showed us exactly the certificate we carry.
        public static bool Same(byte[] shown, byte[] carried)
        {
            if (shown == null || carried == null || shown.Length != carried.Length)
                return false;
            for (int i = 0; i < shown.Length; i++)
            {
                if (shown[i] != carried[i])
                    return false;
            }
            return true;
        }
    }
}
