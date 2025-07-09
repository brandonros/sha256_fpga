# sha256_fpga
Proof-of-concept SHA256 implementation for FPGA

## How to use

```shell
# generate top.v (rust -> verilog)
cargo run
```

## How to test

```shell
cd test
make && make run
```

## How to flash

```shell
# Sipeed Tang Nano 20K: GW2AR-LV18QN88C8/I7

# install deps
apt-get update && apt-get install -y cmake build-essential libboost-all-dev libeigen3-dev python3-pip python3-venv

# activate python venv
python3 -m venv .venv
source .venv/bin/activate

# pip install
pip install apycula

# cmake
mkdir build
cd build
cmake .. \
  -DARCH="himbaechel" \
  -DHIMBAECHEL_UARCH="gowin"

# yosys
yosys -D LEDS_NR=6 -p "read_verilog top.v; synth_gowin -json top.json"

# nextpnr
DEVICE='GW2AR-LV18QN88C8/I7'
BOARD='tangnano20k'
../../nextpnr-himbaechel --json ../../top.json \
                   --write ../../pnrtop.json \
                   --device $DEVICE \
                   --vopt family=GW2A-18C \
                   --vopt cst=$BOARD.cst

# gowin_pack
gowin_pack -d $DEVICE -o pack.fs pnrtop.json


# flash
openFPGALoader -b $BOARD pack.fs
```
