using System;
using Heq.Dsp;
using Heq.Storage;

namespace Heq.Model
{
    public class AbTester
    {
        private readonly EqModel _model;

        private Preset _other;

        private string _otherName;
        private double _otherLevel;
        private string _currentName;
        private double? _folderTarget;

        private bool _busy;

        public AbTester(EqModel model)
            => _model = model ?? throw new ArgumentNullException(nameof(model));

        public event EventHandler Changed;

        public bool Active => _other != null;

        public bool OnB { get; private set; }

        public string CurrentName => _currentName;

        public string AName => OnB ? _otherName : _currentName;
        public string BName => OnB ? _currentName : _otherName;

        public double? FolderTargetDb
        {
            get => _folderTarget;
            set
            {
                if (_folderTarget.Equals(value)) return;
                _folderTarget = value;
                Refresh();
                Changed?.Invoke(this, EventArgs.Empty);
            }
        }

        public void SetB(Preset preset, string name)
        {
            if (preset == null) { Clear(); return; }

            if (OnB) SwitchTo(false);

            _other = preset;
            _otherName = name;
            _otherLevel = Loudness.LevelDb(preset, _model.SampleRate);
            OnB = false;

            Refresh();
            Changed?.Invoke(this, EventArgs.Empty);
        }

        public void Clear()
        {
            if (!Active && _model.LoudnessTrimDb == 0) return;

            _other = null;
            _otherName = null;
            OnB = false;
            _model.LoudnessTrimDb = 0;
            Changed?.Invoke(this, EventArgs.Empty);
        }

        public void SwitchTo(bool toB)
        {
            if (!Active || toB == OnB || _busy) return;

            _busy = true;
            try
            {
                var parked = _other;
                string parkedName = _otherName;

                _other = Preset.FromModel(_model, OnB ? "B" : "A");
                _otherName = _currentName;
                _otherLevel = Loudness.LevelDb(_model);

                OnB = toB;
                _currentName = parkedName;
                parked.ApplyTo(_model);
            }
            finally
            {
                _busy = false;
            }

            Refresh();
            Changed?.Invoke(this, EventArgs.Empty);
        }

        public void NameCurrent(string name)
        {
            if (_currentName == name) return;
            _currentName = name;
            Changed?.Invoke(this, EventArgs.Empty);
        }

        public void Refresh()
        {
            if (_busy) return;

            _busy = true;
            try
            {
                if (!Active && !_folderTarget.HasValue)
                {
                    _model.LoudnessTrimDb = 0;
                    return;
                }

                double mine = Loudness.LevelDb(_model);
                double target = Math.Min(
                    Active ? _otherLevel : double.MaxValue,
                    _folderTarget ?? double.MaxValue);

                _model.LoudnessTrimDb = Math.Min(0, target - mine);
            }
            finally
            {
                _busy = false;
            }
        }
    }
}
