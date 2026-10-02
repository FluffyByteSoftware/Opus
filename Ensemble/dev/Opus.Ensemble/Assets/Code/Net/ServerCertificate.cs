// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/ServerCertificate.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The one certificate this client trusts.  Conductor's is self-signed, so
// no authority vouches for it; instead the client carries a copy of it
// (Assets/Data/Certs/conductor_crt.txt, the same file as
// Content/certs/conductor.crt) and refuses any server that shows it
// anything else.  The name on the certificate isn't checked: the bytes are
// the check, the same as test_client.py --cert.

using System;
using System.Text;

namespace Opus.Net
{
    public static class ServerCertificate
    {
        const string Begin = "-----BEGIN CERTIFICATE-----";
        const string End = "-----END CERTIFICATE-----";

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
