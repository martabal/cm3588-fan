# cm3588-fan

Control the 5V PWM fan on a [CM3588 NAS](https://www.friendlyelec.com/index.php?route=product/product&path=60&product_id=299).
This runs as a background service. You can configure the fan speed based on the temperature.

It requires Rust 1.85.1 or later. The binary included in the release artifacts is compiled using the latest stable Rust version available at the time of release.

## Install

Download the binary from the [latest release](https://github.com/martabal/cm3588-fan/releases/latest/download/cm3588-fan) and install it in `/usr/local/bin/cm3588-fan`. Download the systemd service from the [latest release](https://github.com/martabal/cm3588-fan/releases/latest/download/cm3588-fan.service) and install it in `/etc/systemd/system/cm3588-fan.service`. Then enable and start the service with :

```bash
systemctl enable cm3588-fan.service
systemctl start cm3588-fan.service
```

## Environment variables

| Parameter       | Function                                                                                                          | Default Value |
| --------------- | ----------------------------------------------------------------------------------------------------------------- | ------------- |
| `SLEEP_TIME`    | Time (in seconds) between 2 checks                                                                                | `5`           |
| `LOG_LEVEL`     | Set the output log level (trace, debug, info, warn, error)                                                        | `info`        |
| `MIN_STATE`     | The minimum state for the fan (0=fan disabled, 5=maximum speed)                                                   | `0`           |
| `MAX_STATE`     | The maximum state for the fan (0=fan disabled, 5=maximum speed, will use by default the maximum state of the fan) |               |
| `MIN_THRESHOLD` | Temperature threshold for triggering the minimum state.  (>0 and <=5)                                             | `45`          |
| `MAX_THRESHOLD` | Temperature threshold for triggering the maximum state.    (>0 and <=5 and > MIN_THRESHOLD)                       | `65`          |

With the default parameters, the fan remains off until the CPU temperature reaches the minimum threshold of 45°C. Once this threshold is crossed, the fan speed is set to 2 at 50°C, 3 at 55°C, 4 at 60°C, and 5 when the temperature exceeds 65°C.
