{
  description = "Development environment for the PrimeVul Joern enrichment pipeline";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    joern-cli = {
      url = "https://github.com/joernio/joern/releases/download/v4.0.536/joern-cli.zip";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      joern-cli,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          jdk = pkgs.jdk21_headless;
        in
        {
          joern = pkgs.stdenvNoCC.mkDerivation {
            pname = "joern-cli";
            version = "4.0.536";

            src = joern-cli;
            dontUnpack = true;

            nativeBuildInputs = [
              pkgs.makeWrapper
              pkgs.unzip
            ];

            installPhase = ''
              runHook preInstall

              mkdir -p "$out/bin" "$out/share"
              if [ -d "$src" ]; then
                cp -R "$src" "$out/share/joern-unpacked"
                chmod -R u+w "$out/share/joern-unpacked"
              else
                unzip -q "$src" -d "$out/share/joern-unpacked"
              fi

              if [ -d "$out/share/joern-unpacked/joern-cli" ]; then
                root="$out/share/joern-unpacked/joern-cli"
              elif [ -x "$out/share/joern-unpacked/joern" ]; then
                root="$out/share/joern-unpacked"
              else
                root="$(find "$out/share/joern-unpacked" -maxdepth 1 -mindepth 1 -type d | head -n 1)"
              fi

              if [ -z "''${root:-}" ]; then
                echo "could not locate Joern CLI files in $src" >&2
                exit 1
              fi

              mkdir -p "$out/share/joern-cli"
              cp -R "$root"/. "$out/share/joern-cli"/
              chmod -R u+w "$out/share/joern-cli"

              for exe in "$out/share/joern-cli"/*; do
                if [ -f "$exe" ] && [ -x "$exe" ]; then
                  makeWrapper "$exe" "$out/bin/$(basename "$exe")" \
                    --prefix PATH : "${pkgs.lib.makeBinPath [ jdk ]}" \
                    --set JAVA_HOME "${jdk.home}"
                fi
              done

              runHook postInstall
            '';
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          joern = self.packages.${system}.joern;
          jdk = pkgs.jdk21_headless;
        in
        {
          default = pkgs.mkShell {
            packages = [
              pkgs.rustc
              pkgs.cargo
              pkgs.clippy
              pkgs.rustfmt
              pkgs.rust-analyzer

              joern
              jdk

              pkgs.git
              pkgs.openssh
              pkgs.cacert

              pkgs.clang
              pkgs.gcc
              pkgs.gnumake
              pkgs.cmake
              pkgs.pkg-config
              pkgs.binutils

              pkgs.jq
              pkgs.which
              pkgs.unzip
            ];

            JAVA_HOME = jdk.home;
            SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            GIT_SSL_CAINFO = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";

            shellHook = ''
              echo "Rust: $(rustc --version)"
              echo "Cargo: $(cargo --version)"
              echo "Java: $(java -version 2>&1 | head -n 1)"
              echo "Joern: $(command -v joern)"
              echo
              echo "Try: cargo run -- --skip-joern --input data/primevul/primevul_valid.jsonl --output /tmp/primevul-enriched.jsonl --limit 1"
            '';
          };
        }
      );
    };
}
