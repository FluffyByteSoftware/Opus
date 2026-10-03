// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Translator.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Turns things the game knows into words a player reads.  The first is a
// number written out, made for /who's "There are seven legends currently
// online." (its count is digits since 2026-10-03).  British, with the
// "and", in honour of Discworld (Jacob, 2026-10-02), and the short scale,
// a billion being a thousand million.
// Every int there is has words, from int.MinValue to int.MaxValue.
//
//   0              zero
//   21             twenty-one
//   104            one hundred and four
//   1001           one thousand and one
//   1100           one thousand one hundred
//   2050           two thousand and fifty
//   100005         one hundred thousand and five
//   1000001        one million and one
//   1234567        one million two hundred and thirty-four thousand five
//                  hundred and sixty-seven
//   -7             minus seven
//   2147483647     two billion one hundred and forty-seven million four
//                  hundred and eighty-three thousand six hundred and
//                  forty-seven

using System.Collections.Generic;

namespace Opus
{
    public static class Translator
    {
        static readonly string[] UnderTwenty =
        {
            "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
            "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen"
        };

        static readonly string[] Tens =
        {
            "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"
        };

        // The big groups, biggest first, each a thousand of the next.
        static readonly long[] GroupSizes = { 1000000000L, 1000000L, 1000L };
        static readonly string[] GroupNames = { "billion", "million", "thousand" };

        public static string NumberToWords(int number)
        {
            if (number == 0)
                return "zero";

            // A long, because -2147483648 has no int on the other side of
            // zero to turn it into.
            long left = number;
            string sign = "";
            if (left < 0)
            {
                sign = "minus ";
                left = -left;
            }

            var parts = new List<string>();
            for (int i = 0; i < GroupSizes.Length; i++)
            {
                if (left >= GroupSizes[i])
                {
                    parts.Add(UnderAThousand((int)(left / GroupSizes[i])) + " " + GroupNames[i]);
                    left %= GroupSizes[i];
                }
            }
            if (left > 0)
            {
                // The British "and": a last bit under a hundred, after a
                // bigger group, gets one ("one thousand and one").
                if (parts.Count > 0 && left < 100)
                    parts.Add("and " + UnderAHundred((int)left));
                else
                    parts.Add(UnderAThousand((int)left));
            }
            return sign + string.Join(" ", parts);
        }

        // 1 to 999.
        static string UnderAThousand(int number)
        {
            if (number < 100)
                return UnderAHundred(number);
            string words = UnderTwenty[number / 100] + " hundred";
            if (number % 100 > 0)
                words += " and " + UnderAHundred(number % 100);
            return words;
        }

        // 0 to 99, hyphened past twenty: "twenty-one".
        static string UnderAHundred(int number)
        {
            if (number < 20)
                return UnderTwenty[number];
            string words = Tens[number / 10];
            if (number % 10 > 0)
                words += "-" + UnderTwenty[number % 10];
            return words;
        }
    }
}
