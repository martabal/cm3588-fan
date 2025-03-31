# cm3588-fan

Control the 5V PWM fan on a [CM3588 NAS](https://www.friendlyelec.com/index.php?route=product/product&path=60&product_id=299).
This runs as a background service. You can configure the fan speed based on the temperature.

## Environment variables

| Parameter       | Function                                                   | Default Value |
| --------------- | ---------------------------------------------------------- | ------------- |
| `SLEEP_TIME`    | Time (in seconds) between 2 checks                         | `5`           |
| `LOG_LEVEL`     | Set the output log level (trace, debug, info, warn, error) | `admin`       |
| `MIN_STATE`     | The minimum state for the fan (0-5)                        | `0`           |
| `MIN_THRESHOLD` | Temperate maximum to pass the min state                    | `45`          |
| `MAX_THRESHOLD` | Temperate maximum to activate the max state    (>0 and <=5)            | `65`          |
