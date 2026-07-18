#include <linux/delay.h>
#include <linux/errno.h>
#include <linux/fs.h>
#include <linux/init.h>
#include <linux/kernel.h>
#include <linux/limits.h>
#include <linux/math64.h>
#include <linux/module.h>
#include <linux/slab.h>
#include <linux/string.h>
#include <linux/uaccess.h>
#include <linux/workqueue.h>

#define DEFAULT_TEMP_PATH "/sys/class/thermal/thermal_zone0/temp"
#define DEFAULT_SLEEP_TIME 5
#define DEFAULT_MIN_THRESHOLD_C 45
#define DEFAULT_MAX_THRESHOLD_C 65
#define DEFAULT_MIN_STATE 0
#define DEFAULT_MAX_STATE -1
#define MAX_DISCOVER_DEVICES 256

struct cm3588_fan_ctx {
	struct delayed_work work;
	char temp_path[PATH_MAX];
	char fan_state_path[PATH_MAX];
	char fan_max_state_path[PATH_MAX];
	u8 fan_max_state;
	int last_state;
	bool is_init;
	bool paths_resolved;
};

static struct cm3588_fan_ctx *g_ctx;

static char *temp_path = DEFAULT_TEMP_PATH;
module_param(temp_path, charp, 0644);
MODULE_PARM_DESC(temp_path, "Path to temperature sensor (millidegrees Celsius)");

static char *fan_state_path = "";
module_param(fan_state_path, charp, 0644);
MODULE_PARM_DESC(fan_state_path, "Path to fan cur_state file");

static char *fan_max_state_path = "";
module_param(fan_max_state_path, charp, 0644);
MODULE_PARM_DESC(fan_max_state_path, "Path to fan max_state file");

static uint sleep_time = DEFAULT_SLEEP_TIME;
module_param(sleep_time, uint, 0644);
MODULE_PARM_DESC(sleep_time, "Delay between checks in seconds");

static int min_threshold = DEFAULT_MIN_THRESHOLD_C;
module_param(min_threshold, int, 0644);
MODULE_PARM_DESC(min_threshold, "Temperature threshold (C) where ramp starts");

static int max_threshold = DEFAULT_MAX_THRESHOLD_C;
module_param(max_threshold, int, 0644);
MODULE_PARM_DESC(max_threshold, "Temperature threshold (C) where max speed is used");

static int min_state = DEFAULT_MIN_STATE;
module_param(min_state, int, 0644);
MODULE_PARM_DESC(min_state, "Minimum fan state");

static int max_state = DEFAULT_MAX_STATE;
module_param(max_state, int, 0644);
MODULE_PARM_DESC(max_state, "Maximum fan state (-1 uses device max)");

static ssize_t read_file(struct file *file, char *buf, size_t size)
{
	loff_t pos = 0;

	if (!file || !buf || !size)
		return -EINVAL;

	return kernel_read(file, buf, size - 1, &pos);
}

static int read_file_u32(const char *path, u32 *value)
{
	struct file *file;
	char buf[32];
	ssize_t bytes;
	int ret;

	if (!path || !value)
		return -EINVAL;

	file = filp_open(path, O_RDONLY, 0);
	if (IS_ERR(file))
		return PTR_ERR(file);

	bytes = read_file(file, buf, sizeof(buf));
	filp_close(file, NULL);
	if (bytes < 0)
		return bytes;

	buf[bytes] = '\0';
	ret = kstrtou32(strim(buf), 10, value);
	return ret;
}

static int read_file_i32(const char *path, s32 *value)
{
	struct file *file;
	char buf[32];
	ssize_t bytes;
	int ret;

	if (!path || !value)
		return -EINVAL;

	file = filp_open(path, O_RDONLY, 0);
	if (IS_ERR(file))
		return PTR_ERR(file);

	bytes = read_file(file, buf, sizeof(buf));
	filp_close(file, NULL);
	if (bytes < 0)
		return bytes;

	buf[bytes] = '\0';
	ret = kstrtos32(strim(buf), 10, value);
	return ret;
}

static int read_file_str(const char *path, char *out, size_t out_size)
{
	struct file *file;
	ssize_t bytes;

	if (!path || !out || !out_size)
		return -EINVAL;

	file = filp_open(path, O_RDONLY, 0);
	if (IS_ERR(file))
		return PTR_ERR(file);

	bytes = read_file(file, out, out_size);
	filp_close(file, NULL);
	if (bytes < 0)
		return bytes;

	out[bytes] = '\0';
	strim(out);
	return 0;
}

static int write_file_u32(const char *path, u32 value)
{
	struct file *file;
	char buf[8];
	size_t len;
	loff_t pos = 0;
	ssize_t written;

	if (!path)
		return -EINVAL;

	len = scnprintf(buf, sizeof(buf), "%u", value);
	file = filp_open(path, O_WRONLY, 0);
	if (IS_ERR(file))
		return PTR_ERR(file);

	written = kernel_write(file, buf, len, &pos);
	filp_close(file, NULL);
	if (written < 0)
		return written;
	if (written != len)
		return -EIO;

	return 0;
}

static int discover_fan_paths(struct cm3588_fan_ctx *ctx)
{
	int i;
	char type_path[PATH_MAX];
	char type[32];
	int ret;

	for (i = 0; i < MAX_DISCOVER_DEVICES; i++) {
		scnprintf(type_path, sizeof(type_path),
				 "/sys/class/thermal/cooling_device%d/type", i);
		ret = read_file_str(type_path, type, sizeof(type));
		if (ret)
			continue;

		if (strcmp(type, "pwm-fan") == 0) {
			scnprintf(ctx->fan_state_path, sizeof(ctx->fan_state_path),
				 "/sys/class/thermal/cooling_device%d/cur_state", i);
			scnprintf(ctx->fan_max_state_path, sizeof(ctx->fan_max_state_path),
				 "/sys/class/thermal/cooling_device%d/max_state", i);
			pr_info("cm3588_fan_kmod: using cooling_device%d\n", i);
			return 0;
		}
	}

	return -ENODEV;
}

static int resolve_paths(struct cm3588_fan_ctx *ctx)
{
	if (!ctx)
		return -EINVAL;

	strscpy(ctx->temp_path, temp_path, sizeof(ctx->temp_path));

	if (fan_state_path[0]) {
		strscpy(ctx->fan_state_path, fan_state_path, sizeof(ctx->fan_state_path));
		if (fan_max_state_path[0]) {
			strscpy(ctx->fan_max_state_path, fan_max_state_path,
				sizeof(ctx->fan_max_state_path));
		} else {
			char *suffix;

			strscpy(ctx->fan_max_state_path, ctx->fan_state_path,
				sizeof(ctx->fan_max_state_path));
			suffix = strnstr(ctx->fan_max_state_path, "/cur_state",
					sizeof(ctx->fan_max_state_path));
			if (!suffix)
				return -EINVAL;
			strscpy(suffix, "/max_state", PATH_MAX - (suffix - ctx->fan_max_state_path));
		}

		return 0;
	}

	return discover_fan_paths(ctx);
}

static u8 effective_max_state(const struct cm3588_fan_ctx *ctx)
{
	if (max_state >= 0)
		return min_t(u8, (u8)max_state, ctx->fan_max_state);
	return ctx->fan_max_state;
}

static u8 choose_speed(const struct cm3588_fan_ctx *ctx, s32 temp_milli)
{
	u8 effective_max = effective_max_state(ctx);
	u32 min_t_milli = (u32)(min_threshold * 1000);
	u32 max_t_milli = (u32)(max_threshold * 1000);
	s64 delta;
	u8 slots;
	u8 step_idx;

	if (temp_milli < min_t_milli)
		return (u8)min_state;

	if (temp_milli > max_t_milli)
		return effective_max;

	if (effective_max <= (u8)min_state)
		return (u8)min_state;

	slots = effective_max - (u8)min_state;
	if (slots <= 1)
		return effective_max;

	delta = (s64)temp_milli - min_t_milli;
	step_idx = div64_u64((u64)delta * (slots - 1), max_t_milli - min_t_milli);
	return (u8)min_state + step_idx + 1;
}

static void cm3588_fan_work(struct work_struct *work)
{
	struct cm3588_fan_ctx *ctx;
	s32 temp_milli;
	u32 current_speed;
	u32 fan_max_state_val;
	u8 desired_speed;
	int ret;

	ctx = container_of(to_delayed_work(work), struct cm3588_fan_ctx, work);

	if (!ctx->paths_resolved) {
		ret = resolve_paths(ctx);
		if (ret) {
			pr_err("cm3588_fan_kmod: cannot resolve fan/temp paths (%d)\n", ret);
			goto reschedule;
		}

		ret = read_file_u32(ctx->fan_max_state_path, &fan_max_state_val);
		if (ret) {
			pr_err("cm3588_fan_kmod: cannot read fan max state (%d)\n", ret);
			goto reschedule;
		}
		if (fan_max_state_val > U8_MAX) {
			pr_err("cm3588_fan_kmod: fan max state out of range (%u)\n",
			       fan_max_state_val);
			goto reschedule;
		}
		ctx->fan_max_state = (u8)fan_max_state_val;

		ctx->paths_resolved = true;
		ctx->is_init = false;
		ctx->last_state = -1;
	}

	ret = read_file_i32(ctx->temp_path, &temp_milli);
	if (ret) {
		pr_err("cm3588_fan_kmod: cannot read temperature (%d)\n", ret);
		ctx->paths_resolved = false;
		goto reschedule;
	}

	desired_speed = choose_speed(ctx, temp_milli);
	ret = read_file_u32(ctx->fan_state_path, &current_speed);
	if (ret) {
		pr_err("cm3588_fan_kmod: cannot read fan state (%d)\n", ret);
		ctx->paths_resolved = false;
		goto reschedule;
	}

	if (!ctx->is_init || current_speed != desired_speed || ctx->last_state != desired_speed) {
		ret = write_file_u32(ctx->fan_state_path, desired_speed);
		if (ret) {
			pr_err("cm3588_fan_kmod: cannot write fan state (%d)\n", ret);
			ctx->paths_resolved = false;
			goto reschedule;
		}

		ctx->last_state = desired_speed;
		ctx->is_init = true;
		pr_info("cm3588_fan_kmod: temp=%d mC state=%u\n", temp_milli, desired_speed);
	}

reschedule:
	schedule_delayed_work(&ctx->work, msecs_to_jiffies(sleep_time * 1000));
}

static int __init cm3588_fan_init(void)
{
	if (min_threshold >= max_threshold)
		return -EINVAL;
	if (min_state < 0)
		return -EINVAL;
	if (max_state >= 0 && min_state > max_state)
		return -EINVAL;

	g_ctx = kzalloc(sizeof(*g_ctx), GFP_KERNEL);
	if (!g_ctx)
		return -ENOMEM;

	INIT_DELAYED_WORK(&g_ctx->work, cm3588_fan_work);
	schedule_delayed_work(&g_ctx->work, 0);

	pr_info("cm3588_fan_kmod: loaded\n");
	return 0;
}

static void __exit cm3588_fan_exit(void)
{
	if (g_ctx) {
		cancel_delayed_work_sync(&g_ctx->work);
		kfree(g_ctx);
	}

	pr_info("cm3588_fan_kmod: unloaded\n");
}

MODULE_LICENSE("MIT");
MODULE_AUTHOR("martabal contributors");
MODULE_DESCRIPTION("CM3588 PWM fan controller kernel module");

module_init(cm3588_fan_init);
module_exit(cm3588_fan_exit);
