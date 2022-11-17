{
  outputs = { nixpkgs, ... }: 
  let
    pkgs = nixpkgs.legacyPackages.x86_64-linux;
  in {
    devShell.x86_64-linux = pkgs.mkShell {
      packages = with pkgs; [ rustc cargo pkg-config openssl.dev cmake rust-analyzer ];
      hardeningDisable = [ "fortify" ];
    };
  };
}
