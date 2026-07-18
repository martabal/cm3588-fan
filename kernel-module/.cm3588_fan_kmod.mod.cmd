savedcmd_cm3588_fan_kmod.mod := printf '%s\n'   cm3588_fan_kmod.o | awk '!x[$$0]++ { print("./"$$0) }' > cm3588_fan_kmod.mod
