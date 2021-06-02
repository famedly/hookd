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

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[AGPL-3.0-only](https://choosealicense.com/licenses/agpl-3.0/)
