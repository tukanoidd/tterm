{...}: {
  perSystem = {
    pkgs,
    config,
    ...
  }: {
    nci.projects.tterm.path = ./.;
    nci.crates.tterm = {
      runtimeLibs = with pkgs; [
        vulkan-loader
        libGL

        wayland
        libx11

        libxkbcommon
      ];
    };
  };
}
