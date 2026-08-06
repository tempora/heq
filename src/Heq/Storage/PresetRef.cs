using System;

namespace Heq.Storage
{
    public class PresetRef
    {
        public PresetRef(string folder, string name)
        {
            Folder = folder;
            Name = name;
        }

        public string Folder { get; }
        public string Name { get; }

        public string Display => Folder + " / " + Name;

        public bool Matches(string folder, string name)
            => string.Equals(Folder, folder, StringComparison.OrdinalIgnoreCase)
            && string.Equals(Name, name, StringComparison.OrdinalIgnoreCase);

        public override string ToString() => Name;
    }
}
