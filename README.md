# proxyboi

[![GitHub Actions Workflow](https://github.com/svenstaro/proxyboi/workflows/Build/badge.svg)](https://github.com/svenstaro/proxyboi/actions)
[![Docker Cloud Build Status](https://img.shields.io/docker/cloud/build/svenstaro/proxyboi)](https://cloud.docker.com/repository/docker/svenstaro/proxyboi/)
[![AUR](https://img.shields.io/aur/version/proxyboi.svg)](https://aur.archlinux.org/packages/proxyboi/)
[![Crates.io](https://img.shields.io/crates/v/proxyboi.svg)](https://crates.io/crates/proxyboi)
[![dependency status](https://deps.rs/repo/github/svenstaro/proxyboi/status.svg)](https://deps.rs/repo/github/svenstaro/proxyboi)
[![license](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/svenstaro/proxyboi/blob/master/LICENSE)


**A super simple reverse proxy with TLS support**

## How to run

In order to listen for proxy requests on all local interfaces on port 8080 and proxy them to a service running at example.com, do:

    proxyboi -l 0.0.0.0:8080 http://example.com

You can also feed your TLS certificates into it:

    proxyboi -l 0.0.0.0:8080 --cert mycert.pem --key mykey.key http://example.com

You can see a detailed (and pretty!) verbose log using `-v`:

    proxyboi -l 0.0.0.0:8080 http://example.com -v

![Pretty log](pretty_log.png)

## Releasing

This is mostly a note for me on how to release this thing:

- Update version in `Cargo.toml` and run `cargo update`
- `git commit` and `git tag -s`, `git push`
- Run `cargo publish`
- Releases will automatically be deployed by GitHub Actions
- Update AUR package
