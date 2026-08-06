namespace Heq.Storage
{
    public class Settings
    {
        public string DeviceName { get; set; }
        public bool Bypassed { get; set; }
        public double DbRange { get; set; } = 18;
        public double SampleRate { get; set; } = 48000;
        public string LastFolder { get; set; }
        public string LastPreset { get; set; }
        public double WindowWidth { get; set; } = 1040;
        public double WindowHeight { get; set; } = 660;

        public string AbFolder { get; set; }

        public string AbPreset { get; set; }

        public bool MatchFolderLoudness { get; set; } = true;

        public Preset Current { get; set; }
    }
}
