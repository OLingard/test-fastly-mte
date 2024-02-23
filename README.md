# Fastly Entrypoint in Rust

## Fastly CLI

Make sure that you have the [fastly cli](https://developer.fastly.com/learning/tools/cli/) installed and configured with
the fastly token.

To serve it locally, run

```shell
fastly compute serve
```

or to deploy it, run

```shell
fastly compute build
fastly compute deploy
```

You can remove or update the `service_id` in `fastly.toml` if you wish to deploy it to a different C@E service

## Unit Tests

The `--nocapture` flag shows the `println!()` outputs in the test results.

```shell
cargo test -- --nocapture
```

## Url Transformation
The URL transformation rule handles redirects and rewrites defined by the user. These configs are stored in the fastly KV store. 

The rules structure has two parts, `simple_rules` and `regexes`

A rule is a redirect if it has a `permanent` field value. Otherwise, it is a rewrite.

## Local KV
Inside [fastly.toml](./fastly.toml) the local-server block has been modified to include a mock KV that serves redirects from [local](./local/)

## Formatting
```
cargo fmt --all
```

## Logs
Logs are sent to an opensearch collection within the thgvega stack in eu-west-1.
They can be viewed via the aws dashboard.
