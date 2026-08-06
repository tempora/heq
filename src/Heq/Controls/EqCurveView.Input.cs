using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Heq.Model;
using Heq.Ui;

namespace Heq.Controls
{
    public partial class EqCurveView
    {
        private static readonly double ShelfSplit = Math.Sqrt(FreqMin * FreqMax);

        private static readonly int[] Slopes = { 12, 24, 36, 48 };

        public FilterKind PendingKind => KindFor(XToFreq(_mousePos.X));

        private static FilterKind KindFor(double freq)
        {
            if (IsShiftDown())
                return freq < ShelfSplit ? FilterKind.LowShelf : FilterKind.HighShelf;

            if (freq < 45) return FilterKind.LowShelf;
            if (freq > 12000) return FilterKind.HighShelf;
            return FilterKind.Bell;
        }

        private EqBand HitTest(Point p)
        {
            EqBand best = null;
            double bestDist = HandleHitRadius;

            for (int i = _model.Bands.Count - 1; i >= 0; i--)
            {
                var band = _model.Bands[i];
                double dx = p.X - FreqToX(band.Freq);
                double dy = p.Y - HandleY(band);
                double d = Math.Sqrt(dx * dx + dy * dy);

                if (d > bestDist) continue;
                bestDist = d;
                best = band;
            }
            return best;
        }

        // mouse

        protected override void OnMouseLeftButtonDown(MouseButtonEventArgs e)
        {
            base.OnMouseLeftButtonDown(e);
            if (_model == null) return;

            Focus();
            e.Handled = true;

            var p = e.GetPosition(this);

            if (e.ClickCount == 2)
            {
                var hit = HitTest(p);

                if (hit != null && !ReferenceEquals(hit, _justAdded)) Remove(hit);

                _justAdded = null;
                InvalidateVisual();
                return;
            }

            var band = HitTest(p);
            bool created = band == null;

            if (created)
            {
                band = AddBandAt(p);
                _justAdded = band;
                SetGhost(false);
            }
            else
            {
                _justAdded = null;

                if (IsAltDown())
                {
                    band.Enabled = !band.Enabled;
                    SelectedBand = band;
                    return;
                }
            }

            SelectedBand = band;
            _dragBand = band;
            _dragMode = IsCtrlDown() ? DragMode.Q : DragMode.FreqGain;
            _dragStart = p;
            _dragStartFreq = band.Freq;
            _dragStartGain = band.GainDb;
            _dragStartQ = band.Q;
            CaptureMouse();
        }

        protected override void OnMouseMove(MouseEventArgs e)
        {
            base.OnMouseMove(e);
            if (_model == null) return;

            var p = e.GetPosition(this);
            _mousePos = p;

            if (_dragMode != DragMode.None && _dragBand != null)
            {
                Drag(p);
                InvalidateVisual();
                return;
            }

            var hit = HitTest(p);
            if (!ReferenceEquals(hit, _hovered))
            {
                _hovered = hit;
                Cursor = hit != null ? Cursors.SizeAll : Cursors.Cross;
            }

            SetGhost(hit == null && InPlot(p));
            InvalidateVisual();
        }

        private void Drag(Point p)
        {
            double fine = IsShiftDown() ? 0.22 : 1.0;
            double dy = (p.Y - _dragStart.Y) * fine;

            if (_dragMode == DragMode.Q)
            {
                _dragBand.Q = _dragStartQ * Math.Pow(2, dy / 60.0);
                return;
            }

            double dx = (p.X - _dragStart.X) * fine;
            _dragBand.Freq = XToFreq(FreqToX(_dragStartFreq) + dx);

            if (_dragBand.UsesGain)
                _dragBand.GainDb = ClampToRange(_dragStartGain - dy / (PlotHeight * 0.5) * _dbRange);
        }

        protected override void OnMouseLeftButtonUp(MouseButtonEventArgs e)
        {
            base.OnMouseLeftButtonUp(e);
            if (_dragMode == DragMode.None) return;

            _dragMode = DragMode.None;
            _dragBand = null;
            ReleaseMouseCapture();
            InvalidateVisual();
        }

        protected override void OnMouseLeave(MouseEventArgs e)
        {
            base.OnMouseLeave(e);
            _mousePos = new Point(double.NaN, double.NaN);
            _hovered = null;
            SetGhost(false);
            InvalidateVisual();
        }

        protected override void OnMouseWheel(MouseWheelEventArgs e)
        {
            base.OnMouseWheel(e);
            if (_model == null) return;

            var band = HitTest(e.GetPosition(this)) ?? _selected;
            if (band == null) return;

            int steps = e.Delta / 120;

            if (band.UsesSlope)
            {
                int i = Math.Max(0, Array.IndexOf(Slopes, band.SlopeDbPerOct));
                band.SlopeDbPerOct = Slopes[Math.Clamp(i + steps, 0, Slopes.Length - 1)];
            }
            else
            {
                band.Q *= Math.Pow(IsShiftDown() ? 1.03 : 1.12, steps);
            }

            SelectedBand = band;
            InvalidateVisual();
            e.Handled = true;
        }

        protected override void OnMouseRightButtonUp(MouseButtonEventArgs e)
        {
            base.OnMouseRightButtonUp(e);
            if (_model == null) return;

            var hit = HitTest(e.GetPosition(this));
            if (hit == null) return;

            SelectedBand = hit;
            ContextMenu = BuildBandMenu(hit);
            ContextMenu.PlacementTarget = this;
            ContextMenu.IsOpen = true;
            e.Handled = true;
        }

        private ContextMenu BuildBandMenu(EqBand band)
        {
            var menu = new ContextMenu();

            foreach (FilterKind kind in Enum.GetValues(typeof(FilterKind)))
            {
                var chosen = kind;
                menu.Items.Add(Menus.Check(kind.DisplayName(), band.Kind == kind,
                                           () => Set(() => band.Kind = chosen)));
            }

            menu.Items.Add(new Separator());

            foreach (ChannelTarget ear in Enum.GetValues(typeof(ChannelTarget)))
            {
                var chosen = ear;
                menu.Items.Add(Menus.Check(ear.EarName(), band.Channel == ear,
                                           () => Set(() => band.Channel = chosen)));
            }

            menu.Items.Add(new Separator());
            menu.Items.Add(Menus.Item(band.Enabled ? "Bypass band" : "Enable band",
                                      () => Set(() => band.Enabled = !band.Enabled)));
            menu.Items.Add(Menus.Item("Delete band", () => Set(() => Remove(band))));

            return menu;
        }

        private void Set(Action change)
        {
            change();
            InvalidateVisual();
        }

        private void Remove(EqBand band)
        {
            _model.RemoveBand(band);
            if (!_model.Bands.Contains(_selected)) SelectedBand = null;
        }

        private EqBand AddBandAt(Point p)
        {
            double freq = XToFreq(p.X);
            var band = _model.AddBand(KindFor(freq), freq,
                                      Math.Round(ClampToRange(YToDb(p.Y)), 1), DefaultQ);
            band.Channel = PlaceOnChannel;
            SelectedBand = band;
            return band;
        }

        // keyboard

        protected override void OnKeyDown(KeyEventArgs e)
        {
            base.OnKeyDown(e);
            if (_model == null || _selected == null) return;

            bool fine = IsShiftDown();

            switch (e.Key)
            {
                case Key.Escape:
                    SelectedBand = null;
                    break;
                case Key.Delete:
                case Key.Back:
                    Remove(_selected);
                    break;
                case Key.Left:
                    _selected.Freq *= fine ? 0.999 : 0.98;
                    break;
                case Key.Right:
                    _selected.Freq *= fine ? 1.001 : 1.02;
                    break;
                case Key.Up:
                    Nudge(fine ? 0.1 : 0.5);
                    break;
                case Key.Down:
                    Nudge(fine ? -0.1 : -0.5);
                    break;
                default:
                    return;
            }

            e.Handled = true;
            InvalidateVisual();
        }

        private void Nudge(double db)
        {
            if (_selected.UsesGain) _selected.GainDb = ClampToRange(_selected.GainDb + db);
        }
    }
}
