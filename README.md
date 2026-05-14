<div align="center">
  <img src=".assets/tempest.png" width=256 alt="Tempest Logo" >
  <h1>Tempest</h1>
</div>

Tempest is an automated testing framework for HTTP requests. 

We have some [examples](./examples/tests) if you'd like to see the yaml structure.

You can run the tests by cloning the repo and calling
```bash
cargo run -- test --path ./path/to/my/tests
```

or with docker:
```bash
docker run -v ./path/to/my/tests:/etc/tests mattisthegreatest/tempest test
```

