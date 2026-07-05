using System;
using System.Runtime.InteropServices;

namespace Native
{
    public static class Win32
    {
        [StructLayout(LayoutKind.Sequential)]
        public struct RECT { public int Left, Top, Right, Bottom; }

        [DllImport("user32.dll")]
        public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

        [DllImport("user32.dll")]
        public static extern bool SetForegroundWindow(IntPtr hWnd);

        [DllImport("user32.dll")]
        public static extern bool ShowWindowAsync(IntPtr hWnd, int nCmdShow);

        // Key hold via legacy keybd_event API.
        // SendInput struct alignment has been a recurring issue across
        // PowerShell hosts; keybd_event works without union games.
        [DllImport("user32.dll")]
        public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, IntPtr dwExtraInfo);

        public const uint KEYEVENTF_KEYUP = 0x0002;
        public const uint MOUSEEVENTF_MOVE = 0x0001;

        [DllImport("user32.dll")]
        public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);

        public static uint KeyDown(ushort vk)
        {
            keybd_event((byte)vk, 0, 0, IntPtr.Zero);
            return 1;
        }

        public static uint KeyUp(ushort vk)
        {
            keybd_event((byte)vk, 0, KEYEVENTF_KEYUP, IntPtr.Zero);
            return 1;
        }

        public static void MouseMove(int dx, int dy)
        {
            mouse_event(MOUSEEVENTF_MOVE, dx, dy, 0, IntPtr.Zero);
        }
    }
}
