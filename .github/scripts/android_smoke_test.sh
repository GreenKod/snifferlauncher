#!/usr/bin/env bash
set -e

echo "=== Android Visual Smoke Test ==="

echo "1. Waiting for Android emulator boot completion..."
adb wait-for-device

echo "Waiting for sys.boot_completed property..."
BOOT_TIMEOUT=120
COUNTER=0
while [ $COUNTER -lt $BOOT_TIMEOUT ]; do
    BOOT_STATUS=$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')
    if [ "$BOOT_STATUS" = "1" ]; then
        echo "sys.boot_completed is 1 (${COUNTER}s elapsed)."
        break
    fi
    sleep 2
    COUNTER=$((COUNTER + 2))
done

echo "Waiting for ActivityManager and PackageManager services..."
for i in $(seq 1 30); do
    if adb shell service check activity 2>/dev/null | grep -q "found" && \
       adb shell pm path android >/dev/null 2>&1; then
        echo "Android system services are ready."
        break
    fi
    sleep 2
done

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
    LAUNCHED=false
    for attempt in 1 2 3 4 5; do
        echo "Starting SnifferLauncher (attempt $attempt/5)..."
        if timeout 20s adb shell am start -W -n com.greenkod.snifferlauncher/android.app.NativeActivity; then
            echo "SnifferLauncher started successfully."
            LAUNCHED=true
            break
        fi
        echo "Warning: am start failed on attempt $attempt. Waiting 5s before retry..."
        sleep 5
    done

    if [ "$LAUNCHED" != "true" ]; then
        echo "ERROR: Failed to start SnifferLauncher after 5 attempts."
        adb logcat -d -t 100 || true
        exit 1
    fi
    
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

