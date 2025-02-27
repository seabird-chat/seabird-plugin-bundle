{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
        in
        {
          devShells.default = pkgs.mkShell {
            nativeBuildInputs = [
              (pkgs.rust-bin.stable."1.81.0".default.override {
                extensions = ["rust-src"];
              })
              pkgs.rust-analyzer
              pkgs.sqlx-cli
              pkgs.protobuf
            ];

            RUST_BACKTRACE = 1;
            DATABASE_URL = "postgres:///seabird";
          };
        }
      );
}
