# Commands

build: build-rs build-kernel-module

run device: build-rs _sudo
	sudo ./target/debug/netwatch_cli {{device}}

list: build-rs _sudo
	sudo ./target/debug/netwatch_cli

# Rust

build-rs:
	cargo build

# Linux Kernel Module

build-kernel-module:
	[ "{{os()}}" == "linux" ] && cd pid_inode_map && make

install-kernel-module: build-kernel-module _sudo
	cd pid_inode_map && sudo insmod pid_inode_map.ko

remove-kernel-module: _sudo
	sudo rmmod pid_inode_map

# Helpers

_sudo:
	sudo -v
