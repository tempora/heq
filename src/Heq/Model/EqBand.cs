using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using Heq.Dsp;

namespace Heq.Model
{
    public class EqBand : INotifyPropertyChanged
    {
        public const double MinFreq = 10.0;
        public const double MaxFreq = 24000.0;
        public const double MinQ = 0.025;
        public const double MaxQ = 40.0;
        public const double MaxGain = 30.0;

        private FilterKind _kind = FilterKind.Bell;
        private double _freq = 1000;
        private double _gainDb;
        private double _q = 1.0;
        private int _slopeDbPerOct = 12;
        private bool _enabled = true;
        private ChannelTarget _channel = ChannelTarget.Both;

        public FilterKind Kind
        {
            get => _kind;
            set
            {
                if (_kind == value) return;
                _kind = value;
                OnChanged();
                OnChanged(nameof(UsesGain));
                OnChanged(nameof(UsesSlope));
            }
        }

        public double Freq
        {
            get => _freq;
            set => Set(ref _freq, Clamp(value, MinFreq, MaxFreq));
        }

        public double GainDb
        {
            get => _gainDb;
            set => Set(ref _gainDb, Clamp(value, -MaxGain, MaxGain));
        }

        public double Q
        {
            get => _q;
            set => Set(ref _q, Clamp(value, MinQ, MaxQ));
        }

        public int SlopeDbPerOct
        {
            get => _slopeDbPerOct;
            set { if (_slopeDbPerOct != value) { _slopeDbPerOct = value; OnChanged(); } }
        }

        public bool Enabled
        {
            get => _enabled;
            set { if (_enabled != value) { _enabled = value; OnChanged(); } }
        }

        public ChannelTarget Channel
        {
            get => _channel;
            set { if (_channel != value) { _channel = value; OnChanged(); } }
        }

        public bool UsesGain => Kind.UsesGain();
        public bool UsesSlope => Kind.UsesSlope();

        public IEnumerable<Biquad> Sections(double sampleRate)
        {
            switch (Kind)
            {
                case FilterKind.Bell:
                    yield return Biquad.Peaking(Freq, sampleRate, GainDb, Q);
                    break;
                case FilterKind.LowShelf:
                    yield return Biquad.LowShelf(Freq, sampleRate, GainDb, Q);
                    break;
                case FilterKind.HighShelf:
                    yield return Biquad.HighShelf(Freq, sampleRate, GainDb, Q);
                    break;
                case FilterKind.Notch:
                    yield return Biquad.Notch(Freq, sampleRate, Q);
                    break;
                case FilterKind.BandPass:
                    yield return Biquad.BandPass(Freq, sampleRate, Q);
                    break;
                case FilterKind.AllPass:
                    yield return Biquad.AllPass(Freq, sampleRate, Q);
                    break;
                case FilterKind.LowCut:
                    foreach (var q in CutQs())
                        yield return Biquad.HighPass(Freq, sampleRate, q);
                    break;
                case FilterKind.HighCut:
                    foreach (var q in CutQs())
                        yield return Biquad.LowPass(Freq, sampleRate, q);
                    break;
            }
        }

        public double[] CutQs()
        {
            int order = Math.Max(2, SlopeDbPerOct / 6);
            return order <= 2 ? new[] { Q } : Biquad.ButterworthQs(order);
        }

        public double ResponseDb(double freqHz, double sampleRate)
        {
            if (!Enabled) return 0;

            double w = 2.0 * Math.PI * freqHz / sampleRate;
            double sum = 0;
            foreach (var s in Sections(sampleRate))
                sum += s.GainDb(w);
            return sum;
        }

        private static double Clamp(double v, double lo, double hi)
            => double.IsNaN(v) ? lo : Math.Clamp(v, lo, hi);

        private void Set(ref double field, double value, [CallerMemberName] string name = null)
        {
            if (field == value) return;
            field = value;
            OnChanged(name);
        }

        public event PropertyChangedEventHandler PropertyChanged;

        private void OnChanged([CallerMemberName] string name = null)
            => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
    }
}
