{
  description = "IRC client, clown";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

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
          pname = "clown";
          cargoLock = ./Cargo.lock;
          cargoExtraArgs = "-p clown";
        });
      in
      {
        packages.default = clown;

        apps.default = flake-utils.lib.mkApp {
          drv = clown;
        };

        devShells.default = craneLib.devShell { };
      }
    );
}
