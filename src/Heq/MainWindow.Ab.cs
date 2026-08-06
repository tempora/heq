using System.Windows;
using Heq.Storage;

namespace Heq
{
    public partial class MainWindow
    {
        private PresetRef _abSource;

        private bool _abSwitching;

        private void RestoreB()
        {
            if (string.IsNullOrEmpty(_settings.AbFolder) || string.IsNullOrEmpty(_settings.AbPreset)) return;

            var p = PresetStore.Load(_settings.AbFolder, _settings.AbPreset);
            if (p == null) return;

            _abSource = new PresetRef(_settings.AbFolder, _settings.AbPreset);
            _ab.SetB(p, _settings.AbPreset);
            RefreshPresets();
        }

        private void SetB(string folder, string name)
        {
            if (string.IsNullOrEmpty(name)) { ClearB(); return; }

            var p = PresetStore.Load(folder, name);
            if (p == null) { CouldNotRead(name); return; }

            _abSource = new PresetRef(folder, name);
            _ab.SetB(p, name);
            RefreshPresets();
        }

        private void ReloadB()
        {
            if (_abSource == null) return;

            var p = PresetStore.Load(_abSource);
            if (p == null) ClearB();
            else _ab.SetB(p, _abSource.Name);
        }

        private void ClearB()
        {
            _abSource = null;
            _ab.Clear();
            RefreshPresets();
        }

        private void SwitchAb(bool toB)
        {
            if (!_ab.Active)
            {
                UpdateAbUi();
                return;
            }

            _abSwitching = true;
            try
            {
                _ab.SwitchTo(toB);
                Curve.SelectedBand = null;

                _currentPreset = _ab.CurrentName;
                SetBaseline(_folder, _currentPreset == null
                    ? null
                    : PresetStore.Load(_folder, _currentPreset));

                RefreshPresets();
            }
            finally
            {
                _abSwitching = false;
            }

            UpdateAbUi();
        }

        private void UpdateAbUi()
        {
            if (_abSwitching) return;

            using (Suppressed())
            {
                AbToggle.Opacity = _ab.Active ? 1.0 : 0.4;
                AbToggle.IsChecked = _ab.OnB;

                VisualStateManager.GoToState(AbToggle, _ab.OnB ? "Checked" : "Unchecked", true);

                AbToggle.ToolTip = _ab.Active
                    ? $"A · {SideName(_ab.AName)}    B · {SideName(_ab.BName)}"
                    : "Set a preset as B to compare against it";
            }
        }

        private static string SideName(string name)
            => string.IsNullOrEmpty(name) ? "working EQ" : name;
    }
}
