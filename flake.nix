{
  description = "IRC client, clown";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = craneLib.cleanCargoSource ./.;

        crate = craneLib.crateNameFromCargoToml {
          cargoToml = ./clown/Cargo.toml;
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        clown = craneLib.buildPackage (commonArgs // {
          CARGO_PROFILE = "dist";
          inherit (crate) pname version;
          cargoLock = ./Cargo.lock;
          cargoExtraArgs = "-p clown";
        });
      in
      {
        packages.default = clown;

        apps.default = flake-utils.lib.mkApp {
          drv = clown;
        };

        devShells.default = craneLib.devShell {
          buildInputs = [
            rustToolchain
          ];
        };
      }
    );
}
