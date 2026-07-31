<div align="center">
  <img src="https://raw.githubusercontent.com/matt-andrews/Tempest/main/.assets/tempest.png" width=256 alt="Tempest Logo" >
  <h1>Tempest</h1>

[![Docker Image Size](https://img.shields.io/docker/image-size/mattisthegreatest/tempest?style=for-the-badge)](https://hub.docker.com/r/mattisthegreatest/tempest)
[![Docker Image Version](https://img.shields.io/docker/v/mattisthegreatest/tempest?style=for-the-badge&sort=semver)](https://hub.docker.com/r/mattisthegreatest/tempest)
![GitHub License](https://img.shields.io/github/license/matt-andrews/Tempest?style=for-the-badge)

</div>

Tempest is an automated testing framework executing HTTP requests and validating the response.

## Getting Started

Tempest is currently composed of 3 different types of files:
- `.spec.yml` - a test spec for defining what your tests do
- `.config.yml` - a configuration file for a directory or project
- `.template.yml` - a custom reporter template

We have some [examples](https://github.com/matt-andrews/Tempest/tree/main/examples/tests) if you'd like to learn some complex scenarios.

The easiest way to get started is by creating a test spec:
```yaml
# test.spec.yml
name: "Test"
test:
  route: "https://httpbin.org/status/200"
  assert:
    - "status == 200"
```
and then you can run Tempest at the path:
```bash
docker run --rm -v $PWD:/etc/tests mattisthegreatest/tempest test
```

Once you run the tests, you will get an output in the console window that describes the results.

## Options
There are a few options that you can use to configure your tests (more on the way!):

| Name     | Usage                                                                                                                                                |
|----------|------------------------------------------------------------------------------------------------------------------------------------------------------|
| base_uri | Set the base url so that your routes don't need to be fully qualified                                                                                |
| reports  | Define which report sinks to use. By default `console` is used, but if this option is specified it will clear that default and you must add it again |
| retries  | Define the number of retries a test will attempt before declaring failure.                                                                           |

Options are cascading so you can set top level project defaults, and then override at the directory, or descriptor level!

## Custom Reporters
We use [Liquid Template](https://shopify.github.io/liquid/) syntax for report customization. You can create a template and drop it in the directory and it will be discovered.

To learn more about how you can use templates currently, please view the built-in [examples](https://github.com/matt-andrews/Tempest/tree/main/tempest/src/builtin_reporters). Templates that specify `file:` will write to that file, or else they will just print to the console.

## Assertions
There are several expressions and functions that you can use to assert.

| Name              | Description                                                                                                                              | Example Usage                                       |
|-------------------|------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------|
| `status`          | The response status code                                                                                                                 | `"status == 200"`                                   |
| `json`            | The json response object                                                                                                                 | `'json.name == "myObj"'`                            |
| `body`            | The stringified response body                                                                                                            | `'body == "hello"'`                                 |
| `headers`         | The hashmap headers collection                                                                                                           | `'headers["content-type"] == "application/json"'`   |
| `bytes`           | The response body as a byte array                                                                                                        | `'bytes == fileBytes("img.jpg")'`                   |
| `fileBytes(path)` | Reads a file as bytes for comparison. Relative paths resolve from the current spec file; paths beginning with `/` resolve from the root. | `'fileBytes("img.jpg")'`, `'fileBytes("/img.jpg")'` |

## Variables
You can specify outputs as variables to be used downstream tests. Downstream tests are tests that reside in the same spec file, and are defined after the test declaring the variable.

Example:
```yml
vars:
  - 'my_var={{ body }}'
```

Once a variable has been declared it can be used with liquid templates in the file scope:
```yml
assert:
  - '"body" == {{ file.body }}'
```

You can see some working examples [here](https://github.com/matt-andrews/Tempest/tree/main/examples/tests/pass/misc/vars.spec.yml)

## Exit codes

| Code | Description                                                   |
|------|---------------------------------------------------------------|
| 0    | Success                                                       |
| 1    | One or more tests failed                                      |
| 2    | One or more tests were flaky AND the `--strict` flag was used | 

## Running from repo
To run the example tests locally directly from the repo
```bash
cd tempest
cargo run -- test --path ../examples/tests
```

The console will show a bunch of tests running and give you the following summary

```
- Summary Test Results ❌ ----------
    Passed: 61
    Flaky: 1
    Failed: 2
------------------------------------
```
