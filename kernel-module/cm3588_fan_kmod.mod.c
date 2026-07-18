#include <linux/module.h>
#include <linux/export-internal.h>
#include <linux/compiler.h>

MODULE_INFO(name, KBUILD_MODNAME);

__visible struct module __this_module
__section(".gnu.linkonce.this_module") = {
	.name = KBUILD_MODNAME,
	.init = init_module,
#ifdef CONFIG_MODULE_UNLOAD
	.exit = cleanup_module,
#endif
	.arch = MODULE_ARCH_INIT,
};



static const struct modversion_info ____versions[]
__used __section("__versions") = {
	{ 0xd272d446, "__fentry__" },
	{ 0xd710adbf, "__kmalloc_large_noprof" },
	{ 0x71798f7e, "delayed_work_timer_fn" },
	{ 0x02f9bbf0, "timer_init_key" },
	{ 0xaef1f20d, "system_wq" },
	{ 0x8ce83585, "queue_delayed_work_on" },
	{ 0xbd03ed67, "__ref_stack_chk_guard" },
	{ 0x41495f0d, "strim" },
	{ 0x40a621c5, "scnprintf" },
	{ 0x8d3abd06, "filp_open" },
	{ 0xea68ad92, "kernel_read" },
	{ 0x27701466, "filp_close" },
	{ 0xd272d446, "__stack_chk_fail" },
	{ 0x8e142c2e, "kstrtouint" },
	{ 0x90a48d82, "__ubsan_handle_out_of_bounds" },
	{ 0x9479a1e8, "strnlen" },
	{ 0xd70733be, "sized_strscpy" },
	{ 0xb689121e, "strnstr" },
	{ 0xd09b06f5, "kstrtoint" },
	{ 0xb4552dd3, "kernel_write" },
	{ 0x534ed5f3, "__msecs_to_jiffies" },
	{ 0xe54e0a6b, "__fortify_panic" },
	{ 0xe4de56b4, "__ubsan_handle_load_invalid_value" },
	{ 0x2853920b, "param_ops_int" },
	{ 0x2853920b, "param_ops_uint" },
	{ 0x2853920b, "param_ops_charp" },
	{ 0x85acaba2, "cancel_delayed_work_sync" },
	{ 0xcb8b6ec6, "kfree" },
	{ 0xe8213e80, "_printk" },
	{ 0xd272d446, "__x86_return_thunk" },
	{ 0xc7066f33, "module_layout" },
};

static const u32 ____version_ext_crcs[]
__used __section("__version_ext_crcs") = {
	0xd272d446,
	0xd710adbf,
	0x71798f7e,
	0x02f9bbf0,
	0xaef1f20d,
	0x8ce83585,
	0xbd03ed67,
	0x41495f0d,
	0x40a621c5,
	0x8d3abd06,
	0xea68ad92,
	0x27701466,
	0xd272d446,
	0x8e142c2e,
	0x90a48d82,
	0x9479a1e8,
	0xd70733be,
	0xb689121e,
	0xd09b06f5,
	0xb4552dd3,
	0x534ed5f3,
	0xe54e0a6b,
	0xe4de56b4,
	0x2853920b,
	0x2853920b,
	0x2853920b,
	0x85acaba2,
	0xcb8b6ec6,
	0xe8213e80,
	0xd272d446,
	0xc7066f33,
};
static const char ____version_ext_names[]
__used __section("__version_ext_names") =
	"__fentry__\0"
	"__kmalloc_large_noprof\0"
	"delayed_work_timer_fn\0"
	"timer_init_key\0"
	"system_wq\0"
	"queue_delayed_work_on\0"
	"__ref_stack_chk_guard\0"
	"strim\0"
	"scnprintf\0"
	"filp_open\0"
	"kernel_read\0"
	"filp_close\0"
	"__stack_chk_fail\0"
	"kstrtouint\0"
	"__ubsan_handle_out_of_bounds\0"
	"strnlen\0"
	"sized_strscpy\0"
	"strnstr\0"
	"kstrtoint\0"
	"kernel_write\0"
	"__msecs_to_jiffies\0"
	"__fortify_panic\0"
	"__ubsan_handle_load_invalid_value\0"
	"param_ops_int\0"
	"param_ops_uint\0"
	"param_ops_charp\0"
	"cancel_delayed_work_sync\0"
	"kfree\0"
	"_printk\0"
	"__x86_return_thunk\0"
	"module_layout\0"
;

MODULE_INFO(depends, "");


MODULE_INFO(srcversion, "29C5662388DD19D31188AAD");
