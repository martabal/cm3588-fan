# cm3588-fan kernel module

Linux kernel module version of `cm3588-fan`.

## Build

```bash
cd /home/runner/work/cm3588-fan/cm3588-fan/kernel-module
make
```

This produces `cm3588_fan_kmod.ko`.

## Load

```bash
sudo insmod cm3588_fan_kmod.ko
```

Optional parameters (all module parameters are writable via `/sys/module/cm3588_fan_kmod/parameters/`):

- `sleep_time` (seconds, default `5`)
- `min_threshold` (°C, default `45`)
- `max_threshold` (°C, default `65`)
- `min_state` (default `0`)
- `max_state` (`-1` means use device max)
- `temp_path` (default `/sys/class/thermal/thermal_zone0/temp`)
- `fan_state_path` (empty by default; auto-discovery by `type == pwm-fan`)
- `fan_max_state_path` (empty by default; inferred from `fan_state_path` if provided)

Example:

```bash
sudo insmod cm3588_fan_kmod.ko sleep_time=3 min_threshold=50 max_threshold=75 min_state=1
```

## Unload

```bash
sudo rmmod cm3588_fan_kmod
```
