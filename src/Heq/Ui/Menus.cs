using System;
using System.Windows.Controls;

namespace Heq.Ui
{
    public static class Menus
    {
        public static MenuItem Item(string header, Action action)
        {
            var mi = new MenuItem { Header = header };
            mi.Click += (s, e) => action();
            return mi;
        }

        public static MenuItem Check(string header, bool isChecked, Action action)
        {
            var mi = new MenuItem { Header = header, IsCheckable = true, IsChecked = isChecked };
            mi.Click += (s, e) => action();
            return mi;
        }
    }
}
