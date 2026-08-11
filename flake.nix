{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      nixpkgs,
      flake-parts,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = nixpkgs.lib.systems.flakeExposed;
      perSystem =
        { system, pkgs, ... }:
        {
          formatter = pkgs.treefmt.withConfig {
            runtimeInputs = [
              pkgs.nixfmt
              pkgs.rustfmt
            ];

            settings = {
              on-unmatched = "info";

              formatter.nixfmt = {
                command = "nixfmt";
                includes = [ "*.nix" ];
              };

              formatter.rustfmt = {
                command = "rustfmt";
                includes = [ "*.rs" ];
              };
            };
          };

          packages.default =
            let
              version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
            in
            pkgs.rustPlatform.buildRustPackage {
              inherit version;

              pname = "seabird-plugin-bundle";
              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;

              nativeBuildInputs = [ pkgs.protobuf ];

              # sqlx uses the checked-in .sqlx cache instead of a live database.
              SQLX_OFFLINE = true;

              # Flake builds run against the git tree without a .git directory, so
              # git_version can't read the hash. Stamp a stable version string for
              # the introspection plugin to report instead.
              SEABIRD_GIT_VERSION = "v${version}-nix";
            };

          devShells.default = pkgs.mkShell {
            nativeBuildInputs = [
              pkgs.cargo
              pkgs.rustc
              pkgs.rust-analyzer
              pkgs.sqlx-cli
              pkgs.protobuf
              pkgs.sqlite
            ];

            RUST_BACKTRACE = 1;
            DATABASE_URL = "sqlite://seabird.db";
          };
        };
    };
}
