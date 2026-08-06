using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Threading;
using Heq.Apo;
using Heq.Model;
using Heq.Storage;
using Heq.Ui;

namespace Heq
{
    public partial class MainWindow : Window
    {
        private static readonly CultureInfo Inv = CultureInfo.InvariantCulture;

        private readonly EqModel _model = new EqModel();
        private readonly AbTester _ab;
        private readonly Settings _settings;

        private ApoWriter _writer;
        private List<ApoDevice> _devices = new List<ApoDevice>();

        private readonly DispatcherTimer _applyTimer = new DispatcherTimer
        {
            Interval = TimeSpan.FromMilliseconds(140),
        };

        private bool _suppress;

        private Action _refreshFields;

        private OverlayPage _settingsPage;
        private OverlayPage _correctionPage;
        private bool _drawerOpen;

        public MainWindow()
        {
            InitializeComponent();

            _settings = PresetStore.LoadSettings();
            Width = Math.Max(MinWidth, _settings.WindowWidth);
            Height = Math.Max(MinHeight, _settings.WindowHeight);

            _model.SampleRate = _settings.SampleRate > 0 ? _settings.SampleRate : 48000;
            _ab = new AbTester(_model);

            _settingsPage = new OverlayPage(SettingsPage, PageScrim, PageCard, PageScale);
            _correctionPage = new OverlayPage(CorrectionPage, CorrScrim, CorrCard, CorrScale);

            BuildCombos();
            SetupApo();

            Curve.Model = _model;
            Curve.Overlay = _correctionModel;
            Curve.DbRange = _settings.DbRange;
            Curve.SelectionChanged += (s, e) => UpdateBandStrip();
            CorrCurve.Model = _correctionModel;

            _model.Changed += OnModelChanged;
            _correctionModel.Changed += OnCorrectionChanged;
            _ab.Changed += (s, e) => UpdateAbUi();
            _applyTimer.Tick += (s, e) => { _applyTimer.Stop(); Apply(); };

            WireControls();

            if (_settings.Current != null && _settings.Current.Bands.Count > 0)
                _settings.Current.ApplyTo(_model);

            PowerToggle.IsChecked = !_settings.Bypassed;

            SetupLibrary();
            UpdateBandStrip();
            UpdateFooter();

            EnsureInclude();
            Apply();

            Loaded += (s, e) => Curve.Focus();
            Closing += OnClosing;
        }

        protected override void OnSourceInitialized(EventArgs e)
        {
            base.OnSourceInitialized(e);
            DarkTitleBar.Apply(this);
        }

        // setup

        private void BuildCombos()
        {
            KindCombo.ItemsSource = Enum.GetValues(typeof(FilterKind))
                .Cast<FilterKind>()
                .Select(k => new Item<FilterKind>(k, k.DisplayName(),
                                                  TryFindResource("IconFilter" + k) as Geometry))
                .ToList();

            SlopeCombo.ItemsSource = new[] { 12, 24, 36, 48 }
                .Select(s => new Item<int>(s, s + " dB/oct"))
                .ToList();

            RangeCombo.ItemsSource = new double[] { 6, 12, 18, 30 }
                .Select(r => new Item<double>(r, "± " + r.ToString("0", Inv) + " dB"))
                .ToList();

            foreach (var combo in new[] { SlopeCombo, RangeCombo })
                combo.DisplayMemberPath = "Label";
        }

        private void SetupApo()
        {
            string dir = ApoInstall.FindConfigDir();
            if (dir == null)
            {
                SetStatus("Equalizer APO not found — nothing will be written.", true);
                DeviceCombo.IsEnabled = false;
                return;
            }

            _writer = new ApoWriter(dir);
            _devices = ApoInstall.ScanDevices(dir);
            DeviceCombo.ItemsSource = _devices;

            var saved = _devices.FirstOrDefault(d =>
                string.Equals(d.Name, _settings.DeviceName, StringComparison.OrdinalIgnoreCase));

            if (saved == null)
            {
                var spot = _writer.FindInclude();
                if (spot.Exists && !spot.Duplicated)
                    saved = spot.Device == null
                        ? _devices.FirstOrDefault(d => d.IsAll)
                        : _devices.FirstOrDefault(d =>
                            string.Equals(d.Name, spot.Device, StringComparison.OrdinalIgnoreCase));
            }

            DeviceCombo.SelectedItem = saved ?? _devices.FirstOrDefault();
        }

        private void WireControls()
        {
            WireDisplay();
            WireBandCard();
            WireSettings();
            WireLibrary();
            WireCorrection();

            PreviewKeyDown += OnWindowKey;
        }

        private void WireDisplay()
        {
            PowerToggle.Checked += (s, e) => { _settings.Bypassed = false; ScheduleApply(); };
            PowerToggle.Unchecked += (s, e) => { _settings.Bypassed = true; ScheduleApply(); };

            AbToggle.Checked += (s, e) => { if (!_suppress) SwitchAb(true); };
            AbToggle.Unchecked += (s, e) => { if (!_suppress) SwitchAb(false); };

            AbToggle.PreviewMouseLeftButtonDown += (s, e) =>
            {
                if (_ab.Active) return;
                e.Handled = true;
                ShowDrawer(true);
            };

            MenuBtn.Click += (s, e) => ShowDrawer(true);
            DrawerCloseBtn.Click += (s, e) => ShowDrawer(false);
            Scrim.MouseLeftButtonDown += (s, e) => ShowDrawer(false);
        }

        private void WireBandCard()
        {
            KindCombo.SelectionChanged += (s, e) =>
            {
                if (_suppress || Curve.SelectedBand == null) return;
                if (KindCombo.SelectedItem is Item<FilterKind> item)
                    Curve.SelectedBand.Kind = item.Value;
                UpdateBandStrip();
            };

            SlopeCombo.SelectionChanged += (s, e) =>
            {
                if (_suppress || Curve.SelectedBand == null) return;
                if (SlopeCombo.SelectedItem is Item<int> item)
                    Curve.SelectedBand.SlopeDbPerOct = item.Value;
                UpdateBandStrip();
            };

            BandBypass.Checked += (s, e) => SetBandEnabled(true);
            BandBypass.Unchecked += (s, e) => SetBandEnabled(false);

            NumericField(FreqBox,
                get: b => b.Freq,
                set: (b, v) => b.Freq = v,
                step: (v, fine, dir) => v * Math.Pow(fine ? 1.002 : 1.02, dir),
                format: v => v >= 1000 ? v.ToString("0.#", Inv) : v.ToString("0.##", Inv));

            NumericField(GainBox,
                get: b => b.GainDb,
                set: (b, v) => b.GainDb = v,
                step: (v, fine, dir) => v + dir * (fine ? 0.1 : 0.5),
                format: v => v.ToString("0.0", Inv));

            NumericField(QBox,
                get: b => b.Q,
                set: (b, v) => b.Q = v,
                step: (v, fine, dir) => v * Math.Pow(fine ? 1.01 : 1.06, dir),
                format: v => v.ToString("0.00", Inv));
        }

        private void SetBandEnabled(bool enabled)
        {
            if (_suppress || Curve.SelectedBand == null) return;
            Curve.SelectedBand.Enabled = enabled;
            UpdateBandStrip();
        }

        private void WireSettings()
        {
            DeviceCombo.SelectionChanged += (s, e) =>
            {
                if (_suppress) return;
                _settings.DeviceName = (DeviceCombo.SelectedItem as ApoDevice)?.Name;
                UpdateSettingsSummary();
                EnsureInclude();
                ScheduleApply();
            };

            PreampBox.KeyDown += (s, e) =>
            {
                if (e.Key != Key.Enter) return;
                if (NumberText.TryParse(PreampBox.Text, out double v))
                {
                    _model.AutoGain = false;
                    _model.PreampDb = v;
                }
                UpdateFooter();
                Curve.Focus();
            };
            PreampBox.LostFocus += (s, e) =>
            {
                if (NumberText.TryParse(PreampBox.Text, out double v) && !_model.AutoGain)
                    _model.PreampDb = v;
                UpdateFooter();
            };
            PreampBox.MouseWheel += (s, e) =>
            {
                if (_model.AutoGain) return;
                _model.PreampDb += (e.Delta > 0 ? 1 : -1) * (IsFine() ? 0.1 : 0.5);
                UpdateFooter();
                e.Handled = true;
            };

            AutoGainCheck.Checked += (s, e) =>
            {
                if (_suppress) return;
                _model.AutoGain = true;
                UpdateFooter();
            };
            AutoGainCheck.Unchecked += (s, e) =>
            {
                if (_suppress) return;

                double current = _model.BasePreampDb;
                _model.AutoGain = false;
                _model.PreampDb = current;
                UpdateFooter();
            };

            RangeCombo.SelectionChanged += (s, e) =>
            {
                if (_suppress || !(RangeCombo.SelectedItem is Item<double> item)) return;
                Curve.DbRange = item.Value;
                _settings.DbRange = item.Value;
                UpdateSettingsSummary();
            };

            FolderMatchCheck.Checked += (s, e) => SetFolderMatch(true);
            FolderMatchCheck.Unchecked += (s, e) => SetFolderMatch(false);

            OpenSettingsBtn.Click += (s, e) => ShowSettings(true);
            PageCloseBtn.Click += (s, e) => ShowSettings(false);
            PageScrim.MouseLeftButtonDown += (s, e) => ShowSettings(false);
        }

        private void OnWindowKey(object sender, KeyEventArgs e)
        {
            if (e.Key == Key.Escape)
            {
                if (_correctionPage.IsOpen) ShowCorrection(false);
                else if (_settingsPage.IsOpen) ShowSettings(false);
                else return;

                e.Handled = true;
                return;
            }

            if (e.Key != Key.B || Keyboard.Modifiers != ModifierKeys.None) return;
            if (Keyboard.FocusedElement is TextBox || !_ab.Active) return;

            AbToggle.IsChecked = !_ab.OnB;
            e.Handled = true;
        }

        // the preset drawer

        private void ShowDrawer(bool open)
        {
            if (_drawerOpen == open) return;
            _drawerOpen = open;

            var slide = new DoubleAnimation(open ? 0 : -Drawer.Width,
                                            TimeSpan.FromMilliseconds(open ? 240 : 190))
            {
                EasingFunction = new CubicEase { EasingMode = open ? EasingMode.EaseOut : EasingMode.EaseIn },
            };
            var fade = new DoubleAnimation(open ? 1 : 0, TimeSpan.FromMilliseconds(open ? 200 : 170));

            if (open)
            {
                Drawer.Visibility = Visibility.Visible;
                Scrim.Visibility = Visibility.Visible;
            }
            else
            {
                slide.Completed += (s, e) =>
                {
                    if (_drawerOpen) return;
                    Drawer.Visibility = Visibility.Collapsed;
                    Scrim.Visibility = Visibility.Collapsed;
                    Curve.Focus();
                };
            }

            DrawerSlide.BeginAnimation(TranslateTransform.XProperty, slide);
            Scrim.BeginAnimation(OpacityProperty, fade);
        }

        private void ShowSettings(bool open)
            => _settingsPage.Show(open, UpdateSettingsSummary);

        private void UpdateSettingsSummary()
        {
            string device = (DeviceCombo.SelectedItem as ApoDevice)?.Display ?? "no device";
            string preamp = _model.AutoGain
                ? "auto preamp"
                : "preamp " + _model.PreampDb.ToString("0.0", Inv) + " dB";

            SettingsSummary.Text = $"{device} · {preamp} · ± {Curve.DbRange.ToString("0", Inv)} dB";
        }

        // numeric fields

        private void NumericField(TextBox box,
                                  Func<EqBand, double> get,
                                  Action<EqBand, double> set,
                                  Func<double, bool, int, double> step,
                                  Func<double, string> format)
        {
            string shown = null;

            void Show(EqBand b)
            {
                using (Suppressed())
                {
                    shown = format(get(b));
                    box.Text = shown;
                }
            }

            void Commit()
            {
                var b = Curve.SelectedBand;
                if (b == null) return;
                if (box.Text != shown && NumberText.TryParse(box.Text, out double v)) set(b, v);
                Show(b);
            }

            _refreshFields += () =>
            {
                var b = Curve.SelectedBand;
                if (b != null && !box.IsFocused) Show(b);
            };

            box.KeyDown += (s, e) =>
            {
                if (e.Key == Key.Enter) { Commit(); Curve.Focus(); }
                else if (e.Key == Key.Escape) { UpdateBandStrip(); Curve.Focus(); }
            };
            box.LostFocus += (s, e) => Commit();

            box.MouseWheel += (s, e) =>
            {
                var b = Curve.SelectedBand;
                if (b == null) return;

                set(b, step(get(b), IsFine(), e.Delta > 0 ? 1 : -1));
                Show(b);
                Curve.InvalidateVisual();
                e.Handled = true;
            };
        }

        private static bool IsFine() => (Keyboard.Modifiers & ModifierKeys.Shift) != 0;

        // ui refresh

        private void OnModelChanged(object sender, EventArgs e)
        {
            _ab.Refresh();

            bool edited = _baseline != null && !_baseline.Matches(_model);
            if (edited != _edited)
            {
                _edited = edited;
                ShowEditedStatus();
                RefreshPresets();
            }

            UpdateBandStrip();
            UpdateFooter();
            UpdateAbUi();
            ScheduleApply();
        }

        private void UpdateBandStrip()
        {
            var band = Curve.SelectedBand;
            if (band == null || !_model.Bands.Contains(band))
            {
                BandCard.Visibility = Visibility.Collapsed;
                return;
            }

            bool appearing = BandCard.Visibility != Visibility.Visible;
            BandCard.Visibility = Visibility.Visible;
            if (appearing) AnimateCardIn();

            using (Suppressed())
            {
                int index = _model.Bands.IndexOf(band);
                var colour = BandPalette.BrushAt(index);

                BandBypass.Content = (index + 1).ToString(Inv);
                BandBypass.Background = colour;
                BandBypass.BorderBrush = BandPalette.Tint(colour, 0xAA);
                BandBypass.IsChecked = band.Enabled;

                BandCard.BorderBrush = BandPalette.Tint(colour, band.Enabled ? (byte)0x44 : (byte)0x22);

                SelectItem(KindCombo, band.Kind);
                SelectItem(SlopeCombo, band.SlopeDbPerOct);
                _refreshFields?.Invoke();

                GainBox.IsEnabled = band.UsesGain;
                SlopeCell.Visibility = band.UsesSlope ? Visibility.Visible : Visibility.Collapsed;

                QBox.IsEnabled = !(band.UsesSlope && band.SlopeDbPerOct > 12);
            }
        }

        private void AnimateCardIn()
        {
            BandCard.BeginAnimation(OpacityProperty,
                new DoubleAnimation(0, 1, TimeSpan.FromMilliseconds(170)));
            CardSlide.BeginAnimation(TranslateTransform.YProperty,
                new DoubleAnimation(14, 0, TimeSpan.FromMilliseconds(220))
                {
                    EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut },
                });
        }

        private void UpdateFooter()
        {
            using (Suppressed())
            {
                if (!PreampBox.IsFocused)
                    PreampBox.Text = _model.BasePreampDb.ToString("0.0", Inv);

                PreampBox.IsEnabled = !_model.AutoGain;
                AutoGainCheck.IsChecked = _model.AutoGain;
                SelectItem(RangeCombo, Curve.DbRange);
                UpdateSettingsSummary();
            }
        }

        private static void SelectItem<T>(ComboBox combo, T value)
        {
            foreach (var o in combo.ItemsSource)
                if (o is Item<T> item && EqualityComparer<T>.Default.Equals(item.Value, value))
                {
                    combo.SelectedItem = o;
                    return;
                }
        }

        private IDisposable Suppressed()
        {
            bool prev = _suppress;
            _suppress = true;
            return new Scope(() => _suppress = prev);
        }

        // importing

        private void Import()
        {
            var dlg = new Microsoft.Win32.OpenFileDialog
            {
                Title = "Import parametric EQ",
                Filter = "EQ text files (*.txt)|*.txt|All files (*.*)|*.*",
                InitialDirectory = _writer != null && Directory.Exists(_writer.ConfigDir)
                    ? _writer.ConfigDir
                    : Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            };
            if (dlg.ShowDialog(this) != true) return;

            try
            {
                var parsed = ApoFormat.Parse(File.ReadAllText(dlg.FileName));
                if (parsed.Bands.Count == 0)
                {
                    SetStatus("No filters found in that file.", true);
                    return;
                }

                using (_model.Batch())
                {
                    _model.Bands.Clear();
                    foreach (var b in parsed.Bands) _model.Bands.Add(b);
                }

                Curve.SelectedBand = null;

                int skipped = parsed.Warnings.Count;
                if (skipped > 0)
                    SetStatus($"Imported {parsed.Bands.Count} bands, skipped {skipped} line{Plural(skipped)}.",
                              true);
            }
            catch (Exception ex)
            {
                SetStatus("Import failed: " + ex.Message, true);
            }
        }

        // applying

        private void ScheduleApply()
        {
            _applyTimer.Stop();
            _applyTimer.Start();
        }

        private void Apply()
        {
            if (_writer == null) return;

            try
            {
                _writer.WriteEq(_model,
                                _correction.Applies ? _correction.ToBands() : null,
                                PowerToggle.IsChecked != true);
            }
            catch (Exception ex)
            {
                SetStatus("Could not write heq.txt: " + ex.Message, true);
            }
        }

        private void EnsureInclude()
        {
            if (_writer == null) return;

            try
            {
                var status = _writer.EnsureInclude(DeviceCombo.SelectedItem as ApoDevice);
                if (!status.Ok) SetStatus(status.Message, true);
            }
            catch (Exception ex)
            {
                SetStatus("Could not update config.txt: " + ex.Message, true);
            }
        }

        private void SetStatus(string text, bool warn = false)
        {
            StatusText.Text = text ?? string.Empty;
            StatusText.Foreground = (Brush)FindResource(warn ? "Warn" : "TextDim");
        }

        // lifecycle

        private void OnClosing(object sender, CancelEventArgs e)
        {
            if (_correctionPage.IsOpen) CommitCorrection();

            _applyTimer.Stop();
            Apply();

            _settings.WindowWidth = ActualWidth;
            _settings.WindowHeight = ActualHeight;
            _settings.DbRange = Curve.DbRange;
            _settings.SampleRate = _model.SampleRate;
            _settings.Bypassed = PowerToggle.IsChecked != true;
            _settings.LastFolder = _folder;
            _settings.LastPreset = _currentPreset;
            _settings.AbFolder = _abSource?.Folder;
            _settings.AbPreset = _abSource?.Name;
            _settings.Current = Preset.FromModel(_model, "current");

            PresetStore.SaveSettings(_settings);
        }

        private class Item<T>
        {
            public Item(T value, string label, Geometry glyph = null)
            {
                Value = value;
                Label = label;
                Glyph = glyph;
            }

            public T Value { get; }
            public string Label { get; }
            public Geometry Glyph { get; }

            public override string ToString() => Label;
        }
    }
}
