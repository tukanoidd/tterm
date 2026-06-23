{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nci = {
      url = "github:90-008/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    home-manager.url = "github:nix-community/home-manager";

    iced-comet = {
      # 0.14.0 tag
      url = "github:iced-rs/comet?rev=bb2a21dc9475b44b90bfebea57ac539502d2535b";
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
      systems = ["x86_64-linux"];
      imports = [
        nci.flakeModule
        inputs.home-manager.flakeModules.home-manager

        ./crates.nix
      ];
      perSystem = {
        pkgs,
        config,
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
        };

        devShells.default = ttermOutputs.devShell.overrideAttrs (old: {
          packages = with pkgs; [
            cargo-expand

            icedCometOutputs.packages.release
          ];
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
              configFilePath = mkOption {
                type = types.str;
                default = ".config/tterm/config.ron";
                description = "Path to config file in $HOME";
              };
              configFile = mkOption {
                type = types.nullOr types.path;
                default = null;
                description = "Path to .ron config file to link to the 'configFilePath' location";
              };
            };
          };

          config = mkIf cfg.enable {
            home = {
              packages = [cfg.package];

              file.${cfg.configFilePath} = mkIf (cfg.configFile != null) {
                source = cfg.configFile;
              };
            };
          };
        };
    });
}
