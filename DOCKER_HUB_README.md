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

Options are cascading so you can set top level project defaults, and then override at the directory, or descriptor level!

## Custom Reporters
We use [Liquid Template](https://shopify.github.io/liquid/) syntax for report customization. You can create a template and drop it in the directory and it will be discovered.

To learn more about how you can use templates currently, please view the built-in [examples](https://github.com/matt-andrews/Tempest/tree/main/tempest/src/builtin_reporters). Templates that specify `file:` will write to that file, or else they will just print to the console.

## License

&copy; 2026 Matthew Andrews

This project is licensed under either of

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([`LICENSE-APACHE`](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([`LICENSE-MIT`](LICENSE-MIT))

at your option.

The [SPDX](https://spdx.dev) license identifier for this project is `MIT OR Apache-2.0`.