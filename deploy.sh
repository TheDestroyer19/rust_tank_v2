#!/bin/sh

#move to right dir
pwd

#build for armv6
echo "Building" &&
cargo +nightly build --target=arm-unknown-linux-gnueabihf &&

#copy file to target system (pi@raspberrypi.local)
echo "Tranfering to pi@raspberrypi.local" &&
scp target/arm-unknown-linux-gnueabihf/debug/rust_tank_v2 pi@raspberrypi.local:~/debug_rust_tank &&

#tell remote to start program
echo "Starting Program" &&
ssh -t pi@raspberrypi.local "sudo RUST_BACKTRACE=1 ./debug_rust_tank 2> msgs.txt"
