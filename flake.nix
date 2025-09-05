{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    nixpkgs,
    utils,
    ...
  } @ inputs:
    utils.lib.eachDefaultSystem
    (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            inputs.fenix.overlays.default
          ];
        };
        packages = with pkgs; [
          pre-commit
          diesel-cli
          cargo-tarpaulin
        ];
        nativeBuildInputs = with pkgs; [
          (fenix.stable.withComponents [
            "cargo"
            "clippy"
            "rust-src"
            "rustc"
            "rustfmt"
            "rust-analyzer"
          ])
        ];
        buildInputs = with pkgs; [
          postgresql.dev
        ];
      in {
        packages = rec {
          default = bearmark;
          bearmark = let
            inherit (pkgs.fenix.stable) toolchain;
            rustPlatform = pkgs.makeRustPlatform {
              cargo = toolchain;
              rustc = toolchain;
            };
            name = (builtins.fromTOML (builtins.readFile ./bearmark-api/Cargo.toml)).package.name;
            version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          in
            rustPlatform.buildRustPackage {
              pname = name;
              inherit version;
              meta = {
                description = "Bearmark Server";
                mainProgram = name;
              };
              src = ./.;
              cargoLock = {lockFile = ./Cargo.lock;};
            };
        };
        devShells = {
          default = pkgs.mkShell {
            inherit nativeBuildInputs;
            inherit buildInputs;
            inherit packages;
          };
        };
      }
    );
}
