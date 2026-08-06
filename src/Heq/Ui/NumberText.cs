using System;
using System.Globalization;

namespace Heq.Ui
{
    public static class NumberText
    {
        private static readonly CultureInfo Inv = CultureInfo.InvariantCulture;

        private static readonly string[] Suffixes = { "khz", "hz", "db", "k" };

        public static bool TryParse(string s, out double value)
        {
            s = (s ?? string.Empty).Trim().Replace(',', '.');

            foreach (var suffix in Suffixes)
            {
                if (!s.EndsWith(suffix, StringComparison.OrdinalIgnoreCase)) continue;

                bool kilo = suffix == "khz" || suffix == "k";
                s = s.Substring(0, s.Length - suffix.Length).Trim();

                if (!double.TryParse(s, NumberStyles.Float, Inv, out value)) return false;
                if (kilo) value *= 1000;
                return true;
            }

            return double.TryParse(s, NumberStyles.Float, Inv, out value);
        }
    }
}
