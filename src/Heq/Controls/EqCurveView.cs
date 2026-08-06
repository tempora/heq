using System;
using System.Windows;
using System.Windows.Input;
using System.Windows.Media.Animation;
using Heq.Model;

namespace Heq.Controls
{
    public partial class EqCurveView : FrameworkElement
    {
        public const double FreqMin = 20.0;
        public const double FreqMax = 20000.0;

        public const double DefaultQ = 0.7;

        private const double HandleRadius = 6.0;
        private const double HandleHitRadius = 11.0;
        private const double AxisBottom = 18.0;  // room for frequency labels
        private const double AxisRight = 30.0;   // room for dB labels

        private static readonly double LogMin = Math.Log10(FreqMin);
        private static readonly double LogSpan = Math.Log10(FreqMax) - LogMin;

        private enum DragMode { None, FreqGain, Q }

        private EqModel _model;
        private EqModel _overlay;
        private EqBand _selected;
        private EqBand _hovered;
        private EqBand _dragBand;
        private DragMode _dragMode;
        private Point _dragStart;
        private double _dragStartFreq, _dragStartGain, _dragStartQ;
        private Point _mousePos = new Point(double.NaN, double.NaN);
        private double _dbRange = 18;

        private EqBand _justAdded;

        private bool _ghostShown;
        private bool _hooked;

        public EqCurveView()
        {
            Focusable = true;
            ClipToBounds = true;

            Loaded += (s, e) =>
            {
                if (_hooked) return;
                var w = Window.GetWindow(this);
                if (w == null) return;

                _hooked = true;
                w.PreviewKeyDown += OnModifierChanged;
                w.PreviewKeyUp += OnModifierChanged;
            };
        }

        public event EventHandler SelectionChanged;

        public EqModel Model
        {
            get => _model;
            set => Rebind(ref _model, value);
        }

        public EqModel Overlay
        {
            get => _overlay;
            set => Rebind(ref _overlay, value);
        }

        private void Rebind(ref EqModel field, EqModel value)
        {
            if (field == value) return;
            if (field != null) field.Changed -= OnModelChanged;
            field = value;
            if (field != null) field.Changed += OnModelChanged;
            InvalidateVisual();
        }

        public EqBand SelectedBand
        {
            get => _selected;
            set
            {
                if (_selected == value) return;
                _selected = value;
                SelectionChanged?.Invoke(this, EventArgs.Empty);
                InvalidateVisual();
            }
        }

        public ChannelTarget PlaceOnChannel { get; set; } = ChannelTarget.Both;

        public double DbRange
        {
            get => _dbRange;
            set
            {
                double v = Math.Clamp(value, 3, 36);
                if (_dbRange == v) return;
                _dbRange = v;
                InvalidateVisual();
            }
        }

        public static readonly DependencyProperty GhostOpacityProperty =
            DependencyProperty.Register(nameof(GhostOpacity), typeof(double), typeof(EqCurveView),
                new FrameworkPropertyMetadata(0.0, FrameworkPropertyMetadataOptions.AffectsRender));

        public double GhostOpacity
        {
            get => (double)GetValue(GhostOpacityProperty);
            set => SetValue(GhostOpacityProperty, value);
        }

        private void OnModelChanged(object sender, EventArgs e) => InvalidateVisual();

        private void OnModifierChanged(object sender, KeyEventArgs e)
        {
            bool shift = e.Key == Key.LeftShift || e.Key == Key.RightShift
                      || e.SystemKey == Key.LeftShift || e.SystemKey == Key.RightShift;
            if (shift && _ghostShown) InvalidateVisual();
        }

        private void SetGhost(bool on)
        {
            if (_ghostShown == on) return;
            _ghostShown = on;

            BeginAnimation(GhostOpacityProperty,
                new DoubleAnimation(on ? 1.0 : 0.0, TimeSpan.FromMilliseconds(on ? 150 : 110))
                {
                    EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut },
                });
        }

        // geometry

        private double PlotWidth => Math.Max(1, ActualWidth - AxisRight);
        private double PlotHeight => Math.Max(1, ActualHeight - AxisBottom);

        private double FreqToX(double f)
            => (Math.Log10(Math.Max(1e-6, f)) - LogMin) / LogSpan * PlotWidth;

        private double XToFreq(double x)
            => Math.Pow(10, LogMin + x / PlotWidth * LogSpan);

        private double DbToY(double db) => PlotHeight * 0.5 * (1 - db / _dbRange);

        private double YToDb(double y) => (1 - y / (PlotHeight * 0.5)) * _dbRange;

        private double ClampToRange(double db) => Math.Clamp(db, -_dbRange, _dbRange);

        private static bool IsShiftDown() => (Keyboard.Modifiers & ModifierKeys.Shift) != 0;
        private static bool IsCtrlDown() => (Keyboard.Modifiers & ModifierKeys.Control) != 0;
        private static bool IsAltDown() => (Keyboard.Modifiers & ModifierKeys.Alt) != 0;
    }
}
