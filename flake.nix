{
    description = "Rust project";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

        flake-parts = {
            url = "github:hercules-ci/flake-parts";
            inputs.nixpkgs-lib.follows = "nixpkgs";
        };

        crane.url = "github:ipetkov/crane";

        treefmt-nix = {
            url = "github:numtide/treefmt-nix";
            inputs.nixpkgs.follows = "nixpkgs";
        };
    };

    outputs =
        inputs@{
            flake-parts,
            ...
        }:
        flake-parts.lib.mkFlake { inherit inputs; } {
            imports = [
                inputs.treefmt-nix.flakeModule
            ];

            systems = [
                "x86_64-linux"
                "aarch64-linux"
            ];

            perSystem =
                {
                    config,
                    pkgs,
                    self',
                    ...
                }:
                let
                    craneLib = inputs.crane.mkLib pkgs;
                    cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
                    packageName = cargoToml.package.name;
                    src = craneLib.cleanCargoSource ./.;

                    commonArgs = {
                        inherit src;
                        strictDeps = true;
                    };

                    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
                    package = craneLib.buildPackage (
                        commonArgs
                        // {
                            inherit cargoArtifacts;
                        }
                    );
                in
                {
                    packages.default = package;

                    apps.default = {
                        type = "app";
                        program = "${self'.packages.default}/bin/${packageName}";
                        meta.description = "Run ${packageName}";
                    };

                    checks = {
                        inherit package;

                        clippy = craneLib.cargoClippy (
                            commonArgs
                            // {
                                inherit cargoArtifacts;
                                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
                            }
                        );

                        doc = craneLib.cargoDoc (
                            commonArgs
                            // {
                                inherit cargoArtifacts;
                            }
                        );

                        fmt = craneLib.cargoFmt {
                            inherit src;
                        };

                        nextest = craneLib.cargoNextest (
                            commonArgs
                            // {
                                inherit cargoArtifacts;
                                cargoNextestExtraArgs = "--no-tests pass";
                            }
                        );
                    };

                    devShells.default = craneLib.devShell {
                        checks = self'.checks;

                        packages = with pkgs; [
                            cargo-nextest
                            rust-analyzer
                        ];

                        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
                    };

                    treefmt.config = {
                        projectRootFile = "flake.nix";

                        programs = {
                            nixfmt = {
                                enable = true;
                                indent = 4;
                            };
                            rustfmt.enable = true;
                            taplo.enable = true;
                        };
                    };

                    formatter = config.treefmt.build.wrapper;
                };
        };
}
