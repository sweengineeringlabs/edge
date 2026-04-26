# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial daemon workspace: `RuntimeManager` trait, `DefaultRuntimeManager` impl
- `IngressGateway` and `EgressGateway` boundary types wiring ingress/egress ports
- systemd `sd_notify` support (READY=1, STOPPING=1)
