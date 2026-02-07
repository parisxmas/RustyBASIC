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
    rt_powf: Option<FunctionValue<'ctx>>,
    rt_array_alloc: Option<FunctionValue<'ctx>>,
    rt_array_free: Option<FunctionValue<'ctx>>,
    rt_array_bounds_check: Option<FunctionValue<'ctx>>,

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
            rt_powf: None,
            rt_array_alloc: None,
            rt_array_free: None,
            rt_array_bounds_check: None,
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

    // ── Compilation entry point ─────────────────────────────

    pub fn compile(&mut self, program: &Program) -> Result<()> {
        let triple = TargetTriple::create(&self.target_config.triple);
        self.module.set_triple(&triple);

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
                VarType::Float
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
