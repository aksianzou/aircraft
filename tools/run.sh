#!/bin/sh
# Cargo's runner for the cartridge. Test binaries go to the headless
# `mgba-test-runner`:
#   cargo install --git https://github.com/agbrs/agb --tag v0.25.0 mgba-test-runner
# The game opens in mGBA with the cartridge's log on stdout: `mgba-qt` when it
# is on the path, otherwise the macOS app.
case "$1" in
*/deps/*) exec mgba-test-runner "$@" ;;
esac
if command -v mgba-qt >/dev/null 2>&1; then
    mgba=mgba-qt
else
    mgba=/Applications/mGBA.app/Contents/MacOS/mGBA
fi
exec "$mgba" -C logToStdout=1 -C logLevel.gba.debug=127 "$@"
