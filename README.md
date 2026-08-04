<div align="center">
  <img src="https://raw.githubusercontent.com/matt-andrews/Tempest/main/.assets/tempest.png" width=256 alt="Tempest Logo" >
  <h1>Tempest</h1>

[![Docker Image Size](https://img.shields.io/docker/image-size/mattisthegreatest/tempest?style=for-the-badge)](https://hub.docker.com/r/mattisthegreatest/tempest)
[![Docker Image Version](https://img.shields.io/docker/v/mattisthegreatest/tempest?style=for-the-badge&sort=semver)](https://hub.docker.com/r/mattisthegreatest/tempest)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=for-the-badge)](#license)

</div>

Tempest is a YAML-based HTTP API test runner built for readable, composable test suites.

## Features

- Declarative HTTP tests in YAML
- [CEL](https://cel.dev/) and [Liquid Templates](https://shopify.github.io/liquid/) are used for test preparation and assertions
- Automatic retries with flaky test detection
- Built-in console and json reports - or build your own!
- Nested structure with cascading directory and descriptor options

## Quick start

Create a directory for your tests and add `my-test.spec.yml`:

```yaml
# my-test.spec.yml
name: "Fetch the octocat/Hello-World repo"
test:
  route: "https://api.github.com/repos/octocat/Hello-World"
  headers:
    Accept: "application/vnd.github.v3+json"
  assert:
    - status == 200
    - body.json().full_name == "octocat/Hello-World"
    - body.json().owner.login == "octocat"
```

Mount that directory at `/etc/tests` and run the `test` command:

```bash
# Bash
docker run --rm --volume "$PWD:/etc/tests" mattisthegreatest/tempest:latest test
```

The Docker image supports Linux AMD64 and ARM64.

More executable scenarios are available under [`examples/tests`](https://github.com/matt-andrews/Tempest/tree/main/examples/tests).

## CLI

Tempest currently provides one command:

```text
tempest test [OPTIONS]
```

| Option | Default | Description |
|---|---:|---|
| `--path <PATH>` | `/etc/tests` | Test-project root to discover. |
| `--run <PATH>` | `/` | Run only one spec file or subdirectory relative to `--path`. |
| `-d`, `--debug` | `false` | Render the resolved route and detailed response information. This may expose sensitive response headers or bodies. |
| `--retries <N>` | `0` | Default number of additional attempts after an assertion failure. |
| `--workers <N>` | unset | Maximum number of spec files in flight. Must be greater than zero and enables file concurrency when supplied. |
| `-s`, `--strict` | `false` | Return exit code 2 when the run contains flaky tests but no failures. |
| `-w`, `--warn-as-err` | `false` | Return exit code 1 when Tempest emits any warning. |
| `-h`, `--help` | | Print command help. |

## Project files and discovery

Tempest recursively scans the directory selected by `--path` and recognizes these files:

| Pattern | Purpose |
|---|---|
| `*.spec.yml`, `*.spec.yaml` | Test descriptor trees. |
| `*.config.yml`, `*.config.yaml` | Cascading run options. |
| `*.template.yml`, `*.template.yaml` | Custom report templates. |
| `*.env` | Simple `KEY=value` inputs exposed through Liquid's `env` object. |

The filename suffix is significant: `users.spec.yml` is discovered, while `users.yml` is ignored.

Environment files support simple `KEY=value` lines. Blank lines are ignored, and `#` begins a comment even when it appears after a value. These files are project inputs rather than a full shell parser, so shell expansion, `export`, and complex quoting should not be relied upon.

```dotenv
API_BASE_URI=https://api.example.com
API_TOKEN=secret
```

```yaml
test:
  route: "{{ env.API_BASE_URI }}/users"
  headers:
    Authorization: "Bearer {{ env.API_TOKEN }}"
```

Configuration and environment values discovered in a directory are available to its descendants. Prefer one config and one environment file at each configuration level so precedence remains obvious.

`--run` filters which specs execute; Tempest still discovers the project from `--path` so relevant configuration, environment values, and report templates remain available.

## Test specifications

A spec file contains a descriptor. A descriptor can contain a test, nested descriptors under `describe`, or both.

```yaml
name: "Display name"
description: "Optional longer explanation"
tags:
  - smoke
  - api

options:
  base_uri: https://api.example.com
  retries: 1

describe:
  - name: "Nested section"
    test:
      route: /health
      assert:
        - status == 200

test:
  route: /status
  assert:
    - status == 200
```

Nested descriptors execute depth-first in source order. Options on a descriptor are inherited by its descendants; options on one sibling do not affect another sibling. `tags` is currently metadata only and cannot yet be used to select tests.

### HTTP test fields

```yaml
test:
  route: /posts?postId=1
  verb: POST
  body: '{"title":"Tempest","userId":1}'
  headers:
    Content-Type: application/json
    Accept: application/json
  assert:
    - status == 201u
    - body.json().title == "Tempest"
  vars:
    post_id: body.json().id
```

| Field | Required | Description |
|---|---:|---|
| `route` | Yes | Absolute HTTP(S) URL or path joined to `base_uri`. Query parameters should be written directly in this string. |
| `verb` | No | HTTP method. Defaults to `GET`; matching is case-insensitive. |
| `body` | No | String request body sent with POST, PUT, and PATCH. |
| `headers` | No | Mapping of request header names to string values. |
| `assert` | No | List of CEL expressions. Every expression must evaluate to a boolean. |
| `vars` | No | Mapping of names to CEL expressions saved for later tests in this spec file. |

Supported methods are GET, POST, PUT, PATCH, DELETE, and HEAD. An unsupported method currently produces an empty result with status `0` and sends no request.

HTTP status alone does not determine whether a test passes. A test with no assertions is considered passed, even if the response is a 4xx or 5xx response. Write the expected status explicitly.

Transport failures produce an empty response with status `504`; the underlying error is available in `status_message`.

## Configuration

Run options can be placed directly in a `*.config.yml` file:

```yaml
base_uri: https://api.example.com
debug: false
retries: 1
reports:
  - console
  - json
concurrent: true
```

The same fields can appear under `options` on any descriptor:

```yaml
name: "Eventually consistent endpoint"
options:
  retries: 3
test:
  route: /jobs/123
  assert:
    - status == 200
```

| Option | Description |
|---|---|
| `base_uri` | Prefix for routes that do not start with `http://` or `https://`. Joining normalizes the slash between the two values. |
| `debug` | Enables detailed request and response output through reporters that define a debug template. |
| `retries` | Number of additional attempts allowed after assertion failure. |
| `reports` | Names of report templates to use. Defaults to `console`. A configured list replaces, rather than extends, the inherited list. |
| `concurrent` | Enables concurrent spec-file execution. Treat this as a root project setting. |

Options cascade from command defaults through parent-directory configs, child-directory configs, and nested descriptor options. The more local configured value wins.

If a config or descriptor defines `reports`, include `console` explicitly if console output should remain enabled:

```yaml
reports:
  - console
  - json
```

## Liquid input interpolation

Tempest renders spec inputs with Liquid before sending a request. This includes descriptor names and descriptions, `base_uri`, routes, verbs, bodies, header names and values, assertions, and variable expressions.

Available input globals are:

| Global           | Description |
|------------------|---|
| `env`            | Values loaded from discovered `*.env` files. |
| `vars`           | Values saved by earlier tests in the same spec file. |
| `file_name`      | Current spec-file path as a string. |
| `retry_attempts` | Zero-based attempt number: `0` for the initial attempt, `1` for the first retry, and so on. |

```yaml
test:
  route: "/users/{{ vars.user_id }}"
  headers:
    Authorization: "Bearer {{ env.API_TOKEN }}"
```

CEL and Liquid have different jobs:

- Use **CEL** to inspect the current HTTP response in `assert` and on the right-hand side of `vars`.
- Use **Liquid** to interpolate environment values, retry state, and values saved by earlier tests.

When a Liquid value is rendered into a CEL string comparison, keep the CEL string quotes:

```yaml
assert:
  - 'body.json().name == "{{ vars.expected_name }}"'
```

Numbers and booleans can normally be rendered without quotes:

```yaml
assert:
  - 'body.json().id == {{ vars.expected_id }}'
```

Input interpolation and custom report rendering use the same Liquid engine, but each receives a different set of globals.

## CEL assertions

Every expression under `assert` is evaluated against the HTTP response and must return a boolean. `false`, parse errors, execution errors, and non-boolean results all fail the assertion; evaluation errors are included in reporter output.

### Response variables

| Variable | CEL type | Description |
|---|---|---|
| `status` | unsigned integer | HTTP status code. |
| `status_message` | string | Canonical HTTP status text, or the transport error message. |
| `body` | string | Response bytes decoded lossily as text. |
| `bytes` | bytes | Raw response body. |
| `headers` | map | Response headers, normally addressed with lowercase names. |
| `duration` | unsigned integer | Request duration in whole milliseconds. |

```yaml
assert:
  - status == 200
  - status_message == "OK"
  - headers["content-type"].contains("application/json")
  - duration < 1000u
  - body.contains("Tempest")
```

Normal CEL operators, methods, macros, and literals supported by the bundled interpreter are available. Existing examples use `contains`, `startsWith`, `endsWith`, `matches`, `size`, `all`, `exists`, `exists_one`, `filter`, and `map`.

### JSON

Call `.json()` on a string to parse it into CEL-compatible JSON data:

```yaml
assert:
  - body.json().id == 1
  - body.json()["display-name"] != ""
  - body.json().all(item, item.id > 0)
```

Dotted access works for identifier-like keys; bracket access works for keys containing punctuation. Invalid JSON produces an assertion evaluation error.

### HTML and CSS selectors

Call `.css(selector)` on an HTML string. It returns a list of matches containing `tag`, normalized `text`, and an `attrs` map:

```yaml
assert:
  - body.css("title").exists(element, element.text == "Example Domain")
  - body.css("a[href]").all(element, element.attrs.href.startsWith("https://"))
  - body.css("main article").size() > 0
```

An invalid selector produces an assertion evaluation error.

### XML and XPath

Call `.xpath(expression)` on an XML string. Boolean, number, and string XPath results become the corresponding CEL scalar; node sets become a list of strings.

```yaml
assert:
  - body.xpath("count(/slideshow/slide)") == 2.0
  - body.xpath("string(/slideshow/@title)") == "Sample Slide Show"
  - body.xpath("/slideshow/slide/title") == ["First", "Second"]
```

### Bytes and files

`fileBytes(path)` reads a file and returns bytes. Relative paths resolve from the current spec file's directory. Paths beginning with `/` resolve from the suite root selected by `--path`; paths may not escape that root.

```yaml
assert:
  - bytes == fileBytes("fixtures/avatar.png")
  - bytes == fileBytes("/shared/avatar.png")
```

Use `.toBase64()` to encode bytes and `.fromBase64()` to decode a Base64 string:

```yaml
assert:
  - bytes.toBase64() == "aGVsbG8="
  - '"aGVsbG8=".fromBase64() == b"hello"'
```

Missing files, rejected paths, and invalid Base64 produce evaluation errors.

## Saved variables

Use `vars` to evaluate response-derived values and make them available to later tests in the same spec file. Each value is a CEL expression evaluated with the same variables and functions available to assertions.

```yaml
name: "Load a user"
test:
  route: /users/1
  assert:
    - status == 200
  vars:
    user_id: body.json().id
    user_name: body.json().name
    response_type: headers["content-type"]
```

Later descriptors access those values through Liquid's `vars` object:

```yaml
name: "Use the saved user"
test:
  route: "/users/{{ vars.user_id }}"
  assert:
    - status == 200
    - 'body.json().name == "{{ vars.user_name }}"'
```

Variable behavior:

- Producer and consumer tests must be in the same spec file.
- The producer must appear before the consumer in execution order.
- Variables do not cross spec-file boundaries.
- CEL values are converted to JSON-compatible Liquid values.
- If a variable expression fails or cannot be converted, Tempest stores `null`; assignment failure does not itself fail the test.
- Mutations from a failed retry attempt are rolled back before the next attempt.

## Retries and flaky tests

`retries` is the number of additional attempts after an assertion failure. With `retries: 1`, a test can run at most twice.

```yaml
options:
  retries: 2
test:
  route: /eventually-consistent-resource
  assert:
    - status == 200
```

A test that fails and later passes is reported as flaky. Every attempt is rendered, but the final summary counts the descriptor once.

The zero-based `retry_attempts` Liquid value can vary request inputs by attempt:

```yaml
options:
  retries: 1
test:
  route: "/status/{% if retry_attempts == 0 %}500{% else %}200{% endif %}"
  assert:
    - status == 200
```

Flaky tests normally return exit code 0. Use `--strict` to return exit code 2 when a run contains flaky tests and no failures.

## Concurrency

Spec files execute serially by default. Tests within one spec file always remain sequential so saved variables and retries have deterministic ordering.

Enable concurrent spec files in the root project config:

```yaml
concurrent: true
```

Without an explicit worker limit, Tempest uses the machine's available parallelism. Use `--workers N` to set a positive cap; providing `--workers` enables file concurrency even when `concurrent` is absent or false.

```bash
# At most four spec files at once.
tempest test --path ./tests --workers 4

# Explicitly force serial execution.
tempest test --path ./tests --workers 1
```

`concurrent` is a suite scheduler setting, so configure it at the project root rather than on individual descriptors.

## Reporters

Tempest includes two built-in report templates:

| Reporter | Description                                                            |
|---|------------------------------------------------------------------------|
| `console` | Default human-readable terminal output. (default)                      |
| `json` | JSON output written under `./tempest-reports/report-<timestamp>.json`. |

Select reporters with the `reports` option:

```yaml
reports:
  - console
  - json
```

### Custom report templates

Create a `*.template.yml` or `*.template.yaml` file anywhere in the discovered test tree. Its registered name is the lowercase filename without `.template.yml`; for example, `JUnit.template.yml` is selected as `junit`.

```yaml
# concise.template.yml
title_template: |
  Running {{ test_count }} tests

section_template: |
  # {{ full_name }}

test_template: |
  {{ full_name }}: {{ status }} {{ status_message }}

summary_template: |
  Passed: {{ passed }}, flaky: {{ flaky }}, failed: {{ failed }}

error_template: |
  Template error: {{ liquid_error_message }}

debug_template: |
  {{ debug_message }}

file:
  dir: ./tempest-reports
  file_name: report-{{ start_timestamp }}.txt
```

If `file` is omitted, rendered content is printed to the console. File-backed reporters create their directory when needed and append output to the target file. `start_timestamp` is available when rendering `file_name`.

Template fields may contain inline Liquid or refer to a `.liquid` file beside the YAML template:

```yaml
test_template: concise.test.liquid
summary_template: concise.summary.liquid
```

Unknown report names are ignored.

### Report template globals

| Event/template | Available globals |
|---|---|
| `title_template` | `test_count` |
| `section_template` | `name`, `description`, `title_path`, `full_name`, `passed`, `test_count`, `retry_count`, `assertions` |
| `test_template` | Section globals plus `status`, `status_message`, `body`, `duration_ms`, and `headers` |
| `summary_template` | `passed`, `failed`, `flaky` |
| `error_template` | `liquid_error_message` |
| `debug_template` | `debug_message` |

Each item in `assertions` contains `expr`, `passed`, and `error`.

The Liquid engine includes its standard library plus Tempest's `json`, `color_status`, and `color_duration` filters and ANSI color filters such as `red`, `green`, `yellow`, `bright_red`, and their supported `on_*` background variants. The built-in templates under [`tempest/src/builtin_reporters`](https://github.com/matt-andrews/Tempest/tree/main/tempest/src/builtin_reporters) provide complete examples.

The Liquid `json` report filter remains valid and is unrelated to the obsolete CEL `json` response variable.

## Exit codes

| Code | Meaning |
|---:|---|
| `0` | No failed tests. Flaky tests also return 0 unless `--strict` is enabled. |
| `1` | One or more tests failed, or a warning occurred with `--warn-as-err`. |
| `2` | One or more tests were flaky, none failed, and `--strict` was enabled. |

Warning-as-error handling takes precedence over the flaky exit code.

## Current limitations

- Tempest currently runs HTTP tests only.
- `tags` are metadata and cannot currently filter `--run` selections.
- File-backed reports append to existing output files.
- The repository examples call public services and therefore require network access.

## Developing Tempest

Run the passing examples from the repository:

```bash
cd tempest
cargo run -- test --path ../examples/tests --run pass
```

Run the automated checks:

```bash
cd tempest
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## License

Tempest is available under either of the following licenses, at your option:

- [Apache License 2.0](https://github.com/matt-andrews/Tempest/blob/main/LICENSE-APACHE)
- [MIT License](https://github.com/matt-andrews/Tempest/blob/main/LICENSE-MIT)
