using System;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Animation;

namespace Heq.Ui
{
    public class OverlayPage
    {
        private readonly UIElement _host;
        private readonly UIElement _scrim;
        private readonly UIElement _card;
        private readonly ScaleTransform _scale;

        public OverlayPage(UIElement host, UIElement scrim, UIElement card, ScaleTransform scale)
        {
            _host = host;
            _scrim = scrim;
            _card = card;
            _scale = scale;
        }

        public bool IsOpen { get; private set; }

        public void Show(bool open, Action closed = null)
        {
            if (IsOpen == open) return;
            IsOpen = open;

            var fade = new DoubleAnimation(open ? 1 : 0, Ms(open ? 150 : 120));
            var scale = new DoubleAnimation(open ? 1.0 : 0.97, Ms(open ? 190 : 130))
            {
                EasingFunction = new CubicEase { EasingMode = open ? EasingMode.EaseOut : EasingMode.EaseIn },
            };

            if (open)
            {
                _host.Visibility = Visibility.Visible;
            }
            else
            {
                fade.Completed += (s, e) =>
                {
                    if (IsOpen) return;
                    _host.Visibility = Visibility.Collapsed;
                    closed?.Invoke();
                };
            }

            _scrim.BeginAnimation(UIElement.OpacityProperty, fade);
            _card.BeginAnimation(UIElement.OpacityProperty, fade);
            _scale.BeginAnimation(ScaleTransform.ScaleXProperty, scale);
            _scale.BeginAnimation(ScaleTransform.ScaleYProperty, scale);
        }

        private static Duration Ms(int ms) => new Duration(TimeSpan.FromMilliseconds(ms));
    }
}
