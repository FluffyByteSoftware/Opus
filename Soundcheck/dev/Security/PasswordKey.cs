// File:       Opus/Soundcheck/dev/Security/PasswordKey.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Turns a username and password into the login's key, the moment SUBMIT is
// pressed, so the password itself never goes over the wire or onto the
// disk.  The key is what's sent where the password was, and what Remember
// Me keeps.  design/client-security.md is the contract: Conductor makes the
// same key from what the admin types on the Accounts tab, so every number
// here is fixed once accounts exist.  Change one and nobody can log in.
// The same file as Ensemble's; it moved here with the login.

using System;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;

namespace Opus.Security
{
    public static class PasswordKey
    {
        // Put in front of the username to make the salt, so the same
        // password on two accounts makes two different keys, and our keys
        // are never the same as another game's that hashed the same way.
        // The "v1" is there in case the recipe ever has to change.
        public const string SaltPrefix = "Opus login v1:";

        // How many times PBKDF2 goes round.  More is slower for us and for
        // anybody trying to guess the password back out of a stolen key.
        // Unity took 2852 ms over it; here it's 208 ms on Jacob's machine
        // (2026-10-02).  SUBMIT logs what it took.
        public const int Rounds = 600000;

        // 32 bytes of key, sent as 64 lowercase hex characters.
        public const int KeyBytes = 32;
        public const int KeyChars = KeyBytes * 2;

        // Makes the key.  Slow on purpose, so never call it on the window's
        // thread; MakeAsync() is the way in.
        public static string Make(string username, string password)
        {
            byte[] salt = Encoding.UTF8.GetBytes(SaltPrefix + AsciiLower(username));
            byte[] secret = Encoding.UTF8.GetBytes(password);
            byte[] key = null;
            try
            {
                key = Rfc2898DeriveBytes.Pbkdf2(secret, salt, Rounds, HashAlgorithmName.SHA256, KeyBytes);
                return Convert.ToHexStringLower(key);
            }
            finally
            {
                // The bytes we made can be wiped; the string the password
                // came in can't (C# never lets a string change), so it sits
                // in memory until it's reused.
                Array.Clear(secret, 0, secret.Length);
                if (key != null)
                    Array.Clear(key, 0, key.Length);
            }
        }

        // Makes the key on a worker thread.
        public static Task<string> MakeAsync(string username, string password)
        {
            return Task.Run(() => Make(username, password));
        }

        // Whether a string could be a key: 64 of 0-9 and a-f.  What a
        // Remember Me file is checked against before it's believed.
        public static bool LooksLikeKey(string text)
        {
            if (text == null || text.Length != KeyChars)
                return false;
            foreach (char c in text)
            {
                bool digit = c >= '0' && c <= '9';
                bool letter = c >= 'a' && c <= 'f';
                if (!digit && !letter)
                    return false;
            }
            return true;
        }

        // A username the way Conductor reads it: A to Z made lowercase and
        // nothing else touched.  ToLowerInvariant() would also change letters
        // outside A to Z, which Conductor doesn't, and the salt has to match.
        public static string AsciiLower(string text)
        {
            var lower = new StringBuilder(text.Length);
            foreach (char c in text)
                lower.Append(c >= 'A' && c <= 'Z' ? (char)(c + 32) : c);
            return lower.ToString();
        }
    }
}
