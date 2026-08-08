{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
      in
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
            cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          in
          pkgs.rustPlatform.buildRustPackage {
            pname = "seabird-plugin-bundle";
            version = cargoToml.package.version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [ pkgs.protobuf ];

            # sqlx uses the checked-in .sqlx cache instead of a live database.
            SQLX_OFFLINE = true;

            # Flake builds run against the git tree without a .git directory, so
            # git_version can't read the hash. Stamp a stable version string for
            # the introspection plugin to report instead.
            SEABIRD_GIT_VERSION = "v${cargoToml.package.version}-nix";
          };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            (pkgs.rust-bin.stable."1.93.0".default.override {
              extensions = [ "rust-src" ];
            })
            pkgs.rust-analyzer
            pkgs.sqlx-cli
            pkgs.protobuf
            pkgs.sqlite
          ];

          RUST_BACKTRACE = 1;
          DATABASE_URL = "sqlite://seabird.db";
        };
      }
    );
}
