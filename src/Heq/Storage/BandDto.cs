using System;
using Heq.Model;

namespace Heq.Storage
{
    public class BandDto
    {
        public FilterKind Type { get; set; }
        public double Freq { get; set; }
        public double Gain { get; set; }
        public double Q { get; set; }
        public int Slope { get; set; } = 12;
        public bool Enabled { get; set; } = true;
        public ChannelTarget Channel { get; set; }

        public static BandDto From(EqBand b) => new BandDto
        {
            Type = b.Kind,
            Freq = b.Freq,
            Gain = b.GainDb,
            Q = b.Q,
            Slope = b.SlopeDbPerOct,
            Enabled = b.Enabled,
            Channel = b.Channel,
        };

        public EqBand ToBand() => new EqBand
        {
            Kind = Type,
            Freq = Freq,
            GainDb = Gain,
            Q = Q,
            SlopeDbPerOct = Slope <= 0 ? 12 : Slope,
            Enabled = Enabled,
            Channel = Channel,
        };

        public bool Matches(EqBand b)
            => b.Kind == Type
            && b.Enabled == Enabled
            && b.Channel == Channel
            && b.SlopeDbPerOct == (Slope <= 0 ? 12 : Slope)
            && Same(b.Freq, Freq)
            && Same(b.GainDb, Gain)
            && Same(b.Q, Q);

        private static bool Same(double a, double b) => Math.Abs(a - b) <= 1e-9;
    }
}
