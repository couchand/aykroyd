# Changelog

All notable changes to this project will be documented in this file.

The format of this file is based on the recommendations in
[Keep a Changelog].
Like most crates in the Rust ecosystem this project adheres to
[Semantic Versioning].

## [Unreleased]

- Support for the MySQL and SQLite databases, in addition to
  PostgreSQL.
- `FromRow` for nested structs using `FromColumnsIndexed` od
  `FromColumnsNamed`.
- The `Either` combinator, for dynamic query choice.
- Explicit column names and indexes and parameter indexes have
  been removed temporarily.  Expect to see them again soon!

## [v0.2.0] - 2023-04-18 ([Log][v0.2.0-log])

- The feature `derive` is no longer default.

## [v0.1.1] - 2020-08-12 ([Log][v0.1.1-log])

- Documentation has been improved.

## [v0.1.0] - 2020-08-12 ([Log][v0.1.0-log])

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
