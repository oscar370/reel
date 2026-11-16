{
  lib,
  stdenv,
  rustPlatform,
  cargo,
  rustc,
  meson,
  ninja,
  pkg-config,
  blueprint-compiler,
  desktop-file-utils,
  gettext,
  glib,
  wrapGAppsHook4,
  gtk4,
  libadwaita,
  libseccomp,
  lcms2,
  fontconfig,
  bubblewrap,
  glycin-loaders,
  gst_all_1,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "reel";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./COPYING
      ./README.md
      ./meson.build
      ./src
      ./data
      ./po
    ];
  };

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
  };

  mesonBuildType = "release";

  nativeBuildInputs = [
    rustPlatform.cargoSetupHook
    cargo
    rustc
    meson
    ninja
    pkg-config
    blueprint-compiler
    desktop-file-utils
    gettext
    glib
    wrapGAppsHook4
  ];

  buildInputs = [
    gtk4
    libadwaita
    glib
    libseccomp
    lcms2
    fontconfig
    glycin-loaders
    bubblewrap
    gst_all_1.gst-plugins-base
    gst_all_1.gst-plugins-good
    gst_all_1.gst-plugins-bad
    gst_all_1.gst-libav
  ];

  preFixup = ''
    gappsWrapperArgs+=(
      --prefix XDG_DATA_DIRS : "${glycin-loaders}/share"
      --prefix PATH : "${lib.makeBinPath [ bubblewrap ]}"
    )
  '';

  meta = {
    description = "Photo and video viewer for GNOME";
    homepage = "https://github.com/oscar370/reel";
    license = lib.licenses.gpl3Plus;
    mainProgram = "reel";
    platforms = lib.platforms.linux;
  };
})
