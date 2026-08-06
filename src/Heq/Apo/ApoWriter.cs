using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using Heq.Model;

namespace Heq.Apo
{
    public class ApoWriter
    {
        private const string BackupSuffix = ".heq-backup";

        public string ConfigDir { get; }

        public ApoWriter(string configDir) => ConfigDir = configDir;

        public string HeqPath => Path.Combine(ConfigDir, ApoInstall.HeqFileName);
        public string MainConfigPath => Path.Combine(ConfigDir, ApoInstall.MainConfigName);

        public void WriteEq(EqModel model, bool bypassed) => WriteEq(model, null, bypassed);

        public void WriteEq(EqModel model, IEnumerable<EqBand> correction, bool bypassed)
            => AtomicWrite(HeqPath, bypassed ? string.Empty : ApoFormat.BuildConfig(model, correction));

        public class IncludeStatus
        {
            public bool Ok = true;
            public bool Changed;
            public string Message;
        }

        public class IncludeSpot
        {
            public bool Exists;
            public bool Duplicated;

            public string Device;
        }

        public IncludeSpot FindInclude()
        {
            if (!File.Exists(MainConfigPath)) return new IncludeSpot();

            List<string> lines;
            try
            {
                lines = File.ReadAllLines(MainConfigPath).ToList();
            }
            catch (IOException)
            {
                return new IncludeSpot();
            }

            var found = IndicesOfInclude(lines);
            if (found.Count == 0) return new IncludeSpot();

            return new IncludeSpot
            {
                Exists = true,
                Duplicated = found.Count > 1,
                Device = DeviceAbove(lines, found[0]),
            };
        }

        public IncludeStatus EnsureInclude(ApoDevice device)
        {
            var lines = File.Exists(MainConfigPath)
                ? File.ReadAllLines(MainConfigPath).ToList()
                : new List<string>();

            if (IndicesOfInclude(lines).Count > 0) return new IncludeStatus();

            var updated = StripMarkers(lines);

            int at = 0;
            if (device != null && !device.IsAll)
            {
                at = IndexOfDevice(updated, device.Name);
                if (at < 0)
                    return new IncludeStatus
                    {
                        Ok = false,
                        Message = $"{device.Display} has no active Device line in config.txt.",
                    };
                at++;
            }

            updated.Insert(at, "Include: " + ApoInstall.HeqFileName);

            EnsureBackup();
            AtomicWrite(MainConfigPath, string.Join("\r\n", updated) + "\r\n");
            return new IncludeStatus { Changed = true, Message = "Added Include: heq.txt to config.txt." };
        }

        private static int IndexOfDevice(List<string> lines, string name)
        {
            for (int i = 0; i < lines.Count; i++)
            {
                string t = lines[i].Trim();
                if (t.StartsWith("#") || !t.StartsWith("Device:", StringComparison.OrdinalIgnoreCase)) continue;
                if (string.Equals(t.Substring(7).Trim(), name, StringComparison.OrdinalIgnoreCase)) return i;
            }
            return -1;
        }

        private static string DeviceAbove(List<string> lines, int index)
        {
            for (int i = index - 1; i >= 0; i--)
            {
                string t = lines[i].Trim();
                if (t.StartsWith("#") || !t.StartsWith("Device:", StringComparison.OrdinalIgnoreCase)) continue;
                return t.Substring(7).Trim();
            }
            return null;
        }

        private static List<int> IndicesOfInclude(List<string> lines)
        {
            var found = new List<int>();
            for (int i = 0; i < lines.Count; i++)
                if (IsHeqInclude(lines[i].Trim())) found.Add(i);
            return found;
        }

        private static List<string> StripMarkers(List<string> lines)
        {
            return lines.Where(l =>
            {
                string t = l.Trim();
                return !t.StartsWith("# >>> heq") && !t.StartsWith("# <<< heq");
            }).ToList();
        }

        private static bool IsHeqInclude(string trimmed)
        {
            if (!trimmed.StartsWith("Include:", StringComparison.OrdinalIgnoreCase)) return false;
            return string.Equals(trimmed.Substring(8).Trim().Trim('"'),
                                 ApoInstall.HeqFileName, StringComparison.OrdinalIgnoreCase);
        }

        private void EnsureBackup()
        {
            string backup = MainConfigPath + BackupSuffix;
            try
            {
                if (File.Exists(MainConfigPath) && !File.Exists(backup))
                    File.Copy(MainConfigPath, backup);
            }
            catch (IOException) { }
        }

        private static void AtomicWrite(string path, string content)
        {
            string tmp = Path.Combine(Path.GetDirectoryName(path), Path.GetFileName(path) + ".tmp");
            File.WriteAllText(tmp, content, new UTF8Encoding(false));

            try
            {
                if (File.Exists(path)) File.Replace(tmp, path, null);
                else File.Move(tmp, path);
            }
            catch (IOException)
            {
                File.Copy(tmp, path, true);
                try { File.Delete(tmp); } catch (IOException) { }
            }
        }
    }
}
