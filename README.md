# proxyboi

[![GitHub Actions Workflow](https://github.com/svenstaro/proxyboi/workflows/Build/badge.svg)](https://github.com/svenstaro/proxyboi/actions)
[![Docker Cloud Build Status](https://img.shields.io/docker/cloud/build/svenstaro/proxyboi)](https://cloud.docker.com/repository/docker/svenstaro/proxyboi/)

**A super simple reverse proxy with TLS support**

## How to run

In order to listen for proxy requests on all local interfaces on port 8080 and proxy them to a service running at example.com, do:

    proxyboi -l 0.0.0.0:8080 http://example.com

You can also feed your TLS certificates into it:

    proxyboi -l 0.0.0.0:8080 --cert mycert.pem --key mykey.key http://example.com

## Releasing

This is mostly a note for me on how to release this thing:

- Update version in `Cargo.toml` and run `cargo update`
- `git commit` and `git tag -s`, `git push`
- Run `cargo publish`
- Releases will automatically be deployed by GitHub Actions
- Update AUR package
