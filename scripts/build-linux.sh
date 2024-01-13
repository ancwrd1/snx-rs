#!/bin/bash

cross build --target=x86_64-unknown-linux-gnu --release --features reqwest/native-tls-vendored,tray-icon
