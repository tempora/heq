using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Microsoft.Win32;

namespace Heq.Apo
{
    public static class ApoInstall
    {
        public const string HeqFileName = "heq.txt";
        public const string MainConfigName = "config.txt";

        public static string FindConfigDir()
        {
            foreach (var root in CandidateRoots())
            {
                if (string.IsNullOrWhiteSpace(root)) continue;
                try
                {
                    string cfg = Path.Combine(root, "config");
                    if (Directory.Exists(cfg)) return cfg;
                }
                catch (ArgumentException) { /* malformed registry value */ }
            }
            return null;
        }

        private static IEnumerable<string> CandidateRoots()
        {
            foreach (var view in new[] { RegistryView.Registry64, RegistryView.Registry32 })
            {
                string v = ReadRegistry(view);
                if (v != null) yield return v;
            }

            yield return @"C:\Program Files\EqualizerAPO";
            yield return @"C:\Program Files (x86)\EqualizerAPO";
        }

        private static string ReadRegistry(RegistryView view)
        {
            try
            {
                using (var baseKey = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, view))
                using (var key = baseKey.OpenSubKey(@"SOFTWARE\EqualizerAPO"))
                {
                    return key?.GetValue("InstallPath") as string;
                }
            }
            catch (Exception)
            {
                return null;
            }
        }

        public static List<ApoDevice> ScanDevices(string configDir)
        {
            var list = new List<ApoDevice> { ApoDevice.AllDevices };
            if (configDir == null) return list;

            string path = Path.Combine(configDir, MainConfigName);
            if (!File.Exists(path)) return list;

            try
            {
                foreach (var raw in File.ReadAllLines(path))
                {
                    string line = raw.Trim();
                    bool commented = line.StartsWith("#");
                    if (commented) line = line.TrimStart('#').Trim();

                    if (!line.StartsWith("Device:", StringComparison.OrdinalIgnoreCase)) continue;

                    string name = line.Substring(7).Trim();
                    if (name.Length == 0 || name.Equals("all", StringComparison.OrdinalIgnoreCase)) continue;
                    if (list.Any(d => string.Equals(d.Name, name, StringComparison.OrdinalIgnoreCase))) continue;

                    list.Add(new ApoDevice(name, commented));
                }
            }
            catch (IOException) { /* fall through with what we have */ }

            return list;
        }
    }

    public class ApoDevice
    {
        public static readonly ApoDevice AllDevices = new ApoDevice(null, false);

        public ApoDevice(string name, bool currentlyDisabled)
        {
            Name = name;
            CurrentlyDisabled = currentlyDisabled;
        }

        public string Name { get; }

        public bool CurrentlyDisabled { get; }

        public bool IsAll => Name == null;

        public string Display
        {
            get
            {
                if (IsAll) return "All devices";
                int brace = Name.LastIndexOf('{');
                string shown = brace > 0 ? Name.Substring(0, brace).Trim() : Name;
                return CurrentlyDisabled ? shown + "  (disabled)" : shown;
            }
        }

        public override string ToString() => Display;
    }
}
