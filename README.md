# hookd

A simple webhook daemon that supports multiple hooks, passing env vars and
reading stdout/stderr. Similar to
[microhookd](https://github.com/the-maldridge/microhookd) and
[webhook](https://github.com/adnanh/webhook).

## Installation

Use the package manager [cargo](https://doc.rust-lang.org/cargo/) to install hookd.

```bash
cargo install hookd
```

## Usage

Create a config file in `~/.config/hookd/config.yaml` based on the `config.sample.yaml` in this repo.

### Healthcheck for Docker container
The service API implements the `/health` check for the Docker containers.

*IMPORTANT*: In order the Docker container to be able to perform the check, the image MUST provide the `curl` tool. If changing or updating the base image's version, please ensure the `curl` availability!

````BASH
curl -s http://localhost:9320/health || exit 1
````

S. [Dockerfile](./Dockerfile) for details.

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[AGPL-3.0-only](https://choosealicense.com/licenses/agpl-3.0/)
