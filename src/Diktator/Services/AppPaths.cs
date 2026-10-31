namespace Diktator.Services;

internal static class AppPaths
{
    public static string Root { get; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "Diktator");

    public static string SettingsFile => Path.Combine(Root, "settings.json");
    public static string TranscriptsFile => Path.Combine(Root, "transcripts.json");
    public static string Inbox => Path.Combine(Root, "inbox");

    public static void EnsureDirectories()
    {
        Directory.CreateDirectory(Root);
        Directory.CreateDirectory(Inbox);
    }
}
