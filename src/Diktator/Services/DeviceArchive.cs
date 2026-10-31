namespace Diktator.Services;

internal static class DeviceArchive
{
    public const string FolderName = "old";

    public static bool MoveToOld(string deviceRoot, string fileName)
    {
        var source = Path.Combine(deviceRoot, fileName);
        if (!File.Exists(source))
            return false;

        var destDir = Path.Combine(deviceRoot, FolderName);
        Directory.CreateDirectory(destDir);

        var dest = Path.Combine(destDir, fileName);
        if (File.Exists(dest))
        {
            var stem = Path.GetFileNameWithoutExtension(fileName);
            var ext = Path.GetExtension(fileName);
            dest = Path.Combine(destDir, $"{stem}_{DateTime.Now:yyyyMMdd_HHmmss}{ext}");
        }

        MoveWithRetry(source, dest);

        var sidecar = Path.Combine(deviceRoot, "._" + fileName);
        if (File.Exists(sidecar))
        {
            try
            {
                MoveWithRetry(sidecar, Path.Combine(destDir, "._" + Path.GetFileName(dest)));
            }
            catch
            {
                // AppleDouble is optional.
            }
        }

        return true;
    }

    private static void MoveWithRetry(string source, string dest)
    {
        Exception? last = null;
        for (var i = 0; i < 5; i++)
        {
            try
            {
                File.Move(source, dest);
                return;
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                last = ex;
                Thread.Sleep(350);
            }
        }

        throw last ?? new IOException($"Konnte {Path.GetFileName(source)} nicht nach old verschieben.");
    }
}
