{pkgs, ...}: {
  packages = [pkgs.figlet pkgs.lolcat];

  languages.rust = {
    channel = "stable";
    enable = true;
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-analyzer"];
  };

  enterShell = ''
    echo DRSM | figlet -c -f slant | lolcat -a
  '';

  git-hooks.hooks = {
    clippy = {
      enable = true;
      packageOverrides.cargo = pkgs.cargo;
      packageOverrides.clippy = pkgs.clippy;
      settings.allFeatures = true;
      settings.extraArgs = "--all-targets -- -D -warnings";
    };
    rustfmt = {
      enable = true;
      packageOverrides.cargo = pkgs.cargo;
      packageOverrides.rustfmt = pkgs.rustfmt;
      settings.all = true;
    };
    unit-tests = {
      enable = true;
      name = "Unit Tests";
      entry = "cargo test";
      files = "\\.rs$";
      pass_filenames = false;
    };
  };
}
