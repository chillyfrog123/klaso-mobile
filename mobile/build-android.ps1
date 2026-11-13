param([switch]$SkipInstall, [switch]$Offline)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
if (!$env:ANDROID_HOME -and $env:ANDROID_SDK_ROOT) { $env:ANDROID_HOME = $env:ANDROID_SDK_ROOT }
if (!$env:ANDROID_HOME) { throw 'Set ANDROID_HOME to your Android SDK directory. See MOBILE_SETUP.md.' }
if (!$env:ANDROID_NDK_HOME) { $env:ANDROID_NDK_HOME = Join-Path $env:ANDROID_HOME 'ndk/28.2.13676358' }
if (!(Test-Path (Join-Path $env:ANDROID_NDK_HOME 'source.properties'))) { throw 'Install Android NDK 28.2.13676358 or set ANDROID_NDK_HOME.' }
function Check-Exit([string]$Step) { if ($LASTEXITCODE -ne 0) { throw "$Step failed ($LASTEXITCODE)." } }
$cargoExtra = @()
$gradleExtra = @('--no-daemon')
if ($Offline) {
    $cargoExtra += '--offline'
    $gradleExtra += '--offline'
    # Gradle's wrapper can download even in --offline mode when no distribution exists.
    $gradleCache = if ($env:GRADLE_USER_HOME) { $env:GRADLE_USER_HOME } else { Join-Path $env:USERPROFILE '.gradle' }
    $launcher = Get-ChildItem (Join-Path $gradleCache 'wrapper/dists') -Filter 'gradle-launcher-8.14.3.jar' -Recurse -ErrorAction SilentlyContinue
    if (!$launcher) { throw 'Offline build requires the Gradle 8.14.3 distribution to be cached.' }
}
if (!$SkipInstall) {
    if ($Offline) { npm ci --offline --no-audit --no-fund } else { npm ci }
    Check-Exit 'npm ci'
}
npm run android:sync; Check-Exit 'Web build and Capacitor sync'
Push-Location rust
try {
    cargo ndk -t arm64-v8a --platform 24 -o ../android/app/src/main/jniLibs build --release --locked @cargoExtra
    Check-Exit 'Rust Android build'
} finally { Pop-Location }
Push-Location android
try {
    ./gradlew.bat :app:assembleDebug --console=plain @gradleExtra
    Check-Exit 'Android package'
} finally { Pop-Location }
New-Item -ItemType Directory -Force artifacts | Out-Null
Copy-Item -LiteralPath 'android/app/build/outputs/apk/debug/app-debug.apk' -Destination 'artifacts/klaso-mobile-0.1.2-arm64-debug.apk' -Force
Get-FileHash 'artifacts/klaso-mobile-0.1.2-arm64-debug.apk' -Algorithm SHA256
Write-Host 'APK: mobile/artifacts/klaso-mobile-0.1.2-arm64-debug.apk'
