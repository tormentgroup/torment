{
  description = "torment devShell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};
      in {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl.dev

            glib.dev
            gtk3.dev
            webkitgtk_4_1.dev
            libsoup_3.dev

            cairo.dev
            pango.dev
            gdk-pixbuf.dev
            atk.dev

            libayatana-appindicator.dev

            # The two important ones for the Wayland/font/scaling issue:
            gsettings-desktop-schemas
            glib-networking

            eza
            fd
            rust-bin.stable.latest.default
          ];

          shellHook = with pkgs; ''
                alias ls=eza
                alias find=fd

                # Make GTK/WebKit find GSettings schemas (fixes broken scaling/fonts on Wayland in dev shells)
                export XDG_DATA_DIRS=${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}:\
            ${gtk3}/share/gsettings-schemas/${gtk3.name}:$XDG_DATA_DIRS

                # Make GLib load networking/TLS modules (prevents “TLS/SSL support not available; install glib-networking”)
                export GIO_MODULE_DIR="${glib-networking}/lib/gio/modules/"
          '';
        };
      }
    );
}
