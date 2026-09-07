#!/usr/bin/env bash
set -e

echo "=== Android Visual Smoke Test ==="

echo "1. Waiting for Android emulator..."
adb wait-for-device
sleep 5

echo "2. Searching for APK in workspace..."
find . -name "*.apk" || true

APK_FILE=$(find . -name "*x86_64*.apk" | head -n 1)
if [ -z "$APK_FILE" ]; then
    APK_FILE=$(find . -name "*.apk" | head -n 1)
fi

echo "Resolved APK file: $APK_FILE"

if [ -n "$APK_FILE" ] && [ -f "$APK_FILE" ]; then
    echo "3. Installing APK: $APK_FILE..."
    adb install -r "$APK_FILE"
    
    echo "4. Starting SnifferLauncher..."
    adb shell am start -n com.greenkod.snifferlauncher/android.app.NativeActivity
    
    echo "5. Waiting for UI to render..."
    sleep 8
    
    echo "6. Capturing screenshot..."
    adb exec-out screencap -p > android_screen.png
    
    if [ -s android_screen.png ]; then
        echo "SUCCESS: Screenshot captured successfully ($(ls -lh android_screen.png | awk '{print $5}'))"
    else
        echo "WARNING: Screenshot file was empty!"
    fi
else
    echo "ERROR: No APK file found in workspace! Directory contents:"
    ls -la
    exit 1
fi
