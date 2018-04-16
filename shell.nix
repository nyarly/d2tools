{ pkgs ? import <nixpkgs> {} }:
with pkgs;
stdenv.mkDerivation {
  name = "d2tools";
  buildInputs = [ openssl zlib sqlite sqlite-interactive apacheHttpd ];
  nativeBuildInputs = [ pkgconfig ];
}
