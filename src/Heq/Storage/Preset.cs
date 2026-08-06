using System;
using System.Collections.Generic;
using System.Linq;
using Heq.Model;

namespace Heq.Storage
{
    public class Preset
    {
        public string Name { get; set; }
        public double Preamp { get; set; }
        public bool AutoGain { get; set; } = true;

        public bool ExcludeFromLoudness { get; set; }

        public List<BandDto> Bands { get; set; } = new List<BandDto>();

        public static Preset FromModel(EqModel m, string name) => new Preset
        {
            Name = name,
            Preamp = m.PreampDb,
            AutoGain = m.AutoGain,
            Bands = m.Bands.Select(BandDto.From).ToList(),
        };

        public void ApplyTo(EqModel m)
        {
            using (m.Batch())
            {
                m.Bands.Clear();
                foreach (var d in Bands) m.Bands.Add(d.ToBand());
                m.AutoGain = AutoGain;
                m.PreampDb = Preamp;
                m.Touch();
            }
        }

        public bool Matches(EqModel m)
        {
            if (m == null || m.AutoGain != AutoGain || m.Bands.Count != Bands.Count) return false;
            if (Math.Abs(m.PreampDb - Preamp) > 1e-9) return false;

            for (int i = 0; i < Bands.Count; i++)
                if (!Bands[i].Matches(m.Bands[i])) return false;

            return true;
        }
    }
}
