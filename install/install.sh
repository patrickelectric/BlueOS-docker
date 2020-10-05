#!/usr/bin/env bash

# Set desired version to be installed
VERSION="${VERSION:-master}"
REMOTE="${REMOTE:-https://raw.githubusercontent.com/bluerobotics/companion-docker}"
REMOTE="$REMOTE/$VERSION"

# Exit immediately if a command exits with a non-zero status
set -e

# Check if the script is running in ARM architecture
[[ "$(uname -m)" != "arm"* ]] && (
    echo "Companion only supports ARM computers."
    exit 1
)

# Check if the script is running as root
[[ $EUID != 0 ]] && echo "Script must run as root."  && exit 1

# Detect CPU and do necessary hardware configuration for each supported hardware
CPU_INFO="$(cat /proc/cpuinfo)"
echo $CPU_INFO | grep -Eq "BCM(27|28)" && (
    curl -fsSL $REMOTE/install/boards/bcm_28_27.sh | bash
)

echo "Checking for blocked wifi and bluetooth."
rfkill unblock all

# Check for docker and install it if not found
echo "Checking for docker."
docker --version || curl -fsSL https://get.docker.com | sh && systemctl enable docker

# Stop and remove all docker if NO_CLEAN is not defined
test $NO_CLEAN || (
    # Check if there is any docker installed
    [[ $(docker ps -a -q) ]] && (
        echo "Stopping running dockers."
        docker stop $(docker ps -a -q)

        echo "Removing dockers."
        docker rm $(docker ps -a -q)
    ) || true
)

# Start installing necessary files and system configuration
echo "Going to install companion-docker version ${VERSION}."

echo "Downloading and installing udev rules."
curl -fsSL $REMOTE/install/udev/100.autopilot.rules -o /etc/udev/rules.d/100.autopilot.rules

#echo "Check necessary amount of memory."
#curl -fsSL $REMOTE/install/configure_swap.sh | bash
#RAM_MEMORY_TOTAL_MB=$(free -mt | grep Total | grep -oE [0-9]+ | head -n1)
#DISK_MEMORY_AVAILABLE_Mb=$(df -B1MB --output=avail,target | grep /$ | grep -oE [0-9]+)
#if (( $RAM_MEMORY_TOTAL_MB < 1024 ));then
#fi

echo "Downloading bootstrap"
COMPANION_BOOTSTRAP="bluerobotics/companion-bootstrap:master" # We don't have others tags for now
docker pull $COMPANION_BOOTSTRAP
# Start bootstrap for the first time to fetch the other images and allow docker to restart it after reboot
docker run \
    -it \
    --restart unless-stopped \
    --name companion-bootstrap \
    -v $HOME/.config/companion:/root/.config/companion \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -e COMPANION_CONFIG_PATH=$HOME/.config/companion \
    $COMPANION_BOOTSTRAP

echo "Installation finished successfully."
echo "System will reboot in 10 seconds."
sleep 10 && reboot