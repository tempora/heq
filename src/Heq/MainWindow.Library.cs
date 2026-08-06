using System;
using System.Collections.Generic;
using System.Linq;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Heq.Dsp;
using Heq.Storage;
using Heq.Ui;

namespace Heq
{
    public partial class MainWindow
    {
        private const string UnsavedMessage = "Unsaved changes";

        private string _folder;

        private string _currentPreset;

        private Preset _baseline;

        private string _baselineFolder;
        private bool _edited;

        private readonly HashSet<string> _excluded =
            new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        private double? _folderLevel;

        private void WireLibrary()
        {
            FolderCombo.SelectionChanged += (s, e) =>
            {
                if (_suppress) return;
                _folder = FolderCombo.SelectedItem as string;
                _settings.LastFolder = _folder;

                RecomputeFolderTarget();
                ClearToDefault();
            };

            NewFolderBtn.Click += (s, e) => NewFolder();
            RenameFolderBtn.Click += (s, e) => RenameFolder();
            DeleteFolderBtn.Click += (s, e) => DeleteFolder();

            PresetList.SelectionChanged += (s, e) =>
            {
                if (_suppress) return;
                if (PresetList.SelectedItem is PresetItem item) LoadPreset(item.Folder, item.Name);
            };
            PresetList.PreviewMouseRightButtonUp += OnPresetRightClick;

            PresetList.MouseDoubleClick += (s, e) =>
            {
                if (ItemUnder(e.OriginalSource as DependencyObject) == null) return;
                e.Handled = true;
                ClearToDefault();
            };

            SavePresetBtn.Click += (s, e) => SavePreset();
            DeletePresetBtn.Click += (s, e) => DeletePreset();

            ImportBtn.Click += (s, e) => Import();
            ClearBtn.Click += (s, e) =>
            {
                _model.Clear();
                Curve.SelectedBand = null;
            };
        }

        private void SetupLibrary()
        {
            int moved = PresetStore.MigrateLoosePresets();

            _folder = _settings.LastFolder;
            _currentPreset = _settings.LastPreset;
            _ab.NameCurrent(_currentPreset);

            using (Suppressed()) FolderMatchCheck.IsChecked = _settings.MatchFolderLoudness;

            SetBaseline(_folder, _currentPreset == null ? null : PresetStore.Load(_folder, _currentPreset));
            RefreshFolders();
            RestoreB();
            UpdateAbUi();

            if (moved > 0)
                SetStatus($"Moved {moved} preset{Plural(moved)} into “{PresetStore.MigratedFolder}”.");
        }

        private static string Plural(int n) => n == 1 ? "" : "s";

        private void RefreshFolders(string select = null)
        {
            var folders = PresetStore.ListFolders();
            string want = select ?? _folder;

            using (Suppressed())
            {
                FolderCombo.ItemsSource = folders;
                _folder = folders.FirstOrDefault(f => Same(f, want)) ?? folders.FirstOrDefault();
                FolderCombo.SelectedItem = _folder;
            }

            _settings.LastFolder = _folder;

            RecomputeFolderTarget();
            RefreshPresets();
        }

        private void RefreshPresets(string select = null)
        {
            bool ownFolder = _baselineFolder != null && Same(_folder, _baselineFolder);
            string want = select ?? (ownFolder ? _currentPreset : null);

            using (Suppressed())
            {
                var items = PresetStore.ListPresets(_folder)
                    .Select(n => new PresetItem(
                        _folder, n,
                        isB: _abSource != null && _abSource.Matches(_folder, n),
                        isEdited: _edited && ownFolder && Same(n, _currentPreset),
                        isExcluded: _excluded.Contains(n)))
                    .ToList();

                PresetList.ItemsSource = items;
                PresetList.SelectedItem = items.FirstOrDefault(i => Same(i.Name, want));

                bool empty = items.Count == 0;
                PresetList.Visibility = empty ? Visibility.Collapsed : Visibility.Visible;
                LibraryEmptyText.Visibility = empty ? Visibility.Visible : Visibility.Collapsed;
                LibraryEmptyText.Text = _folder == null
                    ? "No groups yet."
                    : "Nothing saved here yet.";

                DeletePresetBtn.IsEnabled = PresetList.SelectedItem != null;
                RenameFolderBtn.IsEnabled = _folder != null;
                DeleteFolderBtn.IsEnabled = _folder != null;
            }
        }

        // loudness target

        private void RecomputeFolderTarget()
        {
            _excluded.Clear();
            double? target = null;

            foreach (var name in PresetStore.ListPresets(_folder))
            {
                var p = PresetStore.Load(_folder, name);
                if (p == null) continue;

                if (p.ExcludeFromLoudness) { _excluded.Add(name); continue; }

                double level = Loudness.LevelDb(p, _model.SampleRate);
                if (!target.HasValue || level < target.Value) target = level;
            }

            _folderLevel = target;
            ApplyFolderTarget();
        }

        private void ApplyFolderTarget()
        {
            bool loadedHere = _currentPreset != null
                && _baselineFolder != null
                && Same(_folder, _baselineFolder);

            _ab.FolderTargetDb = _settings.MatchFolderLoudness && loadedHere ? _folderLevel : null;
        }

        private void SetFolderMatch(bool on)
        {
            if (_suppress) return;
            _settings.MatchFolderLoudness = on;
            ApplyFolderTarget();
        }

        private void ToggleLoudnessExclusion(string folder, string name)
        {
            var p = PresetStore.Load(folder, name);
            if (p == null) { CouldNotRead(name); return; }

            p.ExcludeFromLoudness = !p.ExcludeFromLoudness;
            PresetStore.Save(p, folder);

            RecomputeFolderTarget();
            RefreshPresets();
        }

        // presets

        private void LoadPreset(string folder, string name)
        {
            if (string.IsNullOrEmpty(folder) || string.IsNullOrEmpty(name)) return;

            var p = PresetStore.Load(folder, name);
            if (p == null) { CouldNotRead(name); return; }

            p.ApplyTo(_model);
            Curve.SelectedBand = null;

            _currentPreset = name;
            _settings.LastPreset = name;
            SetBaseline(folder, p);
            _ab.NameCurrent(name);
            _ab.Refresh();

            RefreshPresets();
        }

        private void SetBaseline(string folder, Preset saved)
        {
            _baseline = saved;
            _baselineFolder = saved == null ? null : folder;
            _edited = saved != null && !saved.Matches(_model);
            ShowEditedStatus();

            ApplyFolderTarget();
            if (!_correctionPage.IsOpen) LoadCorrectionFor(_baselineFolder);
        }

        private void ShowEditedStatus()
        {
            if (_edited) SetStatus(UnsavedMessage, true);
            else if (StatusText.Text == UnsavedMessage) SetStatus(null);
        }

        private void ClearToDefault()
        {
            using (_model.Batch())
            {
                _model.Clear();
                _model.AutoGain = true;
                _model.PreampDb = 0;
            }

            Curve.SelectedBand = null;
            _currentPreset = null;
            _settings.LastPreset = null;
            SetBaseline(null, null);
            _ab.NameCurrent(null);

            RefreshPresets();
        }

        private void SavePreset()
        {
            if (_folder == null)
            {
                NewFolder();
                if (_folder == null) return;
            }

            string name = Dialog.Ask(this, "Save preset", $"Name, in {_folder}",
                                           _currentPreset ?? "New preset");
            if (name == null) return;

            if (PresetStore.Exists(_folder, name) && !Same(name, _currentPreset)
                && !Confirm("Save preset", $"Replace “{name}” in {_folder}?", "Replace"))
                return;

            var saved = Preset.FromModel(_model, name);

            var replaced = PresetStore.Load(_folder, name);
            if (replaced != null) saved.ExcludeFromLoudness = replaced.ExcludeFromLoudness;

            PresetStore.Save(saved, _folder);

            _currentPreset = PresetStore.Sanitize(name);
            _settings.LastPreset = _currentPreset;
            SetBaseline(_folder, saved);
            _ab.NameCurrent(_currentPreset);

            if (_abSource != null && _abSource.Matches(_folder, _currentPreset)) ReloadB();

            RecomputeFolderTarget();
            RefreshPresets(_currentPreset);
        }

        private void DeletePreset()
        {
            if (PresetList.SelectedItem is PresetItem item) DeletePreset(item.Folder, item.Name);
        }

        private void DeletePreset(string folder, string name)
        {
            if (!Confirm("Delete preset", $"Delete “{name}” from {folder}?", "Delete", danger: true))
                return;

            PresetStore.Delete(folder, name);
            if (_abSource != null && _abSource.Matches(folder, name)) ClearB();

            if (Same(_currentPreset, name))
            {
                _currentPreset = null;
                SetBaseline(null, null);
            }

            RecomputeFolderTarget();
            RefreshPresets();
        }

        private void RenamePreset(string folder, string name)
        {
            string wanted = Dialog.Ask(this, "Rename preset", "New name", name);
            if (wanted == null) return;

            string stored = PresetStore.Rename(folder, name, wanted);
            if (stored == null) { CouldNotRename(wanted); return; }

            if (_abSource != null && _abSource.Matches(folder, name))
            {
                _abSource = new PresetRef(folder, stored);
                _ab.SetB(PresetStore.Load(_abSource), stored);
            }

            if (Same(_currentPreset, name))
            {
                _currentPreset = stored;
                _ab.NameCurrent(stored);
                SetBaseline(folder, PresetStore.Load(folder, stored));
            }

            RecomputeFolderTarget();
            RefreshPresets();
        }

        private void MovePreset(string folder, string name, string toFolder)
        {
            if (!PresetStore.Move(folder, name, toFolder))
            {
                SetStatus($"{toFolder} already has a preset called “{name}”.", true);
                return;
            }

            if (_abSource != null && _abSource.Matches(folder, name))
                _abSource = new PresetRef(toFolder, name);

            RecomputeFolderTarget();
            RefreshPresets();
        }

        // folders

        private void NewFolder()
        {
            string name = Dialog.Ask(this, "New group", "Name", "");
            if (name == null) return;

            string created = PresetStore.CreateFolder(name);
            if (created == null)
            {
                SetStatus($"“{name}” already exists.", true);
                RefreshFolders(PresetStore.Sanitize(name));
                return;
            }

            _currentPreset = null;
            SetBaseline(null, null);
            RefreshFolders(created);
        }

        private void RenameFolder()
        {
            if (_folder == null) return;

            string wanted = Dialog.Ask(this, "Rename group", "New name", _folder);
            if (wanted == null) return;

            string from = _folder;
            string stored = PresetStore.RenameFolder(from, wanted);
            if (stored == null) { CouldNotRename(wanted); return; }

            if (_abSource != null && Same(_abSource.Folder, from))
                _abSource = new PresetRef(stored, _abSource.Name);

            RefreshFolders(stored);
        }

        private void DeleteFolder()
        {
            if (_folder == null) return;

            int count = PresetStore.ListPresets(_folder).Count;
            string what = count == 0 ? "" : $" and its {count} preset{Plural(count)}";
            if (!Confirm("Delete group", $"Delete “{_folder}”{what}?", "Delete", danger: true)) return;

            string gone = _folder;
            try
            {
                PresetStore.DeleteFolder(gone);
            }
            catch (System.IO.IOException ex)
            {
                SetStatus("Could not delete the group: " + ex.Message, true);
                return;
            }

            if (_abSource != null && Same(_abSource.Folder, gone)) ClearB();

            _folder = null;
            _currentPreset = null;
            SetBaseline(null, null);
            RefreshFolders();
        }

        // the row menu

        private void OnPresetRightClick(object sender, MouseButtonEventArgs e)
        {
            var item = ItemUnder(e.OriginalSource as DependencyObject);
            if (item == null) return;
            e.Handled = true;

            bool isB = _abSource != null && _abSource.Matches(item.Folder, item.Name);
            var menu = new ContextMenu { PlacementTarget = PresetList };

            menu.Items.Add(Menus.Item("Load", () => LoadPreset(item.Folder, item.Name)));
            menu.Items.Add(Menus.Item(isB ? "Clear B" : "Set as B",
                                      () => { if (isB) ClearB(); else SetB(item.Folder, item.Name); }));

            menu.Items.Add(new Separator());
            menu.Items.Add(Menus.Check("Don't count toward the group's level", item.IsExcluded,
                                       () => ToggleLoudnessExclusion(item.Folder, item.Name)));

            menu.Items.Add(new Separator());
            menu.Items.Add(Menus.Item($"Left / right correction for {item.Folder}…",
                                      () => OpenCorrection(item.Folder)));

            menu.Items.Add(new Separator());
            menu.Items.Add(Menus.Item("Rename…", () => RenamePreset(item.Folder, item.Name)));

            var others = PresetStore.ListFolders().Where(f => !Same(f, item.Folder)).ToList();
            if (others.Count > 0)
            {
                var move = new MenuItem { Header = "Move to" };
                foreach (var folder in others)
                {
                    string target = folder;
                    move.Items.Add(Menus.Item(target, () => MovePreset(item.Folder, item.Name, target)));
                }
                menu.Items.Add(move);
            }

            menu.Items.Add(Menus.Item("Delete", () => DeletePreset(item.Folder, item.Name)));
            menu.IsOpen = true;
        }

        private void OnSlotToggleDown(object sender, MouseButtonEventArgs e)
        {
            e.Handled = true;
            if (!((sender as FrameworkElement)?.DataContext is PresetItem item)) return;

            if (item.IsB) ClearB();
            else SetB(item.Folder, item.Name);
        }

        private static PresetItem ItemUnder(DependencyObject d)
        {
            while (d != null && !(d is ListBoxItem))
                d = d is Visual || d is System.Windows.Media.Media3D.Visual3D
                    ? VisualTreeHelper.GetParent(d)
                    : LogicalTreeHelper.GetParent(d);

            return (d as ListBoxItem)?.DataContext as PresetItem;
        }

        private static bool Same(string a, string b)
            => string.Equals(a, b, StringComparison.OrdinalIgnoreCase);

        private bool Confirm(string title, string question, string action, bool danger = false)
            => Dialog.Confirm(this, title, question, action, danger);

        private void CouldNotRead(string name) => SetStatus($"Could not read “{name}”.", true);

        private void CouldNotRename(string name) => SetStatus($"Could not rename to “{name}”.", true);
    }
}
