#!/usr/bin/env bash
set -e

echo "=== Android Visual Smoke Test ==="

echo "1. Waiting for Android emulator..."
adb wait-for-device
sleep 3

echo "Unlocking emulator screen..."
adb shell wm dismiss-keyguard || true
adb shell input keyevent 82 || true
sleep 2

echo "2. Searching for APK in workspace..."
find . -name "*.apk" || true

APK_FILE=$(find . -name "*x86_64*.apk" | head -n 1)
if [ -z "$APK_FILE" ]; then
    APK_FILE=$(find . -name "*.apk" | head -n 1)
fi

echo "Resolved APK file: $APK_FILE"

if [ -n "$APK_FILE" ] && [ -f "$APK_FILE" ]; then
    echo "3. Installing APK: $APK_FILE..."
    timeout 60s adb install -r "$APK_FILE"
    
    echo "Clearing logcat buffer..."
    adb logcat -c || true

    echo "4. Starting SnifferLauncher..."
    timeout 20s adb shell am start -n com.greenkod.snifferlauncher/android.app.NativeActivity
    
    echo "5. Waiting for UI to render..."
    sleep 10
    
    echo "6. Capturing screenshot..."
    # Attempt 1: screencap to device storage then pull (most reliable in headless/CI)
    timeout 30s adb shell screencap -p /sdcard/screen.png || true
    timeout 30s adb pull /sdcard/screen.png android_screen.png || true
    
    # Attempt 2: fallback to exec-out with strict timeout if pull failed or file is empty
    if [ ! -s android_screen.png ]; then
        echo "Fallback: Trying adb exec-out screencap with timeout..."
        timeout 25s adb exec-out screencap -p > android_screen.png || true
    fi

    # Save logcat snippet for diagnostic artifact
    adb logcat -d -t 200 > logcat_snippet.txt 2>/dev/null || true
    echo "--- Recent Logcat Output ---"
    tail -n 25 logcat_snippet.txt || true
    echo "----------------------------"
    
    if [ -s android_screen.png ]; then
        echo "SUCCESS: Screenshot captured successfully ($(ls -lh android_screen.png | awk '{print $5}'))"
    else
        echo "WARNING: Screenshot file was empty or could not be captured!"
    fi
else
    echo "ERROR: No APK file found in workspace! Directory contents:"
    ls -la
    exit 1
fi

