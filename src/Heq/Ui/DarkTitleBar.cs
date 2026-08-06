using System;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;

namespace Heq.Ui
{
    public static class DarkTitleBar
    {
        private const int UseImmersiveDarkMode = 20;
        private const int UseImmersiveDarkModeBefore20H1 = 19;

        [DllImport("dwmapi.dll", PreserveSig = true)]
        private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attr, ref int value, int size);

        public static void Apply(Window window)
        {
            try
            {
                IntPtr hwnd = new WindowInteropHelper(window).Handle;
                if (hwnd == IntPtr.Zero) return;

                int enabled = 1;
                if (DwmSetWindowAttribute(hwnd, UseImmersiveDarkMode, ref enabled, sizeof(int)) != 0)
                    DwmSetWindowAttribute(hwnd, UseImmersiveDarkModeBefore20H1, ref enabled, sizeof(int));
            }
            catch (DllNotFoundException)
            {
                // Pre-Windows 10; the light title bar is the only option.
            }
            catch (EntryPointNotFoundException)
            {
            }
        }
    }
}
