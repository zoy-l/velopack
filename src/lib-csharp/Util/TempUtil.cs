using System;
using System.IO;

namespace Velopack.Util
{
    internal static class TempUtil
    {
        public static string GetDefaultTempBaseDirectory()
        {
            string tempDir;

            if (VelopackRuntimeInfo.IsOSX || VelopackRuntimeInfo.IsLinux) {
                tempDir = "/tmp/velopack";
            } else if (VelopackRuntimeInfo.IsWindows) {
                tempDir = Path.Combine(Path.GetTempPath(), "Velopack");
            } else {
                throw new PlatformNotSupportedException();
            }

            if (Environment.GetEnvironmentVariable("VELOPACK_TEMP") is var squirrlTmp
                && !string.IsNullOrWhiteSpace(squirrlTmp))
                tempDir = squirrlTmp;

            var di = new DirectoryInfo(tempDir);
            if (!di.Exists) di.Create();

            return di.FullName;
        }

        private static string GetNextTempName(string tempDir)
        {
            return Path.Combine(tempDir, Guid.NewGuid().ToString("N"));
        }

        public static IDisposable GetTempDirectory(out string newTempDirectory)
        {
            return GetTempDirectory(out newTempDirectory, GetDefaultTempBaseDirectory());
        }

        public static IDisposable GetTempDirectory(out string newTempDirectory, string rootTempDir)
        {
            var disp = GetTempFileName(out newTempDirectory, rootTempDir);
            Directory.CreateDirectory(newTempDirectory);
            return disp;
        }

        public static IDisposable GetTempFileName(out string newTempFile)
        {
            return GetTempFileName(out newTempFile, GetDefaultTempBaseDirectory());
        }

        public static IDisposable GetTempFileName(out string newTempFile, string rootTempDir)
        {
            var path = GetNextTempName(rootTempDir);
            newTempFile = path;
            return Disposable.Create(() => IoUtil.DeleteFileOrDirectoryHard(path, throwOnFailure: false));
        }
    }
}