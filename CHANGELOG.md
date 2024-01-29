# Changelog

All notable changes to this project will be documented in this file.

The format of this file is based on the recommendations in
[Keep a Changelog].
Like most crates in the Rust ecosystem this project adheres to
[Semantic Versioning].

## [Unreleased]

- *nothing yet*

## [v0.3.0] - 2024-01-29 ([Log][v0.3.0-log])

- Support for the MySQL and SQLite databases, in addition to
  PostgreSQL.
- `FromRow` for nested structs using `FromColumnsIndexed` or
  `FromColumnsNamed`.
- The `Either` combinator, for dynamic query choice.

### Breaking

- The PostgreSQL clients have been moved into modules matching
  the backend crate: `sync_client::Client` is now `postgres::Client`
  and `async_client::Client` is now `tokio_postgres::Client`.
- The derive macros for `FromRow`, `Statement`, `Query`, and `QueryOne`
  now use the attribute `aykroyd` for all configuration parameters
  rather than `query`.
- Explicit column indexes are now only allowed if every field in the
  struct is annotated.
- Queries loaded from file are now taken from a `queries/` directory
  at the crate root.

## [v0.2.0] - 2023-12-11 ([Log][v0.2.0-log])

- The feature `derive` is no longer default.

## [v0.1.1] - 2020-12-11 ([Log][v0.1.1-log])

- Documentation has been improved.

## [v0.1.0] - 2020-12-11 ([Log][v0.1.0-log])

- Initial release of `aykroyd`.
- Traits for `Query`, `QueryOne`, and `Statement` types, binding
  SQL text, parameter, and result types together.
- The `FromRow` trait for deserializing database rows.
- PostgreSQL clients, both synchronous and asynchronous, by
  wrapping `tokio-postgres`.

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html
[Unreleased]: https://git.sr.ht/~couch/aykroyd/log
[v0.2.0]: https://git.sr.ht/~couch/aykroyd/refs/v0.2.0
[v0.2.0-log]: https://git.sr.ht/~couch/aykroyd/log/v0.2.0
[v0.1.1]: https://git.sr.ht/~couch/aykroyd/refs/v0.1.1
[v0.1.1-log]: https://git.sr.ht/~couch/aykroyd/log/v0.1.1
[v0.1.0]: https://git.sr.ht/~couch/aykroyd/refs/v0.1.0
[v0.1.0-log]: https://git.sr.ht/~couch/aykroyd/log/v0.1.0
