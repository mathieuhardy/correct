{
  description = "correct — correcteur orthographique et grammatical";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };

      correct = pkgs.rustPlatform.buildRustPackage {
        pname = "correct";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.copyDesktopItems
          pkgs.makeWrapper
        ];

        buildInputs = [
          pkgs.openssl
          pkgs.libxkbcommon
          pkgs.wayland
          pkgs.libGL
          pkgs.libx11
          pkgs.libxcursor
          pkgs.libxrandr
          pkgs.libxi
        ];

        desktopItems = [
          (pkgs.makeDesktopItem {
            name = "correct";
            exec = "correct";
            icon = "correct";
            desktopName = "Correct";
            comment = "Correcteur orthographique et grammatical";
            categories = [
              "Office"
              "Utility"
            ];
            terminal = false;
          })
        ];

        postInstall = ''
          # Install icon
          mkdir -p $out/share/icons/hicolor/128x128/apps
          cp data/icon.png $out/share/icons/hicolor/128x128/apps/correct.png

          wrapProgram $out/bin/correct \
            --prefix LD_LIBRARY_PATH : ${
              pkgs.lib.makeLibraryPath [
                pkgs.wayland
                pkgs.libGL
                pkgs.libxkbcommon
              ]
            }
        '';

        doCheck = false;
        meta.mainProgram = "correct";
      };
    in
    {
      packages.${system} = {
        default = correct;
        inherit correct;
      };

      apps.${system}.default = {
        type = "app";
        program = "${correct}/bin/correct";
      };

      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          (pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          })
          pkgs.pkg-config
          pkgs.openssl

          # egui/eframe — dépendances GUI Linux
          pkgs.libxkbcommon
          pkgs.wayland
          pkgs.libGL
          pkgs.libx11
          pkgs.libxcursor
          pkgs.libxrandr
          pkgs.libxi
        ];

        shellHook = ''
          export LD_LIBRARY_PATH="${
            pkgs.lib.makeLibraryPath [
              pkgs.libGL
              pkgs.wayland
              pkgs.libxkbcommon
            ]
          }:$LD_LIBRARY_PATH"
        '';
      };
    };
}
