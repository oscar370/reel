{
  pkgs,
  lib,
  config,
  ...
}:
{
  # https://devenv.sh/languages/
  languages = {
    rust = {
      enable = true;
      channel = "stable";
      components = [
        "rustc"
        "cargo"
        "rust-analyzer"
        "clippy"
        "rustfmt"
      ];
    };
  };

  # https://devenv.sh/packages/
  packages = with pkgs; [
    pkg-config
    gtk4
    libadwaita
    glib
    libseccomp
    lcms2
    fontconfig
    glycin-loaders
    meson
    ninja
    desktop-file-utils
    blueprint-compiler
    adwaita-icon-theme
  ];

  # https://devenv.sh/scripts/
  scripts.build.exec = ''
    cd "$DEVENV_ROOT"
    if [ ! -d build ]; then
      meson setup build --prefix="$DEVENV_ROOT/build/install"
    fi
    meson compile -C build
  '';

  scripts.dev.exec = ''
    cd "$DEVENV_ROOT"
    build
    meson install -C build
    exec build/install/bin/reel "$@"
  '';

  # See full reference at https://devenv.sh/reference/options/
}
