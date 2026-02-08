use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use inkwell::builder::Builder;
use inkwell::context::Context as LlvmContext;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FloatType, IntType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::AddressSpace;
use inkwell::FloatPredicate;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;

use rustybasic_parser::ast::*;
use rustybasic_sema::SemaResult;

/// Target configuration for code generation.
pub struct TargetConfig {
    pub triple: String,
    pub cpu: String,
    pub features: String,
}

impl TargetConfig {
    /// ESP32-C3 RISC-V target.
    pub fn esp32c3() -> Self {
        Self {
            triple: "riscv32-unknown-none-elf".to_string(),
            cpu: "generic-rv32".to_string(),
            features: "+m,+c".to_string(),
        }
    }

    /// Native host target (for testing).
    pub fn host() -> Self {
        Self {
            triple: TargetMachine::get_default_triple()
                .as_str()
                .to_string_lossy()
                .into_owned(),
            cpu: TargetMachine::get_host_cpu_name()
                .to_string_lossy()
                .into_owned(),
            features: TargetMachine::get_host_cpu_features()
                .to_string_lossy()
                .into_owned(),
        }
    }
}

/// Array metadata for codegen.
struct ArrayInfo<'ctx> {
    data_ptr_alloca: PointerValue<'ctx>,
    dim_size_allocas: Vec<PointerValue<'ctx>>,
    total_size_alloca: PointerValue<'ctx>,
    element_vt: VarType,
}

pub struct Codegen<'ctx> {
    context: &'ctx LlvmContext,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    target_config: TargetConfig,

    // Types
    i32_type: IntType<'ctx>,
    f32_type: FloatType<'ctx>,
    ptr_type: inkwell::types::PointerType<'ctx>,

    // Variables: name -> (alloca pointer, VarType)
    variables: HashMap<String, (PointerValue<'ctx>, VarType)>,

    // Arrays: name -> ArrayInfo
    arrays: HashMap<String, ArrayInfo<'ctx>>,

    // Runtime function declarations
    rt_print_int: Option<FunctionValue<'ctx>>,
    rt_print_float: Option<FunctionValue<'ctx>>,
    rt_print_string: Option<FunctionValue<'ctx>>,
    rt_print_newline: Option<FunctionValue<'ctx>>,
    rt_input_int: Option<FunctionValue<'ctx>>,
    rt_input_float: Option<FunctionValue<'ctx>>,
    rt_input_string: Option<FunctionValue<'ctx>>,
    rt_string_alloc: Option<FunctionValue<'ctx>>,
    rt_string_concat: Option<FunctionValue<'ctx>>,
    rt_string_compare: Option<FunctionValue<'ctx>>,
    rt_string_release: Option<FunctionValue<'ctx>>,
    rt_panic: Option<FunctionValue<'ctx>>,
    rt_gpio_mode: Option<FunctionValue<'ctx>>,
    rt_gpio_set: Option<FunctionValue<'ctx>>,
    rt_gpio_read: Option<FunctionValue<'ctx>>,
    rt_delay: Option<FunctionValue<'ctx>>,
    rt_i2c_setup: Option<FunctionValue<'ctx>>,
    rt_i2c_write: Option<FunctionValue<'ctx>>,
    rt_i2c_read: Option<FunctionValue<'ctx>>,
    rt_spi_setup: Option<FunctionValue<'ctx>>,
    rt_spi_transfer: Option<FunctionValue<'ctx>>,
    rt_wifi_connect: Option<FunctionValue<'ctx>>,
    rt_wifi_status: Option<FunctionValue<'ctx>>,
    rt_wifi_disconnect: Option<FunctionValue<'ctx>>,
    rt_adc_read: Option<FunctionValue<'ctx>>,
    rt_pwm_setup: Option<FunctionValue<'ctx>>,
    rt_pwm_duty: Option<FunctionValue<'ctx>>,
    rt_uart_setup: Option<FunctionValue<'ctx>>,
    rt_uart_write: Option<FunctionValue<'ctx>>,
    rt_uart_read: Option<FunctionValue<'ctx>>,
    rt_timer_start: Option<FunctionValue<'ctx>>,
    rt_timer_elapsed: Option<FunctionValue<'ctx>>,
    rt_http_get: Option<FunctionValue<'ctx>>,
    rt_http_post: Option<FunctionValue<'ctx>>,
    rt_nvs_write: Option<FunctionValue<'ctx>>,
    rt_nvs_read: Option<FunctionValue<'ctx>>,
    rt_mqtt_connect: Option<FunctionValue<'ctx>>,
    rt_mqtt_disconnect: Option<FunctionValue<'ctx>>,
    rt_mqtt_publish: Option<FunctionValue<'ctx>>,
    rt_mqtt_subscribe: Option<FunctionValue<'ctx>>,
    rt_mqtt_receive: Option<FunctionValue<'ctx>>,
    rt_ble_init: Option<FunctionValue<'ctx>>,
    rt_ble_advertise: Option<FunctionValue<'ctx>>,
    rt_ble_scan: Option<FunctionValue<'ctx>>,
    rt_ble_send: Option<FunctionValue<'ctx>>,
    rt_ble_receive: Option<FunctionValue<'ctx>>,
    rt_json_get: Option<FunctionValue<'ctx>>,
    rt_json_set: Option<FunctionValue<'ctx>>,
    rt_json_count: Option<FunctionValue<'ctx>>,
    rt_led_setup: Option<FunctionValue<'ctx>>,
    rt_led_set: Option<FunctionValue<'ctx>>,
    rt_led_show: Option<FunctionValue<'ctx>>,
    rt_led_clear: Option<FunctionValue<'ctx>>,
    rt_deepsleep: Option<FunctionValue<'ctx>>,
    rt_espnow_init: Option<FunctionValue<'ctx>>,
    rt_espnow_send: Option<FunctionValue<'ctx>>,
    rt_espnow_receive: Option<FunctionValue<'ctx>>,
    rt_powf: Option<FunctionValue<'ctx>>,
    rt_array_alloc: Option<FunctionValue<'ctx>>,
    rt_array_free: Option<FunctionValue<'ctx>>,
    rt_array_bounds_check: Option<FunctionValue<'ctx>>,
    rt_array_check_dim_size: Option<FunctionValue<'ctx>>,
    rt_data_read_int: Option<FunctionValue<'ctx>>,
    rt_data_read_float: Option<FunctionValue<'ctx>>,
    rt_data_read_string: Option<FunctionValue<'ctx>>,
    rt_data_restore: Option<FunctionValue<'ctx>>,

    // New classic BASIC extensions
    rt_randomize: Option<FunctionValue<'ctx>>,
    rt_print_using_int: Option<FunctionValue<'ctx>>,
    rt_print_using_float: Option<FunctionValue<'ctx>>,
    rt_print_using_string: Option<FunctionValue<'ctx>>,
    rt_error_clear: Option<FunctionValue<'ctx>>,
    rt_fn_string_s: Option<FunctionValue<'ctx>>,
    rt_fn_space_s: Option<FunctionValue<'ctx>>,
    // New hardware extensions
    rt_touch_read: Option<FunctionValue<'ctx>>,
    rt_servo_attach: Option<FunctionValue<'ctx>>,
    rt_servo_write: Option<FunctionValue<'ctx>>,
    rt_tone: Option<FunctionValue<'ctx>>,
    rt_irq_attach: Option<FunctionValue<'ctx>>,
    rt_irq_detach: Option<FunctionValue<'ctx>>,
    rt_temp_read: Option<FunctionValue<'ctx>>,
    rt_ota_update: Option<FunctionValue<'ctx>>,
    rt_oled_init: Option<FunctionValue<'ctx>>,
    rt_oled_print: Option<FunctionValue<'ctx>>,
    rt_oled_pixel: Option<FunctionValue<'ctx>>,
    rt_oled_line: Option<FunctionValue<'ctx>>,
    rt_oled_clear: Option<FunctionValue<'ctx>>,
    rt_oled_show: Option<FunctionValue<'ctx>>,
    rt_lcd_init: Option<FunctionValue<'ctx>>,
    rt_lcd_print: Option<FunctionValue<'ctx>>,
    rt_lcd_clear: Option<FunctionValue<'ctx>>,
    rt_lcd_pos: Option<FunctionValue<'ctx>>,
    rt_udp_init: Option<FunctionValue<'ctx>>,
    rt_udp_send: Option<FunctionValue<'ctx>>,
    rt_udp_receive: Option<FunctionValue<'ctx>>,

    // String built-in function declarations
    rt_fn_len: Option<FunctionValue<'ctx>>,
    rt_fn_asc: Option<FunctionValue<'ctx>>,
    rt_fn_chr_s: Option<FunctionValue<'ctx>>,
    rt_fn_left_s: Option<FunctionValue<'ctx>>,
    rt_fn_right_s: Option<FunctionValue<'ctx>>,
    rt_fn_mid_s: Option<FunctionValue<'ctx>>,
    rt_fn_instr: Option<FunctionValue<'ctx>>,
    rt_fn_str_s: Option<FunctionValue<'ctx>>,
    rt_fn_val: Option<FunctionValue<'ctx>>,
    rt_fn_ucase_s: Option<FunctionValue<'ctx>>,
    rt_fn_lcase_s: Option<FunctionValue<'ctx>>,
    rt_fn_trim_s: Option<FunctionValue<'ctx>>,

    // Math built-in function declarations
    rt_fn_sqr: Option<FunctionValue<'ctx>>,
    rt_fn_abs: Option<FunctionValue<'ctx>>,
    rt_fn_sin: Option<FunctionValue<'ctx>>,
    rt_fn_cos: Option<FunctionValue<'ctx>>,
    rt_fn_tan: Option<FunctionValue<'ctx>>,
    rt_fn_atn: Option<FunctionValue<'ctx>>,
    rt_fn_log: Option<FunctionValue<'ctx>>,
    rt_fn_exp: Option<FunctionValue<'ctx>>,
    rt_fn_int: Option<FunctionValue<'ctx>>,
    rt_fn_fix: Option<FunctionValue<'ctx>>,
    rt_fn_sgn: Option<FunctionValue<'ctx>>,
    rt_fn_rnd: Option<FunctionValue<'ctx>>,

    // GOSUB support
    gosub_return_var: Option<PointerValue<'ctx>>,
    gosub_dispatch_bb: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    gosub_counter: i32,
    gosub_return_points: Vec<(i32, inkwell::basic_block::BasicBlock<'ctx>)>,

    // Loop exit stacks (for EXIT FOR / EXIT DO)
    for_exit_stack: Vec<inkwell::basic_block::BasicBlock<'ctx>>,
    do_exit_stack: Vec<inkwell::basic_block::BasicBlock<'ctx>>,

    // Current function context
    current_function: Option<FunctionValue<'ctx>>,
    current_exit_bb: Option<inkwell::basic_block::BasicBlock<'ctx>>,

    // Label basic blocks (string-based)
    label_bbs: HashMap<String, inkwell::basic_block::BasicBlock<'ctx>>,

    // User-defined SUB/FUNCTION LLVM declarations
    user_functions: HashMap<String, FunctionValue<'ctx>>,

    // Sema results
    sema: SemaResult,
}

impl<'ctx> Codegen<'ctx> {
    pub fn new(
        context: &'ctx LlvmContext,
        module_name: &str,
        target_config: TargetConfig,
        sema: SemaResult,
    ) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let i32_type = context.i32_type();
        let f32_type = context.f32_type();
        let ptr_type = context.ptr_type(AddressSpace::default());

        let mut cg = Self {
            context,
            module,
            builder,
            target_config,
            i32_type,
            f32_type,
            ptr_type,
            variables: HashMap::new(),
            arrays: HashMap::new(),
            rt_print_int: None,
            rt_print_float: None,
            rt_print_string: None,
            rt_print_newline: None,
            rt_input_int: None,
            rt_input_float: None,
            rt_input_string: None,
            rt_string_alloc: None,
            rt_string_concat: None,
            rt_string_compare: None,
            rt_string_release: None,
            rt_panic: None,
            rt_gpio_mode: None,
            rt_gpio_set: None,
            rt_gpio_read: None,
            rt_delay: None,
            rt_i2c_setup: None,
            rt_i2c_write: None,
            rt_i2c_read: None,
            rt_spi_setup: None,
            rt_spi_transfer: None,
            rt_wifi_connect: None,
            rt_wifi_status: None,
            rt_wifi_disconnect: None,
            rt_adc_read: None,
            rt_pwm_setup: None,
            rt_pwm_duty: None,
            rt_uart_setup: None,
            rt_uart_write: None,
            rt_uart_read: None,
            rt_timer_start: None,
            rt_timer_elapsed: None,
            rt_http_get: None,
            rt_http_post: None,
            rt_nvs_write: None,
            rt_nvs_read: None,
            rt_mqtt_connect: None,
            rt_mqtt_disconnect: None,
            rt_mqtt_publish: None,
            rt_mqtt_subscribe: None,
            rt_mqtt_receive: None,
            rt_ble_init: None,
            rt_ble_advertise: None,
            rt_ble_scan: None,
            rt_ble_send: None,
            rt_ble_receive: None,
            rt_json_get: None,
            rt_json_set: None,
            rt_json_count: None,
            rt_led_setup: None,
            rt_led_set: None,
            rt_led_show: None,
            rt_led_clear: None,
            rt_deepsleep: None,
            rt_espnow_init: None,
            rt_espnow_send: None,
            rt_espnow_receive: None,
            rt_powf: None,
            rt_array_alloc: None,
            rt_array_free: None,
            rt_array_bounds_check: None,
            rt_array_check_dim_size: None,
            rt_data_read_int: None,
            rt_data_read_float: None,
            rt_data_read_string: None,
            rt_data_restore: None,
            rt_randomize: None,
            rt_print_using_int: None,
            rt_print_using_float: None,
            rt_print_using_string: None,
            rt_error_clear: None,
            rt_fn_string_s: None,
            rt_fn_space_s: None,
            rt_touch_read: None,
            rt_servo_attach: None,
            rt_servo_write: None,
            rt_tone: None,
            rt_irq_attach: None,
            rt_irq_detach: None,
            rt_temp_read: None,
            rt_ota_update: None,
            rt_oled_init: None,
            rt_oled_print: None,
            rt_oled_pixel: None,
            rt_oled_line: None,
            rt_oled_clear: None,
            rt_oled_show: None,
            rt_lcd_init: None,
            rt_lcd_print: None,
            rt_lcd_clear: None,
            rt_lcd_pos: None,
            rt_udp_init: None,
            rt_udp_send: None,
            rt_udp_receive: None,
            rt_fn_len: None,
            rt_fn_asc: None,
            rt_fn_chr_s: None,
            rt_fn_left_s: None,
            rt_fn_right_s: None,
            rt_fn_mid_s: None,
            rt_fn_instr: None,
            rt_fn_str_s: None,
            rt_fn_val: None,
            rt_fn_ucase_s: None,
            rt_fn_lcase_s: None,
            rt_fn_trim_s: None,
            rt_fn_sqr: None,
            rt_fn_abs: None,
            rt_fn_sin: None,
            rt_fn_cos: None,
            rt_fn_tan: None,
            rt_fn_atn: None,
            rt_fn_log: None,
            rt_fn_exp: None,
            rt_fn_int: None,
            rt_fn_fix: None,
            rt_fn_sgn: None,
            rt_fn_rnd: None,
            gosub_return_var: None,
            gosub_dispatch_bb: None,
            gosub_counter: 0,
            gosub_return_points: Vec::new(),
            for_exit_stack: Vec::new(),
            do_exit_stack: Vec::new(),
            current_function: None,
            current_exit_bb: None,
            label_bbs: HashMap::new(),
            user_functions: HashMap::new(),
            sema,
        };
        cg.declare_runtime_functions();
        cg
    }

    fn declare_runtime_functions(&mut self) {
        let i32_t = self.i32_type;
        let f32_t = self.f32_type;
        let ptr_t = self.ptr_type;
        let void_t = self.context.void_type();

        self.rt_print_int = Some(self.module.add_function(
            "rb_print_int",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_print_float = Some(self.module.add_function(
            "rb_print_float",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(f32_t)], false),
            None,
        ));
        self.rt_print_string = Some(self.module.add_function(
            "rb_print_string",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_print_newline = Some(self.module.add_function(
            "rb_print_newline",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_input_int = Some(self.module.add_function(
            "rb_input_int",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_input_float = Some(self.module.add_function(
            "rb_input_float",
            f32_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_input_string = Some(self.module.add_function(
            "rb_input_string",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_string_alloc = Some(self.module.add_function(
            "rb_string_alloc",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_string_concat = Some(self.module.add_function(
            "rb_string_concat",
            ptr_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_string_compare = Some(self.module.add_function(
            "rb_string_compare",
            i32_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_string_release = Some(self.module.add_function(
            "rb_string_release",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_panic = Some(self.module.add_function(
            "rb_panic",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_gpio_mode = Some(self.module.add_function(
            "rb_gpio_mode",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_gpio_set = Some(self.module.add_function(
            "rb_gpio_set",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_gpio_read = Some(self.module.add_function(
            "rb_gpio_read",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_delay = Some(self.module.add_function(
            "rb_delay",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_i2c_setup = Some(self.module.add_function(
            "rb_i2c_setup",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_i2c_write = Some(self.module.add_function(
            "rb_i2c_write",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_i2c_read = Some(self.module.add_function(
            "rb_i2c_read",
            i32_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_spi_setup = Some(self.module.add_function(
            "rb_spi_setup",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_spi_transfer = Some(self.module.add_function(
            "rb_spi_transfer",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_wifi_connect = Some(self.module.add_function(
            "rb_wifi_connect",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_wifi_status = Some(self.module.add_function(
            "rb_wifi_status",
            i32_t.fn_type(&[], false),
            None,
        ));
        self.rt_wifi_disconnect = Some(self.module.add_function(
            "rb_wifi_disconnect",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_adc_read = Some(self.module.add_function(
            "rb_adc_read",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_pwm_setup = Some(self.module.add_function(
            "rb_pwm_setup",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_pwm_duty = Some(self.module.add_function(
            "rb_pwm_duty",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_uart_setup = Some(self.module.add_function(
            "rb_uart_setup",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_uart_write = Some(self.module.add_function(
            "rb_uart_write",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_uart_read = Some(self.module.add_function(
            "rb_uart_read",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_timer_start = Some(self.module.add_function(
            "rb_timer_start",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_timer_elapsed = Some(self.module.add_function(
            "rb_timer_elapsed",
            i32_t.fn_type(&[], false),
            None,
        ));
        self.rt_http_get = Some(self.module.add_function(
            "rb_http_get",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_http_post = Some(self.module.add_function(
            "rb_http_post",
            ptr_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_nvs_write = Some(self.module.add_function(
            "rb_nvs_write",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_nvs_read = Some(self.module.add_function(
            "rb_nvs_read",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_mqtt_connect = Some(self.module.add_function(
            "rb_mqtt_connect",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_mqtt_disconnect = Some(self.module.add_function(
            "rb_mqtt_disconnect",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_mqtt_publish = Some(self.module.add_function(
            "rb_mqtt_publish",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_mqtt_subscribe = Some(self.module.add_function(
            "rb_mqtt_subscribe",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_mqtt_receive = Some(self.module.add_function(
            "rb_mqtt_receive",
            ptr_t.fn_type(&[], false),
            None,
        ));
        self.rt_ble_init = Some(self.module.add_function(
            "rb_ble_init",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_ble_advertise = Some(self.module.add_function(
            "rb_ble_advertise",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_ble_scan = Some(self.module.add_function(
            "rb_ble_scan",
            ptr_t.fn_type(&[], false),
            None,
        ));
        self.rt_ble_send = Some(self.module.add_function(
            "rb_ble_send",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_ble_receive = Some(self.module.add_function(
            "rb_ble_receive",
            ptr_t.fn_type(&[], false),
            None,
        ));
        self.rt_json_get = Some(self.module.add_function(
            "rb_json_get",
            ptr_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_json_set = Some(self.module.add_function(
            "rb_json_set",
            ptr_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_json_count = Some(self.module.add_function(
            "rb_json_count",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_led_setup = Some(self.module.add_function(
            "rb_led_setup",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_led_set = Some(self.module.add_function(
            "rb_led_set",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_led_show = Some(self.module.add_function(
            "rb_led_show",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_led_clear = Some(self.module.add_function(
            "rb_led_clear",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_deepsleep = Some(self.module.add_function(
            "rb_deepsleep",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_espnow_init = Some(self.module.add_function(
            "rb_espnow_init",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_espnow_send = Some(self.module.add_function(
            "rb_espnow_send",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_espnow_receive = Some(self.module.add_function(
            "rb_espnow_receive",
            ptr_t.fn_type(&[], false),
            None,
        ));
        self.rt_powf = Some(self.module.add_function(
            "powf",
            f32_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(f32_t),
                    BasicMetadataTypeEnum::from(f32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_array_alloc = Some(self.module.add_function(
            "rb_array_alloc",
            ptr_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_array_free = Some(self.module.add_function(
            "rb_array_free",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_array_bounds_check = Some(self.module.add_function(
            "rb_array_bounds_check",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_array_check_dim_size = Some(self.module.add_function(
            "rb_array_check_dim_size",
            void_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));

        // ── String built-in functions ──
        self.rt_fn_len = Some(self.module.add_function(
            "rb_fn_len",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_fn_asc = Some(self.module.add_function(
            "rb_fn_asc",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_fn_chr_s = Some(self.module.add_function(
            "rb_fn_chr_s",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_fn_left_s = Some(self.module.add_function(
            "rb_fn_left_s",
            ptr_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_fn_right_s = Some(self.module.add_function(
            "rb_fn_right_s",
            ptr_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_fn_mid_s = Some(self.module.add_function(
            "rb_fn_mid_s",
            ptr_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(i32_t),
                    BasicMetadataTypeEnum::from(i32_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_fn_instr = Some(self.module.add_function(
            "rb_fn_instr",
            i32_t.fn_type(
                &[
                    BasicMetadataTypeEnum::from(ptr_t),
                    BasicMetadataTypeEnum::from(ptr_t),
                ],
                false,
            ),
            None,
        ));
        self.rt_fn_str_s = Some(self.module.add_function(
            "rb_fn_str_s",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(f32_t)], false),
            None,
        ));
        self.rt_fn_val = Some(self.module.add_function(
            "rb_fn_val",
            f32_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_fn_ucase_s = Some(self.module.add_function(
            "rb_fn_ucase_s",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_fn_lcase_s = Some(self.module.add_function(
            "rb_fn_lcase_s",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_fn_trim_s = Some(self.module.add_function(
            "rb_fn_trim_s",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));

        // ── Math built-in functions ──
        let f32_to_f32 = f32_t.fn_type(&[BasicMetadataTypeEnum::from(f32_t)], false);
        let f32_to_i32 = i32_t.fn_type(&[BasicMetadataTypeEnum::from(f32_t)], false);

        self.rt_fn_sqr = Some(self.module.add_function("rb_fn_sqr", f32_to_f32, None));
        self.rt_fn_abs = Some(self.module.add_function("rb_fn_abs", f32_to_f32, None));
        self.rt_fn_sin = Some(self.module.add_function("rb_fn_sin", f32_to_f32, None));
        self.rt_fn_cos = Some(self.module.add_function("rb_fn_cos", f32_to_f32, None));
        self.rt_fn_tan = Some(self.module.add_function("rb_fn_tan", f32_to_f32, None));
        self.rt_fn_atn = Some(self.module.add_function("rb_fn_atn", f32_to_f32, None));
        self.rt_fn_log = Some(self.module.add_function("rb_fn_log", f32_to_f32, None));
        self.rt_fn_exp = Some(self.module.add_function("rb_fn_exp", f32_to_f32, None));
        self.rt_fn_int = Some(self.module.add_function("rb_fn_int", f32_to_i32, None));
        self.rt_fn_fix = Some(self.module.add_function("rb_fn_fix", f32_to_i32, None));
        self.rt_fn_sgn = Some(self.module.add_function("rb_fn_sgn", f32_to_i32, None));
        self.rt_fn_rnd = Some(self.module.add_function(
            "rb_fn_rnd",
            f32_t.fn_type(&[], false),
            None,
        ));

        // ── DATA/READ/RESTORE runtime functions ──
        self.rt_data_read_int = Some(self.module.add_function(
            "rb_data_read_int",
            i32_t.fn_type(&[], false),
            None,
        ));
        self.rt_data_read_float = Some(self.module.add_function(
            "rb_data_read_float",
            f32_t.fn_type(&[], false),
            None,
        ));
        self.rt_data_read_string = Some(self.module.add_function(
            "rb_data_read_string",
            ptr_t.fn_type(&[], false),
            None,
        ));
        self.rt_data_restore = Some(self.module.add_function(
            "rb_data_restore",
            void_t.fn_type(&[], false),
            None,
        ));
        // ── New classic BASIC extensions ──
        self.rt_randomize = Some(self.module.add_function(
            "rb_randomize",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_print_using_int = Some(self.module.add_function(
            "rb_print_using_int",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_print_using_float = Some(self.module.add_function(
            "rb_print_using_float",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t), BasicMetadataTypeEnum::from(f32_t)], false),
            None,
        ));
        self.rt_print_using_string = Some(self.module.add_function(
            "rb_print_using_string",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t), BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_error_clear = Some(self.module.add_function(
            "rb_error_clear",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_fn_string_s = Some(self.module.add_function(
            "rb_fn_string_s",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_fn_space_s = Some(self.module.add_function(
            "rb_fn_space_s",
            ptr_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        // ── New hardware extensions ──
        self.rt_touch_read = Some(self.module.add_function(
            "rb_touch_read",
            i32_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_servo_attach = Some(self.module.add_function(
            "rb_servo_attach",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_servo_write = Some(self.module.add_function(
            "rb_servo_write",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_tone = Some(self.module.add_function(
            "rb_tone",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_irq_attach = Some(self.module.add_function(
            "rb_irq_attach",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_irq_detach = Some(self.module.add_function(
            "rb_irq_detach",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_temp_read = Some(self.module.add_function(
            "rb_temp_read",
            f32_t.fn_type(&[], false),
            None,
        ));
        self.rt_ota_update = Some(self.module.add_function(
            "rb_ota_update",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_oled_init = Some(self.module.add_function(
            "rb_oled_init",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_oled_print = Some(self.module.add_function(
            "rb_oled_print",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_oled_pixel = Some(self.module.add_function(
            "rb_oled_pixel",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_oled_line = Some(self.module.add_function(
            "rb_oled_line",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_oled_clear = Some(self.module.add_function(
            "rb_oled_clear",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_oled_show = Some(self.module.add_function(
            "rb_oled_show",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_lcd_init = Some(self.module.add_function(
            "rb_lcd_init",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_lcd_print = Some(self.module.add_function(
            "rb_lcd_print",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_lcd_clear = Some(self.module.add_function(
            "rb_lcd_clear",
            void_t.fn_type(&[], false),
            None,
        ));
        self.rt_lcd_pos = Some(self.module.add_function(
            "rb_lcd_pos",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_udp_init = Some(self.module.add_function(
            "rb_udp_init",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(i32_t)], false),
            None,
        ));
        self.rt_udp_send = Some(self.module.add_function(
            "rb_udp_send",
            void_t.fn_type(&[BasicMetadataTypeEnum::from(ptr_t), BasicMetadataTypeEnum::from(i32_t), BasicMetadataTypeEnum::from(ptr_t)], false),
            None,
        ));
        self.rt_udp_receive = Some(self.module.add_function(
            "rb_udp_receive",
            ptr_t.fn_type(&[], false),
            None,
        ));
    }

    fn qb_to_var(qb: &QBType) -> VarType {
        VarType::from(qb)
    }

    fn var_llvm_type(&self, vt: VarType) -> BasicTypeEnum<'ctx> {
        match vt {
            VarType::Integer => self.i32_type.as_basic_type_enum(),
            VarType::Float => self.f32_type.as_basic_type_enum(),
            VarType::String => self.ptr_type.as_basic_type_enum(),
        }
    }

    // ── DATA globals emission ─────────────────────────────

    fn emit_data_globals(&self) -> Result<()> {
        use rustybasic_parser::ast::DataItem;

        let items = &self.sema.data_items;
        let n = items.len();

        // Build parallel arrays of type tags, ints, floats, and string pointers
        let mut type_vals = Vec::with_capacity(n);
        let mut int_vals = Vec::with_capacity(n);
        let mut float_vals = Vec::with_capacity(n);
        let mut string_vals = Vec::with_capacity(n);

        for (i, item) in items.iter().enumerate() {
            match item {
                DataItem::Int(v) => {
                    type_vals.push(self.i32_type.const_int(0, false));
                    int_vals.push(self.i32_type.const_int(*v as u64, true));
                    float_vals.push(self.f32_type.const_float(0.0));
                    string_vals.push(self.ptr_type.const_null());
                }
                DataItem::Float(v) => {
                    type_vals.push(self.i32_type.const_int(1, false));
                    int_vals.push(self.i32_type.const_zero());
                    float_vals.push(self.f32_type.const_float(*v as f64));
                    string_vals.push(self.ptr_type.const_null());
                }
                DataItem::Str(s) => {
                    type_vals.push(self.i32_type.const_int(2, false));
                    int_vals.push(self.i32_type.const_zero());
                    float_vals.push(self.f32_type.const_float(0.0));
                    let gstr = self.module.add_global(
                        self.context.i8_type().array_type(s.len() as u32 + 1),
                        None,
                        &format!("rb_data_str_{i}"),
                    );
                    gstr.set_initializer(
                        &self.context.const_string(s.as_bytes(), true),
                    );
                    gstr.set_constant(true);
                    gstr.set_unnamed_addr(true);
                    string_vals.push(gstr.as_pointer_value());
                }
            }
        }

        // rb_data_types
        let types_arr = self.i32_type.const_array(&type_vals);
        let g_types = self.module.add_global(
            self.i32_type.array_type(n as u32),
            None,
            "rb_data_types",
        );
        g_types.set_initializer(&types_arr);
        g_types.set_constant(true);

        // rb_data_ints
        let ints_arr = self.i32_type.const_array(&int_vals);
        let g_ints = self.module.add_global(
            self.i32_type.array_type(n as u32),
            None,
            "rb_data_ints",
        );
        g_ints.set_initializer(&ints_arr);
        g_ints.set_constant(true);

        // rb_data_floats
        let floats_arr = self.f32_type.const_array(&float_vals);
        let g_floats = self.module.add_global(
            self.f32_type.array_type(n as u32),
            None,
            "rb_data_floats",
        );
        g_floats.set_initializer(&floats_arr);
        g_floats.set_constant(true);

        // rb_data_strings
        let strings_arr = self.ptr_type.const_array(&string_vals);
        let g_strings = self.module.add_global(
            self.ptr_type.array_type(n as u32),
            None,
            "rb_data_strings",
        );
        g_strings.set_initializer(&strings_arr);
        g_strings.set_constant(true);

        // rb_data_count
        let g_count = self.module.add_global(
            self.i32_type,
            None,
            "rb_data_count",
        );
        g_count.set_initializer(&self.i32_type.const_int(n as u64, false));
        g_count.set_constant(true);

        Ok(())
    }

    // ── Compilation entry point ─────────────────────────────

    pub fn compile(&mut self, program: &Program) -> Result<()> {
        let triple = TargetTriple::create(&self.target_config.triple);
        self.module.set_triple(&triple);

        // Emit DATA pool globals
        self.emit_data_globals()?;

        // Declare LLVM functions for user SUBs and FUNCTIONs
        for sub_def in &program.subs {
            self.declare_user_sub(sub_def)?;
        }
        for fn_def in &program.functions {
            self.declare_user_function(fn_def)?;
        }

        // Create basic_program_entry: void basic_program_entry(void)
        let fn_type = self.context.void_type().fn_type(&[], false);
        let entry_fn = self
            .module
            .add_function("basic_program_entry", fn_type, None);
        let entry_bb = self.context.append_basic_block(entry_fn, "entry");
        self.builder.position_at_end(entry_bb);
        self.current_function = Some(entry_fn);

        // Allocate all top-level variables
        for (name, info) in &self.sema.variables {
            if !info.is_array {
                let vt = Self::qb_to_var(&info.qb_type);
                let llvm_type = self.var_llvm_type(vt);
                let alloca = self.builder.build_alloca(llvm_type, name)?;
                match vt {
                    VarType::Integer => {
                        self.builder
                            .build_store(alloca, self.i32_type.const_zero())?;
                    }
                    VarType::Float => {
                        self.builder
                            .build_store(alloca, self.f32_type.const_zero())?;
                    }
                    VarType::String => {
                        self.builder
                            .build_store(alloca, self.ptr_type.const_null())?;
                    }
                }
                self.variables.insert(name.clone(), (alloca, vt));
            }
        }

        // Collect labels and create basic blocks
        self.collect_labels(&program.body, entry_fn);

        // GOSUB support
        if self.sema.has_gosub {
            let alloca = self
                .builder
                .build_alloca(self.i32_type, "gosub_return_idx")?;
            self.builder
                .build_store(alloca, self.i32_type.const_zero())?;
            self.gosub_return_var = Some(alloca);
            self.gosub_dispatch_bb = Some(
                self.context
                    .append_basic_block(entry_fn, "gosub_dispatch"),
            );
        }

        let exit_bb = self.context.append_basic_block(entry_fn, "exit");
        self.current_exit_bb = Some(exit_bb);

        // Compile top-level body
        self.compile_body(&program.body)?;

        // Branch to exit if not terminated
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder.build_unconditional_branch(exit_bb)?;
        }

        // GOSUB dispatch
        if let Some(dispatch_bb) = self.gosub_dispatch_bb {
            self.builder.position_at_end(dispatch_bb);
            let return_var = self.gosub_return_var.unwrap();
            let idx = self
                .builder
                .build_load(self.i32_type, return_var, "ret_idx")?
                .into_int_value();
            let cases: Vec<(IntValue<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> = self
                .gosub_return_points
                .iter()
                .map(|(i, bb)| (self.i32_type.const_int(*i as u64, false), *bb))
                .collect();
            if cases.is_empty() {
                self.builder.build_unconditional_branch(exit_bb)?;
            } else {
                self.builder.build_switch(idx, exit_bb, &cases)?;
            }
        }

        // Exit block
        self.builder.position_at_end(exit_bb);
        self.builder.build_return(None)?;

        // Compile SUB bodies
        for sub_def in &program.subs {
            self.compile_sub_body(sub_def)?;
        }
        // Compile FUNCTION bodies
        for fn_def in &program.functions {
            self.compile_function_body(fn_def)?;
        }

        Ok(())
    }

    // ── Label collection ────────────────────────────────────

    fn collect_labels(&mut self, stmts: &[Statement], function: FunctionValue<'ctx>) {
        for stmt in stmts {
            if let Statement::Label { name, .. } = stmt {
                if !self.label_bbs.contains_key(name) {
                    let bb = self
                        .context
                        .append_basic_block(function, &format!("label_{name}"));
                    self.label_bbs.insert(name.clone(), bb);
                }
            }
            match stmt {
                Statement::If {
                    then_body,
                    else_if_clauses,
                    else_body,
                    ..
                } => {
                    self.collect_labels(then_body, function);
                    for clause in else_if_clauses {
                        self.collect_labels(&clause.body, function);
                    }
                    self.collect_labels(else_body, function);
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::DoLoop { body, .. } => {
                    self.collect_labels(body, function);
                }
                Statement::SelectCase {
                    cases, else_body, ..
                } => {
                    for case in cases {
                        self.collect_labels(&case.body, function);
                    }
                    self.collect_labels(else_body, function);
                }
                _ => {}
            }
        }
    }

    // ── SUB/FUNCTION declarations ───────────────────────────

    fn declare_user_sub(&mut self, sub_def: &SubDef) -> Result<()> {
        let void_t = self.context.void_type();
        let param_types: Vec<BasicMetadataTypeEnum<'ctx>> = sub_def
            .params
            .iter()
            .map(|p| {
                let vt = Self::qb_to_var(&p.param_type);
                BasicMetadataTypeEnum::from(self.var_llvm_type(vt))
            })
            .collect();
        let fn_type = void_t.fn_type(&param_types, false);
        let fn_name = format!("qb_sub_{}", sub_def.name.to_lowercase());
        let func = self.module.add_function(&fn_name, fn_type, None);
        self.user_functions.insert(sub_def.name.clone(), func);
        Ok(())
    }

    fn declare_user_function(&mut self, fn_def: &FunctionDef) -> Result<()> {
        let ret_vt = Self::qb_to_var(&fn_def.return_type);
        let ret_type = self.var_llvm_type(ret_vt);
        let param_types: Vec<BasicMetadataTypeEnum<'ctx>> = fn_def
            .params
            .iter()
            .map(|p| {
                let vt = Self::qb_to_var(&p.param_type);
                BasicMetadataTypeEnum::from(self.var_llvm_type(vt))
            })
            .collect();
        let fn_type = ret_type.fn_type(&param_types, false);
        let fn_name = format!("qb_fn_{}", fn_def.name.to_lowercase());
        let func = self.module.add_function(&fn_name, fn_type, None);
        self.user_functions.insert(fn_def.name.clone(), func);
        Ok(())
    }

    // ── SUB/FUNCTION body compilation ───────────────────────

    fn compile_sub_body(&mut self, sub_def: &SubDef) -> Result<()> {
        let func = *self.user_functions.get(&sub_def.name).unwrap();
        let entry_bb = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_bb);

        let saved_vars = std::mem::take(&mut self.variables);
        let saved_arrays = std::mem::take(&mut self.arrays);
        let saved_fn = self.current_function;
        let saved_exit = self.current_exit_bb;
        let saved_labels = std::mem::take(&mut self.label_bbs);
        self.current_function = Some(func);

        let exit_bb = self.context.append_basic_block(func, "exit");
        self.current_exit_bb = Some(exit_bb);

        for (i, param) in sub_def.params.iter().enumerate() {
            let vt = Self::qb_to_var(&param.param_type);
            let llvm_type = self.var_llvm_type(vt);
            let alloca = self.builder.build_alloca(llvm_type, &param.name)?;
            let param_val = func.get_nth_param(i as u32).unwrap();
            self.builder.build_store(alloca, param_val)?;
            self.variables.insert(param.name.clone(), (alloca, vt));
        }

        self.collect_labels(&sub_def.body, func);
        self.compile_body(&sub_def.body)?;

        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder.build_unconditional_branch(exit_bb)?;
        }

        self.builder.position_at_end(exit_bb);
        self.builder.build_return(None)?;

        self.variables = saved_vars;
        self.arrays = saved_arrays;
        self.current_function = saved_fn;
        self.current_exit_bb = saved_exit;
        self.label_bbs = saved_labels;
        Ok(())
    }

    fn compile_function_body(&mut self, fn_def: &FunctionDef) -> Result<()> {
        let func = *self.user_functions.get(&fn_def.name).unwrap();
        let entry_bb = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_bb);

        let saved_vars = std::mem::take(&mut self.variables);
        let saved_arrays = std::mem::take(&mut self.arrays);
        let saved_fn = self.current_function;
        let saved_exit = self.current_exit_bb;
        let saved_labels = std::mem::take(&mut self.label_bbs);
        self.current_function = Some(func);

        let exit_bb = self.context.append_basic_block(func, "exit");
        self.current_exit_bb = Some(exit_bb);

        // Return value variable (function name = return value in QBASIC)
        let ret_vt = Self::qb_to_var(&fn_def.return_type);
        let ret_type = self.var_llvm_type(ret_vt);
        let ret_alloca = self.builder.build_alloca(ret_type, &fn_def.name)?;
        match ret_vt {
            VarType::Integer => {
                self.builder
                    .build_store(ret_alloca, self.i32_type.const_zero())?;
            }
            VarType::Float => {
                self.builder
                    .build_store(ret_alloca, self.f32_type.const_zero())?;
            }
            VarType::String => {
                self.builder
                    .build_store(ret_alloca, self.ptr_type.const_null())?;
            }
        }
        self.variables
            .insert(fn_def.name.clone(), (ret_alloca, ret_vt));

        for (i, param) in fn_def.params.iter().enumerate() {
            let vt = Self::qb_to_var(&param.param_type);
            let llvm_type = self.var_llvm_type(vt);
            let alloca = self.builder.build_alloca(llvm_type, &param.name)?;
            let param_val = func.get_nth_param(i as u32).unwrap();
            self.builder.build_store(alloca, param_val)?;
            self.variables.insert(param.name.clone(), (alloca, vt));
        }

        self.collect_labels(&fn_def.body, func);
        self.compile_body(&fn_def.body)?;

        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            self.builder.build_unconditional_branch(exit_bb)?;
        }

        self.builder.position_at_end(exit_bb);
        let ret_val = self.builder.build_load(ret_type, ret_alloca, "retval")?;
        self.builder.build_return(Some(&ret_val))?;

        self.variables = saved_vars;
        self.arrays = saved_arrays;
        self.current_function = saved_fn;
        self.current_exit_bb = saved_exit;
        self.label_bbs = saved_labels;
        Ok(())
    }

    // ── Body compilation ────────────────────────────────────

    fn compile_body(&mut self, stmts: &[Statement]) -> Result<()> {
        for stmt in stmts {
            if self
                .builder
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_some()
            {
                if let Statement::Label { name, .. } = stmt {
                    if let Some(&bb) = self.label_bbs.get(name) {
                        self.builder.position_at_end(bb);
                        continue;
                    }
                }
                break;
            }
            self.compile_statement(stmt)?;
        }
        Ok(())
    }

    // ── Statement compilation ───────────────────────────────

    fn compile_statement(&mut self, stmt: &Statement) -> Result<()> {
        let function = self.current_function.unwrap();
        let exit_bb = self.current_exit_bb.unwrap();

        match stmt {
            Statement::Label { name, .. } => {
                if let Some(&bb) = self.label_bbs.get(name) {
                    if self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_none()
                    {
                        self.builder.build_unconditional_branch(bb)?;
                    }
                    self.builder.position_at_end(bb);
                }
            }
            Statement::Dim {
                name,
                var_type,
                dimensions,
                ..
            } => {
                if !dimensions.is_empty() {
                    // Array DIM — allocate on heap
                    if !self.arrays.contains_key(name) {
                        let vt = Self::qb_to_var(var_type);
                        let element_size: i32 = match vt {
                            VarType::Integer => 4,
                            VarType::Float => 4,
                            VarType::String => 8, // pointer size
                        };

                        // Evaluate each dimension, compute size = dim_val + 1 (QBASIC: DIM arr(N) = 0..N)
                        let mut dim_size_allocas = Vec::new();
                        let mut total_val = self.i32_type.const_int(1, false);
                        for (di, dim_expr) in dimensions.iter().enumerate() {
                            let dim_val = self.compile_expr(dim_expr, VarType::Integer)?.into_int_value();
                            // Check for negative dimension size
                            self.builder.build_call(
                                self.rt_array_check_dim_size.unwrap(),
                                &[
                                    dim_val.into(),
                                    self.i32_type.const_int(di as u64 + 1, false).into(),
                                ],
                                "",
                            )?;
                            let size = self.builder.build_int_add(
                                dim_val,
                                self.i32_type.const_int(1, false),
                                &format!("dim_size_{di}"),
                            )?;
                            let size_alloca = self.builder.build_alloca(
                                self.i32_type.as_basic_type_enum(),
                                &format!("{name}_dim{di}_size"),
                            )?;
                            self.builder.build_store(size_alloca, size)?;
                            dim_size_allocas.push(size_alloca);
                            total_val = self.builder.build_int_mul(total_val, size, "total_mul")?;
                        }

                        // Overflow check: if total_val <= 0 after multiplications, panic
                        let overflow_cond = self.builder.build_int_compare(
                            IntPredicate::SLE,
                            total_val,
                            self.i32_type.const_zero(),
                            "overflow_check",
                        )?;
                        let overflow_bb = self.context.append_basic_block(function, "arr_overflow");
                        let ok_bb = self.context.append_basic_block(function, "arr_ok");
                        self.builder.build_conditional_branch(overflow_cond, overflow_bb, ok_bb)?;

                        self.builder.position_at_end(overflow_bb);
                        let msg = self.builder.build_global_string_ptr(
                            "array total size overflow",
                            "overflow_msg",
                        )?;
                        self.builder.build_call(
                            self.rt_panic.unwrap(),
                            &[msg.as_pointer_value().into()],
                            "",
                        )?;
                        self.builder.build_unreachable()?;

                        self.builder.position_at_end(ok_bb);

                        // Store total size
                        let total_alloca = self.builder.build_alloca(
                            self.i32_type.as_basic_type_enum(),
                            &format!("{name}_total"),
                        )?;
                        self.builder.build_store(total_alloca, total_val)?;

                        // Call rb_array_alloc(element_size, total_elements)
                        let elem_sz = self.i32_type.const_int(element_size as u64, false);
                        let heap_ptr = self
                            .builder
                            .build_call(
                                self.rt_array_alloc.unwrap(),
                                &[elem_sz.into(), total_val.into()],
                                &format!("{name}_data"),
                            )?
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();

                        let data_alloca = self.builder.build_alloca(
                            self.ptr_type.as_basic_type_enum(),
                            &format!("{name}_ptr"),
                        )?;
                        self.builder.build_store(data_alloca, heap_ptr)?;

                        self.arrays.insert(
                            name.clone(),
                            ArrayInfo {
                                data_ptr_alloca: data_alloca,
                                dim_size_allocas,
                                total_size_alloca: total_alloca,
                                element_vt: vt,
                            },
                        );
                    }
                } else if !self.variables.contains_key(name) {
                    // Scalar DIM
                    let vt = Self::qb_to_var(var_type);
                    let llvm_type = self.var_llvm_type(vt);
                    let alloca = self.builder.build_alloca(llvm_type, name)?;
                    match vt {
                        VarType::Integer => {
                            self.builder
                                .build_store(alloca, self.i32_type.const_zero())?;
                        }
                        VarType::Float => {
                            self.builder
                                .build_store(alloca, self.f32_type.const_zero())?;
                        }
                        VarType::String => {
                            self.builder
                                .build_store(alloca, self.ptr_type.const_null())?;
                        }
                    }
                    self.variables.insert(name.clone(), (alloca, vt));
                }
            }
            Statement::Const { name, value, .. } => {
                let vt = self.infer_expr_type(value);
                let val = self.compile_expr(value, vt)?;
                if !self.variables.contains_key(name) {
                    let llvm_type = self.var_llvm_type(vt);
                    let alloca = self.builder.build_alloca(llvm_type, name)?;
                    self.variables.insert(name.clone(), (alloca, vt));
                }
                if let Some((alloca, _)) = self.variables.get(name) {
                    self.builder.build_store(*alloca, val)?;
                }
            }
            Statement::Let {
                name,
                var_type,
                expr,
                ..
            } => {
                let vt = Self::qb_to_var(var_type);
                if !self.variables.contains_key(name) {
                    let llvm_type = self.var_llvm_type(vt);
                    let alloca = self.builder.build_alloca(llvm_type, name)?;
                    self.variables.insert(name.clone(), (alloca, vt));
                }
                let actual_vt = self.variables.get(name).map(|(_, v)| *v).unwrap_or(vt);
                let val = self.compile_expr(expr, actual_vt)?;
                if let Some((alloca, _)) = self.variables.get(name) {
                    self.builder.build_store(*alloca, val)?;
                }
            }
            Statement::FieldAssign {
                object,
                field,
                expr,
                ..
            } => {
                let flat_name = format!("{object}.{field}");
                let vt = self.infer_expr_type(expr);
                if !self.variables.contains_key(&flat_name) {
                    let llvm_type = self.var_llvm_type(vt);
                    let alloca = self.builder.build_alloca(llvm_type, &flat_name)?;
                    self.variables.insert(flat_name.clone(), (alloca, vt));
                }
                let val = self.compile_expr(expr, vt)?;
                if let Some((alloca, _)) = self.variables.get(&flat_name) {
                    self.builder.build_store(*alloca, val)?;
                }
            }
            Statement::Print { items, .. } => {
                let mut needs_newline = true;
                for item in items {
                    match item {
                        PrintItem::Expr(expr) => {
                            let expr_type = self.infer_expr_type(expr);
                            match expr_type {
                                VarType::Integer => {
                                    let val = self.compile_expr(expr, VarType::Integer)?;
                                    self.builder.build_call(
                                        self.rt_print_int.unwrap(),
                                        &[BasicMetadataValueEnum::from(val.into_int_value())],
                                        "",
                                    )?;
                                }
                                VarType::Float => {
                                    let val = self.compile_expr(expr, VarType::Float)?;
                                    self.builder.build_call(
                                        self.rt_print_float.unwrap(),
                                        &[BasicMetadataValueEnum::from(
                                            val.into_float_value(),
                                        )],
                                        "",
                                    )?;
                                }
                                VarType::String => {
                                    let val = self.compile_expr(expr, VarType::String)?;
                                    self.builder.build_call(
                                        self.rt_print_string.unwrap(),
                                        &[BasicMetadataValueEnum::from(
                                            val.into_pointer_value(),
                                        )],
                                        "",
                                    )?;
                                }
                            }
                            needs_newline = true;
                        }
                        PrintItem::Semicolon => needs_newline = false,
                        PrintItem::Comma => {
                            let tab_str =
                                self.builder.build_global_string_ptr("\t", "tab")?;
                            self.builder.build_call(
                                self.rt_print_string.unwrap(),
                                &[BasicMetadataValueEnum::from(
                                    tab_str.as_pointer_value(),
                                )],
                                "",
                            )?;
                            needs_newline = true;
                        }
                    }
                }
                if needs_newline {
                    self.builder
                        .build_call(self.rt_print_newline.unwrap(), &[], "")?;
                }
            }
            Statement::Input {
                prompt,
                name,
                var_type,
                ..
            } => {
                let vt = Self::qb_to_var(var_type);
                let prompt_ptr = if let Some(p) = prompt {
                    self.builder
                        .build_global_string_ptr(p, "prompt")?
                        .as_pointer_value()
                } else {
                    self.ptr_type.const_null()
                };
                if !self.variables.contains_key(name) {
                    let llvm_type = self.var_llvm_type(vt);
                    let alloca = self.builder.build_alloca(llvm_type, name)?;
                    self.variables.insert(name.clone(), (alloca, vt));
                }
                let rt_fn = match vt {
                    VarType::Integer => self.rt_input_int.unwrap(),
                    VarType::Float => self.rt_input_float.unwrap(),
                    VarType::String => self.rt_input_string.unwrap(),
                };
                let val = self
                    .builder
                    .build_call(
                        rt_fn,
                        &[BasicMetadataValueEnum::from(prompt_ptr)],
                        "input_val",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                if let Some((alloca, _)) = self.variables.get(name) {
                    self.builder.build_store(*alloca, val)?;
                }
            }
            Statement::LineInput {
                prompt, name, ..
            } => {
                let prompt_ptr = if let Some(p) = prompt {
                    self.builder
                        .build_global_string_ptr(p, "prompt")?
                        .as_pointer_value()
                } else {
                    self.ptr_type.const_null()
                };
                if !self.variables.contains_key(name) {
                    let alloca = self
                        .builder
                        .build_alloca(self.ptr_type.as_basic_type_enum(), name)?;
                    self.variables
                        .insert(name.clone(), (alloca, VarType::String));
                }
                let val = self
                    .builder
                    .build_call(
                        self.rt_input_string.unwrap(),
                        &[BasicMetadataValueEnum::from(prompt_ptr)],
                        "linput_val",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                if let Some((alloca, _)) = self.variables.get(name) {
                    self.builder.build_store(*alloca, val)?;
                }
            }
            Statement::If {
                condition,
                then_body,
                else_if_clauses,
                else_body,
                ..
            } => {
                let cond_val = self.compile_condition(condition)?;
                let then_bb = self.context.append_basic_block(function, "if.then");
                let merge_bb = self.context.append_basic_block(function, "if.merge");

                if else_if_clauses.is_empty() && else_body.is_empty() {
                    self.builder
                        .build_conditional_branch(cond_val, then_bb, merge_bb)?;
                    self.builder.position_at_end(then_bb);
                    self.compile_body(then_body)?;
                    if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                        self.builder.build_unconditional_branch(merge_bb)?;
                    }
                } else {
                    let else_bb = self.context.append_basic_block(function, "if.else");
                    self.builder
                        .build_conditional_branch(cond_val, then_bb, else_bb)?;

                    self.builder.position_at_end(then_bb);
                    self.compile_body(then_body)?;
                    if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                        self.builder.build_unconditional_branch(merge_bb)?;
                    }

                    self.builder.position_at_end(else_bb);
                    if !else_if_clauses.is_empty() {
                        for (ei_idx, clause) in else_if_clauses.iter().enumerate() {
                            let ei_cond = self.compile_condition(&clause.condition)?;
                            let ei_then = self.context.append_basic_block(
                                function,
                                &format!("elseif.then.{ei_idx}"),
                            );
                            let ei_next = if ei_idx + 1 < else_if_clauses.len() {
                                self.context.append_basic_block(
                                    function,
                                    &format!("elseif.next.{ei_idx}"),
                                )
                            } else if !else_body.is_empty() {
                                self.context
                                    .append_basic_block(function, "if.else.final")
                            } else {
                                merge_bb
                            };
                            self.builder
                                .build_conditional_branch(ei_cond, ei_then, ei_next)?;
                            self.builder.position_at_end(ei_then);
                            self.compile_body(&clause.body)?;
                            if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                                self.builder.build_unconditional_branch(merge_bb)?;
                            }
                            self.builder.position_at_end(ei_next);
                        }
                        if !else_body.is_empty() {
                            self.compile_body(else_body)?;
                        }
                        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                            self.builder.build_unconditional_branch(merge_bb)?;
                        }
                    } else {
                        self.compile_body(else_body)?;
                        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                            self.builder.build_unconditional_branch(merge_bb)?;
                        }
                    }
                }
                self.builder.position_at_end(merge_bb);
            }
            Statement::For {
                var, from, to, step, body, ..
            } => {
                if !self.variables.contains_key(var) {
                    let alloca = self
                        .builder
                        .build_alloca(self.f32_type.as_basic_type_enum(), var)?;
                    self.variables
                        .insert(var.clone(), (alloca, VarType::Float));
                }
                let (var_alloca, _) = *self.variables.get(var).unwrap();
                let from_val = self.compile_expr(from, VarType::Float)?;
                self.builder.build_store(var_alloca, from_val)?;
                let to_val = self.compile_expr(to, VarType::Float)?;
                let step_val = if let Some(s) = step {
                    self.compile_expr(s, VarType::Float)?
                } else {
                    self.f32_type.const_float(1.0).as_basic_value_enum()
                };
                let loop_bb = self.context.append_basic_block(function, "for.loop");
                let body_bb = self.context.append_basic_block(function, "for.body");
                let after_bb = self.context.append_basic_block(function, "for.after");

                self.for_exit_stack.push(after_bb);
                self.builder.build_unconditional_branch(loop_bb)?;

                self.builder.position_at_end(loop_bb);
                let current = self
                    .builder
                    .build_load(self.f32_type, var_alloca, "for_cur")?
                    .into_float_value();
                let cmp = self.builder.build_float_compare(
                    FloatPredicate::OLE,
                    current,
                    to_val.into_float_value(),
                    "for_cond",
                )?;
                self.builder
                    .build_conditional_branch(cmp, body_bb, after_bb)?;

                self.builder.position_at_end(body_bb);
                self.compile_body(body)?;

                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    let current = self
                        .builder
                        .build_load(self.f32_type, var_alloca, "for_cur2")?
                        .into_float_value();
                    let next_val = self.builder.build_float_add(
                        current,
                        step_val.into_float_value(),
                        "for_next",
                    )?;
                    self.builder.build_store(var_alloca, next_val)?;
                    self.builder.build_unconditional_branch(loop_bb)?;
                }
                self.for_exit_stack.pop();
                self.builder.position_at_end(after_bb);
            }
            Statement::DoLoop {
                pre_condition,
                post_condition,
                body,
                ..
            } => {
                let cond_bb = self.context.append_basic_block(function, "do.cond");
                let body_bb = self.context.append_basic_block(function, "do.body");
                let after_bb = self.context.append_basic_block(function, "do.after");

                self.do_exit_stack.push(after_bb);

                if let Some(cond) = pre_condition {
                    self.builder.build_unconditional_branch(cond_bb)?;
                    self.builder.position_at_end(cond_bb);
                    let cond_val = self.compile_condition(&cond.expr)?;
                    if cond.is_while {
                        self.builder
                            .build_conditional_branch(cond_val, body_bb, after_bb)?;
                    } else {
                        self.builder
                            .build_conditional_branch(cond_val, after_bb, body_bb)?;
                    }
                } else {
                    self.builder.build_unconditional_branch(body_bb)?;
                }

                self.builder.position_at_end(body_bb);
                self.compile_body(body)?;

                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    if let Some(cond) = post_condition {
                        let cond_val = self.compile_condition(&cond.expr)?;
                        if cond.is_while {
                            self.builder
                                .build_conditional_branch(cond_val, body_bb, after_bb)?;
                        } else {
                            self.builder
                                .build_conditional_branch(cond_val, after_bb, body_bb)?;
                        }
                    } else if pre_condition.is_some() {
                        self.builder.build_unconditional_branch(cond_bb)?;
                    } else {
                        self.builder.build_unconditional_branch(body_bb)?;
                    }
                }
                self.do_exit_stack.pop();
                self.builder.position_at_end(after_bb);
            }
            Statement::While {
                condition, body, ..
            } => {
                let cond_bb = self.context.append_basic_block(function, "while.cond");
                let body_bb = self.context.append_basic_block(function, "while.body");
                let after_bb = self.context.append_basic_block(function, "while.after");
                self.builder.build_unconditional_branch(cond_bb)?;
                self.builder.position_at_end(cond_bb);
                let cond_val = self.compile_condition(condition)?;
                self.builder
                    .build_conditional_branch(cond_val, body_bb, after_bb)?;
                self.builder.position_at_end(body_bb);
                self.compile_body(body)?;
                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(cond_bb)?;
                }
                self.builder.position_at_end(after_bb);
            }
            Statement::SelectCase {
                expr,
                cases,
                else_body,
                ..
            } => {
                let select_val = self.compile_expr(expr, self.infer_expr_type(expr))?;
                let select_type = self.infer_expr_type(expr);
                let merge_bb = self.context.append_basic_block(function, "select.merge");
                let else_bb = self.context.append_basic_block(function, "select.else");

                // Build chain of case tests
                for (ci, case) in cases.iter().enumerate() {
                    let case_body_bb = self.context.append_basic_block(
                        function,
                        &format!("case.body.{ci}"),
                    );
                    let next_test = if ci + 1 < cases.len() {
                        self.context
                            .append_basic_block(function, &format!("case.test.{}", ci + 1))
                    } else {
                        else_bb
                    };

                    // OR together all tests for this CASE
                    let mut any_match = self.i32_type.const_int(0, false);
                    for test in &case.tests {
                        let test_result =
                            self.compile_case_test(test, select_val, select_type)?;
                        any_match =
                            self.builder.build_or(any_match, test_result, "case_or")?;
                    }
                    let cond = self.builder.build_int_compare(
                        IntPredicate::NE,
                        any_match,
                        self.i32_type.const_zero(),
                        "case_cond",
                    )?;
                    self.builder
                        .build_conditional_branch(cond, case_body_bb, next_test)?;

                    self.builder.position_at_end(case_body_bb);
                    self.compile_body(&case.body)?;
                    if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                        self.builder.build_unconditional_branch(merge_bb)?;
                    }

                    self.builder.position_at_end(next_test);
                }

                // CASE ELSE or fall-through
                if cases.is_empty() {
                    self.builder.build_unconditional_branch(else_bb)?;
                    self.builder.position_at_end(else_bb);
                }
                if !else_body.is_empty() {
                    self.compile_body(else_body)?;
                }
                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(merge_bb)?;
                }
                self.builder.position_at_end(merge_bb);
            }
            Statement::Goto { target, .. } => {
                if let Some(&target_bb) = self.label_bbs.get(target) {
                    self.builder.build_unconditional_branch(target_bb)?;
                } else {
                    self.builder.build_unconditional_branch(exit_bb)?;
                }
                let after = self.context.append_basic_block(function, "after_goto");
                self.builder.position_at_end(after);
            }
            Statement::Gosub { target, .. } => {
                if let (Some(return_var), Some(&target_bb)) =
                    (self.gosub_return_var, self.label_bbs.get(target))
                {
                    self.gosub_counter += 1;
                    let idx = self.gosub_counter;
                    self.builder.build_store(
                        return_var,
                        self.i32_type.const_int(idx as u64, false),
                    )?;
                    let return_bb = self
                        .context
                        .append_basic_block(function, &format!("gosub_ret_{idx}"));
                    self.gosub_return_points.push((idx, return_bb));
                    self.builder.build_unconditional_branch(target_bb)?;
                    self.builder.position_at_end(return_bb);
                } else {
                    self.builder.build_unconditional_branch(exit_bb)?;
                    let after = self.context.append_basic_block(function, "after_gosub");
                    self.builder.position_at_end(after);
                }
            }
            Statement::Return { .. } => {
                if let Some(dispatch_bb) = self.gosub_dispatch_bb {
                    self.builder.build_unconditional_branch(dispatch_bb)?;
                } else {
                    self.builder.build_unconditional_branch(exit_bb)?;
                }
                let after = self.context.append_basic_block(function, "after_return");
                self.builder.position_at_end(after);
            }
            Statement::CallSub { name, args, .. } => {
                if let Some(&func_val) = self.user_functions.get(name) {
                    let mut arg_vals: Vec<BasicMetadataValueEnum> = Vec::new();
                    let param_types = func_val.get_type().get_param_types();
                    for (i, arg) in args.iter().enumerate() {
                        let param_type = if let Some(param) = param_types.get(i) {
                            if param.is_int_type() {
                                VarType::Integer
                            } else if param.is_float_type() {
                                VarType::Float
                            } else {
                                VarType::String
                            }
                        } else {
                            self.infer_expr_type(arg)
                        };
                        let val = self.compile_expr(arg, param_type)?;
                        arg_vals.push(val.into());
                    }
                    self.builder.build_call(func_val, &arg_vals, "")?;
                }
            }
            Statement::End { .. } => {
                self.builder.build_unconditional_branch(exit_bb)?;
                let after = self.context.append_basic_block(function, "after_end");
                self.builder.position_at_end(after);
            }
            Statement::ExitFor { .. } => {
                if let Some(&after_bb) = self.for_exit_stack.last() {
                    self.builder.build_unconditional_branch(after_bb)?;
                    let cont = self.context.append_basic_block(function, "after_exitfor");
                    self.builder.position_at_end(cont);
                }
            }
            Statement::ExitDo { .. } => {
                if let Some(&after_bb) = self.do_exit_stack.last() {
                    self.builder.build_unconditional_branch(after_bb)?;
                    let cont = self.context.append_basic_block(function, "after_exitdo");
                    self.builder.position_at_end(cont);
                }
            }
            Statement::ExitSub { .. } | Statement::ExitFunction { .. } => {
                self.builder.build_unconditional_branch(exit_bb)?;
                let cont = self.context.append_basic_block(function, "after_exit");
                self.builder.position_at_end(cont);
            }
            Statement::ArrayAssign {
                name,
                var_type: _,
                indices,
                expr,
                ..
            } => {
                if let Some(arr_info) = self.arrays.get(name) {
                    // Copy values out to avoid borrow conflict
                    let element_vt = arr_info.element_vt;
                    let total_alloca = arr_info.total_size_alloca;
                    let data_alloca = arr_info.data_ptr_alloca;

                    let linear_idx = self.compile_array_linear_index(name, indices)?;

                    // Bounds check
                    let total = self
                        .builder
                        .build_load(self.i32_type, total_alloca, "total_sz")?
                        .into_int_value();
                    self.builder.build_call(
                        self.rt_array_bounds_check.unwrap(),
                        &[linear_idx.into(), total.into()],
                        "",
                    )?;

                    // GEP to element
                    let data_ptr = self
                        .builder
                        .build_load(self.ptr_type, data_alloca, "data_ptr")?
                        .into_pointer_value();
                    let elem_llvm_type = self.var_llvm_type(element_vt);
                    let elem_ptr = unsafe {
                        self.builder.build_gep(
                            elem_llvm_type,
                            data_ptr,
                            &[linear_idx],
                            "elem_ptr",
                        )?
                    };

                    // For string arrays, release old value
                    if element_vt == VarType::String {
                        let old_val = self
                            .builder
                            .build_load(self.ptr_type, elem_ptr, "old_str")?
                            .into_pointer_value();
                        self.builder.build_call(
                            self.rt_string_release.unwrap(),
                            &[old_val.into()],
                            "",
                        )?;
                    }

                    // Store new value
                    let val = self.compile_expr(expr, element_vt)?;
                    self.builder.build_store(elem_ptr, val)?;
                }
            }
            Statement::Rem { .. } => {}
            Statement::GpioMode { pin, mode, .. } => {
                let pin_val = self.compile_expr_as_i32(pin)?;
                let mode_val = self.compile_expr_as_i32(mode)?;
                self.builder.build_call(
                    self.rt_gpio_mode.unwrap(),
                    &[
                        BasicMetadataValueEnum::from(pin_val),
                        BasicMetadataValueEnum::from(mode_val),
                    ],
                    "",
                )?;
            }
            Statement::GpioSet { pin, value, .. } => {
                let pin_val = self.compile_expr_as_i32(pin)?;
                let val = self.compile_expr_as_i32(value)?;
                self.builder.build_call(
                    self.rt_gpio_set.unwrap(),
                    &[
                        BasicMetadataValueEnum::from(pin_val),
                        BasicMetadataValueEnum::from(val),
                    ],
                    "",
                )?;
            }
            Statement::GpioRead {
                pin, target, var_type, ..
            } => {
                let pin_val = self.compile_expr_as_i32(pin)?;
                let result = self
                    .builder
                    .build_call(
                        self.rt_gpio_read.unwrap(),
                        &[BasicMetadataValueEnum::from(pin_val)],
                        "gpio_val",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::Delay { ms, .. } => {
                let ms_val = self.compile_expr_as_i32(ms)?;
                self.builder.build_call(
                    self.rt_delay.unwrap(),
                    &[BasicMetadataValueEnum::from(ms_val)],
                    "",
                )?;
            }
            Statement::I2cSetup {
                bus, sda, scl, freq, ..
            } => {
                let b = self.compile_expr_as_i32(bus)?;
                let s = self.compile_expr_as_i32(sda)?;
                let c = self.compile_expr_as_i32(scl)?;
                let f = self.compile_expr_as_i32(freq)?;
                self.builder.build_call(
                    self.rt_i2c_setup.unwrap(),
                    &[b.into(), s.into(), c.into(), f.into()],
                    "",
                )?;
            }
            Statement::I2cWrite { addr, data, .. } => {
                let a = self.compile_expr_as_i32(addr)?;
                let d = self.compile_expr_as_i32(data)?;
                self.builder
                    .build_call(self.rt_i2c_write.unwrap(), &[a.into(), d.into()], "")?;
            }
            Statement::I2cRead {
                addr, length, target, var_type, ..
            } => {
                let a = self.compile_expr_as_i32(addr)?;
                let l = self.compile_expr_as_i32(length)?;
                let result = self
                    .builder
                    .build_call(self.rt_i2c_read.unwrap(), &[a.into(), l.into()], "i2c_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::SpiSetup {
                bus, clk, mosi, miso, freq, ..
            } => {
                let b = self.compile_expr_as_i32(bus)?;
                let c = self.compile_expr_as_i32(clk)?;
                let mo = self.compile_expr_as_i32(mosi)?;
                let mi = self.compile_expr_as_i32(miso)?;
                let f = self.compile_expr_as_i32(freq)?;
                self.builder.build_call(
                    self.rt_spi_setup.unwrap(),
                    &[b.into(), c.into(), mo.into(), mi.into(), f.into()],
                    "",
                )?;
            }
            Statement::SpiTransfer {
                data, target, var_type, ..
            } => {
                let d = self.compile_expr_as_i32(data)?;
                let result = self
                    .builder
                    .build_call(self.rt_spi_transfer.unwrap(), &[d.into()], "spi_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::WifiConnect { ssid, password, .. } => {
                let s = self.compile_expr(ssid, VarType::String)?.into_pointer_value();
                let p = self
                    .compile_expr(password, VarType::String)?
                    .into_pointer_value();
                self.builder.build_call(
                    self.rt_wifi_connect.unwrap(),
                    &[s.into(), p.into()],
                    "",
                )?;
            }
            Statement::WifiStatus {
                target, var_type, ..
            } => {
                let result = self
                    .builder
                    .build_call(self.rt_wifi_status.unwrap(), &[], "wifi_stat")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::WifiDisconnect { .. } => {
                self.builder
                    .build_call(self.rt_wifi_disconnect.unwrap(), &[], "")?;
            }
            Statement::AdcRead {
                pin, target, var_type, ..
            } => {
                let pin_val = self.compile_expr_as_i32(pin)?;
                let result = self
                    .builder
                    .build_call(
                        self.rt_adc_read.unwrap(),
                        &[BasicMetadataValueEnum::from(pin_val)],
                        "adc_val",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::PwmSetup {
                channel, pin, freq, resolution, ..
            } => {
                let ch = self.compile_expr_as_i32(channel)?;
                let p = self.compile_expr_as_i32(pin)?;
                let f = self.compile_expr_as_i32(freq)?;
                let r = self.compile_expr_as_i32(resolution)?;
                self.builder.build_call(
                    self.rt_pwm_setup.unwrap(),
                    &[ch.into(), p.into(), f.into(), r.into()],
                    "",
                )?;
            }
            Statement::PwmDuty { channel, duty, .. } => {
                let ch = self.compile_expr_as_i32(channel)?;
                let d = self.compile_expr_as_i32(duty)?;
                self.builder
                    .build_call(self.rt_pwm_duty.unwrap(), &[ch.into(), d.into()], "")?;
            }
            Statement::UartSetup {
                port, baud, tx, rx, ..
            } => {
                let p = self.compile_expr_as_i32(port)?;
                let b = self.compile_expr_as_i32(baud)?;
                let t = self.compile_expr_as_i32(tx)?;
                let r = self.compile_expr_as_i32(rx)?;
                self.builder.build_call(
                    self.rt_uart_setup.unwrap(),
                    &[p.into(), b.into(), t.into(), r.into()],
                    "",
                )?;
            }
            Statement::UartWrite { port, data, .. } => {
                let p = self.compile_expr_as_i32(port)?;
                let d = self.compile_expr_as_i32(data)?;
                self.builder
                    .build_call(self.rt_uart_write.unwrap(), &[p.into(), d.into()], "")?;
            }
            Statement::UartRead {
                port, target, var_type, ..
            } => {
                let p = self.compile_expr_as_i32(port)?;
                let result = self
                    .builder
                    .build_call(self.rt_uart_read.unwrap(), &[p.into()], "uart_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::TimerStart { .. } => {
                self.builder
                    .build_call(self.rt_timer_start.unwrap(), &[], "")?;
            }
            Statement::TimerElapsed {
                target, var_type, ..
            } => {
                let result = self
                    .builder
                    .build_call(self.rt_timer_elapsed.unwrap(), &[], "timer_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::HttpGet {
                url, target, var_type, ..
            } => {
                let u = self.compile_expr(url, VarType::String)?.into_pointer_value();
                let result = self
                    .builder
                    .build_call(self.rt_http_get.unwrap(), &[u.into()], "http_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::HttpPost {
                url, body, target, var_type, ..
            } => {
                let u = self.compile_expr(url, VarType::String)?.into_pointer_value();
                let b = self.compile_expr(body, VarType::String)?.into_pointer_value();
                let result = self
                    .builder
                    .build_call(
                        self.rt_http_post.unwrap(),
                        &[u.into(), b.into()],
                        "http_val",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::NvsWrite { key, value, .. } => {
                let k = self.compile_expr(key, VarType::String)?.into_pointer_value();
                let v = self.compile_expr_as_i32(value)?;
                self.builder
                    .build_call(self.rt_nvs_write.unwrap(), &[k.into(), v.into()], "")?;
            }
            Statement::NvsRead {
                key, target, var_type, ..
            } => {
                let k = self.compile_expr(key, VarType::String)?.into_pointer_value();
                let result = self
                    .builder
                    .build_call(self.rt_nvs_read.unwrap(), &[k.into()], "nvs_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::MqttConnect { broker, port, .. } => {
                let b = self.compile_expr(broker, VarType::String)?.into_pointer_value();
                let p = self.compile_expr_as_i32(port)?;
                self.builder.build_call(
                    self.rt_mqtt_connect.unwrap(),
                    &[b.into(), p.into()],
                    "",
                )?;
            }
            Statement::MqttDisconnect { .. } => {
                self.builder
                    .build_call(self.rt_mqtt_disconnect.unwrap(), &[], "")?;
            }
            Statement::MqttPublish { topic, message, .. } => {
                let t = self.compile_expr(topic, VarType::String)?.into_pointer_value();
                let m = self.compile_expr(message, VarType::String)?.into_pointer_value();
                self.builder.build_call(
                    self.rt_mqtt_publish.unwrap(),
                    &[t.into(), m.into()],
                    "",
                )?;
            }
            Statement::MqttSubscribe { topic, .. } => {
                let t = self.compile_expr(topic, VarType::String)?.into_pointer_value();
                self.builder
                    .build_call(self.rt_mqtt_subscribe.unwrap(), &[t.into()], "")?;
            }
            Statement::MqttReceive {
                target, var_type, ..
            } => {
                let result = self
                    .builder
                    .build_call(self.rt_mqtt_receive.unwrap(), &[], "mqtt_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::BleInit { name, .. } => {
                let n = self.compile_expr(name, VarType::String)?.into_pointer_value();
                self.builder
                    .build_call(self.rt_ble_init.unwrap(), &[n.into()], "")?;
            }
            Statement::BleAdvertise { mode, .. } => {
                let m = self.compile_expr_as_i32(mode)?;
                self.builder.build_call(
                    self.rt_ble_advertise.unwrap(),
                    &[BasicMetadataValueEnum::from(m)],
                    "",
                )?;
            }
            Statement::BleScan {
                target, var_type, ..
            } => {
                let result = self
                    .builder
                    .build_call(self.rt_ble_scan.unwrap(), &[], "ble_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::BleSend { data, .. } => {
                let d = self.compile_expr(data, VarType::String)?.into_pointer_value();
                self.builder
                    .build_call(self.rt_ble_send.unwrap(), &[d.into()], "")?;
            }
            Statement::BleReceive {
                target, var_type, ..
            } => {
                let result = self
                    .builder
                    .build_call(self.rt_ble_receive.unwrap(), &[], "ble_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::JsonGet {
                json, key, target, var_type, ..
            } => {
                let j = self.compile_expr(json, VarType::String)?.into_pointer_value();
                let k = self.compile_expr(key, VarType::String)?.into_pointer_value();
                let result = self
                    .builder
                    .build_call(
                        self.rt_json_get.unwrap(),
                        &[j.into(), k.into()],
                        "json_val",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::JsonSet {
                json, key, value, target, var_type, ..
            } => {
                let j = self.compile_expr(json, VarType::String)?.into_pointer_value();
                let k = self.compile_expr(key, VarType::String)?.into_pointer_value();
                let v = self.compile_expr(value, VarType::String)?.into_pointer_value();
                let result = self
                    .builder
                    .build_call(
                        self.rt_json_set.unwrap(),
                        &[j.into(), k.into(), v.into()],
                        "json_val",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::JsonCount {
                json, target, var_type, ..
            } => {
                let j = self.compile_expr(json, VarType::String)?.into_pointer_value();
                let result = self
                    .builder
                    .build_call(
                        self.rt_json_count.unwrap(),
                        &[j.into()],
                        "json_cnt",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::LedSetup { pin, count, .. } => {
                let p = self.compile_expr_as_i32(pin)?;
                let c = self.compile_expr_as_i32(count)?;
                self.builder.build_call(
                    self.rt_led_setup.unwrap(),
                    &[p.into(), c.into()],
                    "",
                )?;
            }
            Statement::LedSet { index, r, g, b, .. } => {
                let idx = self.compile_expr_as_i32(index)?;
                let rv = self.compile_expr_as_i32(r)?;
                let gv = self.compile_expr_as_i32(g)?;
                let bv = self.compile_expr_as_i32(b)?;
                self.builder.build_call(
                    self.rt_led_set.unwrap(),
                    &[idx.into(), rv.into(), gv.into(), bv.into()],
                    "",
                )?;
            }
            Statement::LedShow { .. } => {
                self.builder
                    .build_call(self.rt_led_show.unwrap(), &[], "")?;
            }
            Statement::LedClear { .. } => {
                self.builder
                    .build_call(self.rt_led_clear.unwrap(), &[], "")?;
            }
            Statement::DeepSleep { ms, .. } => {
                let ms_val = self.compile_expr_as_i32(ms)?;
                self.builder.build_call(
                    self.rt_deepsleep.unwrap(),
                    &[BasicMetadataValueEnum::from(ms_val)],
                    "",
                )?;
            }
            Statement::EspnowInit { .. } => {
                self.builder
                    .build_call(self.rt_espnow_init.unwrap(), &[], "")?;
            }
            Statement::EspnowSend { peer, data, .. } => {
                let p = self.compile_expr(peer, VarType::String)?.into_pointer_value();
                let d = self.compile_expr(data, VarType::String)?.into_pointer_value();
                self.builder.build_call(
                    self.rt_espnow_send.unwrap(),
                    &[p.into(), d.into()],
                    "",
                )?;
            }
            Statement::EspnowReceive {
                target, var_type, ..
            } => {
                let result = self
                    .builder
                    .build_call(self.rt_espnow_receive.unwrap(), &[], "espnow_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::Data { .. } => {
                // No-op: data items are collected during sema and emitted as globals
            }
            Statement::Read { variables, .. } => {
                for (name, var_type) in variables {
                    let vt = Self::qb_to_var(var_type);
                    self.ensure_var(name, vt)?;
                    let val = match vt {
                        VarType::Integer => {
                            self.builder
                                .build_call(self.rt_data_read_int.unwrap(), &[], "data_int")?
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                        }
                        VarType::Float => {
                            self.builder
                                .build_call(self.rt_data_read_float.unwrap(), &[], "data_float")?
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                        }
                        VarType::String => {
                            self.builder
                                .build_call(self.rt_data_read_string.unwrap(), &[], "data_str")?
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                        }
                    };
                    if let Some((alloca, _)) = self.variables.get(name) {
                        self.builder.build_store(*alloca, val)?;
                    }
                }
            }
            Statement::Restore { .. } => {
                self.builder
                    .build_call(self.rt_data_restore.unwrap(), &[], "")?;
            }

            // ── Classic BASIC extensions ────────────────────────────
            Statement::OnGoto { expr, targets, .. } => {
                let val = self.compile_expr_as_i32(expr)?;
                let after_bb = self.context.append_basic_block(function, "after_on_goto");
                let cases: Vec<_> = targets
                    .iter()
                    .enumerate()
                    .filter_map(|(i, label)| {
                        self.label_bbs.get(label).map(|&bb| {
                            (self.i32_type.const_int((i + 1) as u64, false), bb)
                        })
                    })
                    .collect();
                self.builder.build_switch(val, after_bb, &cases)?;
                self.builder.position_at_end(after_bb);
            }
            Statement::OnGosub { expr, targets, .. } => {
                let val = self.compile_expr_as_i32(expr)?;
                let after_bb = self.context.append_basic_block(function, "after_on_gosub");
                // Pre-create trampoline blocks so we can reference them in the switch
                let mut trampolines: Vec<(usize, String, inkwell::basic_block::BasicBlock<'ctx>)> = Vec::new();
                for (i, label) in targets.iter().enumerate() {
                    if self.label_bbs.contains_key(label) {
                        let trampoline = self.context.append_basic_block(function, &format!("on_gosub_tramp_{i}"));
                        trampolines.push((i, label.clone(), trampoline));
                    }
                }
                // Build the switch instruction
                let cases: Vec<_> = trampolines
                    .iter()
                    .map(|(i, _, bb)| (self.i32_type.const_int((*i + 1) as u64, false), *bb))
                    .collect();
                self.builder.build_switch(val, after_bb, &cases)?;
                // Now fill each trampoline: set gosub return index, branch to target
                for (_i, label, trampoline) in &trampolines {
                    self.builder.position_at_end(*trampoline);
                    if let (Some(return_var), Some(&target_bb)) =
                        (self.gosub_return_var, self.label_bbs.get(label))
                    {
                        self.gosub_counter += 1;
                        let idx = self.gosub_counter;
                        self.builder.build_store(
                            return_var,
                            self.i32_type.const_int(idx as u64, false),
                        )?;
                        self.gosub_return_points.push((idx, after_bb));
                        self.builder.build_unconditional_branch(target_bb)?;
                    }
                }
                self.builder.position_at_end(after_bb);
            }
            Statement::Swap {
                var1, var1_type, var2, var2_type, ..
            } => {
                let vt1 = Self::qb_to_var(var1_type);
                let vt2 = Self::qb_to_var(var2_type);
                self.ensure_var(var1, vt1)?;
                self.ensure_var(var2, vt2)?;
                if let (Some(&(a1, actual_vt1)), Some(&(a2, actual_vt2))) =
                    (self.variables.get(var1), self.variables.get(var2))
                {
                    let lt1 = self.var_llvm_type(actual_vt1);
                    let lt2 = self.var_llvm_type(actual_vt2);
                    let v1 = self.builder.build_load(lt1, a1, "swap_v1")?;
                    let v2 = self.builder.build_load(lt2, a2, "swap_v2")?;
                    self.builder.build_store(a1, v2)?;
                    self.builder.build_store(a2, v1)?;
                }
            }
            Statement::DefFn {
                name, params, body, ..
            } => {
                // Create a new LLVM function for the DEF FN
                let ret_type = if name.ends_with('$') {
                    VarType::String
                } else if name.ends_with('%') {
                    VarType::Integer
                } else {
                    VarType::Float
                };
                let param_types: Vec<BasicMetadataTypeEnum> = params
                    .iter()
                    .map(|(_, t)| {
                        let vt = Self::qb_to_var(t);
                        match vt {
                            VarType::Integer => BasicMetadataTypeEnum::from(self.i32_type),
                            VarType::Float => BasicMetadataTypeEnum::from(self.f32_type),
                            VarType::String => BasicMetadataTypeEnum::from(self.ptr_type),
                        }
                    })
                    .collect();
                let fn_type = match ret_type {
                    VarType::Integer => self.i32_type.fn_type(&param_types, false),
                    VarType::Float => self.f32_type.fn_type(&param_types, false),
                    VarType::String => self.ptr_type.fn_type(&param_types, false),
                };
                let fn_name = format!("rb_deffn_{}", name.to_lowercase());
                let fn_val = self.module.add_function(&fn_name, fn_type, None);
                self.user_functions.insert(name.clone(), fn_val);

                // Save state
                let saved_vars = std::mem::take(&mut self.variables);
                let saved_fn = self.current_function;
                let saved_exit = self.current_exit_bb;
                self.current_function = Some(fn_val);

                let entry = self.context.append_basic_block(fn_val, "entry");
                self.builder.position_at_end(entry);

                // Bind parameters
                for (i, (pname, ptype)) in params.iter().enumerate() {
                    let vt = Self::qb_to_var(ptype);
                    let lt = self.var_llvm_type(vt);
                    let alloca = self.builder.build_alloca(lt, pname)?;
                    self.builder
                        .build_store(alloca, fn_val.get_nth_param(i as u32).unwrap())?;
                    self.variables.insert(pname.clone(), (alloca, vt));
                }

                let result = self.compile_expr(body, ret_type)?;
                self.builder.build_return(Some(&result))?;

                // Restore state
                self.variables = saved_vars;
                self.current_function = saved_fn;
                self.current_exit_bb = saved_exit;
                // Position back in the main function
                if let Some(block) = self.builder.get_insert_block() {
                    if block.get_terminator().is_some() {
                        // We need a new block after the def fn
                        let after = self.context.append_basic_block(function, "after_deffn");
                        self.builder.position_at_end(after);
                    }
                }
            }
            Statement::PrintUsing { format, items, .. } => {
                let fmt = self
                    .compile_expr(format, VarType::String)?
                    .into_pointer_value();
                for item in items {
                    let vt = self.infer_expr_type(item);
                    match vt {
                        VarType::Integer => {
                            let v = self.compile_expr_as_i32(item)?;
                            self.builder.build_call(
                                self.rt_print_using_int.unwrap(),
                                &[fmt.into(), v.into()],
                                "",
                            )?;
                        }
                        VarType::Float => {
                            let v = self.compile_expr(item, VarType::Float)?;
                            self.builder.build_call(
                                self.rt_print_using_float.unwrap(),
                                &[fmt.into(), v.into()],
                                "",
                            )?;
                        }
                        VarType::String => {
                            let v = self
                                .compile_expr(item, VarType::String)?
                                .into_pointer_value();
                            self.builder.build_call(
                                self.rt_print_using_string.unwrap(),
                                &[fmt.into(), v.into()],
                                "",
                            )?;
                        }
                    }
                }
                self.builder
                    .build_call(self.rt_print_newline.unwrap(), &[], "")?;
            }
            Statement::OnErrorGoto { target, .. } => {
                match target {
                    Some(_label) => {
                        // Enable error handler: set rb_error_handler_active = 1
                        // For simplicity, we just call rb_error_clear to reset state
                        // Full setjmp integration would require more complex codegen
                        self.builder
                            .build_call(self.rt_error_clear.unwrap(), &[], "")?;
                    }
                    None => {
                        // ON ERROR GOTO 0 — disable error handler
                        self.builder
                            .build_call(self.rt_error_clear.unwrap(), &[], "")?;
                    }
                }
            }
            Statement::Randomize { seed, .. } => {
                let s = self.compile_expr_as_i32(seed)?;
                self.builder.build_call(
                    self.rt_randomize.unwrap(),
                    &[BasicMetadataValueEnum::from(s)],
                    "",
                )?;
            }

            // ── New hardware statements (ESP32 extensions) ─────────
            Statement::TouchRead {
                pin, target, var_type, ..
            } => {
                let p = self.compile_expr_as_i32(pin)?;
                let result = self
                    .builder
                    .build_call(self.rt_touch_read.unwrap(), &[p.into()], "touch_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::ServoAttach { channel, pin, .. } => {
                let c = self.compile_expr_as_i32(channel)?;
                let p = self.compile_expr_as_i32(pin)?;
                self.builder.build_call(
                    self.rt_servo_attach.unwrap(),
                    &[c.into(), p.into()],
                    "",
                )?;
            }
            Statement::ServoWrite { channel, angle, .. } => {
                let c = self.compile_expr_as_i32(channel)?;
                let a = self.compile_expr_as_i32(angle)?;
                self.builder.build_call(
                    self.rt_servo_write.unwrap(),
                    &[c.into(), a.into()],
                    "",
                )?;
            }
            Statement::Tone {
                pin, freq, duration, ..
            } => {
                let p = self.compile_expr_as_i32(pin)?;
                let f = self.compile_expr_as_i32(freq)?;
                let d = self.compile_expr_as_i32(duration)?;
                self.builder.build_call(
                    self.rt_tone.unwrap(),
                    &[p.into(), f.into(), d.into()],
                    "",
                )?;
            }
            Statement::IrqAttach { pin, mode, .. } => {
                let p = self.compile_expr_as_i32(pin)?;
                let m = self.compile_expr_as_i32(mode)?;
                self.builder.build_call(
                    self.rt_irq_attach.unwrap(),
                    &[p.into(), m.into()],
                    "",
                )?;
            }
            Statement::IrqDetach { pin, .. } => {
                let p = self.compile_expr_as_i32(pin)?;
                self.builder.build_call(
                    self.rt_irq_detach.unwrap(),
                    &[BasicMetadataValueEnum::from(p)],
                    "",
                )?;
            }
            Statement::TempRead {
                target, var_type, ..
            } => {
                let result = self
                    .builder
                    .build_call(self.rt_temp_read.unwrap(), &[], "temp_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
            Statement::OtaUpdate { url, .. } => {
                let u = self.compile_expr(url, VarType::String)?.into_pointer_value();
                self.builder
                    .build_call(self.rt_ota_update.unwrap(), &[u.into()], "")?;
            }
            Statement::OledInit { width, height, .. } => {
                let w = self.compile_expr_as_i32(width)?;
                let h = self.compile_expr_as_i32(height)?;
                self.builder.build_call(
                    self.rt_oled_init.unwrap(),
                    &[w.into(), h.into()],
                    "",
                )?;
            }
            Statement::OledPrint { x, y, text, .. } => {
                let xv = self.compile_expr_as_i32(x)?;
                let yv = self.compile_expr_as_i32(y)?;
                let t = self.compile_expr(text, VarType::String)?.into_pointer_value();
                self.builder.build_call(
                    self.rt_oled_print.unwrap(),
                    &[xv.into(), yv.into(), t.into()],
                    "",
                )?;
            }
            Statement::OledPixel { x, y, color, .. } => {
                let xv = self.compile_expr_as_i32(x)?;
                let yv = self.compile_expr_as_i32(y)?;
                let c = self.compile_expr_as_i32(color)?;
                self.builder.build_call(
                    self.rt_oled_pixel.unwrap(),
                    &[xv.into(), yv.into(), c.into()],
                    "",
                )?;
            }
            Statement::OledLine {
                x1, y1, x2, y2, color, ..
            } => {
                let x1v = self.compile_expr_as_i32(x1)?;
                let y1v = self.compile_expr_as_i32(y1)?;
                let x2v = self.compile_expr_as_i32(x2)?;
                let y2v = self.compile_expr_as_i32(y2)?;
                let cv = self.compile_expr_as_i32(color)?;
                self.builder.build_call(
                    self.rt_oled_line.unwrap(),
                    &[x1v.into(), y1v.into(), x2v.into(), y2v.into(), cv.into()],
                    "",
                )?;
            }
            Statement::OledClear { .. } => {
                self.builder
                    .build_call(self.rt_oled_clear.unwrap(), &[], "")?;
            }
            Statement::OledShow { .. } => {
                self.builder
                    .build_call(self.rt_oled_show.unwrap(), &[], "")?;
            }
            Statement::LcdInit { cols, rows, .. } => {
                let c = self.compile_expr_as_i32(cols)?;
                let r = self.compile_expr_as_i32(rows)?;
                self.builder.build_call(
                    self.rt_lcd_init.unwrap(),
                    &[c.into(), r.into()],
                    "",
                )?;
            }
            Statement::LcdPrint { text, .. } => {
                let t = self.compile_expr(text, VarType::String)?.into_pointer_value();
                self.builder
                    .build_call(self.rt_lcd_print.unwrap(), &[t.into()], "")?;
            }
            Statement::LcdClear { .. } => {
                self.builder
                    .build_call(self.rt_lcd_clear.unwrap(), &[], "")?;
            }
            Statement::LcdPos { col, row, .. } => {
                let c = self.compile_expr_as_i32(col)?;
                let r = self.compile_expr_as_i32(row)?;
                self.builder.build_call(
                    self.rt_lcd_pos.unwrap(),
                    &[c.into(), r.into()],
                    "",
                )?;
            }
            Statement::UdpInit { port, .. } => {
                let p = self.compile_expr_as_i32(port)?;
                self.builder.build_call(
                    self.rt_udp_init.unwrap(),
                    &[BasicMetadataValueEnum::from(p)],
                    "",
                )?;
            }
            Statement::UdpSend {
                host, port, data, ..
            } => {
                let h = self.compile_expr(host, VarType::String)?.into_pointer_value();
                let p = self.compile_expr_as_i32(port)?;
                let d = self.compile_expr(data, VarType::String)?.into_pointer_value();
                self.builder.build_call(
                    self.rt_udp_send.unwrap(),
                    &[h.into(), p.into(), d.into()],
                    "",
                )?;
            }
            Statement::UdpReceive {
                target, var_type, ..
            } => {
                let result = self
                    .builder
                    .build_call(self.rt_udp_receive.unwrap(), &[], "udp_val")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                let vt = Self::qb_to_var(var_type);
                self.ensure_var(target, vt)?;
                if let Some((alloca, _)) = self.variables.get(target) {
                    self.builder.build_store(*alloca, result)?;
                }
            }
        }
        Ok(())
    }

    fn ensure_var(&mut self, name: &str, vt: VarType) -> Result<()> {
        if !self.variables.contains_key(name) {
            let llvm_type = self.var_llvm_type(vt);
            let alloca = self.builder.build_alloca(llvm_type, name)?;
            self.variables.insert(name.to_string(), (alloca, vt));
        }
        Ok(())
    }

    // ── Array helpers ────────────────────────────────────────

    fn compile_array_linear_index(
        &mut self,
        name: &str,
        indices: &[Expr],
    ) -> Result<IntValue<'ctx>> {
        let dim_size_allocas: Vec<PointerValue<'ctx>> =
            self.arrays.get(name).unwrap().dim_size_allocas.clone();

        // Row-major linearization: linear = idx[0]; for d in 1..N: linear = linear * dim_sizes[d] + idx[d]
        let first_idx = self.compile_expr(&indices[0], VarType::Integer)?.into_int_value();
        let mut linear = first_idx;
        for d in 1..indices.len() {
            let dim_size = self
                .builder
                .build_load(self.i32_type, dim_size_allocas[d], &format!("dsz_{d}"))?
                .into_int_value();
            linear = self.builder.build_int_mul(linear, dim_size, "lin_mul")?;
            let idx_d = self.compile_expr(&indices[d], VarType::Integer)?.into_int_value();
            linear = self.builder.build_int_add(linear, idx_d, "lin_add")?;
        }
        Ok(linear)
    }

    fn compile_array_read(
        &mut self,
        name: &str,
        indices: &[Expr],
        target_type: VarType,
    ) -> Result<BasicValueEnum<'ctx>> {
        let (element_vt, total_alloca, data_alloca) = {
            let arr_info = self.arrays.get(name).unwrap();
            (arr_info.element_vt, arr_info.total_size_alloca, arr_info.data_ptr_alloca)
        };

        let linear_idx = self.compile_array_linear_index(name, indices)?;

        // Bounds check
        let total = self
            .builder
            .build_load(self.i32_type, total_alloca, "total_sz")?
            .into_int_value();
        self.builder.build_call(
            self.rt_array_bounds_check.unwrap(),
            &[linear_idx.into(), total.into()],
            "",
        )?;

        // GEP to element
        let data_ptr = self
            .builder
            .build_load(self.ptr_type, data_alloca, "data_ptr")?
            .into_pointer_value();
        let elem_llvm_type = self.var_llvm_type(element_vt);
        let elem_ptr = unsafe {
            self.builder.build_gep(
                elem_llvm_type,
                data_ptr,
                &[linear_idx],
                "elem_ptr",
            )?
        };

        let val = self.builder.build_load(elem_llvm_type, elem_ptr, "arr_val")?;
        self.coerce_value(val, element_vt, target_type)
    }

    // ── SELECT CASE test compilation ────────────────────────

    fn compile_case_test(
        &mut self,
        test: &CaseTest,
        select_val: BasicValueEnum<'ctx>,
        select_type: VarType,
    ) -> Result<IntValue<'ctx>> {
        match test {
            CaseTest::Value(expr) => {
                let test_val = self.compile_expr(expr, select_type)?;
                self.compile_value_compare(select_val, test_val, select_type, BinOp::Eq)
            }
            CaseTest::Range(lo, hi) => {
                let lo_val = self.compile_expr(lo, select_type)?;
                let hi_val = self.compile_expr(hi, select_type)?;
                let ge =
                    self.compile_value_compare(select_val, lo_val, select_type, BinOp::Ge)?;
                let le =
                    self.compile_value_compare(select_val, hi_val, select_type, BinOp::Le)?;
                Ok(self.builder.build_and(ge, le, "range_and")?)
            }
            CaseTest::Is(op, expr) => {
                let test_val = self.compile_expr(expr, select_type)?;
                self.compile_value_compare(select_val, test_val, select_type, *op)
            }
        }
    }

    fn compile_value_compare(
        &mut self,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
        vt: VarType,
        op: BinOp,
    ) -> Result<IntValue<'ctx>> {
        match vt {
            VarType::Integer => {
                let pred = match op {
                    BinOp::Eq => IntPredicate::EQ,
                    BinOp::Neq => IntPredicate::NE,
                    BinOp::Lt => IntPredicate::SLT,
                    BinOp::Gt => IntPredicate::SGT,
                    BinOp::Le => IntPredicate::SLE,
                    BinOp::Ge => IntPredicate::SGE,
                    _ => IntPredicate::EQ,
                };
                let cmp = self.builder.build_int_compare(
                    pred,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "cmp",
                )?;
                Ok(self
                    .builder
                    .build_int_z_extend(cmp, self.i32_type, "cmp_ext")?)
            }
            VarType::Float => {
                let pred = match op {
                    BinOp::Eq => FloatPredicate::OEQ,
                    BinOp::Neq => FloatPredicate::ONE,
                    BinOp::Lt => FloatPredicate::OLT,
                    BinOp::Gt => FloatPredicate::OGT,
                    BinOp::Le => FloatPredicate::OLE,
                    BinOp::Ge => FloatPredicate::OGE,
                    _ => FloatPredicate::OEQ,
                };
                let cmp = self.builder.build_float_compare(
                    pred,
                    lhs.into_float_value(),
                    rhs.into_float_value(),
                    "fcmp",
                )?;
                Ok(self
                    .builder
                    .build_int_z_extend(cmp, self.i32_type, "fcmp_ext")?)
            }
            VarType::String => {
                let cmp_val = self
                    .builder
                    .build_call(
                        self.rt_string_compare.unwrap(),
                        &[
                            BasicMetadataValueEnum::from(lhs.into_pointer_value()),
                            BasicMetadataValueEnum::from(rhs.into_pointer_value()),
                        ],
                        "str_cmp",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let pred = match op {
                    BinOp::Eq => IntPredicate::EQ,
                    BinOp::Neq => IntPredicate::NE,
                    BinOp::Lt => IntPredicate::SLT,
                    BinOp::Gt => IntPredicate::SGT,
                    BinOp::Le => IntPredicate::SLE,
                    BinOp::Ge => IntPredicate::SGE,
                    _ => IntPredicate::EQ,
                };
                let result = self.builder.build_int_compare(
                    pred,
                    cmp_val,
                    self.i32_type.const_zero(),
                    "scmp",
                )?;
                Ok(self
                    .builder
                    .build_int_z_extend(result, self.i32_type, "scmp_ext")?)
            }
        }
    }

    // ── Condition compilation ───────────────────────────────

    fn compile_condition(&mut self, expr: &Expr) -> Result<IntValue<'ctx>> {
        let val = self.compile_expr(expr, VarType::Integer)?;
        let int_val = val.into_int_value();
        let zero = self.i32_type.const_zero();
        Ok(self
            .builder
            .build_int_compare(IntPredicate::NE, int_val, zero, "cond")?)
    }

    // ── Expression compilation ──────────────────────────────

    fn compile_expr(
        &mut self,
        expr: &Expr,
        target_type: VarType,
    ) -> Result<BasicValueEnum<'ctx>> {
        match expr {
            Expr::IntLiteral { value, .. } => {
                let val = self.i32_type.const_int(*value as u64, true);
                match target_type {
                    VarType::Integer => Ok(val.as_basic_value_enum()),
                    VarType::Float => Ok(self
                        .builder
                        .build_signed_int_to_float(val, self.f32_type, "itof")?
                        .as_basic_value_enum()),
                    VarType::String => Ok(val.as_basic_value_enum()),
                }
            }
            Expr::FloatLiteral { value, .. } => {
                let val = self.f32_type.const_float(*value as f64);
                match target_type {
                    VarType::Float => Ok(val.as_basic_value_enum()),
                    VarType::Integer => Ok(self
                        .builder
                        .build_float_to_signed_int(val, self.i32_type, "ftoi")?
                        .as_basic_value_enum()),
                    VarType::String => Ok(val.as_basic_value_enum()),
                }
            }
            Expr::StringLiteral { value, .. } => {
                let global = self
                    .builder
                    .build_global_string_ptr(value, "str_lit")?;
                let ptr = global.as_pointer_value();
                let result = self
                    .builder
                    .build_call(
                        self.rt_string_alloc.unwrap(),
                        &[BasicMetadataValueEnum::from(ptr)],
                        "str_val",
                    )?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                Ok(result)
            }
            Expr::Variable {
                name, var_type, ..
            } => {
                if let Some((alloca, stored_vt)) = self.variables.get(name).copied() {
                    let llvm_type = self.var_llvm_type(stored_vt);
                    let val = self.builder.build_load(llvm_type, alloca, name)?;
                    self.coerce_value(val, stored_vt, target_type)
                } else {
                    let vt = Self::qb_to_var(var_type);
                    match vt {
                        VarType::Integer => Ok(self.i32_type.const_zero().as_basic_value_enum()),
                        VarType::Float => Ok(self.f32_type.const_zero().as_basic_value_enum()),
                        VarType::String => Ok(self.ptr_type.const_null().as_basic_value_enum()),
                    }
                }
            }
            Expr::FieldAccess { object, field, .. } => {
                if let Expr::Variable { name, .. } = object.as_ref() {
                    let flat_name = format!("{name}.{field}");
                    if let Some((alloca, stored_vt)) = self.variables.get(&flat_name).copied() {
                        let llvm_type = self.var_llvm_type(stored_vt);
                        let val = self.builder.build_load(llvm_type, alloca, &flat_name)?;
                        return self.coerce_value(val, stored_vt, target_type);
                    }
                }
                Ok(self.i32_type.const_zero().as_basic_value_enum())
            }
            Expr::BinaryOp {
                op, left, right, ..
            } => self.compile_binary_op(*op, left, right, target_type),
            Expr::UnaryOp { op, operand, .. } => match op {
                UnaryOp::Neg => {
                    let et = self.infer_expr_type(operand);
                    if et == VarType::Float {
                        let val = self
                            .compile_expr(operand, VarType::Float)?
                            .into_float_value();
                        let result = self.builder.build_float_neg(val, "fneg")?;
                        self.coerce_value(result.as_basic_value_enum(), VarType::Float, target_type)
                    } else {
                        let val = self
                            .compile_expr(operand, VarType::Integer)?
                            .into_int_value();
                        let result = self.builder.build_int_neg(val, "ineg")?;
                        self.coerce_value(
                            result.as_basic_value_enum(),
                            VarType::Integer,
                            target_type,
                        )
                    }
                }
                UnaryOp::Not => {
                    let val = self
                        .compile_expr(operand, VarType::Integer)?
                        .into_int_value();
                    Ok(self.builder.build_not(val, "not")?.as_basic_value_enum())
                }
            },
            Expr::FnCall { name, args, .. } => {
                // Check if this is actually an array read
                if self.arrays.contains_key(name) {
                    return self.compile_array_read(name, args, target_type);
                }
                if let Some(&func) = self.user_functions.get(name) {
                    let mut arg_vals: Vec<BasicMetadataValueEnum> = Vec::new();
                    let param_types = func.get_type().get_param_types();
                    for (i, arg) in args.iter().enumerate() {
                        let param_type = if let Some(param) = param_types.get(i) {
                            if param.is_int_type() {
                                VarType::Integer
                            } else if param.is_float_type() {
                                VarType::Float
                            } else {
                                VarType::String
                            }
                        } else {
                            self.infer_expr_type(arg)
                        };
                        let val = self.compile_expr(arg, param_type)?;
                        arg_vals.push(val.into());
                    }
                    let result = self
                        .builder
                        .build_call(func, &arg_vals, "fn_call")?
                        .try_as_basic_value()
                        .left()
                        .unwrap_or(self.f32_type.const_zero().as_basic_value_enum());
                    Ok(result)
                } else {
                    // Built-in string functions with correct type signatures
                    let upper = name.to_uppercase();
                    match upper.as_str() {
                        "LEN" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let result = self.builder.build_call(
                                self.rt_fn_len.unwrap(), &[s.into()], "len"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "ASC" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let result = self.builder.build_call(
                                self.rt_fn_asc.unwrap(), &[s.into()], "asc"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "CHR$" => {
                            let n = self.compile_expr(&args[0], VarType::Integer)?;
                            let result = self.builder.build_call(
                                self.rt_fn_chr_s.unwrap(), &[n.into()], "chr"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "LEFT$" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let n = self.compile_expr(&args[1], VarType::Integer)?;
                            let result = self.builder.build_call(
                                self.rt_fn_left_s.unwrap(), &[s.into(), n.into()], "left"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "RIGHT$" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let n = self.compile_expr(&args[1], VarType::Integer)?;
                            let result = self.builder.build_call(
                                self.rt_fn_right_s.unwrap(), &[s.into(), n.into()], "right"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "MID$" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let start = self.compile_expr(&args[1], VarType::Integer)?;
                            let len = self.compile_expr(&args[2], VarType::Integer)?;
                            let result = self.builder.build_call(
                                self.rt_fn_mid_s.unwrap(), &[s.into(), start.into(), len.into()], "mid"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "INSTR" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let find = self.compile_expr(&args[1], VarType::String)?;
                            let result = self.builder.build_call(
                                self.rt_fn_instr.unwrap(), &[s.into(), find.into()], "instr"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "STR$" => {
                            let n = self.compile_expr(&args[0], VarType::Float)?;
                            let result = self.builder.build_call(
                                self.rt_fn_str_s.unwrap(), &[n.into()], "str"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "VAL" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let result = self.builder.build_call(
                                self.rt_fn_val.unwrap(), &[s.into()], "val"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "UCASE$" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let result = self.builder.build_call(
                                self.rt_fn_ucase_s.unwrap(), &[s.into()], "ucase"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "LCASE$" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let result = self.builder.build_call(
                                self.rt_fn_lcase_s.unwrap(), &[s.into()], "lcase"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        "TRIM$" => {
                            let s = self.compile_expr(&args[0], VarType::String)?;
                            let result = self.builder.build_call(
                                self.rt_fn_trim_s.unwrap(), &[s.into()], "trim"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        // Math built-in functions: (f32) -> f32
                        "SQR" | "ABS" | "SIN" | "COS" | "TAN" | "ATN" | "LOG" | "EXP" => {
                            let x = self.compile_expr(&args[0], VarType::Float)?;
                            let func = match upper.as_str() {
                                "SQR" => self.rt_fn_sqr.unwrap(),
                                "ABS" => self.rt_fn_abs.unwrap(),
                                "SIN" => self.rt_fn_sin.unwrap(),
                                "COS" => self.rt_fn_cos.unwrap(),
                                "TAN" => self.rt_fn_tan.unwrap(),
                                "ATN" => self.rt_fn_atn.unwrap(),
                                "LOG" => self.rt_fn_log.unwrap(),
                                "EXP" => self.rt_fn_exp.unwrap(),
                                _ => unreachable!(),
                            };
                            let label = upper.to_lowercase();
                            let result = self.builder.build_call(
                                func, &[x.into()], &label
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        // Math built-in functions: (f32) -> i32
                        "INT" | "FIX" | "SGN" => {
                            let x = self.compile_expr(&args[0], VarType::Float)?;
                            let func = match upper.as_str() {
                                "INT" => self.rt_fn_int.unwrap(),
                                "FIX" => self.rt_fn_fix.unwrap(),
                                "SGN" => self.rt_fn_sgn.unwrap(),
                                _ => unreachable!(),
                            };
                            let label = upper.to_lowercase();
                            let result = self.builder.build_call(
                                func, &[x.into()], &label
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        // STRING$(n, charcode) -> string
                        "STRING$" => {
                            let n = self.compile_expr(&args[0], VarType::Integer)?;
                            let c = self.compile_expr(&args[1], VarType::Integer)?;
                            let result = self.builder.build_call(
                                self.rt_fn_string_s.unwrap(), &[n.into(), c.into()], "string_s"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        // SPACE$(n) -> string
                        "SPACE$" => {
                            let n = self.compile_expr(&args[0], VarType::Integer)?;
                            let result = self.builder.build_call(
                                self.rt_fn_space_s.unwrap(), &[n.into()], "space_s"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        // Math built-in function: () -> f32
                        "RND" => {
                            let result = self.builder.build_call(
                                self.rt_fn_rnd.unwrap(), &[], "rnd"
                            )?.try_as_basic_value().left().unwrap();
                            Ok(result)
                        }
                        _ => {
                            // Generic fallback for other built-ins (math, etc.)
                            let fn_name = format!("rb_fn_{}", name.to_lowercase());
                            if let Some(func) = self.module.get_function(&fn_name) {
                                let mut arg_vals: Vec<BasicMetadataValueEnum> = Vec::new();
                                for arg in args {
                                    let val = self.compile_expr(arg, VarType::Float)?;
                                    arg_vals.push(val.into());
                                }
                                let result = self
                                    .builder
                                    .build_call(func, &arg_vals, "fn_call")?
                                    .try_as_basic_value()
                                    .left()
                                    .unwrap_or(self.f32_type.const_zero().as_basic_value_enum());
                                Ok(result)
                            } else {
                                Ok(self.f32_type.const_zero().as_basic_value_enum())
                            }
                        }
                    }
                }
            }
            Expr::ArrayAccess {
                name, indices, ..
            } => {
                if self.arrays.contains_key(name) {
                    self.compile_array_read(name, indices, target_type)
                } else {
                    Ok(self.i32_type.const_zero().as_basic_value_enum())
                }
            }
        }
    }

    fn compile_binary_op(
        &mut self,
        op: BinOp,
        left: &Expr,
        right: &Expr,
        _target_type: VarType,
    ) -> Result<BasicValueEnum<'ctx>> {
        // String operations
        if self.infer_expr_type(left) == VarType::String
            || self.infer_expr_type(right) == VarType::String
        {
            let lhs = self
                .compile_expr(left, VarType::String)?
                .into_pointer_value();
            let rhs = self
                .compile_expr(right, VarType::String)?
                .into_pointer_value();
            return match op {
                BinOp::Add => {
                    let result = self
                        .builder
                        .build_call(
                            self.rt_string_concat.unwrap(),
                            &[lhs.into(), rhs.into()],
                            "str_cat",
                        )?
                        .try_as_basic_value()
                        .left()
                        .unwrap();
                    Ok(result)
                }
                BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                    let cmp = self
                        .builder
                        .build_call(
                            self.rt_string_compare.unwrap(),
                            &[lhs.into(), rhs.into()],
                            "str_cmp",
                        )?
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_int_value();
                    let pred = match op {
                        BinOp::Eq => IntPredicate::EQ,
                        BinOp::Neq => IntPredicate::NE,
                        BinOp::Lt => IntPredicate::SLT,
                        BinOp::Gt => IntPredicate::SGT,
                        BinOp::Le => IntPredicate::SLE,
                        BinOp::Ge => IntPredicate::SGE,
                        _ => unreachable!(),
                    };
                    let result = self.builder.build_int_compare(
                        pred,
                        cmp,
                        self.i32_type.const_zero(),
                        "scmp",
                    )?;
                    Ok(self
                        .builder
                        .build_int_z_extend(result, self.i32_type, "scmp_ext")?
                        .as_basic_value_enum())
                }
                _ => Ok(self.i32_type.const_zero().as_basic_value_enum()),
            };
        }

        // IntDiv and Xor always use integer
        if op == BinOp::IntDiv || op == BinOp::Xor {
            let lhs = self.compile_expr(left, VarType::Integer)?.into_int_value();
            let rhs = self
                .compile_expr(right, VarType::Integer)?
                .into_int_value();
            let result = match op {
                BinOp::IntDiv => self.builder.build_int_signed_div(lhs, rhs, "intdiv")?,
                BinOp::Xor => self.builder.build_xor(lhs, rhs, "xor")?,
                _ => unreachable!(),
            };
            return Ok(result.as_basic_value_enum());
        }

        let result_type = self.infer_binary_result_type(op, left, right);
        let use_float = result_type == VarType::Float;

        if use_float {
            let lhs = self.compile_expr(left, VarType::Float)?.into_float_value();
            let rhs = self
                .compile_expr(right, VarType::Float)?
                .into_float_value();
            match op {
                BinOp::Add => Ok(self.builder.build_float_add(lhs, rhs, "fadd")?.as_basic_value_enum()),
                BinOp::Sub => Ok(self.builder.build_float_sub(lhs, rhs, "fsub")?.as_basic_value_enum()),
                BinOp::Mul => Ok(self.builder.build_float_mul(lhs, rhs, "fmul")?.as_basic_value_enum()),
                BinOp::Div => Ok(self.builder.build_float_div(lhs, rhs, "fdiv")?.as_basic_value_enum()),
                BinOp::Mod => Ok(self.builder.build_float_rem(lhs, rhs, "fmod")?.as_basic_value_enum()),
                BinOp::Pow => {
                    let result = self
                        .builder
                        .build_call(
                            self.rt_powf.unwrap(),
                            &[lhs.into(), rhs.into()],
                            "pow",
                        )?
                        .try_as_basic_value()
                        .left()
                        .unwrap();
                    Ok(result)
                }
                BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                    let pred = match op {
                        BinOp::Eq => FloatPredicate::OEQ,
                        BinOp::Neq => FloatPredicate::ONE,
                        BinOp::Lt => FloatPredicate::OLT,
                        BinOp::Gt => FloatPredicate::OGT,
                        BinOp::Le => FloatPredicate::OLE,
                        BinOp::Ge => FloatPredicate::OGE,
                        _ => unreachable!(),
                    };
                    let cmp = self.builder.build_float_compare(pred, lhs, rhs, "fcmp")?;
                    Ok(self
                        .builder
                        .build_int_z_extend(cmp, self.i32_type, "fcmp_ext")?
                        .as_basic_value_enum())
                }
                BinOp::And | BinOp::Or => {
                    let li = self
                        .builder
                        .build_float_to_signed_int(lhs, self.i32_type, "land_l")?;
                    let ri = self
                        .builder
                        .build_float_to_signed_int(rhs, self.i32_type, "land_r")?;
                    let result = match op {
                        BinOp::And => self.builder.build_and(li, ri, "land")?,
                        BinOp::Or => self.builder.build_or(li, ri, "lor")?,
                        _ => unreachable!(),
                    };
                    Ok(result.as_basic_value_enum())
                }
                _ => Ok(self.f32_type.const_zero().as_basic_value_enum()),
            }
        } else {
            let lhs = self
                .compile_expr(left, VarType::Integer)?
                .into_int_value();
            let rhs = self
                .compile_expr(right, VarType::Integer)?
                .into_int_value();
            match op {
                BinOp::Add => Ok(self.builder.build_int_add(lhs, rhs, "iadd")?.as_basic_value_enum()),
                BinOp::Sub => Ok(self.builder.build_int_sub(lhs, rhs, "isub")?.as_basic_value_enum()),
                BinOp::Mul => Ok(self.builder.build_int_mul(lhs, rhs, "imul")?.as_basic_value_enum()),
                BinOp::Div => Ok(self.builder.build_int_signed_div(lhs, rhs, "idiv")?.as_basic_value_enum()),
                BinOp::Mod => Ok(self.builder.build_int_signed_rem(lhs, rhs, "imod")?.as_basic_value_enum()),
                BinOp::Pow => {
                    let fl = self.builder.build_signed_int_to_float(lhs, self.f32_type, "pow_l")?;
                    let fr = self.builder.build_signed_int_to_float(rhs, self.f32_type, "pow_r")?;
                    let result = self
                        .builder
                        .build_call(self.rt_powf.unwrap(), &[fl.into(), fr.into()], "pow")?
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_float_value();
                    Ok(self
                        .builder
                        .build_float_to_signed_int(result, self.i32_type, "pow_i")?
                        .as_basic_value_enum())
                }
                BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                    let pred = match op {
                        BinOp::Eq => IntPredicate::EQ,
                        BinOp::Neq => IntPredicate::NE,
                        BinOp::Lt => IntPredicate::SLT,
                        BinOp::Gt => IntPredicate::SGT,
                        BinOp::Le => IntPredicate::SLE,
                        BinOp::Ge => IntPredicate::SGE,
                        _ => unreachable!(),
                    };
                    let cmp = self.builder.build_int_compare(pred, lhs, rhs, "icmp")?;
                    Ok(self
                        .builder
                        .build_int_z_extend(cmp, self.i32_type, "icmp_ext")?
                        .as_basic_value_enum())
                }
                BinOp::And => Ok(self.builder.build_and(lhs, rhs, "iand")?.as_basic_value_enum()),
                BinOp::Or => Ok(self.builder.build_or(lhs, rhs, "ior")?.as_basic_value_enum()),
                _ => Ok(self.i32_type.const_zero().as_basic_value_enum()),
            }
        }
    }

    fn compile_expr_as_i32(&mut self, expr: &Expr) -> Result<IntValue<'ctx>> {
        let val = self.compile_expr(expr, VarType::Integer)?;
        Ok(val.into_int_value())
    }

    fn coerce_value(
        &mut self,
        val: BasicValueEnum<'ctx>,
        from: VarType,
        to: VarType,
    ) -> Result<BasicValueEnum<'ctx>> {
        if from == to || to == VarType::String || from == VarType::String {
            return Ok(val);
        }
        if from == VarType::Integer && to == VarType::Float {
            Ok(self
                .builder
                .build_signed_int_to_float(val.into_int_value(), self.f32_type, "itof")?
                .as_basic_value_enum())
        } else if from == VarType::Float && to == VarType::Integer {
            Ok(self
                .builder
                .build_float_to_signed_int(val.into_float_value(), self.i32_type, "ftoi")?
                .as_basic_value_enum())
        } else {
            Ok(val)
        }
    }

    // ── Type inference ──────────────────────────────────────

    fn infer_expr_type(&self, expr: &Expr) -> VarType {
        match expr {
            Expr::IntLiteral { .. } => VarType::Integer,
            Expr::FloatLiteral { .. } => VarType::Float,
            Expr::StringLiteral { .. } => VarType::String,
            Expr::Variable { name, var_type, .. } => {
                if let Some((_, vt)) = self.variables.get(name) {
                    return *vt;
                }
                Self::qb_to_var(var_type)
            }
            Expr::FieldAccess { object, field, .. } => {
                if let Expr::Variable { name, .. } = object.as_ref() {
                    let flat = format!("{name}.{field}");
                    if let Some((_, vt)) = self.variables.get(&flat) {
                        return *vt;
                    }
                }
                VarType::Float
            }
            Expr::BinaryOp { op, left, right, .. } => {
                self.infer_binary_result_type(*op, left, right)
            }
            Expr::UnaryOp { operand, .. } => self.infer_expr_type(operand),
            Expr::FnCall { name, .. } => {
                // Check if this is an array read
                if let Some(arr_info) = self.arrays.get(name) {
                    return arr_info.element_vt;
                }
                if let Some(var_info) = self.sema.variables.get(name) {
                    if var_info.is_array {
                        return Self::qb_to_var(&var_info.qb_type);
                    }
                }
                if let Some(func_info) = self.sema.functions.get(name) {
                    return Self::qb_to_var(&func_info.return_type);
                }
                // Built-in function type inference
                let upper = name.to_uppercase();
                if upper.ends_with('$') {
                    return VarType::String;
                }
                match upper.as_str() {
                    "LEN" | "ASC" | "INSTR" | "INT" | "FIX" | "SGN" => VarType::Integer,
                    "VAL" => VarType::Float,
                    _ => VarType::Float,
                }
            }
            Expr::ArrayAccess { var_type, .. } => Self::qb_to_var(var_type),
        }
    }

    fn infer_binary_result_type(&self, op: BinOp, left: &Expr, right: &Expr) -> VarType {
        let lt = self.infer_expr_type(left);
        let rt = self.infer_expr_type(right);
        if lt == VarType::String || rt == VarType::String {
            if op == BinOp::Add {
                return VarType::String;
            }
            return VarType::Integer;
        }
        if matches!(
            op,
            BinOp::Eq
                | BinOp::Neq
                | BinOp::Lt
                | BinOp::Gt
                | BinOp::Le
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or
                | BinOp::Xor
                | BinOp::IntDiv
        ) {
            return VarType::Integer;
        }
        if lt == VarType::Float || rt == VarType::Float {
            VarType::Float
        } else {
            VarType::Integer
        }
    }

    // ── IR and object file output ───────────────────────────

    pub fn dump_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    pub fn write_object_file(&self, path: &Path) -> Result<()> {
        self.init_target()?;
        let triple = TargetTriple::create(&self.target_config.triple);
        let target = Target::from_triple(&triple)
            .map_err(|e| anyhow::anyhow!("failed to get target: {}", e))?;
        let machine = target
            .create_target_machine(
                &triple,
                &self.target_config.cpu,
                &self.target_config.features,
                OptimizationLevel::Default,
                RelocMode::PIC,
                CodeModel::Small,
            )
            .context("failed to create target machine")?;
        machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| anyhow::anyhow!("failed to write object file: {}", e))?;
        Ok(())
    }

    fn init_target(&self) -> Result<()> {
        if self.target_config.triple.contains("riscv") {
            Target::initialize_riscv(&InitializationConfig::default());
        } else {
            Target::initialize_native(&InitializationConfig::default())
                .map_err(|e| anyhow::anyhow!("failed to initialize native target: {e}"))?;
        }
        Ok(())
    }
}

/// Initialize all LLVM targets (call once at program start).
pub fn init_all_targets() {
    Target::initialize_all(&InitializationConfig::default());
}
