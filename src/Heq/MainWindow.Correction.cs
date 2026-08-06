using System;
using Heq.Model;
using Heq.Storage;

namespace Heq
{
    public partial class MainWindow
    {
        private Correction _correction = new Correction();

        private readonly EqModel _correctionModel = new EqModel();

        private string _correctionFolder;

        private bool _correctionLoading;

        private void WireCorrection()
        {
            CorrCloseBtn.Click += (s, e) => ShowCorrection(false);
            CorrScrim.MouseLeftButtonDown += (s, e) => ShowCorrection(false);
            CorrClearBtn.Click += (s, e) =>
            {
                _correctionModel.Clear();
                CorrCurve.SelectedBand = null;
            };

            EarToggle.Checked += (s, e) => CorrCurve.PlaceOnChannel = ChannelTarget.Right;
            EarToggle.Unchecked += (s, e) => CorrCurve.PlaceOnChannel = ChannelTarget.Left;

            CorrEnabledCheck.Checked += (s, e) => OnCorrectionChanged(null, null);
            CorrEnabledCheck.Unchecked += (s, e) => OnCorrectionChanged(null, null);
        }

        private void LoadCorrectionFor(string folder)
        {
            _correction = _currentPreset != null && folder != null
                ? PresetStore.LoadCorrection(folder)
                : new Correction();

            _correctionLoading = true;
            try { _correction.ApplyTo(_correctionModel); }
            finally { _correctionLoading = false; }
        }

        private void OpenCorrection(string folder)
        {
            _correctionFolder = folder;
            var c = PresetStore.LoadCorrection(folder);

            _correctionLoading = true;
            try
            {
                using (Suppressed())
                {
                    c.ApplyTo(_correctionModel);
                    CorrEnabledCheck.IsChecked = c.Enabled;
                    EarToggle.IsChecked = false;
                    CorrCurve.PlaceOnChannel = ChannelTarget.Left;
                    CorrCurve.SelectedBand = null;
                    CorrCurve.DbRange = Curve.DbRange;
                    CorrFolderText.Text = folder;
                }
            }
            finally
            {
                _correctionLoading = false;
            }

            UpdateCorrectionSummary();
            ShowCorrection(true);
        }

        private void ShowCorrection(bool open)
        {
            if (!open && _correctionPage.IsOpen) CommitCorrection();

            _correctionPage.Show(open, () =>
            {
                LoadCorrectionFor(_baselineFolder);
                _correctionFolder = null;
                Curve.Focus();
            });
        }

        private void CommitCorrection()
        {
            if (_correctionFolder == null) return;

            var edited = Edited();
            PresetStore.SaveCorrection(_correctionFolder, edited);

            if (FeedsOutput()) _correction = edited;
        }

        private void OnCorrectionChanged(object sender, EventArgs e)
        {
            if (_correctionLoading || _suppress) return;

            UpdateCorrectionSummary();
            if (!_correctionPage.IsOpen || !FeedsOutput()) return;

            _correction = Edited();
            ScheduleApply();
        }

        private Correction Edited()
        {
            var c = Correction.FromModel(_correctionModel);
            c.Enabled = CorrEnabledCheck.IsChecked == true;
            return c;
        }

        private bool FeedsOutput()
            => _currentPreset != null && Same(_correctionFolder, _baselineFolder);

        private void UpdateCorrectionSummary()
            => CorrSummaryText.Text = Correction.FromModel(_correctionModel).Summary();
    }
}
