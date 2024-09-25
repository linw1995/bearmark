{
  description = "Development Setup";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    stable.url = "github:NixOS/nixpkgs/nixos-24.05";
    unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    stable,
    unstable,
    utils,
    ...
  }:
    utils.lib.eachDefaultSystem
    (
      system: let
        pkgs = import stable {inherit system;};
        lib = pkgs.lib;
        unstable-pkgs = import unstable {inherit system;};
      in {
        # Used by `nix develop`
        devShells.default = pkgs.mkShell {
          buildInputs =
            [
              pkgs.pkg-config

              pkgs.openssl.dev
              pkgs.libiconv
              unstable-pkgs.postgresql.dev
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];
        };
      }
    );
}
