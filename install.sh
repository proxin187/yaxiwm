#!/bin/sh

cargo build --release --bin yokac

sudo cp target/release/yokac /usr/bin/yokac

cargo build --release --bin yokai

sudo cp target/release/yokai /usr/bin/yokai

