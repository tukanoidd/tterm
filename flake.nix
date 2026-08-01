{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    home-manager.url = "github:nix-community/home-manager";

    nci = {
      url = "github:90-008/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    iced-comet = {
      # 0.14.0 tag
      url = "github:iced-rs/comet?rev=bb2a21dc9475b44b90bfebea57ac539502d2535b";
      flake = false;
    };

    mozjs-x86_64-linux = {
      url = "file+https://github.com/servo/mozjs/releases/download/mozjs-sys-v140.12.0-1-lts/libmozjs-x86_64-unknown-linux-gnu.tar.gz";
      flake = false;
    };
    mozjs-aarch64-linux = {
      url = "file+https://github.com/servo/mozjs/releases/download/mozjs-sys-v140.12.0-1-lts/libmozjs-aarch64-unknown-linux-gnu.tar.gz";
      flake = false;
    };
  };

  outputs = inputs @ {
    parts,
    nci,
    iced-comet,
    ...
  }:
    parts.lib.mkFlake {inherit inputs;} ({self, ...}: let
      outPkg = pkgs: self.packages.${pkgs.stdenv.hostPlatform.system}.default;
    in {
      systems = ["x86_64-linux" "aarch64-linux"];
      imports = [
        nci.flakeModule
        inputs.home-manager.flakeModules.home-manager
      ];
      perSystem = {
        pkgs,
        config,
        system,
        ...
      }: let
        outputs = config.nci.outputs;
        icedCometOutputs = outputs.iced_comet;
        ttermOutputs = outputs.tterm;
      in {
        nci = {
          projects.comet = {
            path = iced-comet;
            export = true;
          };

          crates.iced_comet = {};

          projects.tterm = {
            path = ./.;
            export = true;
          };
          crates = {
            tterm = let
              commonDrvConfig = {
                mkDerivation = {
                  nativeBuildInputs = with pkgs; [
                    llvm
                    llvmPackages.libstdcxxClang
                    pkg-config
                    python3
                  ];

                  buildInputs = with pkgs; [
                    rustPlatform.bindgenHook
                    fontconfig
                    rust-jemalloc-sys
                    libclang.lib
                  ];
                };

                env = {
                  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

                  NIX_CFLAGS_COMPILE = "-Wno-error=format-security";

                  MOZJS_ARCHIVE = inputs."mozjs-${system}";
                };
              };
            in {
              runtimeLibs = with pkgs; [
                vulkan-loader
                libGL

                wayland
                libx11

                libxkbcommon

                stdenv.cc.cc.lib
                fontconfig
                freetype
                rust-jemalloc-sys
              ];

              drvConfig = commonDrvConfig;
              depsDrvConfig = commonDrvConfig;
            };
            tterm-macros = {};
          };
        };

        devShells.default = ttermOutputs.devShell.overrideAttrs (old: {
          packages = with pkgs; [
            cargo-expand

            icedCometOutputs.packages.release

            sccache
          ];

          shellHook =
            (old.shellHook or "")
            + ''
              export RUSTC_WRAPPER=${pkgs.sccache}/bin/sccache
            '';
        });
        packages.default = ttermOutputs.packages.release;
      };

      flake.homeModules.default = {
        config,
        lib,
        pkgs,
        ...
      }: let
        cfg = config.programs.tterm;
        pkg = outPkg pkgs;
      in
        with lib; {
          options = {
            programs.tterm = {
              enable = mkEnableOption "tterm";
              package = mkOption {
                type = types.package;
                default = pkg;
                description = "tterm package derivation";
              };
              configFile = mkOption {
                type = types.nullOr types.path;
                default = null;
                description = "Path to .ron/.json config file to link to the '~/.config/tterm/config.[ron/json]'";
              };
              config = mkOption {
                type = types.nullOr types.attrs;
                default = builtins.fromJSON (builtins.readFile ./assets/config/default.json);
                description = "Nix expression resolving to config.json file";
              };
            };
          };

          config = mkIf cfg.enable {
            xdg = {
              configFile = let
                makeFile = (cfg.config != null) || (cfg.configFile != null);

                isJsonPath = (lib.strings.match ".+\\.json" (builtins.toString cfg.configFile)) != null;

                ext =
                  if cfg.configFile != null && !isJsonPath
                  then "ron"
                  else "json";
              in {
                "tterm/config.${ext}" = mkIf makeFile {
                  text =
                    if (cfg.configFile != null)
                    then (builtins.readFile cfg.configFile)
                    else (builtins.toJSON cfg.config);
                };
              };

              desktopEntries.tterm = {
                categories = ["System" "Development" "Network"];
                exec = "tterm";
                genericName = "Terminal Emulator";
                name = "TTerm";
                terminal = false;
              };
            };

            home.packages = [cfg.package];
          };
        };
    });
}
