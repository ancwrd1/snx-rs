## NixOS Installation and Configuration

The following example from `/etc/nixos/configuration.nix` installs `snx-rs` and configures the system.

```nix
{ config, pkgs, ... }:
let unstable = import <nixos-unstable> {};
in {
  environment.systemPackages = with pkgs; [
    unstable.snx-rs
  ];

  systemd.services.snx-rs = {
    enable = true;
    path = [pkgs.iproute2 pkgs.kmod]; # a required parameter to run the "ip" and "modprobe"
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
