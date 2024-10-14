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
        cross = import stable {
          inherit system;
          crossSystem = {config = "x86_64-unknown-linux-musl";};
        };
      in {
        # Used by `nix develop`
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs =
            [
              pkgs.openssl.dev
              pkgs.libiconv
              unstable-pkgs.postgresql.dev
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];
          packages = with pkgs; [
            pre-commit
          ];
        };
        devShells.x86_64-unknown-linux-musl = cross.mkShell (with cross.pkgsMusl; {
          nativeBuildInputs = [
            pkg-config
          ];
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${stdenv.cc.targetPrefix}cc";
        });
      }
    );
}
