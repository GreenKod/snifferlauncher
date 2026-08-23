#!/usr/bin/env bash
git config core.hooksPath .githooks
chmod +x .githooks/* || true
echo "✔ Git hooks successfully configured to use .githooks directory!"
