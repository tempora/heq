using System;
using System.Windows;
using System.Windows.Threading;
using Heq.Ui;

namespace Heq
{
    public partial class App : Application
    {
        protected override void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);
            DispatcherUnhandledException += OnUnhandled;
        }

        private void OnUnhandled(object sender, DispatcherUnhandledExceptionEventArgs e)
        {
            e.Handled = true;

            try
            {
                Dialog.Error("heq — unexpected error", e.Exception.Message, e.Exception.ToString());
            }
            catch (Exception)
            {
                // The themed dialog needs the theme; if that is what broke, say it plainly.
                MessageBox.Show(e.Exception.ToString(), "heq — unexpected error",
                                MessageBoxButton.OK, MessageBoxImage.Error);
            }
        }
    }
}
