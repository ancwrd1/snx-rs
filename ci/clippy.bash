#!/usr/bin/env bash
# Script for building your rust projects.
set -e

source ci/common.bash

# $1 {path} = Path to cross/cargo executable
CROSS=$1
# $2 {string} = <Target Triple>
TARGET_TRIPLE=$2

required_arg "$CROSS" 'CROSS'
required_arg "$TARGET_TRIPLE" '<Target Triple>'

$CROSS clippy --target "$TARGET_TRIPLE" --all-features --workspace
