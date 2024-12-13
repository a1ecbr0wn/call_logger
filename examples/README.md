# Setup notes for `example/log_to_discord`

## Prerequisites

- Python3
- Run the following to ensure the correct python libraries are installed:

``` sh
pip3 install json
pip3 install requests
```

## Running the example

Create a webhook in the server of your choice and save it as an environment variable `CALL_LOGGER_DISCORD`.

``` sh
cargo run --example log_to_discord_script
```

... and the output of my discord tests are [here](https://discord.gg/eQzwkH5xSh)
