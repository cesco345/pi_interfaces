WORKDIR=$(cd $(dirname $0); pwd)
sudo apt-get install libusb-dev libpcsclite-dev libtool 
sudo apt-get install automake autoconf
cd ${WORKDIR}/libnfc
autoreconf -vis
./configure --prefix=/usr --sysconfdir=/etc
make  
sudo make install
sudo mkdir -p /etc/nfc
sudo cp ${WORKDIR}/libnfc/libnfc.conf.sample /etc/nfc/libnfc.conf
echo 'device.connstring = "pn532_uart:/dev/ttyS0"' | sudo tee -a /etc/nfc/libnfc.conf
