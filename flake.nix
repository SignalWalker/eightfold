{
  description = "";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane = {
      url = "github:ipetkov/crane";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs @ {
    self,
    nixpkgs,
    ...
  }:
    with builtins; let
      std = nixpkgs.lib;

      systems = attrNames inputs.crane.packages;
      nixpkgsFor = std.genAttrs systems (system:
        import nixpkgs {
          localSystem = builtins.currentSystem or system;
          crossSystem = system;
          overlays = [inputs.rust-overlay.overlays.default];
        });

      toolchainToml = fromTOML (readFile ./rust-toolchain.toml);
      toolchainFor = std.mapAttrs (system: pkgs: pkgs.rust-bin.fromRustupToolchain toolchainToml.toolchain) nixpkgsFor;

      craneFor = std.mapAttrs (system: pkgs: (inputs.crane.mkLib pkgs).overrideToolchain toolchainFor.${system}) nixpkgsFor;

      stdenvFor = std.mapAttrs (system: pkgs: pkgs.stdenvAdapters.useMoldLinker pkgs.llvmPackages_latest.stdenv) nixpkgsFor;

      commonArgsFor =
        std.mapAttrs (system: pkgs: let
          crane = craneFor.${system};
        in {
          src = crane.cleanCargoSource (crane.path ./.);
          stdenv = stdenvFor.${system};
          strictDeps = true;
          hardeningDisable = [
            "fortify"
          ];
          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
          ];
          buildInputs = with pkgs; [
            fontconfig
            # for avif decoding
            dav1d
          ];
        })
        nixpkgsFor;

      cargoToml = fromTOML (readFile ./Cargo.toml);
      name = cargoToml.package.metadata.crane.name or cargoToml.package.name or cargoToml.workspace.metadata.crane.name;
      version = cargoToml.package.version or cargoToml.workspace.package.version;
    in {
      formatter = std.mapAttrs (system: pkgs: pkgs.nixfmt-rfc-style) nixpkgsFor;
      packages =
        std.mapAttrs (system: pkgs: let
          crane = craneFor.${system};
          src = crane.cleanCargoSource (crane.path ./.);
        in {
          default = self.packages.${system}.${name};
          "${name}-artifacts" = crane.buildDepsOnly commonArgsFor.${system};
          ${name} = crane.buildPackage (commonArgsFor.${system}
            // {
              cargoArtifacts = self.packages.${system}."${name}-artifacts";
            });
        })
        nixpkgsFor;
      checks =
        std.mapAttrs (system: pkgs: let
          crane = craneFor.${system};
          commonArgs = commonArgsFor.${system};
          cargoArtifacts = self.packages.${system}."${name}-artifacts";
        in {
          ${name} = pkgs.${name};
          "${name}-clippy" = crane.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
            });
          "${name}-coverage" = crane.cargoTarpaulin (commonArgs
            // {
              inherit cargoArtifacts;
            });
          "${name}-audit" = crane.cargoAudit (commonArgs
            // {
              pname = name;
              inherit version;
              inherit cargoArtifacts;
              advisory-db = inputs.advisory-db;
            });
          "${name}-deny" = crane.cargoDeny (commonArgs
            // {
              inherit cargoArtifacts;
            });
        })
        self.packages;
      apps =
        std.mapAttrs (system: pkgs: {
          ${name} = {
            type = "app";
            program = "${pkgs.${name}}/bin/${name}";
          };
          default = self.apps.${system}.${name};
        })
        self.packages;
      devShells =
        std.mapAttrs (system: pkgs: let
          selfPkgs = self.packages.${system};
          toolchain = toolchainFor.${system}.override {
            extensions = [
              "rust-analyzer"
              "rustfmt"
              "clippy"
              "rust-src"
            ];
          };
          crane = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;
          commonArgs = commonArgsFor.${system};
        in {
          ${name} = (pkgs.mkShell.override {inherit (commonArgs) stdenv;}) {
            inputsFrom = (attrValues self.checks.${system}) ++ [selfPkgs.${name}];
            packages =
              [toolchain]
              ++ (with pkgs; [
                cargo-audit
                cargo-license
                cargo-dist
                renderdoc
              ]);
            inherit (commonArgs) hardeningDisable;
            shellHook = let
              extraLdPaths = pkgs.lib.makeLibraryPath (with pkgs; [
                vulkan-loader
                vulkan-validation-layers
                libGL
                libxkbcommon
                wayland
                # winit x11 support
                xorg.libX11
                xorg.libXcursor
                xorg.libxcb
                xorg.libXi
              ]);
              extraVkLayersExplicit = pkgs.lib.makeSearchPath "share/vulkan/explicit_layer.d" (with pkgs; [
                vulkan-validation-layers
              ]);
              extraVkLayersImplicit = pkgs.lib.makeSearchPath "share/vulkan/implicit_layer.d" (with pkgs; [
                renderdoc
              ]);
            in ''
              export LD_LIBRARY_PATH="${extraLdPaths}:$LD_LIBRARY_PATH"
              export VK_ADD_LAYER_PATH="${extraVkLayersExplicit}:${extraVkLayersImplicit}:$VK_ADD_LAYER_PATH"
            '';
            env = {
              VK_LOADER_LAYERS_ENABLE = "*validation";
            };
          };
          default = self.devShells.${system}.${name};
        })
        nixpkgsFor;
    };
}
