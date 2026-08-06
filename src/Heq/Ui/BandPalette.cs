using System.Windows.Media;

namespace Heq.Ui
{
    public static class BandPalette
    {
        private static readonly Color[] Wheel =
        {
            Color.FromRgb(0xFF, 0x7A, 0x52), // ember
            Color.FromRgb(0xFF, 0xA0, 0x3C), // orange
            Color.FromRgb(0xF5, 0xC7, 0x4C), // amber
            Color.FromRgb(0xD8, 0xCE, 0x62), // wheat
            Color.FromRgb(0xAE, 0xC2, 0x63), // olive
            Color.FromRgb(0x8F, 0xB8, 0x82), // sage
            Color.FromRgb(0xC9, 0x8E, 0x5A), // caramel
            Color.FromRgb(0xE0, 0x8B, 0x6B), // clay
            Color.FromRgb(0xF0, 0x92, 0x8E), // salmon
            Color.FromRgb(0xE8, 0x7E, 0xA8), // rose
            Color.FromRgb(0xC2, 0x86, 0xB8), // mauve
            Color.FromRgb(0xB8, 0x6F, 0x6F), // brick
        };

        public static Color ColorAt(int index)
            => Wheel[(index < 0 ? 0 : index) % Wheel.Length];

        public static SolidColorBrush BrushAt(int index) => Frozen(ColorAt(index));

        public static Color WithAlpha(Color c, byte a) => Color.FromArgb(a, c.R, c.G, c.B);

        public static SolidColorBrush Frozen(Color c)
        {
            var b = new SolidColorBrush(c);
            b.Freeze();
            return b;
        }

        public static Brush Tint(Brush source, byte alpha)
            => source is SolidColorBrush s ? Frozen(WithAlpha(s.Color, alpha)) : source;
    }
}
