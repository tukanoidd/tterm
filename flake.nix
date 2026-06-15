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
    parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      imports = [
        nci.flakeModule
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
    };
}
