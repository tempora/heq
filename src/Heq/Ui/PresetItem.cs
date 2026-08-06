namespace Heq.Ui
{
    public class PresetItem
    {
        public PresetItem(string folder, string name, bool isB, bool isEdited, bool isExcluded)
        {
            Folder = folder;
            Name = name;
            IsB = isB;
            IsEdited = isEdited;
            IsExcluded = isExcluded;
        }

        public string Folder { get; }
        public string Name { get; }

        public bool IsB { get; }

        public bool IsEdited { get; }

        public bool IsExcluded { get; }

        public override string ToString() => Name;
    }
}
