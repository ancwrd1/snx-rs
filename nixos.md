## NixOS Installation and Configuration

1. Add the unstable channel: `sudo nix-channel --add https://nixos.org/channels/nixos-unstable nixos-unstable`
2. Update channels: `sudo nix-channel --update`
3. Modify `/etc/nixos/configuration.nix` which will install `snx-rs` and configure the system.

```nix
{ config, pkgs, ... }:
let unstable = import <nixos-unstable> {};
in {
  environment.systemPackages = with pkgs; [
    unstable.snx-rs
  ];

  systemd.services.snx-rs = {
    enable = true;
    path = [pkgs.iproute2 pkgs.kmod pkgs.networkmanager]; # for ip, modprobe and nmcli commands
    description = "SNX-RS VPN client for Linux";
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
        ExecStart = "${unstable.pkgs.snx-rs}/bin/snx-rs -m command -l debug";
        Type = "simple";
    };
  };
  
  # update the firewall rule to allow keepalive traffic
  networking.firewall.checkReversePath = "loose";
```
