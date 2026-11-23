{
  description = "Viewfinder: the SvelteKit site and the sight locator";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forEach = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forEach (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            # Site. pnpm reads `packageManager` in package.json and runs that exact version.
            nodejs_24
            pnpm
            # Locator (sight/).
            cargo
            rustc
            clippy
            rustfmt
            # Terrain: building and cropping the ENVI grid (brain/locator/data/three-counties.md).
            gdal
            # HEIC uploads: `sight serve` converts them with heif-convert off macOS. libheif
            # 1.18+ renamed it heif-dec (same arguments); the container's Debian keeps the old name.
            libheif
            (writeShellScriptBin "heif-convert" ''exec ${libheif}/bin/heif-dec "$@"'')
          ];
          # rust-analyzer and editors find the standard library here.
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });
    };
}
