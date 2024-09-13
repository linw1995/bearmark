{
  description = "Development Setup";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    nixpkgs,
    utils,
    ...
  }:
    utils.lib.eachDefaultSystem
    (
      system: let
        pkgs = import nixpkgs {inherit system;};
      in
      {
        # Used by `nix develop`
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            pkg-config

            postgresql.lib
            openssl.dev
            libiconv

            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
        };
      }
    );
}
