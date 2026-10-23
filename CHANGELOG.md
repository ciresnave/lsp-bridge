# Changelog

All notable changes to the LSP Bridge project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1]

### Changed

- Minimum supported Rust version is now declared: `rust-version = "1.89"` (edition 2024; `uuid` needs 1.89).
- Dev/runtime dependencies updated: `rand` 0.10, `criterion` 0.8 (dev); lockfile refreshed.
- CI gains an MSRV job (1.89).

### Fixed

- README quick-start pinned `lsp-bridge = "0.1"`; it now says `"0.2"`.
- README CI badge and clone URL pointed at `your-username/lsp-bridge`; they now point at `ciresnave/lsp-bridge`.

### Added

- Complete implementation of LSP 3.17 specification
- Server lifecycle management (startup, shutdown, crash recovery)
- Asynchronous communication with tokio runtime
- Document synchronization and state management
- Comprehensive error handling with detailed context
- Support for multiple LSP servers running simultaneously
- Automatic server capability detection and negotiation
- Performance monitoring and profiling capabilities
- Memory usage tracking and optimization
- Resource limiting and hardening features
- Comprehensive testing framework with real LSP integration
- Thread-safe concurrent operations with DashMap
- Structured logging with tracing integration
- Builder pattern configuration with validation
- Request/response correlation and timeout handling
- Process management with proper cleanup
- Circuit breaker pattern for robustness
- Rate limiting and backpressure handling

### Core Features Implemented

- **Bridge Management**: Multi-server coordination and routing
- **Document Operations**: Open, close, update, and synchronization
- **LSP Features**:
  - Code completion with filtering and ranking
  - Hover information with rich content
  - Go-to-definition with link support
  - Document formatting and range formatting
  - Diagnostics publishing and clearing
  - Find references with context
  - Rename operations with validation
  - Code actions and quick fixes
  - Workspace symbols search
- **Configuration**: Flexible server configuration with validation
- **Monitoring**: Health checks, metrics collection, and observability
- **Security**: Input validation, resource limits, and safe defaults

### Testing & Quality

- Unit tests for all public APIs (34 passing tests)
- Integration tests with real LSP servers
- Performance benchmarks and profiling
- Memory leak detection and validation
- Error condition testing and recovery
- Concurrent access testing
- Property-based testing for protocol compliance

### Documentation

- Comprehensive API documentation with examples
- Architecture documentation with diagrams
- Contributing guidelines for new developers
- Real-world usage examples and tutorials
- Performance optimization guide
- Troubleshooting and debugging guide

### Development Tools

- Automated CI/CD pipeline setup
- Code formatting and linting with clippy
- Performance regression detection
- Memory usage monitoring
- Security vulnerability scanning
- Dependency license compliance checking

## Versioning Guidelines

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR** (x.0.0): Breaking API changes
- **MINOR** (0.x.0): New features, backward compatible
- **PATCH** (0.0.x): Bug fixes, backward compatible

### Pre-1.0.0 Versioning

During the 0.x.x phase:

- **MINOR** versions (0.x.0) may include breaking changes
- **PATCH** versions (0.0.x) are for bug fixes and small improvements
- Breaking changes will be clearly documented in the changelog

### API Stability Promise

Starting with version 1.0.0:

- Public APIs will follow strict semantic versioning
- Breaking changes will only occur in major versions
- Deprecation warnings will be provided for at least one minor version
- Migration guides will be provided for major version upgrades

### Release Schedule

- **Patch releases**: As needed for critical bug fixes
- **Minor releases**: Monthly for new features and improvements  
- **Major releases**: As needed for significant architectural changes

### Supported Rust Versions

- **Minimum Supported Rust Version (MSRV)**: 1.64.0
- MSRV changes are considered breaking changes
- Support for the latest stable Rust version is guaranteed
- Support for the previous 6 stable releases is maintained when possible

### Fixed

- None (initial release)

### Security

- Input validation for all messages received from LSP servers
- Rate limiting for request frequencies
- Resource limiting to prevent excessive memory usage
- Circuit breakers to handle misbehaving servers

## [0.1.0] - TBD

Initial release

# Versioning Guidelines

LSP Bridge follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version for incompatible API changes
- **MINOR** version for new functionality in a backwards compatible manner
- **PATCH** version for backwards compatible bug fixes

## Version Numbering

- **0.x.y**: Alpha/Beta releases (API may change without warning)
- **1.x.y**: Stable releases (API stable except for major version bumps)

## API Stability Guarantees

Once LSP Bridge reaches 1.0.0, the following stability guarantees apply:

1. **Public API**: All public functions, structs, and traits in non-experimental modules
   are considered stable and will only have breaking changes in major releases.

2. **Feature Flags**: Enabling different feature flags may change behavior but not in
   a breaking way for existing code.

3. **Dependencies**: Major version changes to public dependencies may trigger a major
   version change in LSP Bridge.

## Experimental Features

Some features may be marked as experimental using:

- Documentation comments stating "**EXPERIMENTAL**"
- Modules prefixed with `experimental_`
- Features flags prefixed with `experimental-`

Experimental features do not have the same stability guarantees.

## Release Process

1. Update version in Cargo.toml
2. Update CHANGELOG.md with release notes
3. Create a git tag with the version number
4. Publish to crates.io
5. Create a GitHub release

## Deprecation Policy

1. Features are marked as deprecated at least one minor release before removal
2. Deprecation warnings provide migration paths where possible
3. Deprecated features will only be removed in major releases
