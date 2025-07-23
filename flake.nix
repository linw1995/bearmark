{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    nixpkgs,
    nixpkgs-unstable,
    utils,
    ...
  }:
    utils.lib.eachDefaultSystem
    (
      system: let
        pkgs = import nixpkgs {inherit system;};
        unstable-pkgs = import nixpkgs-unstable {inherit system;};
        lib = pkgs.lib;
        packages = with pkgs; [
          pre-commit
        ];
        nativeBuildInputs = with pkgs; [
          pkg-config
        ];
        buildInputs =
          (with pkgs; [
            openssl.dev
            libiconv
          ])
          ++ (with unstable-pkgs; [
            postgresql.dev
          ]);
      in {
        devShells =
          {
            web = pkgs.mkShell {
              packages = with pkgs; [
                nodejs
              ];
            };
            default = pkgs.mkShell {
              inherit nativeBuildInputs;
              inherit buildInputs;
              inherit packages;
            };
          }
          // builtins.listToAttrs (map (target: {
            name = target;
            value = let
              cross = import nixpkgs {
                inherit system;
                crossSystem = {config = target;};
              };
              cpkgs = cross.pkgsMusl;
            in (cross.mkShell {
              inherit nativeBuildInputs;
              inherit buildInputs;
              inherit packages;
              env = let
                normalized = lib.strings.toUpper (builtins.replaceStrings ["-"] ["_"] target);
              in
                with cpkgs; {
                  # https://doc.rust-lang.org/cargo/reference/environment-variables.html#configuration-environment-variables
                  "CARGO_TARGET_${normalized}_LINKER" = "${stdenv.cc.targetPrefix}cc";
                };
            });
          }) ["x86_64-unknown-linux-musl" "aarch64-unknown-linux-musl"]);
      }
    );
}
