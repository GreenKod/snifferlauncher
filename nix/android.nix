# =============================================================================
#  nix/android.nix — Android SDK + NDK composition
# =============================================================================
#
#  Returns an attribute set:
#    sdk        — the composed Android SDK derivation
#    ndkVersion — the NDK version string (used in shellHook paths)
#
{ pkgs }:

let
  ndkVersion = "27.2.12479018";

  androidComposition = pkgs.androidenv.composeAndroidPackages {
    cmdLineToolsVersion  = "13.0";
    toolsVersion         = "26.1.1";
    platformToolsVersion = "35.0.2";
    buildToolsVersions   = [ "34.0.0" "35.0.0" ];

    includeEmulator     = false;   # not needed for CI / cross-compilation builds
    emulatorVersion     = "35.1.4";

    platformVersions    = [ "33" "34" "35" ];
    includeSources      = false;
    includeSystemImages = false;

    includeNDK          = true;
    ndkVersions         = [ ndkVersion ];
    cmakeVersions       = [ "3.22.1" ];
  };

in {
  inherit ndkVersion;
  sdk = androidComposition.androidsdk;
}
