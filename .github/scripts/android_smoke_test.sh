#!/usr/bin/env bash
set -eo pipefail

echo "=== Android Visual Smoke & Health Test ==="

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

echo "2. Resolving x86_64 emulator APK in workspace..."
find . -name "*.apk" -ls || true

APK_FILE=$(find . -name "*x86_64*.apk" | head -n 1)
if [ -z "$APK_FILE" ]; then
    echo "Warning: No specific *x86_64*.apk found, falling back to any available APK..."
    APK_FILE=$(find . -name "*.apk" | head -n 1)
fi

if [ -z "$APK_FILE" ] || [ ! -f "$APK_FILE" ]; then
    echo "ERROR: No APK file found in workspace!"
    exit 1
fi

echo "Resolved APK file: $APK_FILE"

echo "3. Installing APK: $APK_FILE..."
timeout 90s adb install -r "$APK_FILE"

echo "Clearing logcat buffer before launch..."
adb logcat -c || true

echo "4. Starting SnifferLauncher (NativeActivity)..."
LAUNCHED=false
for attempt in 1 2 3 4 5; do
    echo "Starting SnifferLauncher (attempt $attempt/5)..."
    START_OUT=$(adb shell am start -W -n com.greenkod.snifferlauncher/android.app.NativeActivity 2>&1 | tr -d '\r')
    echo "$START_OUT" > am_start_output.txt
    cat am_start_output.txt

    if echo "$START_OUT" | grep -q "Status: ok" || echo "$START_OUT" | grep -q "Complete"; then
        echo "SnifferLauncher started successfully."
        LAUNCHED=true
        break
    fi
    echo "Warning: am start did not return ok on attempt $attempt. Waiting 5s before retry..."
    sleep 5
done

if [ "$LAUNCHED" != "true" ]; then
    echo "ERROR: Failed to launch SnifferLauncher NativeActivity!"
    adb logcat -d -t 200 > logcat_snippet.txt || true
    exit 1
fi

echo "5. Verifying process liveness..."
sleep 4
APP_PID=$(adb shell pidof com.greenkod.snifferlauncher 2>/dev/null | tr -d '\r' || true)
if [ -z "$APP_PID" ]; then
    # Fallback to ps inspection
    APP_PID=$(adb shell "ps -ef | grep com.greenkod.snifferlauncher | grep -v grep" 2>/dev/null | awk '{print $2}' | tr -d '\r' || true)
fi

if [ -z "$APP_PID" ]; then
    echo "ERROR: com.greenkod.snifferlauncher is not running after launch!"
    adb logcat -d -t 300 > logcat_snippet.txt || true
    exit 1
fi
echo "SnifferLauncher is alive with PID: $APP_PID"

echo "6. Waiting for UI to render..."
sleep 8

echo "7. Capturing screenshot..."
# Attempt 1: screencap to device storage then pull (most reliable in headless/CI)
timeout 30s adb shell screencap -p /sdcard/screen.png || true
timeout 30s adb pull /sdcard/screen.png android_screen.png || true

# Attempt 2: fallback to exec-out with strict timeout if pull failed or file is empty
if [ ! -s android_screen.png ]; then
    echo "Fallback: Trying adb exec-out screencap with timeout..."
    timeout 25s adb exec-out screencap -p > android_screen.png || true
fi

# Dump full and snippet logcat for diagnostic artifact
adb logcat -d > logcat_full.txt 2>/dev/null || true
tail -n 200 logcat_full.txt > logcat_snippet.txt 2>/dev/null || true
echo "--- Recent Logcat Output ---"
tail -n 30 logcat_snippet.txt || true
echo "----------------------------"

echo "8. Validating screenshot integrity..."
if [ ! -s android_screen.png ]; then
    echo "ERROR: Screenshot android_screen.png was empty or could not be captured!"
    exit 1
fi

SCREEN_BYTES=$(wc -c < android_screen.png | tr -d ' ')
if [ "$SCREEN_BYTES" -lt 1024 ]; then
    echo "ERROR: Screenshot size is suspiciously small ($SCREEN_BYTES bytes). Rendering failed."
    exit 1
fi

if command -v file >/dev/null 2>&1; then
    if ! file android_screen.png | grep -qi "PNG image"; then
        echo "ERROR: android_screen.png is not a valid PNG image!"
        file android_screen.png
        exit 1
    fi
else
    # Check PNG magic bytes (\x89PNG)
    PNG_MAGIC=$(head -c 4 android_screen.png | tr -d '\0')
    if [[ "$PNG_MAGIC" != *"PNG"* ]]; then
        echo "ERROR: android_screen.png does not contain valid PNG magic bytes!"
        exit 1
    fi
fi
echo "SUCCESS: Valid PNG screenshot confirmed ($SCREEN_BYTES bytes)."

echo "9. Scanning logcat for fatal exceptions and crashes..."
CRASH_MATCHES=$(grep -E "FATAL EXCEPTION|Fatal signal|SIGSEGV|signal 11|Abort message:|AndroidRuntime: FATAL" logcat_full.txt || true)
if [ -n "$CRASH_MATCHES" ]; then
    echo "ERROR: Crash or fatal signal detected in logcat!"
    echo "$CRASH_MATCHES"
    exit 1
fi

echo "✅ Android Emulator Visual Smoke & Health Test PASSED successfully!"
