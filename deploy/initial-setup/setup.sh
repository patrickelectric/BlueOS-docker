FILE=raspios_lite_armhf-2021-11-08/2021-10-30-raspios-bullseye-armhf-lite.zip
FILENAME=$(basename $FILE)
wget -nc https://downloads.raspberrypi.org/raspios_lite_armhf/images/$FILE
unzip -n $FILENAME

git clone --depth 1  --branch rpi-4.19.y git@github.com:Williangalvani/linux.git

# umount if previously mounted
sudo umount /dev/loop0p1
sudo partx -a -v *.img
# create mountpoints and mount image
mkdir linux/root
mkdir linux/boot
sudo mount /dev/loop0p1 linux/boot
#mount /dev/loop1 linux/root

sh build.sh
sync
sudo umount /dev/loop0p1