using Microsoft.Win32;

namespace Diktator.Services;

internal static class AutostartService
{
    private const string RunKey = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string ValueName = "Diktator";

    public static bool IsEnabled()
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKey, false);
        return key?.GetValue(ValueName) is string;
    }

    public static void SetEnabled(bool enabled)
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKey, true)
            ?? Registry.CurrentUser.CreateSubKey(RunKey);

        if (enabled)
        {
            var exe = Environment.ProcessPath ?? Path.ChangeExtension(Environment.GetCommandLineArgs()[0], ".exe");
            key.SetValue(ValueName, $"\"{exe}\"");
        }
        else
        {
            key.DeleteValue(ValueName, false);
        }
    }
}
