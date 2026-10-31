using System.Globalization;
using System.Windows;
using System.Windows.Data;
using System.Windows.Media;
using Diktator.Services;

namespace Diktator;

internal sealed class InverseBoolToVisConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => value is true ? Visibility.Collapsed : Visibility.Visible;

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}

internal sealed class StatusDotConverter : IValueConverter
{
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var status = value is ScanStatus s ? s : ScanStatus.Waiting;
        var color = status switch
        {
            ScanStatus.Working or ScanStatus.Checking => Color.FromRgb(0x6D, 0x5E, 0xF6),
            ScanStatus.Success => Color.FromRgb(0x3D, 0xDC, 0x97),
            ScanStatus.Error => Color.FromRgb(0xFF, 0x6B, 0x6B),
            ScanStatus.Device => Color.FromRgb(0x6D, 0x5E, 0xF6),
            _ => Color.FromRgb(0x66, 0x66, 0x70)
        };
        return new SolidColorBrush(color);
    }

    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}
