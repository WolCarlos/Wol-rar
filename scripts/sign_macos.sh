#!/usr/bin/env bash
set -euo pipefail

APP_PATH="${1:?Percorso Wol-rar.app obbligatorio}"

if [[ -n "${MACOS_CERTIFICATE_BASE64:-}" ]]; then
    [[ -n "${MACOS_CERTIFICATE_PASSWORD:-}" ]]
    [[ -n "${MACOS_CERTIFICATE_NAME:-}" ]]
    mkdir -p certs build
    printf '%s' "$MACOS_CERTIFICATE_BASE64" | base64 --decode > certs/signing.p12
    security create-keychain -p "wol-rar-build" build/signing.keychain-db
    security set-keychain-settings -lut 3600 build/signing.keychain-db
    security unlock-keychain -p "wol-rar-build" build/signing.keychain-db
    security import certs/signing.p12 -k build/signing.keychain-db -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
    security set-key-partition-list -S apple-tool:,apple: -s -k "wol-rar-build" build/signing.keychain-db
    codesign --force --deep --options runtime --timestamp --sign "$MACOS_CERTIFICATE_NAME" --keychain build/signing.keychain-db "$APP_PATH"
    security delete-keychain build/signing.keychain-db
    rm -f certs/signing.p12
else
    codesign --force --deep --sign - "$APP_PATH"
fi

codesign --verify --deep --strict --verbose=2 "$APP_PATH"

if [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    ditto -c -k --keepParent "$APP_PATH" build/notarize.zip
    xcrun notarytool submit build/notarize.zip --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
    xcrun stapler staple "$APP_PATH"
fi
