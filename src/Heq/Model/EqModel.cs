using System;
using System.Collections.ObjectModel;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Linq;
using System.Runtime.CompilerServices;

namespace Heq.Model
{
    public class EqModel : INotifyPropertyChanged
    {
        private double _preampDb;
        private bool _autoGain = true;
        private double _sampleRate = 48000;
        private double _loudnessTrimDb;

        private int _hold;
        private bool _pending;

        private int _revision;
        private int _peakRevision = -1;
        private double _peak;

        public ObservableCollection<EqBand> Bands { get; } = new ObservableCollection<EqBand>();

        public EqModel() => Bands.CollectionChanged += OnBandsChanged;

        public double SampleRate
        {
            get => _sampleRate;
            set { if (_sampleRate != value) { _sampleRate = value; OnChanged(); Touch(); } }
        }

        public double PreampDb
        {
            get => _preampDb;
            set
            {
                double v = Math.Clamp(value, -30, 30);
                if (_preampDb == v) return;
                _preampDb = v;
                OnChanged();
                Touch();
            }
        }

        public bool AutoGain
        {
            get => _autoGain;
            set
            {
                if (_autoGain == value) return;
                _autoGain = value;
                OnChanged();
                OnChanged(nameof(EffectivePreampDb));
                Touch();
            }
        }

        public double BasePreampDb => AutoGain ? AutoPreampDb() : PreampDb;

        public double LoudnessTrimDb
        {
            get => _loudnessTrimDb;
            set
            {
                double v = Math.Clamp(double.IsNaN(value) ? 0 : value, -30, 0);
                if (_loudnessTrimDb == v) return;
                _loudnessTrimDb = v;
                OnChanged();
                Touch();
            }
        }

        public double EffectivePreampDb => Math.Clamp(BasePreampDb + LoudnessTrimDb, -60, 30);

        public EqBand AddBand(FilterKind kind, double freq, double gainDb, double q)
        {
            var b = new EqBand { Kind = kind, Freq = freq, GainDb = gainDb, Q = q };
            Bands.Add(b);
            return b;
        }

        public void RemoveBand(EqBand b)
        {
            if (b != null) Bands.Remove(b);
        }

        public void Clear() => Bands.Clear();

        public double ResponseDb(double freqHz, ChannelTarget channel = ChannelTarget.Both)
        {
            double sum = 0;
            for (int i = 0; i < Bands.Count; i++)
            {
                var b = Bands[i];
                if (!b.Enabled || !AppliesTo(b.Channel, channel)) continue;
                sum += b.ResponseDb(freqHz, SampleRate);
            }
            return sum;
        }

        private static bool AppliesTo(ChannelTarget band, ChannelTarget asked)
            => band == ChannelTarget.Both || band == asked;

        public bool HasPerChannelBands => Bands.Any(b => b.Enabled && b.Channel != ChannelTarget.Both);

        private double PeakDb()
        {
            if (_peakRevision != _revision)
            {
                _peak = SweepPeakDb();
                _peakRevision = _revision;
            }
            return _peak;
        }

        private double SweepPeakDb()
        {
            const int steps = 512;
            bool split = HasPerChannelBands;
            double peak = 0;

            for (int i = 0; i <= steps; i++)
            {
                double f = FreqMin * Math.Pow(FreqSpan, i / (double)steps);
                double v = ResponseDb(f);
                if (split)
                {
                    v = Math.Max(v, ResponseDb(f, ChannelTarget.Left));
                    v = Math.Max(v, ResponseDb(f, ChannelTarget.Right));
                }
                if (v > peak) peak = v;
            }
            return peak;
        }

        private const double FreqMin = 20.0;
        private const double FreqSpan = 1000.0; // 20 Hz to 20 kHz

        private double AutoPreampDb()
        {
            double peak = PeakDb();
            return peak <= 0 ? 0 : -Math.Round(peak + 0.2, 1); // headroom for inter-sample peaks
        }

        public event EventHandler Changed;

        public IDisposable Batch()
        {
            _hold++;
            return new Scope(EndBatch);
        }

        private void EndBatch()
        {
            if (_hold == 0 || --_hold > 0 || !_pending) return;
            _pending = false;
            Touch();
        }

        public void Touch()
        {
            _revision++;

            if (_hold > 0) { _pending = true; return; }

            OnChanged(nameof(EffectivePreampDb));
            Changed?.Invoke(this, EventArgs.Empty);
        }

        private void OnBandsChanged(object sender, NotifyCollectionChangedEventArgs e)
        {
            if (e.NewItems != null)
                foreach (EqBand b in e.NewItems)
                    b.PropertyChanged += OnBandPropertyChanged;
            if (e.OldItems != null)
                foreach (EqBand b in e.OldItems)
                    b.PropertyChanged -= OnBandPropertyChanged;
            Touch();
        }

        private void OnBandPropertyChanged(object sender, PropertyChangedEventArgs e) => Touch();

        public event PropertyChangedEventHandler PropertyChanged;

        private void OnChanged([CallerMemberName] string name = null)
            => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
    }
}
