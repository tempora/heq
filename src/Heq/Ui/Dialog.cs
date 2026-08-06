using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace Heq.Ui
{
    public class Dialog : Window
    {
        private TextBox _box;

        private Dialog(Window owner, string title, double width)
        {
            Owner = owner != null && owner.IsLoaded ? owner : null;
            Title = title;
            Width = width;
            SizeToContent = SizeToContent.Height;
            WindowStartupLocation = Owner != null
                ? WindowStartupLocation.CenterOwner
                : WindowStartupLocation.CenterScreen;
            ResizeMode = ResizeMode.NoResize;
            ShowInTaskbar = false;
            Background = Themed<Brush>("Bg", Brushes.Black);
            FontFamily = Themed("UiFont", new FontFamily("Segoe UI"));
        }

        protected override void OnSourceInitialized(EventArgs e)
        {
            base.OnSourceInitialized(e);
            DarkTitleBar.Apply(this);
        }

        public static string Ask(Window owner, string title, string message, string initial = "")
        {
            var d = new Dialog(owner, title, 360);
            var panel = d.Body(message);

            d._box = new TextBox
            {
                Text = initial ?? string.Empty,
                Style = Themed<Style>("Field", null),
                FontFamily = d.FontFamily,
                Height = 26,
                TextAlignment = TextAlignment.Left,
                Padding = new Thickness(6, 3, 6, 3),
            };
            panel.Children.Add(d._box);
            panel.Children.Add(d.Buttons("OK", "Cancel", danger: false));

            d.Loaded += (s, e) => { d._box.Focus(); d._box.SelectAll(); };

            string value = d.ShowDialog() == true ? d._box.Text.Trim() : null;
            return string.IsNullOrEmpty(value) ? null : value;
        }

        public static bool Confirm(Window owner, string title, string message,
                                   string confirm = "OK", bool danger = false)
        {
            var d = new Dialog(owner, title, 380);
            var panel = d.Body(message);
            panel.Children.Add(d.Buttons(confirm, "Cancel", danger));
            return d.ShowDialog() == true;
        }

        public static void Error(string title, string message, string detail)
        {
            var d = new Dialog(Application.Current?.MainWindow, title, 560);
            var panel = d.Body(message);

            panel.Children.Add(new TextBox
            {
                Text = detail,
                IsReadOnly = true,
                Style = Themed<Style>("Field", null),
                FontFamily = Themed("MonoFont", new FontFamily("Consolas")),
                FontSize = 11,
                MaxHeight = 260,
                TextWrapping = TextWrapping.NoWrap,
                HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
                VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                Padding = new Thickness(6),
            });
            panel.Children.Add(d.Buttons("Close", null, danger: false));

            d.ShowDialog();
        }

        private StackPanel Body(string message)
        {
            var panel = new StackPanel { Margin = new Thickness(18, 16, 18, 16) };
            panel.Children.Add(new TextBlock
            {
                Text = message,
                Foreground = Themed<Brush>("Text", Brushes.White),
                FontSize = 13,
                TextWrapping = TextWrapping.Wrap,
                Margin = new Thickness(0, 0, 0, 12),
            });

            Content = panel;
            return panel;
        }

        private StackPanel Buttons(string confirm, string cancel, bool danger)
        {
            var row = new StackPanel
            {
                Orientation = Orientation.Horizontal,
                HorizontalAlignment = HorizontalAlignment.Right,
                Margin = new Thickness(0, 14, 0, 0),
            };

            if (cancel != null) row.Children.Add(Action(cancel, "FlatBtn", isConfirm: false));
            row.Children.Add(Action(confirm, danger ? "DangerBtn" : "PrimaryBtn", isConfirm: true));
            return row;
        }

        private Button Action(string text, string style, bool isConfirm)
        {
            var b = new Button
            {
                Content = text,
                Style = Themed<Style>(style, null),
                MinWidth = 84,
                Height = 28,
                Margin = new Thickness(8, 0, 0, 0),
                IsDefault = isConfirm,
                IsCancel = !isConfirm,
            };

            if (isConfirm) b.Click += (s, e) => DialogResult = true;
            return b;
        }

        private static T Themed<T>(string key, T fallback) where T : class
            => Application.Current?.TryFindResource(key) as T ?? fallback;
    }
}
