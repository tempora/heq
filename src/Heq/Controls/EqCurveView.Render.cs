using System;
using System.Collections.Generic;
using System.Globalization;
using System.Windows;
using System.Windows.Media;
using Heq.Model;
using Heq.Ui;

namespace Heq.Controls
{
    public partial class EqCurveView
    {
        private static Typeface _face;

        private static Typeface Face => _face ??= new Typeface(
            Application.Current?.TryFindResource("UiFont") as FontFamily ?? new FontFamily("Segoe UI"),
            FontStyles.Normal, FontWeights.Normal, FontStretches.Normal);

        private static readonly Brush BgBrush = Gradient("#FF161310", "#FF0C0A08");
        private static readonly Pen GridMinor = Stroke("#FF221D18", 1);
        private static readonly Pen GridMajor = Stroke("#FF322A22", 1);
        private static readonly Pen ZeroLine = Stroke("#FF4C4034", 1);
        private static readonly Pen TotalPen = Stroke("#FFF4E9DA", 1.8);
        private static readonly Pen SharedPen = Stroke("#FFA0917E", 1.0);
        private static readonly Pen LeftPen = Stroke("#C0E8C27A", 1.4);
        private static readonly Pen RightPen = Stroke("#C0E8846B", 1.4);
        private static readonly Pen CrosshairPen = Stroke("#2AF2913D", 1);
        private static readonly Brush LabelBrush = Fill("#FF6C5F4E");
        private static readonly Brush HandleTextBrush = Fill("#FF15120F");
        private static readonly Brush AreaBrush = Gradient("#30F2C98D", "#06F2C98D");

        private static readonly Color Accent = Color.FromRgb(0xF2, 0x91, 0x3D);
        private static readonly Color BypassedFill = Color.FromArgb(0x40, 0x80, 0x88, 0x92);
        private static readonly Color HandleEdge = Color.FromRgb(0x0D, 0x0E, 0x11);
        private static readonly Color GlyphHalo = Color.FromRgb(0x0E, 0x0C, 0x0A);

        private static readonly DashStyle Dashed = Frozen(new DashStyle(new double[] { 4, 3 }, 0));

        private static Brush Fill(string color) => BandPalette.Frozen(Parse(color));

        private static Pen Stroke(string color, double thickness)
            => Frozen(new Pen(Fill(color), thickness));

        private static Brush Gradient(string from, string to)
            => Frozen(new LinearGradientBrush(Parse(from), Parse(to),
                                              new Point(0, 0), new Point(0, 1)));

        private static Color Parse(string s) => (Color)ColorConverter.ConvertFromString(s);

        private static T Frozen<T>(T f) where T : Freezable
        {
            f.Freeze();
            return f;
        }

        protected override void OnRender(DrawingContext dc)
        {
            double w = ActualWidth, h = ActualHeight;
            if (w <= 0 || h <= 0) return;

            // Solid rect first so the element is hit-testable everywhere.
            dc.DrawRectangle(BgBrush, null, new Rect(0, 0, w, h));
            DrawGrid(dc);

            if (_model == null) return;

            var freqs = SampleFrequencies();
            DrawBandCurves(dc, freqs);
            DrawTotalCurve(dc, freqs);
            DrawHandles(dc);
            DrawCrosshair(dc);
            DrawPlacementPreview(dc, freqs);
        }

        private double[] SampleFrequencies()
        {
            int columns = Math.Max(2, (int)Math.Ceiling(PlotWidth));
            var freqs = new double[columns];
            for (int i = 0; i < columns; i++)
                freqs[i] = XToFreq(i * PlotWidth / (columns - 1));
            return freqs;
        }

        private Point[] Trace(double[] freqs, Func<double, double> dbAt)
        {
            var pts = new Point[freqs.Length];
            double last = freqs.Length - 1;
            for (int i = 0; i < freqs.Length; i++)
                pts[i] = new Point(i * PlotWidth / last, DbToY(dbAt(freqs[i])));
            return pts;
        }

        private static StreamGeometry Polyline(Point[] pts)
        {
            var geo = new StreamGeometry();
            using (var ctx = geo.Open())
            {
                ctx.BeginFigure(pts[0], false, false);
                for (int i = 1; i < pts.Length; i++) ctx.LineTo(pts[i], true, false);
            }
            geo.Freeze();
            return geo;
        }

        private static StreamGeometry AreaUnder(Point[] pts, double zeroY)
        {
            var geo = new StreamGeometry();
            using (var ctx = geo.Open())
            {
                ctx.BeginFigure(new Point(pts[0].X, zeroY), true, true);
                for (int i = 0; i < pts.Length; i++) ctx.LineTo(pts[i], true, false);
                ctx.LineTo(new Point(pts[pts.Length - 1].X, zeroY), true, false);
            }
            geo.Freeze();
            return geo;
        }

        // grid

        private void DrawGrid(DrawingContext dc)
        {
            double pw = PlotWidth, ph = PlotHeight;
            double dpi = VisualTreeHelper.GetDpi(this).PixelsPerDip;

            foreach (var (freq, label) in FrequencyTicks())
            {
                double x = Snap(FreqToX(freq));
                if (x < 0 || x > pw) continue;

                dc.DrawLine(label != null ? GridMajor : GridMinor, new Point(x, 0), new Point(x, ph));
                if (label == null) continue;

                var ft = Text(label, 9.5, LabelBrush, dpi);
                dc.DrawText(ft, new Point(Clamp(x - ft.Width / 2, 1, pw - ft.Width - 1), ph + 3));
            }

            for (double db = -_dbRange; db <= _dbRange + 0.001; db += DbStep())
            {
                double y = Snap(DbToY(db));
                bool zero = Math.Abs(db) < 0.001;
                dc.DrawLine(zero ? ZeroLine : GridMinor, new Point(0, y), new Point(pw, y));

                string label = zero ? "0" : (db > 0 ? "+" : "") + db.ToString("0.#", CultureInfo.InvariantCulture);
                var ft = Text(label, 9.5, LabelBrush, dpi);
                dc.DrawText(ft, new Point(pw + 5, Clamp(y - ft.Height / 2, 0, ph - ft.Height)));
            }
        }

        private double DbStep()
        {
            if (_dbRange <= 6) return 2;
            if (_dbRange <= 12) return 3;
            return _dbRange <= 18 ? 6 : 10;
        }

        private static IEnumerable<(double freq, string label)> FrequencyTicks()
        {
            foreach (int decade in new[] { 10, 100, 1000, 10000 })
            {
                for (int m = 1; m <= 9; m++)
                {
                    double f = decade * m;
                    if (f < FreqMin || f > FreqMax) continue;

                    string label = null;
                    if (m == 1 || m == 2 || m == 5)
                        label = f >= 1000
                            ? (f / 1000).ToString("0.#", CultureInfo.InvariantCulture) + "k"
                            : f.ToString("0", CultureInfo.InvariantCulture);

                    yield return (f, label);
                }
            }
        }

        // curves

        private void DrawBandCurves(DrawingContext dc, double[] freqs)
        {
            for (int i = 0; i < _model.Bands.Count; i++)
            {
                var band = _model.Bands[i];
                if (!band.Enabled) continue;

                bool selected = ReferenceEquals(band, _selected);
                var pen = Stroke(BandPalette.ColorAt(i), selected ? 1.6 : 1.1,
                                 selected ? (byte)230 : (byte)120);

                dc.DrawGeometry(null, pen,
                    Polyline(Trace(freqs, f => band.ResponseDb(f, _model.SampleRate))));
            }
        }

        private bool SplitByEar => _model.HasPerChannelBands
                                || (_overlay != null && _overlay.HasPerChannelBands);

        private void DrawTotalCurve(DrawingContext dc, double[] freqs)
        {
            double zeroY = DbToY(0);

            void Draw(ChannelTarget channel, Pen pen, bool fill)
            {
                var pts = Trace(freqs, f => _model.ResponseDb(f, channel)
                                          + (_overlay?.ResponseDb(f, channel) ?? 0));

                if (fill) dc.DrawGeometry(AreaBrush, null, AreaUnder(pts, zeroY));
                dc.DrawGeometry(null, pen, Polyline(pts));
            }

            if (SplitByEar)
            {
                Draw(ChannelTarget.Both, SharedPen, true);
                Draw(ChannelTarget.Left, LeftPen, false);
                Draw(ChannelTarget.Right, RightPen, false);
            }
            else
            {
                Draw(ChannelTarget.Both, TotalPen, true);
            }
        }

        // handles

        private void DrawHandles(DrawingContext dc)
        {
            double dpi = VisualTreeHelper.GetDpi(this).PixelsPerDip;

            for (int i = 0; i < _model.Bands.Count; i++)
            {
                var band = _model.Bands[i];
                double x = FreqToX(band.Freq);
                if (x < -20 || x > PlotWidth + 20) continue;

                var color = BandPalette.ColorAt(i);
                bool selected = ReferenceEquals(band, _selected);
                bool hovered = ReferenceEquals(band, _hovered);

                double y = HandleY(band);
                double r = HandleRadius * (selected ? 1.35 : hovered ? 1.15 : 1.0);

                if (selected)
                {
                    dc.DrawEllipse(BandPalette.Frozen(BandPalette.WithAlpha(color, 0x35)), null,
                                   new Point(x, y), r * 2.4, r * 2.4);
                    dc.DrawLine(Stroke(BandPalette.WithAlpha(color, 0x50), 1),
                                new Point(x, 0), new Point(x, PlotHeight));
                }

                var fill = BandPalette.Frozen(band.Enabled
                    ? BandPalette.WithAlpha(color, selected ? (byte)0xFF : (byte)0xD0)
                    : BypassedFill);
                var edge = Stroke(band.Enabled ? HandleEdge : BandPalette.WithAlpha(color, 0x90), 1.5);

                dc.DrawEllipse(fill, edge, new Point(x, y), r, r);

                var num = Text((i + 1).ToString(CultureInfo.InvariantCulture), selected ? 9.5 : 8.5,
                               band.Enabled ? HandleTextBrush : LabelBrush, dpi);
                dc.DrawText(num, new Point(x - num.Width / 2, y - num.Height / 2));
            }
        }

        private double HandleY(EqBand band) => DbToY(band.UsesGain ? band.GainDb : 0);

        private void DrawCrosshair(DrawingContext dc)
        {
            if (double.IsNaN(_mousePos.X) || _dragMode != DragMode.None) return;
            if (!InPlot(_mousePos)) return;

            double x = Snap(_mousePos.X);
            dc.DrawLine(CrosshairPen, new Point(x, 0), new Point(x, PlotHeight));
        }

        // placement preview

        private const double GlyphSize = 22.0;

        private void DrawPlacementPreview(DrawingContext dc, double[] freqs)
        {
            double o = GhostOpacity;
            if (o <= 0.01 || double.IsNaN(_mousePos.X)) return;

            var kind = PendingKind;
            var preview = new EqBand
            {
                Kind = kind,
                Freq = XToFreq(_mousePos.X),
                GainDb = Math.Round(ClampToRange(YToDb(_mousePos.Y)), 1),
                Q = DefaultQ,
            };

            var curvePen = Stroke(Accent, 1.3, (byte)(0x99 * o), Dashed);
            dc.DrawGeometry(null, curvePen,
                Polyline(Trace(freqs, f => preview.ResponseDb(f, _model.SampleRate))));

            double hx = _mousePos.X;
            double hy = HandleY(preview);

            var ring = Stroke(Accent, 1.4, (byte)(0xCC * o));
            dc.DrawEllipse(BandPalette.Frozen(BandPalette.WithAlpha(Accent, (byte)(0x30 * o))), ring,
                           new Point(hx, hy), HandleRadius, HandleRadius);
            dc.DrawLine(ring, new Point(hx - 2.5, hy), new Point(hx + 2.5, hy));
            dc.DrawLine(ring, new Point(hx, hy - 2.5), new Point(hx, hy + 2.5));

            DrawKindGlyph(dc, kind, hx, hy, o);
        }

        private void DrawKindGlyph(DrawingContext dc, FilterKind kind, double hx, double hy, double o)
        {
            var geo = GlyphFor(kind);
            if (geo == null) return;

            double gx = hx + 15;
            if (gx + GlyphSize > PlotWidth - 4) gx = hx - 15 - GlyphSize;
            double gy = Clamp(hy - GlyphSize - 10, 4, PlotHeight - GlyphSize - 4);

            var halo = Stroke(GlyphHalo, 4.0, (byte)(0xCC * o), rounded: true);
            var stroke = Stroke(Accent, 1.6, (byte)(0xFF * o), rounded: true);

            dc.PushTransform(new TranslateTransform(gx, gy));
            dc.PushTransform(new ScaleTransform(GlyphSize / 16.0, GlyphSize / 16.0));
            dc.DrawGeometry(null, halo, geo);
            dc.DrawGeometry(null, stroke, geo);
            dc.Pop();
            dc.Pop();
        }

        private static readonly Dictionary<FilterKind, Geometry> Glyphs =
            new Dictionary<FilterKind, Geometry>();

        private static Geometry GlyphFor(FilterKind kind)
        {
            if (Glyphs.TryGetValue(kind, out var cached)) return cached;

            var geo = Application.Current?.TryFindResource("IconFilter" + kind) as Geometry;
            if (geo != null && geo.CanFreeze) geo.Freeze();
            return Glyphs[kind] = geo;
        }

        // drawing primitives

        private static Pen Stroke(Color color, double thickness, byte alpha = 0xFF,
                                  DashStyle dash = null, bool rounded = false)
        {
            var p = new Pen(BandPalette.Frozen(BandPalette.WithAlpha(color, alpha)), thickness);

            if (dash != null) p.DashStyle = dash;
            if (rounded)
            {
                p.StartLineCap = PenLineCap.Round;
                p.EndLineCap = PenLineCap.Round;
                p.LineJoin = PenLineJoin.Round;
            }

            p.Freeze();
            return p;
        }

        private FormattedText Text(string s, double size, Brush brush, double dpi)
            => new FormattedText(s, CultureInfo.InvariantCulture, FlowDirection.LeftToRight,
                                 Face, size, brush, dpi);

        private bool InPlot(Point p)
            => p.X >= 0 && p.X <= PlotWidth && p.Y >= 0 && p.Y <= PlotHeight;

        private static double Snap(double v) => Math.Round(v) + 0.5;

        private static double Clamp(double v, double lo, double hi)
            => hi < lo ? lo : Math.Clamp(v, lo, hi);
    }
}

