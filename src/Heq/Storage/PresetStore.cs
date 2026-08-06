using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Heq.Storage
{
    public static class PresetStore
    {
        public const string MigratedFolder = "Unsorted";

        private const string CorrectionFile = "_correction";

        private static readonly JsonSerializerOptions Options = new JsonSerializerOptions
        {
            WriteIndented = true,
            Converters = { new JsonStringEnumConverter() },
        };

        public static string RootDir =>
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "heq");

        public static string PresetDir => Path.Combine(RootDir, "presets");

        private static string SettingsPath => Path.Combine(RootDir, "settings.json");

        public static void EnsureDirs() => Directory.CreateDirectory(PresetDir);

        public static int MigrateLoosePresets()
        {
            EnsureDirs();
            var loose = Directory.EnumerateFiles(PresetDir, "*.json").ToList();
            if (loose.Count == 0) return 0;

            string dir = FolderDir(MigratedFolder);
            Directory.CreateDirectory(dir);

            int moved = 0;
            foreach (var file in loose)
            {
                string target = Path.Combine(dir, Path.GetFileName(file));
                try
                {
                    if (File.Exists(target)) File.Delete(file); // the folder copy wins
                    else File.Move(file, target);
                    moved++;
                }
                catch (IOException) { }
            }
            return moved;
        }

        // folders

        public static string FolderDir(string folder) => Path.Combine(PresetDir, Sanitize(folder));

        public static List<string> ListFolders()
        {
            EnsureDirs();
            return Directory.EnumerateDirectories(PresetDir)
                            .Select(Path.GetFileName)
                            .OrderBy(s => s, StringComparer.OrdinalIgnoreCase)
                            .ToList();
        }

        public static bool FolderExists(string folder)
            => !string.IsNullOrWhiteSpace(folder) && Directory.Exists(FolderDir(folder));

        public static string CreateFolder(string folder)
        {
            string name = Sanitize(folder);
            if (Directory.Exists(FolderDir(name))) return null;
            Directory.CreateDirectory(FolderDir(name));
            return name;
        }

        public static void DeleteFolder(string folder)
        {
            string dir = FolderDir(folder);
            if (Directory.Exists(dir)) Directory.Delete(dir, true);
        }

        public static string RenameFolder(string folder, string newName)
        {
            string from = FolderDir(folder), to = FolderDir(newName);
            if (!Directory.Exists(from) || Directory.Exists(to)) return null;
            if (string.Equals(from, to, StringComparison.OrdinalIgnoreCase)) return null;

            Directory.Move(from, to);
            return Path.GetFileName(to);
        }

        // presets

        public static List<string> ListPresets(string folder)
        {
            if (!FolderExists(folder)) return new List<string>();
            return Directory.EnumerateFiles(FolderDir(folder), "*.json")
                            .Select(Path.GetFileNameWithoutExtension)
                            .Where(n => !IsReserved(n))
                            .OrderBy(s => s, StringComparer.OrdinalIgnoreCase)
                            .ToList();
        }

        public static string PathFor(string folder, string name)
            => Path.Combine(FolderDir(folder), Sanitize(name) + ".json");

        public static void Save(Preset p, string folder)
        {
            Directory.CreateDirectory(FolderDir(folder));
            Write(PathFor(folder, p.Name), p);
        }

        public static Preset Load(string folder, string name) => Read<Preset>(PathFor(folder, name));

        public static Preset Load(PresetRef r) => r == null ? null : Load(r.Folder, r.Name);

        public static bool Exists(string folder, string name) => File.Exists(PathFor(folder, name));

        public static void Delete(string folder, string name)
        {
            string path = PathFor(folder, name);
            if (File.Exists(path)) File.Delete(path);
        }

        public static string Rename(string folder, string name, string newName)
        {
            string from = PathFor(folder, name), to = PathFor(folder, newName);
            if (!File.Exists(from) || File.Exists(to)) return null;
            if (string.Equals(from, to, StringComparison.OrdinalIgnoreCase)) return null;

            var p = Load(folder, name);
            if (p == null) return null;

            p.Name = Sanitize(newName);
            Write(to, p);
            File.Delete(from);
            return p.Name;
        }

        public static bool Move(string folder, string name, string toFolder)
        {
            if (string.Equals(folder, toFolder, StringComparison.OrdinalIgnoreCase)) return false;

            string from = PathFor(folder, name);
            if (!File.Exists(from)) return false;

            Directory.CreateDirectory(FolderDir(toFolder));
            string to = PathFor(toFolder, name);
            if (File.Exists(to)) return false;

            File.Move(from, to);
            return true;
        }

        // the folder's correction

        public static bool IsReserved(string name)
            => string.Equals(name, CorrectionFile, StringComparison.OrdinalIgnoreCase);

        private static string CorrectionPath(string folder)
            => Path.Combine(FolderDir(folder), CorrectionFile + ".json");

        public static Correction LoadCorrection(string folder)
            => (string.IsNullOrEmpty(folder) ? null : Read<Correction>(CorrectionPath(folder)))
               ?? new Correction();

        public static void SaveCorrection(string folder, Correction c)
        {
            if (string.IsNullOrEmpty(folder)) return;

            try
            {
                if (c == null || c.IsEmpty)
                {
                    File.Delete(CorrectionPath(folder));
                    return;
                }

                Directory.CreateDirectory(FolderDir(folder));
                Write(CorrectionPath(folder), c);
            }
            catch (IOException) { }
        }

        // settings

        public static Settings LoadSettings()
        {
            try
            {
                return Read<Settings>(SettingsPath) ?? new Settings();
            }
            catch (Exception)
            {
                return new Settings(); // corrupt settings must never block startup
            }
        }

        public static void SaveSettings(Settings s)
        {
            try
            {
                EnsureDirs();
                Write(SettingsPath, s);
            }
            catch (IOException) { }
        }

        public static string Sanitize(string name)
        {
            if (string.IsNullOrWhiteSpace(name)) return "untitled";

            var bad = Path.GetInvalidFileNameChars();
            string cleaned = new string(name.Select(c => bad.Contains(c) ? '_' : c).ToArray()).Trim();
            if (cleaned.Length == 0) return "untitled";

            return IsReserved(cleaned) ? cleaned + " (preset)" : cleaned;
        }

        private static T Read<T>(string path) where T : class
        {
            if (!File.Exists(path)) return null;
            try
            {
                return JsonSerializer.Deserialize<T>(File.ReadAllText(path), Options);
            }
            catch (JsonException)
            {
                return null;
            }
        }

        private static void Write(string path, object value)
            => File.WriteAllText(path, JsonSerializer.Serialize(value, value.GetType(), Options));
    }
}
