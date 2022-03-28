#!/bin/bash
set -uev

virsh net-autostart default

# Check if the default NAT network is 192.168.122.0/24.
sudo cat /etc/libvirt/qemu/networks/default.xml | grep "ip address='192.168.122.1'"
