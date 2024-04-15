#!/usr/bin/env bash
set -euo pipefail

sudo apt-get update
sudo apt-get install -y --no-install-recommends shellcheck

sudo npm install -g markdownlint-cli
