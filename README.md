OLED Screensaver

A lightweight, screensaver built for gamers using Steam, designed to mitigate burn-in while showing useful at-a-glance info: live weather, clock/date, and system stats (CPU/RAM).

The renderer is written in Rust for fast, efficient drawing directly to a black (OLED-safe) canvas. A small Python daemon handles fetching weather data and system stats. An optional watcher script can automatically launch the screensaver on a secondary monitor whenever a Steam game is running.

Features


Fullscreen, true-black background (ideal for OLED panels)
Live clock, date, weather, and CPU/RAM stats
Pixel-shift burn-in protection — the entire layout slowly drifts every minute so no pixel stays lit in the same spot indefinitely
Closes on any keyboard/mouse input, like a standard screensaver
Targets a specific monitor (useful for a secondary OLED display)
Graceful handling of lost internet connection (shows last known weather, dimmed, with an "offline" indicator)
Steam integration — automatically launches the screensaver on your second monitor when a game is running, and closes it when the game exits