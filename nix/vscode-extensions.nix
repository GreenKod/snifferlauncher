# =============================================================================
#  nix/vscode-extensions.nix — VS Code extension ID list
# =============================================================================
#
#  Returns a list of extension identifiers in "publisher.name" format,
#  as used by `code --install-extension`.
#
#  To add / remove extensions, edit this file only — shell-hook.nix will
#  automatically pick up the changes.
#
[
  # ── Rust ────────────────────────────────────────────────────────────────────
  "rust-lang.rust-analyzer"               # Rust LSP (IntelliSense, inlay hints)
  "tamasfe.even-better-toml"              # TOML syntax + formatting (Cargo.toml)
  "serayuzgur.crates"                     # Crates.io version hints in Cargo.toml
  "vadimcn.vscode-lldb"                   # LLDB native debugger (Rust / C / C++)

  # ── Android / Mobile ────────────────────────────────────────────────────────
  "adelphes.android-dev-ext"              # Android logcat & device management
  "naco-siren.gradle-language"            # Gradle build file support

  # ── C / C++ (NDK interop) ───────────────────────────────────────────────────
  "ms-vscode.cpptools"                    # C/C++ IntelliSense, debugging
  "ms-vscode.cmake-tools"                 # CMake integration

  # ── Cross-platform / Shell ───────────────────────────────────────────────────
  "mkhl.direnv"                           # direnv integration (auto-load shell.nix)
  "jnoortheen.nix-ide"                    # Nix language support + formatting
  "timonwong.shellcheck"                  # Shell script linting (ShellCheck)
  "foxundermoon.shell-format"             # Shell script formatting (shfmt)

  # ── Git & Project hygiene ───────────────────────────────────────────────────
  "mhutchie.git-graph"                    # Git graph visualiser
  "eamodio.gitlens"                       # Git blame, history, code lens
  "editorconfig.editorconfig"             # Respect .editorconfig rules

  # ── General productivity ─────────────────────────────────────────────────────
  "streetsidesoftware.code-spell-checker" # Spell checker (comments / strings)
  "usernamehw.errorlens"                  # Inline error / warning display
  "wayou.vscode-todo-highlight"           # Highlight TODO / FIXME / HACK
]
