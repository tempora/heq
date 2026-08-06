using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using Heq.Model;

namespace Heq.Apo
{
    public static class ApoFormat
    {
        private static readonly CultureInfo Inv = CultureInfo.InvariantCulture;

        public static string BuildConfig(EqModel model) => BuildConfig(model, null);

        public static string BuildConfig(EqModel model, IEnumerable<EqBand> correction)
        {
            var sb = new StringBuilder();

            double preamp = model.EffectivePreampDb;
            if (Math.Abs(preamp) > 0.001)
                sb.Append("Preamp: ").Append(Num(preamp, 1)).Append(" dB\r\n");

            var all = correction == null
                ? (IEnumerable<EqBand>)model.Bands
                : model.Bands.Concat(correction);

            var byEar = all.ToLookup(b => b.Channel);
            var left = byEar[ChannelTarget.Left].ToList();
            var right = byEar[ChannelTarget.Right].ToList();

            AppendBands(sb, byEar[ChannelTarget.Both]);
            AppendChannel(sb, "L", left);
            AppendChannel(sb, "R", right);

            if (left.Count > 0 || right.Count > 0)
                sb.Append("Channel: ALL\r\n");

            return sb.ToString();
        }

        private static void AppendChannel(StringBuilder sb, string channel, List<EqBand> bands)
        {
            if (bands.Count == 0) return;
            sb.Append("Channel: ").Append(channel).Append("\r\n");
            AppendBands(sb, bands);
        }

        private static void AppendBands(StringBuilder sb, IEnumerable<EqBand> bands)
        {
            foreach (var b in bands)
                foreach (var line in FilterLines(b))
                    sb.Append(line).Append("\r\n");
        }

        public static IEnumerable<string> FilterLines(EqBand b)
        {
            string state = b.Enabled ? "ON" : "OFF";
            string f = Num(b.Freq, 2);

            switch (b.Kind)
            {
                case FilterKind.Bell:
                    yield return $"Filter: {state} PK Fc {f} Hz Gain {Num(b.GainDb, 2)} dB Q {Num(b.Q, 4)}";
                    break;
                case FilterKind.LowShelf:
                    yield return $"Filter: {state} LSC Fc {f} Hz Gain {Num(b.GainDb, 2)} dB Q {Num(b.Q, 4)}";
                    break;
                case FilterKind.HighShelf:
                    yield return $"Filter: {state} HSC Fc {f} Hz Gain {Num(b.GainDb, 2)} dB Q {Num(b.Q, 4)}";
                    break;
                case FilterKind.Notch:
                    yield return $"Filter: {state} NO Fc {f} Hz Q {Num(b.Q, 4)}";
                    break;
                case FilterKind.BandPass:
                    yield return $"Filter: {state} BP Fc {f} Hz Q {Num(b.Q, 4)}";
                    break;
                case FilterKind.AllPass:
                    yield return $"Filter: {state} AP Fc {f} Hz Q {Num(b.Q, 4)}";
                    break;
                case FilterKind.LowCut:
                    foreach (var q in b.CutQs())
                        yield return $"Filter: {state} HPQ Fc {f} Hz Q {Num(q, 4)}";
                    break;
                case FilterKind.HighCut:
                    foreach (var q in b.CutQs())
                        yield return $"Filter: {state} LPQ Fc {f} Hz Q {Num(q, 4)}";
                    break;
            }
        }

        public static string Num(double v, int decimals)
        {
            if (double.IsNaN(v) || double.IsInfinity(v)) v = 0;
            string s = Math.Round(v, decimals).ToString("0." + new string('#', decimals), Inv);
            return s == "-0" ? "0" : s;
        }

        // reading

        public class ParseResult
        {
            public List<EqBand> Bands { get; } = new List<EqBand>();
            public double? Preamp { get; set; }
            public List<string> Warnings { get; } = new List<string>();
        }

        public static ParseResult Parse(string text)
        {
            var result = new ParseResult();
            if (string.IsNullOrWhiteSpace(text)) return result;

            var channel = ChannelTarget.Both;

            foreach (var raw in text.Split('\n'))
            {
                string line = raw.Trim().TrimEnd('\r').Trim();
                if (line.Length == 0 || line.StartsWith("#")) continue;

                if (line.StartsWith("Channel:", StringComparison.OrdinalIgnoreCase))
                {
                    string v = line.Substring(8).Trim().ToUpperInvariant();
                    if (v == "L" || v == "1" || v == "LEFT") channel = ChannelTarget.Left;
                    else if (v == "R" || v == "2" || v == "RIGHT") channel = ChannelTarget.Right;
                    else channel = ChannelTarget.Both;
                    continue;
                }

                if (line.StartsWith("Preamp:", StringComparison.OrdinalIgnoreCase))
                {
                    var tok = Tokens(line.Substring(7));
                    if (tok.Count > 0 && TryNum(tok[0], out double p)) result.Preamp = p;
                    continue;
                }

                int colon = line.IndexOf(':');
                if (colon < 0) continue;
                if (!line.Substring(0, colon).Trim().StartsWith("Filter", StringComparison.OrdinalIgnoreCase)) continue;

                var band = ParseFilter(line.Substring(colon + 1), result.Warnings);
                if (band != null)
                {
                    band.Channel = channel;
                    result.Bands.Add(band);
                }
            }

            return result;
        }

        private static EqBand ParseFilter(string body, List<string> warnings)
        {
            var t = Tokens(body);
            if (t.Count == 0) return null;

            int i = 0;
            bool enabled = true;
            if (t[i].Equals("ON", StringComparison.OrdinalIgnoreCase)) i++;
            else if (t[i].Equals("OFF", StringComparison.OrdinalIgnoreCase)) { enabled = false; i++; }

            if (i >= t.Count) return null;
            string type = t[i++].ToUpperInvariant();

            if (type == "NONE") return null; // AutoEq pads unused slots with these

            double? forcedQ = null;
            if ((type == "LS" || type == "HS") && i < t.Count && t[i].EndsWith("dB", StringComparison.OrdinalIgnoreCase))
            {
                forcedQ = t[i].StartsWith("6", StringComparison.Ordinal) ? 0.5 : 0.7071;
                i++;
            }

            double fc = 1000, gain = 0, q = double.NaN, bw = double.NaN;

            for (; i < t.Count; i++)
            {
                string k = t[i].ToUpperInvariant();
                if (k == "FC" && i + 1 < t.Count && TryNum(t[i + 1], out double fv)) { fc = fv; i++; }
                else if (k == "GAIN" && i + 1 < t.Count && TryNum(t[i + 1], out double gv)) { gain = gv; i++; }
                else if (k == "Q" && i + 1 < t.Count && TryNum(t[i + 1], out double qv)) { q = qv; i++; }
                else if (k == "BW" && i + 2 < t.Count && TryNum(t[i + 2], out double bv)) { bw = bv; i += 2; }
            }

            if (double.IsNaN(q))
                q = !double.IsNaN(bw) ? BandwidthToQ(bw) : (forcedQ ?? 0.7071);

            FilterKind kind;
            switch (type)
            {
                case "PK": case "PEQ": case "MODAL": kind = FilterKind.Bell; break;
                case "LS": case "LSC": case "LSQ": kind = FilterKind.LowShelf; break;
                case "HS": case "HSC": case "HSQ": kind = FilterKind.HighShelf; break;
                case "HP": case "HPQ": kind = FilterKind.LowCut; break;
                case "LP": case "LPQ": kind = FilterKind.HighCut; break;
                case "NO": kind = FilterKind.Notch; break;
                case "BP": kind = FilterKind.BandPass; break;
                case "AP": kind = FilterKind.AllPass; break;
                default:
                    warnings.Add($"Skipped unsupported filter type '{type}'.");
                    return null;
            }

            if (type == "HP" || type == "LP") q = 0.7071; // plain HP/LP are Butterworth

            return new EqBand
            {
                Kind = kind,
                Freq = fc,
                GainDb = kind.UsesGain() ? gain : 0,
                Q = q,
                Enabled = enabled,
                SlopeDbPerOct = 12,
            };
        }

        public static double BandwidthToQ(double bwOctaves)
        {
            if (bwOctaves <= 0) return 0.7071;
            double p = Math.Pow(2, bwOctaves);
            return Math.Sqrt(p) / (p - 1);
        }

        private static List<string> Tokens(string s)
            => s.Split(new[] { ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries).ToList();

        private static bool TryNum(string s, out double v)
        {
            s = s.Trim().Replace(',', '.');
            return double.TryParse(s, NumberStyles.Float, Inv, out v);
        }
    }
}
