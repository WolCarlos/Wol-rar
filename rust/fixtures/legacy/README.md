# Fixture legacy controllate

- `stored.arj`: https://raw.githubusercontent.com/mkrueger/unarc-rs/main/crates/unarc-rs/tests/arj/stored.arj — SHA-256 `3108cf45a88a657c298686260eb2c273ca3394016c14ded60d559af00139a01f`.
- `license_none.cab`: https://raw.githubusercontent.com/mkrueger/unarc-rs/main/crates/unarc-rs/tests/cab/license_none.cab — SHA-256 `a4099757555afa1857832087141413bb6311860aa8dbcd28678a2c0d1aec3758`.
- `LICENSE.Z`: https://raw.githubusercontent.com/mkrueger/unarc-rs/main/crates/unarc-rs/tests/Z/LICENSE.Z — SHA-256 `39bdb89bbd77db375b788a3ba0bdf0107d103a8d525b2f5791e0802e4aa2e031`.
- `expected-license.txt`: https://raw.githubusercontent.com/mkrueger/unarc-rs/main/LICENSE — SHA-256 `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`.
- `stored.lzh`: fixture `-lh0-` originale generata localmente con header level 0 e CRC-16 del contenuto atteso — SHA-256 `77afcd60fa45d45fbb52bfe5cec64ff6c722a63acef983c0b007e979b5fb647b`.

Le fixture ARJ/CAB/Z e il contenuto atteso provengono dal repository MIT OR Apache-2.0 di `unarc-rs`; non contengono codice o asset WinRAR. ISO/Joliet e UUE vengono generati durante i test, mentre traversal e troncamenti sono prodotti localmente dalle stesse fixture.
