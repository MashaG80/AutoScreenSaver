import time
import json
import psutil
import requests
from datetime import datetime
import os
import sys

if getattr(sys, 'frozen', False):
    BASE_DIR = os.path.dirname(sys.executable)
else:
    BASE_DIR = os.path.dirname(os.path.abspath(__file__))

CONFIG_PATH = os.path.join(BASE_DIR, "config.json")

def load_config():
    if not os.path.exists(CONFIG_PATH):
        print("ERROR: config.json not found.")
        print("Copy config.example.json to config.json and add your OpenWeatherMap API key.")
        sys.exit(1)
    with open(CONFIG_PATH) as f:
        return json.load(f)

config = load_config()
WEATHER_API_KEY = config["weather_api_key"]
LAT = config["latitude"]
LON = config["longitude"]
WEATHER_REFRESH = 600  # seconds (10 min)

def get_sysinfo():
    return {
        "cpu": psutil.cpu_percent(interval=None),
        "ram": psutil.virtual_memory().percent,
    }

def get_clock():
    now = datetime.now()
    return {
        "time": now.strftime("%I:%M:%S %p"),
        "date": now.strftime("%A, %B %d"),
        "day": now.day,
        "month": now.month,
        "year": now.year,
    }

def get_weather(api_key, lat, lon):
    try:
        resp = requests.get(
            "https://api.openweathermap.org/data/2.5/weather",
            params={"lat": lat, "lon": lon, "appid": api_key, "units": "imperial"},
            timeout=10,
        )
        resp.raise_for_status()
        d = resp.json()
        return {
            "city": d["name"],
            "temp": round(d["main"]["temp"]),
            "feels_like": round(d["main"]["feels_like"]),
            "description": d["weather"][0]["description"].title(),
            "humidity": d["main"]["humidity"],
            "wind": round(d["wind"]["speed"]),
            "stale": False,
        }
    except Exception as e:
        print(f"Weather fetch failed: {e}")
        return None

def write_state(state, path="state.json"):
    with open(path, "w") as f:
        json.dump(state, f)

def main():
    weather = {"stale": True}
    last_weather_fetch = 0
    consecutive_failures = 0
    weather_disabled = False

    while True:
        now = time.time()

        if not weather_disabled and now - last_weather_fetch > WEATHER_REFRESH:
            result = get_weather(WEATHER_API_KEY, LAT, LON)
            if result:
                weather = result
                last_weather_fetch = now
                consecutive_failures = 0
            else:
                weather["stale"] = True
                last_weather_fetch = now  # still respect the refresh interval on failure
                consecutive_failures += 1

                if consecutive_failures >= 10:
                    weather_disabled = True
                    weather = {
                        "stale": True,
                        "error": "Weather service failed - check API key"
                    }
                    print("Weather fetch failed 10 times. Giving up on weather.")

        state = {
            "clock": get_clock(),
            "sysinfo": get_sysinfo(),
            "weather": weather,
        }

        write_state(state)
        time.sleep(1)

if __name__ == "__main__":
    main()