# cm3588-fan

Control the 5V PWM fan on a [CM3588 NAS](https://www.friendlyelec.com/index.php?route=product/product&path=60&product_id=299).
This runs as a background service. You can configure the fan speed based on the temperature.

## Install

Download the binary from the [latest release](https://github.com/martabal/cm3588-fan/releases/download/v0.1.0/cm3588-fan) and install it in `/usr/local/bin/cm3588-fan`. Download the systemd service from the [latest release](https://github.com/martabal/cm3588-fan/releases/download/v0.1.0/fan-cm3588.service) and install it in `/etc/systemd/system/fan-cm3588.service`. Then enable and start the service with :

```bash
systemctl enable primitive-fan-control.service
systemctl start primitive-fan-control.service
```

## Environment variables

| Parameter       | Function                                                    | Default Value |
| --------------- | ----------------------------------------------------------- | ------------- |
| `SLEEP_TIME`    | Time (in seconds) between 2 checks                          | `5`           |
| `LOG_LEVEL`     | Set the output log level (trace, debug, info, warn, error)  | `info`        |
| `MIN_STATE`     | The minimum state for the fan (0-5)                         | `0`           |
| `MIN_THRESHOLD` | Temperate maximum to pass the min state                     | `45`          |
| `MAX_THRESHOLD` | Temperate maximum to activate the max state    (>0 and <=5) | `65`          |
