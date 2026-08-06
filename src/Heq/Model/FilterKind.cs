namespace Heq.Model
{
    public enum FilterKind
    {
        Bell,
        LowShelf,
        HighShelf,
        LowCut,
        HighCut,
        Notch,
        BandPass,
        AllPass,
    }

    public enum ChannelTarget
    {
        Both,
        Left,
        Right,
    }

    public static class FilterKinds
    {
        public static string DisplayName(this FilterKind k) => k switch
        {
            FilterKind.LowShelf => "Low Shelf",
            FilterKind.HighShelf => "High Shelf",
            FilterKind.LowCut => "Low Cut",
            FilterKind.HighCut => "High Cut",
            FilterKind.BandPass => "Band Pass",
            FilterKind.AllPass => "All Pass",
            _ => k.ToString(),
        };

        public static bool UsesGain(this FilterKind k)
            => k == FilterKind.Bell || k == FilterKind.LowShelf || k == FilterKind.HighShelf;

        public static bool UsesSlope(this FilterKind k)
            => k == FilterKind.LowCut || k == FilterKind.HighCut;

        public static string EarName(this ChannelTarget c) => c switch
        {
            ChannelTarget.Left => "Left only",
            ChannelTarget.Right => "Right only",
            _ => "Both ears",
        };
    }
}
