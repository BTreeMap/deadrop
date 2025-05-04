#!/bin/bash
# install-deps.sh - Installs dependencies for deadrop.sh

# Loosely based on https://tailscale.com/install.sh

set -euo pipefail

main() {
	# Step 1: Detect OS, version, and package manager
	OS=""
	VERSION=""
	PACKAGETYPE=""

	if [ -f /etc/os-release ]; then
		. /etc/os-release
		case "$ID" in
			ubuntu|pop|neon|zorin|tuxedo|debian|linuxmint|elementary|parrot|mendel|galliumos|pureos|kaisen|raspbian|kali|Deepin|deepin|pika|sparky|osmc)
				OS="debian_based"
				PACKAGETYPE="apt"
				;;
			centos|ol|rhel|miraclelinux|xenenterprise)
				OS="rhel_based"
				PACKAGETYPE="yum_dnf"
				# Check if dnf is available, otherwise use yum
				if command -v dnf >/dev/null 2>&1; then
					PACKAGETYPE="dnf"
				else
					PACKAGETYPE="yum"
				fi
				;;
			fedora|rocky|almalinux|nobara|openmandriva|sangoma|risios|cloudlinux|alinux|fedora-asahi-remix)
				OS="fedora_based"
				PACKAGETYPE="dnf"
				;;
			amzn)
				OS="amazon-linux"
				PACKAGETYPE="yum"
				;;
			opensuse-leap|sles|opensuse-tumbleweed|sle-micro-rancher)
				OS="opensuse"
				PACKAGETYPE="zypper"
				;;
			arch|archarm|endeavouros|blendos|garuda|archcraft|cachyos|manjaro|manjaro-arm|biglinux)
				OS="arch_based"
				PACKAGETYPE="pacman"
				;;
			alpine|postmarketos)
				OS="alpine"
				PACKAGETYPE="apk"
				;;
			void)
				OS="void"
				PACKAGETYPE="xbps"
				;;
			gentoo)
				OS="gentoo"
				PACKAGETYPE="emerge"
				;;
			freebsd)
				OS="freebsd"
				PACKAGETYPE="pkg"
				;;
			photon)
				OS="photon"
				PACKAGETYPE="tdnf"
				;;
		esac
	fi

	# Fallback using uname if /etc/os-release didn't work
	if [ -z "$OS" ]; then
		if type uname >/dev/null 2>&1; then
			case "$(uname)" in
				FreeBSD)
					OS="freebsd"
					PACKAGETYPE="pkg"
					;;
				Linux)
					# Generic Linux - try common package managers
					if command -v apt-get >/dev/null 2>&1; then PACKAGETYPE="apt"; OS="debian_based";
					elif command -v dnf >/dev/null 2>&1; then PACKAGETYPE="dnf"; OS="fedora_based";
					elif command -v yum >/dev/null 2>&1; then PACKAGETYPE="yum"; OS="rhel_based";
					elif command -v pacman >/dev/null 2>&1; then PACKAGETYPE="pacman"; OS="arch_based";
					elif command -v apk >/dev/null 2>&1; then PACKAGETYPE="apk"; OS="alpine";
					elif command -v zypper >/dev/null 2>&1; then PACKAGETYPE="zypper"; OS="opensuse";
					elif command -v xbps-install >/dev/null 2>&1; then PACKAGETYPE="xbps"; OS="void";
					elif command -v emerge >/dev/null 2>&1; then PACKAGETYPE="emerge"; OS="gentoo";
					fi
					;;
				# Add other OS checks like Darwin if needed, though deadrop.sh targets Linux/BSDs primarily
			esac
		fi
	fi

	if [ -z "$PACKAGETYPE" ]; then
		echo "Error: Could not detect package manager." >&2
		echo "Please install the following dependencies manually: age, curl, jq, coreutils (for base64)" >&2
		exit 1
	fi

	# Step 2: Determine root command
	SUDO=""
	if [ "$(id -u)" != 0 ]; then
		if command -v sudo >/dev/null 2>&1; then
			SUDO="sudo"
		elif command -v doas >/dev/null 2>&1; then
			SUDO="doas"
		else
			echo "Error: This script requires root privileges to install packages." >&2
			echo "Please run as root or install/configure sudo or doas." >&2
			exit 1
		fi
	fi

	# Step 3: Install packages
	echo "Detected package manager: $PACKAGETYPE. Attempting to install dependencies..." >&2

	case "$PACKAGETYPE" in
		apt)
			$SUDO apt-get update
			# Need gnupg for age PPA, coreutils provides base64
			$SUDO apt-get install -y age curl jq coreutils gnupg
			;;
		yum)
			# EPEL might be needed for age on older RHEL/CentOS
			# $SUDO yum install -y epel-release || echo "EPEL release already installed or not needed."
			$SUDO yum install -y age curl jq coreutils
			;;
		dnf)
			$SUDO dnf install -y age curl jq coreutils
			;;
		pacman)
			$SUDO pacman -Sy --noconfirm age curl jq coreutils
			;;
		apk)
			# Ensure community repo is enabled for age
			if ! grep -Eq '^http.*/community$' /etc/apk/repositories; then
				if type setup-apkrepos >/dev/null; then
					$SUDO setup-apkrepos -c -1
				else
					echo "Warning: Community repo may need to be enabled in /etc/apk/repositories for 'age'." >&2
				fi
			fi
			$SUDO apk add age curl jq coreutils
			;;
		pkg)
			$SUDO pkg install -y age curl jq
			# base64 is usually built-in or part of base system on FreeBSD
			;;
		zypper)
			$SUDO zypper --non-interactive install age curl jq coreutils
			;;
		xbps)
			$SUDO xbps-install -Sy age curl jq coreutils
			;;
		emerge)
			$SUDO emerge --ask=n app-crypt/age net-misc/curl app-misc/jq sys-apps/coreutils
			;;
		tdnf)
			$SUDO tdnf install -y age curl jq coreutils
			;;
		*)
			echo "Error: Unsupported package manager '$PACKAGETYPE'." >&2
			echo "Please install the following dependencies manually: age, curl, jq, coreutils (for base64)" >&2
			exit 1
			;;
	esac

	echo "Dependency installation attempt finished." >&2
	echo "Please verify that 'age', 'curl', 'jq', and 'base64' commands are now available." >&2
}

# Run the main function
main "$@"
