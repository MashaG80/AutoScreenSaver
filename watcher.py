import winreg
import subprocess
import time
import os

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
RENDERER_PATH = os.path.join(SCRIPT_DIR, "renderer", "target", "release", "renderer.exe")
POLL_INTERVAL = 2  # seconds

def get_running_appid():
    try:
        key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, r"Software\Valve\Steam")
        value, _ = winreg.QueryValueEx(key, "RunningAppID")
        winreg.CloseKey(key)
        return value
    except FileNotFoundError:
        return 0

def main():
    screensaver_process = None

    print("Watcher started. Monitoring for Steam games...")

    while True:
        appid = get_running_appid()

        if appid != 0 and screensaver_process is None:
            print(f"Game launched (AppID {appid}). Starting screensaver...")
            screensaver_process = subprocess.Popen([RENDERER_PATH])

        elif appid == 0 and screensaver_process is not None:
            print("Game closed. Stopping screensaver...")
            screensaver_process.terminate()
            try:
                screensaver_process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                screensaver_process.kill()
            screensaver_process = None

        # detect that and reset our tracking so it can relaunch if needed
        if screensaver_process is not None and screensaver_process.poll() is not None:
            screensaver_process = None

        time.sleep(POLL_INTERVAL)

if __name__ == "__main__":
    main()