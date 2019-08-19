# proxyboi
A super simple reverse proxy with TLS support

## Releasing

This is mostly a note for me on how to release this thing:

- Update version in `Cargo.toml` and run `cargo update`.
- `git commit` and `git tag -s`, `git push`.
- `cargo publish`
- Releases will automatically be deployed by Travis.
- Update AUR package.
