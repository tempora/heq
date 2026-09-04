{
  description = "heq — headphone EQ for Equalizer APO";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        default = heq;
        heq = pkgs.callPackage ./nix/heq.nix { };
      });

      apps = forAllSystems (pkgs:
        let
          heq = self.packages.${pkgs.system}.heq;
          wine = pkgs.wineWow64Packages.stable;
          run = pkgs.writeShellScriptBin "heq" ''
            exec ${wine}/bin/wine64 ${heq}/bin/heq.exe "$@"
          '';
        in
        nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          default = { type = "app"; program = "${run}/bin/heq"; };
        });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.dotnet-sdk_10
            pkgs.cargo
            pkgs.rustc
            pkgs.rustfmt
            pkgs.clippy
            pkgs.rust-analyzer
          ] ++ pkgs.lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.wineWow64Packages.stable;

          # Building net10.0-windows off Windows needs the ref packs fetched explicitly.
          env = {
            DOTNET_ROOT = "${pkgs.dotnet-sdk_10}";
            DOTNET_CLI_TELEMETRY_OPTOUT = "1";
            DOTNET_NOLOGO = "1";
            EnableWindowsTargeting = "true";
          };
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt-rfc-style);
    };
}
