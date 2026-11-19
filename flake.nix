{
  description = "correct — correcteur orthographique et grammatical";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.pkg-config
            pkgs.openssl

            # egui/eframe — dépendances GUI Linux
            pkgs.libxkbcommon
            pkgs.wayland
            pkgs.libGL
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXrandr
            pkgs.xorg.libXi
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [
              pkgs.libGL
              pkgs.wayland
              pkgs.libxkbcommon
            ]}:$LD_LIBRARY_PATH"
          '';
        };
      }
    );
}
