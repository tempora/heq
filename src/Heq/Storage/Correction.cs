using System.Collections.Generic;
using System.Linq;
using Heq.Model;

namespace Heq.Storage
{
    public class Correction
    {
        public List<BandDto> Bands { get; set; } = new List<BandDto>();

        public bool Enabled { get; set; } = true;

        public bool IsEmpty => Bands.Count == 0;

        public bool Applies => Enabled && Bands.Any(b => b.Enabled);

        public static Correction FromModel(EqModel m)
            => new Correction { Bands = m.Bands.Select(BandDto.From).ToList() };

        public void ApplyTo(EqModel m)
        {
            using (m.Batch())
            {
                m.Bands.Clear();
                foreach (var b in ToBands()) m.Bands.Add(b);

                m.AutoGain = false;
                m.PreampDb = 0;
                m.Touch();
            }
        }

        public IEnumerable<EqBand> ToBands()
        {
            foreach (var d in Bands)
            {
                var b = d.ToBand();
                if (b.Channel == ChannelTarget.Both) b.Channel = ChannelTarget.Left;
                yield return b;
            }
        }

        public string Summary()
        {
            int left = Bands.Count(b => b.Channel != ChannelTarget.Right);
            int right = Bands.Count - left;
            return Bands.Count == 0 ? "no correction" : $"{left} left · {right} right";
        }
    }
}
