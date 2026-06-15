{...}: {
  perSystem = {
    pkgs,
    config,
    ...
  }: {
    nci = {
      projects.tterm = {
        path = ./.;
        export = true;
      };
      crates = {
        tterm = {
          runtimeLibs = with pkgs; [
            vulkan-loader
            libGL

            wayland
            libx11

            libxkbcommon
          ];
        };
        tterm-macros = {};
      };
    };
  };
}
