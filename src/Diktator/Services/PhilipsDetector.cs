using System.IO;
using System.Management;
using System.Runtime.InteropServices;
using System.Text;

namespace Diktator.Services;

internal sealed record PhilipsDevice(string Root, string Label, string VolumeSerial);

internal static class PhilipsDetector
{
    public static PhilipsDevice? Find()
    {
        foreach (var drive in DriveInfo.GetDrives())
        {
            try
            {
                if (!drive.IsReady || drive.DriveType != DriveType.Removable)
                    continue;

                var label = SafeLabel(drive);
                var isPhilipsLabel = string.Equals(label, "PHILIPS", StringComparison.OrdinalIgnoreCase);
                if (!isPhilipsLabel && !IsPhilipsUsb(drive.Name))
                    continue;

                return new PhilipsDevice(
                    Root: drive.RootDirectory.FullName,
                    Label: string.IsNullOrWhiteSpace(label) ? "PHILIPS" : label,
                    VolumeSerial: GetVolumeSerial(drive.Name) ?? drive.Name.TrimEnd('\\'));
            }
            catch
            {
                // Skip drives that disappear mid-enumeration.
            }
        }

        return null;
    }

    private static string SafeLabel(DriveInfo drive)
    {
        try { return drive.VolumeLabel ?? ""; }
        catch { return ""; }
    }

    private static bool IsPhilipsUsb(string root)
    {
        var letter = root.TrimEnd('\\');
        try
        {
            using var partitions = new ManagementObjectSearcher(
                $"ASSOCIATORS OF {{Win32_LogicalDisk.DeviceID='{letter}'}} WHERE AssocClass = Win32_LogicalDiskToPartition");
            foreach (var partition in partitions.Get())
            {
                var partitionId = partition["DeviceID"]?.ToString();
                if (string.IsNullOrEmpty(partitionId))
                    continue;

                using var disks = new ManagementObjectSearcher(
                    $"ASSOCIATORS OF {{Win32_DiskPartition.DeviceID='{partitionId}'}} WHERE AssocClass = Win32_DiskDriveToDiskPartition");
                foreach (var disk in disks.Get())
                {
                    var pnp = disk["PNPDeviceID"]?.ToString() ?? "";
                    var model = disk["Model"]?.ToString() ?? "";
                    var caption = disk["Caption"]?.ToString() ?? "";
                    var blob = $"{pnp} {model} {caption}";
                    if (blob.Contains("VID_0911", StringComparison.OrdinalIgnoreCase) ||
                        blob.Contains("DPM4", StringComparison.OrdinalIgnoreCase) ||
                        blob.Contains("PHILIPS", StringComparison.OrdinalIgnoreCase))
                    {
                        return true;
                    }
                }
            }
        }
        catch
        {
            // WMI is best-effort; volume label still works.
        }

        return false;
    }

    private static string? GetVolumeSerial(string rootPath)
    {
        var sb = new StringBuilder(261);
        if (!GetVolumeInformation(rootPath, sb, sb.Capacity, out var serial, out _, out _, null, 0))
            return null;
        return serial.ToString("X8");
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
    private static extern bool GetVolumeInformation(
        string lpRootPathName,
        StringBuilder lpVolumeNameBuffer,
        int nVolumeNameSize,
        out uint lpVolumeSerialNumber,
        out uint lpMaximumComponentLength,
        out uint lpFileSystemFlags,
        StringBuilder? lpFileSystemNameBuffer,
        int nFileSystemNameSize);
}
