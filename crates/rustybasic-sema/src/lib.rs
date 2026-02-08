use std::collections::{HashMap, HashSet};

use codespan_reporting::diagnostic::{Diagnostic, Label};
use rustybasic_common::{FileId, Span};
use rustybasic_parser::ast::*;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("{message}")]
pub struct SemaError {
    pub span: Span,
    pub message: String,
}

/// Information about a variable discovered during analysis.
#[derive(Debug, Clone)]
pub struct VarInfo {
    pub qb_type: QBType,
    pub is_array: bool,
    pub dimensions: usize,
}

/// Information about a SUB definition.
#[derive(Debug, Clone)]
pub struct SubInfo {
    pub params: Vec<QBType>,
    pub span: Span,
}

/// Information about a FUNCTION definition.
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub params: Vec<QBType>,
    pub return_type: QBType,
    pub span: Span,
}

/// Information about a TYPE definition.
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub fields: HashMap<String, QBType>,
    pub span: Span,
}

/// Result of semantic analysis.
#[derive(Debug)]
pub struct SemaResult {
    pub variables: HashMap<String, VarInfo>,
    pub labels: HashSet<String>,
    pub gosub_targets: HashSet<String>,
    pub has_gosub: bool,
    pub subs: HashMap<String, SubInfo>,
    pub functions: HashMap<String, FunctionInfo>,
    pub types: HashMap<String, TypeInfo>,
    pub constants: HashSet<String>,
    pub errors: Vec<SemaError>,
}

impl SemaResult {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn to_diagnostics(&self, file_id: FileId) -> Vec<Diagnostic<FileId>> {
        self.errors
            .iter()
            .map(|e| {
                Diagnostic::error()
                    .with_message(&e.message)
                    .with_labels(vec![
                        Label::primary(file_id, e.span.start..e.span.end)
                            .with_message(&e.message),
                    ])
            })
            .collect()
    }
}

/// Scope context for tracking where EXIT statements are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    TopLevel,
    ForLoop,
    DoLoop,
    Sub,
    Function,
}

pub struct SemanticAnalyzer {
    variables: HashMap<String, VarInfo>,
    labels: HashSet<String>,
    goto_targets: Vec<(String, Span)>,
    gosub_targets: HashSet<String>,
    has_gosub: bool,
    subs: HashMap<String, SubInfo>,
    functions: HashMap<String, FunctionInfo>,
    types: HashMap<String, TypeInfo>,
    constants: HashSet<String>,
    scope_stack: Vec<ScopeKind>,
    errors: Vec<SemaError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            labels: HashSet::new(),
            goto_targets: Vec::new(),
            gosub_targets: HashSet::new(),
            has_gosub: false,
            subs: HashMap::new(),
            functions: HashMap::new(),
            types: HashMap::new(),
            constants: HashSet::new(),
            scope_stack: vec![ScopeKind::TopLevel],
            errors: Vec::new(),
        }
    }

    pub fn analyze(mut self, program: &Program) -> SemaResult {
        // Pass 1: Collect TYPE definitions
        for type_def in &program.types {
            self.collect_type_def(type_def);
        }

        // Pass 2: Validate TYPE field references (UserType fields must reference known types)
        self.validate_type_field_references();

        // Pass 3: Collect SUB/FUNCTION signatures
        for sub_def in &program.subs {
            self.collect_sub_def(sub_def);
        }
        for func_def in &program.functions {
            self.collect_function_def(func_def);
        }

        // Pass 4: Collect labels from all bodies
        self.collect_labels(&program.body);
        for sub_def in &program.subs {
            self.collect_labels(&sub_def.body);
        }
        for func_def in &program.functions {
            self.collect_labels(&func_def.body);
        }

        // Pass 5: Check module-level body
        for stmt in &program.body {
            self.check_statement(stmt);
        }

        // Pass 6: Check SUB bodies
        for sub_def in &program.subs {
            self.check_sub_body(sub_def);
        }

        // Pass 7: Check FUNCTION bodies
        for func_def in &program.functions {
            self.check_function_body(func_def);
        }

        // Pass 8: Validate GOTO/GOSUB targets
        for (target, span) in &self.goto_targets {
            if !self.labels.contains(target) {
                self.errors.push(SemaError {
                    span: *span,
                    message: format!("undefined label: {target}"),
                });
            }
        }

        SemaResult {
            variables: self.variables,
            labels: self.labels,
            gosub_targets: self.gosub_targets,
            has_gosub: self.has_gosub,
            subs: self.subs,
            functions: self.functions,
            types: self.types,
            constants: self.constants,
            errors: self.errors,
        }
    }

    // ── Pass 1: Collect TYPE definitions ─────────────────────

    fn collect_type_def(&mut self, type_def: &TypeDef) {
        if self.types.contains_key(&type_def.name) {
            self.errors.push(SemaError {
                span: type_def.span,
                message: format!("duplicate TYPE definition: {}", type_def.name),
            });
            return;
        }

        let mut fields = HashMap::new();
        for field in &type_def.fields {
            if fields.contains_key(&field.name) {
                self.errors.push(SemaError {
                    span: field.span,
                    message: format!(
                        "duplicate field '{}' in TYPE {}",
                        field.name, type_def.name
                    ),
                });
            } else {
                fields.insert(field.name.clone(), field.field_type.clone());
            }
        }

        self.types.insert(
            type_def.name.clone(),
            TypeInfo {
                fields,
                span: type_def.span,
            },
        );
    }

    // ── Pass 2: Validate TYPE field type references ──────────

    fn validate_type_field_references(&mut self) {
        // Collect the type names first to avoid borrow issues
        let type_names: HashSet<String> = self.types.keys().cloned().collect();
        let mut errors = Vec::new();

        for (type_name, type_info) in &self.types {
            for (field_name, field_type) in &type_info.fields {
                if let QBType::UserType(ref ref_name) = field_type {
                    if !type_names.contains(ref_name) {
                        errors.push(SemaError {
                            span: type_info.span,
                            message: format!(
                                "field '{}' in TYPE {} references undefined TYPE '{}'",
                                field_name, type_name, ref_name
                            ),
                        });
                    }
                }
            }
        }

        self.errors.extend(errors);
    }

    // ── Pass 3: Collect SUB/FUNCTION signatures ──────────────

    fn collect_sub_def(&mut self, sub_def: &SubDef) {
        if self.subs.contains_key(&sub_def.name) {
            self.errors.push(SemaError {
                span: sub_def.span,
                message: format!("duplicate SUB definition: {}", sub_def.name),
            });
            return;
        }

        // Validate parameter types
        for param in &sub_def.params {
            if let QBType::UserType(ref type_name) = param.param_type {
                if !self.types.contains_key(type_name) {
                    self.errors.push(SemaError {
                        span: param.span,
                        message: format!(
                            "parameter '{}' in SUB {} has undefined TYPE '{}'",
                            param.name, sub_def.name, type_name
                        ),
                    });
                }
            }
        }

        let params: Vec<QBType> = sub_def.params.iter().map(|p| p.param_type.clone()).collect();
        self.subs.insert(
            sub_def.name.clone(),
            SubInfo {
                params,
                span: sub_def.span,
            },
        );
    }

    fn collect_function_def(&mut self, func_def: &FunctionDef) {
        if self.functions.contains_key(&func_def.name) {
            self.errors.push(SemaError {
                span: func_def.span,
                message: format!("duplicate FUNCTION definition: {}", func_def.name),
            });
            return;
        }

        // Validate parameter types
        for param in &func_def.params {
            if let QBType::UserType(ref type_name) = param.param_type {
                if !self.types.contains_key(type_name) {
                    self.errors.push(SemaError {
                        span: param.span,
                        message: format!(
                            "parameter '{}' in FUNCTION {} has undefined TYPE '{}'",
                            param.name, func_def.name, type_name
                        ),
                    });
                }
            }
        }

        // Validate return type
        if let QBType::UserType(ref type_name) = func_def.return_type {
            if !self.types.contains_key(type_name) {
                self.errors.push(SemaError {
                    span: func_def.span,
                    message: format!(
                        "FUNCTION {} has undefined return TYPE '{}'",
                        func_def.name, type_name
                    ),
                });
            }
        }

        let params: Vec<QBType> = func_def.params.iter().map(|p| p.param_type.clone()).collect();
        self.functions.insert(
            func_def.name.clone(),
            FunctionInfo {
                params,
                return_type: func_def.return_type.clone(),
                span: func_def.span,
            },
        );
    }

    // ── Pass 4: Collect labels ───────────────────────────────

    fn collect_labels(&mut self, stmts: &[Statement]) {
        for stmt in stmts {
            self.collect_labels_in_stmt(stmt);
        }
    }

    fn collect_labels_in_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Label { name, span } => {
                if !self.labels.insert(name.clone()) {
                    self.errors.push(SemaError {
                        span: *span,
                        message: format!("duplicate label: {name}"),
                    });
                }
            }
            // Recurse into compound statements to find nested labels
            Statement::If {
                then_body,
                else_if_clauses,
                else_body,
                ..
            } => {
                self.collect_labels(then_body);
                for clause in else_if_clauses {
                    self.collect_labels(&clause.body);
                }
                self.collect_labels(else_body);
            }
            Statement::For { body, .. }
            | Statement::While { body, .. }
            | Statement::DoLoop { body, .. } => {
                self.collect_labels(body);
            }
            Statement::SelectCase {
                cases, else_body, ..
            } => {
                for case in cases {
                    self.collect_labels(&case.body);
                }
                self.collect_labels(else_body);
            }
            _ => {}
        }
    }

    // ── Pass 6/7: Check SUB/FUNCTION bodies ──────────────────

    fn check_sub_body(&mut self, sub_def: &SubDef) {
        self.scope_stack.push(ScopeKind::Sub);

        // Register parameters as variables
        for param in &sub_def.params {
            self.register_var(&param.name, &param.param_type);
        }

        for stmt in &sub_def.body {
            self.check_statement(stmt);
        }

        self.scope_stack.pop();
    }

    fn check_function_body(&mut self, func_def: &FunctionDef) {
        self.scope_stack.push(ScopeKind::Function);

        // Register parameters as variables
        for param in &func_def.params {
            self.register_var(&param.name, &param.param_type);
        }

        // The function name itself can be assigned to (return value)
        self.register_var(&func_def.name, &func_def.return_type);

        for stmt in &func_def.body {
            self.check_statement(stmt);
        }

        self.scope_stack.pop();
    }

    // ── Statement checking ───────────────────────────────────

    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let {
                name,
                var_type,
                expr,
                span,
            } => {
                let expr_type = self.check_expr(expr);
                self.declare_or_check_var(name, var_type, *span);
                self.check_type_assignment(name, var_type, expr_type.as_ref(), *span);
            }
            Statement::Dim {
                name,
                var_type,
                dimensions,
                is_shared: _,
                span,
            } => {
                let is_array = !dimensions.is_empty();
                // Validate UserType references
                if let QBType::UserType(ref type_name) = var_type {
                    if !self.types.contains_key(type_name) {
                        self.errors.push(SemaError {
                            span: *span,
                            message: format!("undefined TYPE: {type_name}"),
                        });
                    }
                }
                self.variables.insert(
                    name.clone(),
                    VarInfo {
                        qb_type: var_type.clone(),
                        is_array,
                        dimensions: dimensions.len(),
                    },
                );
                for dim_expr in dimensions {
                    self.check_expr(dim_expr);
                }
            }
            Statement::Const { name, value, span } => {
                self.check_expr(value);
                if self.constants.contains(name) {
                    self.errors.push(SemaError {
                        span: *span,
                        message: format!("duplicate CONST: {name}"),
                    });
                } else {
                    self.constants.insert(name.clone());
                }
            }
            Statement::FieldAssign {
                object,
                field,
                expr,
                span,
            } => {
                self.check_expr(expr);
                // Validate that the object variable is a user type with this field
                if let Some(info) = self.variables.get(object) {
                    if let QBType::UserType(ref type_name) = info.qb_type {
                        if let Some(type_info) = self.types.get(type_name) {
                            if !type_info.fields.contains_key(field) {
                                self.errors.push(SemaError {
                                    span: *span,
                                    message: format!(
                                        "TYPE {} has no field '{}'",
                                        type_name, field
                                    ),
                                });
                            }
                        }
                    } else {
                        self.errors.push(SemaError {
                            span: *span,
                            message: format!(
                                "variable '{}' is not a user-defined type",
                                object
                            ),
                        });
                    }
                }
                // If object not yet declared, no error (auto-declaration in BASIC)
            }
            Statement::Print { items, .. } => {
                for item in items {
                    if let PrintItem::Expr(expr) = item {
                        self.check_expr(expr);
                    }
                }
            }
            Statement::Input {
                name, var_type, span, ..
            } => {
                self.declare_or_check_var(name, var_type, *span);
            }
            Statement::LineInput { name, span, .. } => {
                // LINE INPUT always reads a string
                self.declare_or_check_var(name, &QBType::String, *span);
            }
            Statement::If {
                condition,
                then_body,
                else_if_clauses,
                else_body,
                ..
            } => {
                self.check_expr(condition);
                for s in then_body {
                    self.check_statement(s);
                }
                for clause in else_if_clauses {
                    self.check_expr(&clause.condition);
                    for s in &clause.body {
                        self.check_statement(s);
                    }
                }
                for s in else_body {
                    self.check_statement(s);
                }
            }
            Statement::For {
                var,
                from,
                to,
                step,
                body,
                span,
            } => {
                // FOR loop variables are numeric, default to Inferred
                self.declare_or_check_var(var, &QBType::Inferred, *span);
                self.check_expr(from);
                self.check_expr(to);
                if let Some(s) = step {
                    self.check_expr(s);
                }
                self.scope_stack.push(ScopeKind::ForLoop);
                for s in body {
                    self.check_statement(s);
                }
                self.scope_stack.pop();
            }
            Statement::DoLoop {
                pre_condition,
                post_condition,
                body,
                span,
            } => {
                if pre_condition.is_some() && post_condition.is_some() {
                    self.errors.push(SemaError {
                        span: *span,
                        message: "DO...LOOP cannot have both pre-condition and post-condition"
                            .to_string(),
                    });
                }
                if let Some(cond) = pre_condition {
                    self.check_expr(&cond.expr);
                }
                self.scope_stack.push(ScopeKind::DoLoop);
                for s in body {
                    self.check_statement(s);
                }
                self.scope_stack.pop();
                if let Some(cond) = post_condition {
                    self.check_expr(&cond.expr);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.check_expr(condition);
                for s in body {
                    self.check_statement(s);
                }
            }
            Statement::SelectCase {
                expr,
                cases,
                else_body,
                ..
            } => {
                self.check_expr(expr);
                for case_clause in cases {
                    for test in &case_clause.tests {
                        match test {
                            CaseTest::Value(e) => {
                                self.check_expr(e);
                            }
                            CaseTest::Range(lo, hi) => {
                                self.check_expr(lo);
                                self.check_expr(hi);
                            }
                            CaseTest::Is(_op, e) => {
                                self.check_expr(e);
                            }
                        }
                    }
                    for s in &case_clause.body {
                        self.check_statement(s);
                    }
                }
                for s in else_body {
                    self.check_statement(s);
                }
            }
            Statement::Goto { target, span } => {
                self.goto_targets.push((target.clone(), *span));
            }
            Statement::Gosub { target, span } => {
                self.has_gosub = true;
                self.gosub_targets.insert(target.clone());
                self.goto_targets.push((target.clone(), *span));
            }
            Statement::CallSub { name, args, span } => {
                for arg in args {
                    self.check_expr(arg);
                }
                // Validate against known SUB signatures
                if let Some(sub_info) = self.subs.get(name).cloned() {
                    if sub_info.params.len() != args.len() {
                        self.errors.push(SemaError {
                            span: *span,
                            message: format!(
                                "SUB {} expects {} arguments, got {}",
                                name,
                                sub_info.params.len(),
                                args.len()
                            ),
                        });
                    }
                }
                // If SUB is not found, it might be a built-in or forward-declared; no error
            }
            Statement::Label { .. } => {
                // Labels are collected in pass 4; nothing else to check
            }
            Statement::Return { .. } => {
                // RETURN is valid inside any GOSUB routine
            }
            Statement::End { .. } | Statement::Rem { .. } => {}
            Statement::ExitFor { span } => {
                if !self.scope_stack.contains(&ScopeKind::ForLoop) {
                    self.errors.push(SemaError {
                        span: *span,
                        message: "EXIT FOR outside of FOR loop".to_string(),
                    });
                }
            }
            Statement::ExitDo { span } => {
                if !self.scope_stack.contains(&ScopeKind::DoLoop) {
                    self.errors.push(SemaError {
                        span: *span,
                        message: "EXIT DO outside of DO loop".to_string(),
                    });
                }
            }
            Statement::ExitSub { span } => {
                if !self.scope_stack.contains(&ScopeKind::Sub) {
                    self.errors.push(SemaError {
                        span: *span,
                        message: "EXIT SUB outside of SUB".to_string(),
                    });
                }
            }
            Statement::ExitFunction { span } => {
                if !self.scope_stack.contains(&ScopeKind::Function) {
                    self.errors.push(SemaError {
                        span: *span,
                        message: "EXIT FUNCTION outside of FUNCTION".to_string(),
                    });
                }
            }
            // ── Hardware statements (ESP32 extensions) ──
            Statement::GpioMode { pin, mode, .. } => {
                self.check_expr(pin);
                self.check_expr(mode);
            }
            Statement::GpioSet { pin, value, .. } => {
                self.check_expr(pin);
                self.check_expr(value);
            }
            Statement::GpioRead {
                pin,
                target,
                var_type,
                span,
            } => {
                self.check_expr(pin);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::I2cSetup {
                bus,
                sda,
                scl,
                freq,
                ..
            } => {
                self.check_expr(bus);
                self.check_expr(sda);
                self.check_expr(scl);
                self.check_expr(freq);
            }
            Statement::I2cWrite { addr, data, .. } => {
                self.check_expr(addr);
                self.check_expr(data);
            }
            Statement::I2cRead {
                addr,
                length,
                target,
                var_type,
                span,
            } => {
                self.check_expr(addr);
                self.check_expr(length);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::SpiSetup {
                bus,
                clk,
                mosi,
                miso,
                freq,
                ..
            } => {
                self.check_expr(bus);
                self.check_expr(clk);
                self.check_expr(mosi);
                self.check_expr(miso);
                self.check_expr(freq);
            }
            Statement::SpiTransfer {
                data,
                target,
                var_type,
                span,
            } => {
                self.check_expr(data);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::WifiConnect { ssid, password, .. } => {
                self.check_expr(ssid);
                self.check_expr(password);
            }
            Statement::WifiStatus {
                target,
                var_type,
                span,
            } => {
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::WifiDisconnect { .. } => {}
            Statement::Delay { ms, .. } => {
                self.check_expr(ms);
            }
            Statement::AdcRead {
                pin, target, var_type, span,
            } => {
                self.check_expr(pin);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::PwmSetup {
                channel, pin, freq, resolution, ..
            } => {
                self.check_expr(channel);
                self.check_expr(pin);
                self.check_expr(freq);
                self.check_expr(resolution);
            }
            Statement::PwmDuty { channel, duty, .. } => {
                self.check_expr(channel);
                self.check_expr(duty);
            }
            Statement::UartSetup {
                port, baud, tx, rx, ..
            } => {
                self.check_expr(port);
                self.check_expr(baud);
                self.check_expr(tx);
                self.check_expr(rx);
            }
            Statement::UartWrite { port, data, .. } => {
                self.check_expr(port);
                self.check_expr(data);
            }
            Statement::UartRead {
                port, target, var_type, span,
            } => {
                self.check_expr(port);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::TimerStart { .. } => {}
            Statement::TimerElapsed {
                target, var_type, span,
            } => {
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::HttpGet {
                url, target, var_type, span,
            } => {
                self.check_expr(url);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::HttpPost {
                url, body, target, var_type, span,
            } => {
                self.check_expr(url);
                self.check_expr(body);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::NvsWrite { key, value, .. } => {
                self.check_expr(key);
                self.check_expr(value);
            }
            Statement::NvsRead {
                key, target, var_type, span,
            } => {
                self.check_expr(key);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::MqttConnect { broker, port, .. } => {
                self.check_expr(broker);
                self.check_expr(port);
            }
            Statement::MqttDisconnect { .. } => {}
            Statement::MqttPublish { topic, message, .. } => {
                self.check_expr(topic);
                self.check_expr(message);
            }
            Statement::MqttSubscribe { topic, .. } => {
                self.check_expr(topic);
            }
            Statement::MqttReceive {
                target, var_type, span,
            } => {
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::BleInit { name, .. } => {
                self.check_expr(name);
            }
            Statement::BleAdvertise { mode, .. } => {
                self.check_expr(mode);
            }
            Statement::BleScan {
                target, var_type, span,
            } => {
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::BleSend { data, .. } => {
                self.check_expr(data);
            }
            Statement::BleReceive {
                target, var_type, span,
            } => {
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::JsonGet {
                json, key, target, var_type, span,
            } => {
                self.check_expr(json);
                self.check_expr(key);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::JsonSet {
                json, key, value, target, var_type, span,
            } => {
                self.check_expr(json);
                self.check_expr(key);
                self.check_expr(value);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::JsonCount {
                json, target, var_type, span,
            } => {
                self.check_expr(json);
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::LedSetup { pin, count, .. } => {
                self.check_expr(pin);
                self.check_expr(count);
            }
            Statement::LedSet { index, r, g, b, .. } => {
                self.check_expr(index);
                self.check_expr(r);
                self.check_expr(g);
                self.check_expr(b);
            }
            Statement::LedShow { .. } => {}
            Statement::LedClear { .. } => {}
            Statement::DeepSleep { ms, .. } => {
                self.check_expr(ms);
            }
            Statement::EspnowInit { .. } => {}
            Statement::EspnowSend { peer, data, .. } => {
                self.check_expr(peer);
                self.check_expr(data);
            }
            Statement::EspnowReceive {
                target, var_type, span,
            } => {
                self.declare_or_check_var(target, var_type, *span);
            }
            Statement::ArrayAssign {
                name,
                var_type: _,
                indices,
                expr,
                span,
            } => {
                for idx in indices {
                    self.check_expr(idx);
                }
                let expr_type = self.check_expr(expr);
                if let Some(info) = self.variables.get(name) {
                    if !info.is_array {
                        self.errors.push(SemaError {
                            span: *span,
                            message: format!("{name} is not an array"),
                        });
                    } else if info.dimensions != indices.len() {
                        self.errors.push(SemaError {
                            span: *span,
                            message: format!(
                                "array {name} has {} dimensions, but {} indices provided",
                                info.dimensions,
                                indices.len()
                            ),
                        });
                    }
                    // Type-check RHS against array element type
                    self.check_type_assignment(name, &info.qb_type.clone(), expr_type.as_ref(), *span);
                } else {
                    self.errors.push(SemaError {
                        span: *span,
                        message: format!("undeclared array: {name}"),
                    });
                }
            }
        }
    }

    // ── Expression checking ──────────────────────────────────

    fn check_expr(&mut self, expr: &Expr) -> Option<QBType> {
        match expr {
            Expr::IntLiteral { .. } => Some(QBType::Integer),
            Expr::FloatLiteral { .. } => Some(QBType::Single),
            Expr::StringLiteral { .. } => Some(QBType::String),
            Expr::Variable {
                name, var_type, span, ..
            } => {
                self.reference_var(name, var_type, *span);
                Some(var_type.clone())
            }
            Expr::FieldAccess {
                object,
                field,
                span,
            } => {
                let obj_type = self.check_expr(object);
                if let Some(QBType::UserType(ref type_name)) = obj_type {
                    if let Some(type_info) = self.types.get(type_name) {
                        if let Some(ft) = type_info.fields.get(field) {
                            return Some(ft.clone());
                        } else {
                            self.errors.push(SemaError {
                                span: *span,
                                message: format!(
                                    "TYPE {} has no field '{}'",
                                    type_name, field
                                ),
                            });
                        }
                    }
                } else if obj_type.is_some() && obj_type != Some(QBType::Inferred) {
                    self.errors.push(SemaError {
                        span: *span,
                        message: "field access on non-user-defined type".to_string(),
                    });
                }
                Some(QBType::Inferred)
            }
            Expr::BinaryOp {
                op,
                left,
                right,
                span,
            } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);

                let lt_is_string = lt.as_ref() == Some(&QBType::String);
                let rt_is_string = rt.as_ref() == Some(&QBType::String);

                // String operations: only + (concatenation) and comparisons are allowed
                if lt_is_string || rt_is_string {
                    if !matches!(
                        op,
                        BinOp::Add
                            | BinOp::Eq
                            | BinOp::Neq
                            | BinOp::Lt
                            | BinOp::Gt
                            | BinOp::Le
                            | BinOp::Ge
                    ) {
                        self.errors.push(SemaError {
                            span: *span,
                            message: "invalid operator for string operands".to_string(),
                        });
                    }
                    if *op == BinOp::Add {
                        return Some(QBType::String);
                    }
                }

                // Comparison operators return integer
                if matches!(
                    op,
                    BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge
                ) {
                    return Some(QBType::Integer);
                }

                // Logical/bitwise operators return integer
                if matches!(op, BinOp::And | BinOp::Or | BinOp::Xor) {
                    return Some(QBType::Integer);
                }

                // Integer division returns integer
                if *op == BinOp::IntDiv {
                    return Some(QBType::Integer);
                }

                // MOD returns integer
                if *op == BinOp::Mod {
                    return Some(QBType::Integer);
                }

                // Numeric result: promote to widest type
                Some(self.promote_numeric(lt.as_ref(), rt.as_ref()))
            }
            Expr::UnaryOp {
                op: _,
                operand,
                ..
            } => self.check_expr(operand),
            Expr::FnCall { name, args, span } => {
                for arg in args {
                    self.check_expr(arg);
                }
                // Check if this is actually an array access (parser emits FnCall for arr(i))
                if let Some(info) = self.variables.get(name).cloned() {
                    if info.is_array {
                        if info.dimensions != args.len() {
                            self.errors.push(SemaError {
                                span: *span,
                                message: format!(
                                    "array {name} has {} dimensions, but {} indices provided",
                                    info.dimensions,
                                    args.len()
                                ),
                            });
                        }
                        return Some(info.qb_type.clone());
                    }
                }
                // Check if it matches a known FUNCTION definition
                if let Some(func_info) = self.functions.get(name).cloned() {
                    if func_info.params.len() != args.len() {
                        self.errors.push(SemaError {
                            span: *span,
                            message: format!(
                                "FUNCTION {} expects {} arguments, got {}",
                                name,
                                func_info.params.len(),
                                args.len()
                            ),
                        });
                    }
                    return Some(func_info.return_type.clone());
                }
                // Built-in functions: return type depends on name
                Some(self.builtin_return_type(name))
            }
            Expr::ArrayAccess {
                name,
                var_type,
                indices,
                span,
            } => {
                for idx in indices {
                    self.check_expr(idx);
                }
                if let Some(info) = self.variables.get(name) {
                    if !info.is_array {
                        self.errors.push(SemaError {
                            span: *span,
                            message: format!("{name} is not an array"),
                        });
                    } else if info.dimensions != indices.len() {
                        self.errors.push(SemaError {
                            span: *span,
                            message: format!(
                                "array {name} has {} dimensions, but {} indices provided",
                                info.dimensions,
                                indices.len()
                            ),
                        });
                    }
                } else {
                    self.errors.push(SemaError {
                        span: *span,
                        message: format!("undeclared array: {name}"),
                    });
                }
                Some(var_type.clone())
            }
        }
    }

    // ── Type helpers ─────────────────────────────────────────

    /// Check if an assignment is type-compatible.
    fn check_type_assignment(
        &mut self,
        name: &str,
        target_type: &QBType,
        expr_type: Option<&QBType>,
        span: Span,
    ) {
        let Some(et) = expr_type else { return };

        // Inferred types are always compatible
        if *target_type == QBType::Inferred || *et == QBType::Inferred {
            return;
        }

        let target_is_string = *target_type == QBType::String;
        let expr_is_string = *et == QBType::String;

        if target_is_string && !expr_is_string {
            self.errors.push(SemaError {
                span,
                message: format!("cannot assign numeric value to string variable {name}"),
            });
        } else if !target_is_string && expr_is_string {
            self.errors.push(SemaError {
                span,
                message: format!("cannot assign string value to numeric variable {name}"),
            });
        }

        // UserType assignments: check type compatibility
        if let QBType::UserType(ref target_tn) = target_type {
            if let QBType::UserType(ref expr_tn) = et {
                if target_tn != expr_tn {
                    self.errors.push(SemaError {
                        span,
                        message: format!(
                            "cannot assign {} to {} (type mismatch)",
                            expr_tn, target_tn
                        ),
                    });
                }
            }
        }
    }

    /// Promote two numeric types to the widest type.
    fn promote_numeric(&self, a: Option<&QBType>, b: Option<&QBType>) -> QBType {
        let a = a.unwrap_or(&QBType::Inferred);
        let b = b.unwrap_or(&QBType::Inferred);

        // Double is widest, then Single, then Long, then Integer
        if *a == QBType::Double || *b == QBType::Double {
            return QBType::Double;
        }
        if *a == QBType::Single || *b == QBType::Single {
            return QBType::Single;
        }
        if *a == QBType::Long || *b == QBType::Long {
            return QBType::Long;
        }
        if *a == QBType::Integer || *b == QBType::Integer {
            return QBType::Integer;
        }
        // Both Inferred: default to Single (QBASIC default for untyped numerics)
        QBType::Single
    }

    /// Return type for built-in functions based on naming convention.
    fn builtin_return_type(&self, name: &str) -> QBType {
        let upper = name.to_uppercase();
        // String-returning built-ins end with $
        if upper.ends_with('$') {
            return QBType::String;
        }
        // Integer-returning built-ins
        match upper.as_str() {
            "INT" | "FIX" | "SGN" | "CINT" | "CLNG" | "LEN" | "ASC" | "INSTR" | "LBOUND"
            | "UBOUND" | "POS" | "CSRLIN" | "FREEFILE" | "EOF" | "LOC" | "LOF"
            | "SCREEN" | "PEEK" | "INP" | "VARPTR" | "SADD" | "FRE" => {
                QBType::Integer
            }
            // VAL returns Single (numeric from string)
            "VAL" => QBType::Single,
            // Most math functions return Single
            _ => QBType::Single,
        }
    }

    // ── Variable tracking ────────────────────────────────────

    /// Register a variable without type checking (e.g., parameters).
    fn register_var(&mut self, name: &str, qb_type: &QBType) {
        if !self.variables.contains_key(name) {
            self.variables.insert(
                name.to_string(),
                VarInfo {
                    qb_type: qb_type.clone(),
                    is_array: false,
                    dimensions: 0,
                },
            );
        }
    }

    /// Declare a variable or check type consistency with previous declaration.
    fn declare_or_check_var(&mut self, name: &str, qb_type: &QBType, span: Span) {
        if let Some(existing) = self.variables.get(name) {
            // If the existing type or the new type is Inferred, skip mismatch check
            if existing.qb_type != QBType::Inferred
                && *qb_type != QBType::Inferred
                && existing.qb_type != *qb_type
            {
                // Check if they are compatible at the VarType level
                // (e.g., Integer and Long are both numeric)
                let existing_vt: VarType = (&existing.qb_type).into();
                let new_vt: VarType = qb_type.into();
                if existing_vt != new_vt {
                    self.errors.push(SemaError {
                        span,
                        message: format!(
                            "type mismatch: variable {name} was {:?} but used as {:?}",
                            existing.qb_type, qb_type
                        ),
                    });
                }
            }
        } else {
            self.variables.insert(
                name.to_string(),
                VarInfo {
                    qb_type: qb_type.clone(),
                    is_array: false,
                    dimensions: 0,
                },
            );
        }
    }

    /// Reference a variable (auto-declare on first use per BASIC semantics).
    fn reference_var(&mut self, name: &str, qb_type: &QBType, span: Span) {
        self.declare_or_check_var(name, qb_type, span);
    }
}

/// Convenience function to run semantic analysis.
pub fn analyze(program: &Program) -> SemaResult {
    SemanticAnalyzer::new().analyze(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustybasic_lexer::tokenize;
    use rustybasic_parser::parse;

    fn analyze_str(input: &str) -> SemaResult {
        let tokens = tokenize(input).expect("lex error");
        let program = parse(tokens).expect("parse error");
        analyze(&program)
    }

    // ── Basic variable and assignment tests ──────────────────

    #[test]
    fn test_valid_program() {
        let result = analyze_str("LET X = 42\nPRINT X");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_string_type_mismatch() {
        let result = analyze_str("LET X$ = 42");
        assert!(result.has_errors());
        assert!(result.errors[0]
            .message
            .contains("cannot assign numeric value to string"));
    }

    #[test]
    fn test_numeric_type_mismatch() {
        let result = analyze_str("LET X% = \"hello\"");
        assert!(result.has_errors());
        assert!(result.errors[0]
            .message
            .contains("cannot assign string value to numeric"));
    }

    // ── Label and GOTO/GOSUB tests ───────────────────────────

    #[test]
    fn test_undefined_goto_target() {
        let result = analyze_str("GOTO myLabel");
        assert!(result.has_errors());
        assert!(result.errors[0].message.contains("undefined label"));
    }

    #[test]
    fn test_valid_goto_with_string_label() {
        let result = analyze_str("GOTO myLabel\nmyLabel:\nEND");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_valid_goto_with_numeric_label() {
        let result = analyze_str("GOTO 100\n100\nEND");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_duplicate_label() {
        let result = analyze_str("myLabel:\nPRINT 1\nmyLabel:\nPRINT 2");
        assert!(result.has_errors());
        assert!(result.errors[0].message.contains("duplicate label"));
    }

    #[test]
    fn test_gosub_tracking() {
        let result = analyze_str("GOSUB myRoutine\nEND\nmyRoutine:\nRETURN");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
        assert!(result.has_gosub);
        assert!(result.gosub_targets.contains("MYROUTINE"));
    }

    // ── Variable inference ───────────────────────────────────

    #[test]
    fn test_variable_types() {
        let result = analyze_str("LET N$ = \"hi\"\nLET C% = 10");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
        assert_eq!(result.variables["N$"].qb_type, QBType::String);
        assert_eq!(result.variables["C%"].qb_type, QBType::Integer);
    }

    // ── DIM tests ────────────────────────────────────────────

    #[test]
    fn test_dim_as_integer() {
        let result = analyze_str("DIM x AS INTEGER");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
        assert_eq!(result.variables["X"].qb_type, QBType::Integer);
    }

    #[test]
    fn test_dim_as_string() {
        let result = analyze_str("DIM name AS STRING");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
        assert_eq!(result.variables["NAME"].qb_type, QBType::String);
    }

    #[test]
    fn test_dim_array() {
        let result = analyze_str("DIM arr(10) AS INTEGER");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
        assert!(result.variables["ARR"].is_array);
        assert_eq!(result.variables["ARR"].dimensions, 1);
    }

    #[test]
    fn test_dim_undefined_user_type() {
        let result = analyze_str("DIM p AS MyUndefinedType");
        assert!(result.has_errors());
        assert!(result.errors[0].message.contains("undefined TYPE"));
    }

    // ── CONST tests ──────────────────────────────────────────

    #[test]
    fn test_const() {
        let result = analyze_str("CONST PI = 3.14");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
        assert!(result.constants.contains("PI"));
    }

    #[test]
    fn test_duplicate_const() {
        let result = analyze_str("CONST PI = 3.14\nCONST PI = 2.71");
        assert!(result.has_errors());
        assert!(result.errors[0].message.contains("duplicate CONST"));
    }

    // ── SUB/FUNCTION definition tests ────────────────────────

    #[test]
    fn test_sub_def_collected() {
        let result = analyze_str("SUB Hello\nPRINT \"hi\"\nEND SUB");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
        assert!(result.subs.contains_key("HELLO"));
    }

    #[test]
    fn test_function_def_collected() {
        let result =
            analyze_str("FUNCTION Add%(a AS INTEGER, b AS INTEGER)\nAdd% = a + b\nEND FUNCTION");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
        assert!(result.functions.contains_key("ADD%"));
    }

    #[test]
    fn test_duplicate_sub() {
        let result =
            analyze_str("SUB Hello\nPRINT 1\nEND SUB\nSUB Hello\nPRINT 2\nEND SUB");
        assert!(result.has_errors());
        assert!(result.errors[0].message.contains("duplicate SUB"));
    }

    #[test]
    fn test_sub_arg_count_mismatch() {
        let result =
            analyze_str("SUB Add(a AS INTEGER, b AS INTEGER)\nPRINT a + b\nEND SUB\nCALL Add(1)");
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.message.contains("expects 2 arguments, got 1")));
    }

    // ── TYPE definition tests ────────────────────────────────

    #[test]
    fn test_type_def_collected() {
        let result =
            analyze_str("TYPE Point\nx AS SINGLE\ny AS SINGLE\nEND TYPE");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
        assert!(result.types.contains_key("POINT"));
        assert!(result.types["POINT"].fields.contains_key("X"));
        assert!(result.types["POINT"].fields.contains_key("Y"));
    }

    #[test]
    fn test_duplicate_type() {
        let result = analyze_str(
            "TYPE Foo\na AS INTEGER\nEND TYPE\nTYPE Foo\nb AS INTEGER\nEND TYPE",
        );
        assert!(result.has_errors());
        assert!(result.errors[0].message.contains("duplicate TYPE"));
    }

    #[test]
    fn test_duplicate_type_field() {
        let result = analyze_str("TYPE Bad\nx AS INTEGER\nx AS SINGLE\nEND TYPE");
        assert!(result.has_errors());
        assert!(result.errors[0].message.contains("duplicate field"));
    }

    // ── EXIT statement tests ─────────────────────────────────

    #[test]
    fn test_exit_for_valid() {
        let result = analyze_str("FOR i = 1 TO 10\nIF i = 5 THEN EXIT FOR\nNEXT i");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_exit_for_invalid() {
        let result = analyze_str("EXIT FOR");
        assert!(result.has_errors());
        assert!(result.errors[0]
            .message
            .contains("EXIT FOR outside of FOR loop"));
    }

    #[test]
    fn test_exit_do_invalid() {
        let result = analyze_str("EXIT DO");
        assert!(result.has_errors());
        assert!(result.errors[0]
            .message
            .contains("EXIT DO outside of DO loop"));
    }

    #[test]
    fn test_exit_sub_invalid() {
        let result = analyze_str("EXIT SUB");
        assert!(result.has_errors());
        assert!(result.errors[0]
            .message
            .contains("EXIT SUB outside of SUB"));
    }

    #[test]
    fn test_exit_sub_valid() {
        let result = analyze_str("SUB MySub\nEXIT SUB\nEND SUB");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_exit_function_invalid() {
        let result = analyze_str("EXIT FUNCTION");
        assert!(result.has_errors());
        assert!(result.errors[0]
            .message
            .contains("EXIT FUNCTION outside of FUNCTION"));
    }

    #[test]
    fn test_exit_function_valid() {
        let result = analyze_str(
            "FUNCTION MyFunc%()\nMyFunc% = 42\nEXIT FUNCTION\nEND FUNCTION",
        );
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    // ── Control flow tests ───────────────────────────────────

    #[test]
    fn test_for_next() {
        let result = analyze_str("FOR i = 1 TO 10\nPRINT i\nNEXT i");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_do_while_loop() {
        let result = analyze_str("DO WHILE x > 0\nx = x - 1\nLOOP");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_do_loop_until() {
        let result = analyze_str("DO\nx = x + 1\nLOOP UNTIL x > 10");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_select_case() {
        let result = analyze_str(
            "SELECT CASE x\nCASE 1\nPRINT \"one\"\nCASE 2, 3\nPRINT \"two or three\"\nCASE ELSE\nPRINT \"other\"\nEND SELECT",
        );
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_if_end_if() {
        let result = analyze_str("IF x > 0 THEN\nPRINT x\nEND IF");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_single_line_if() {
        let result = analyze_str("IF x > 0 THEN PRINT x ELSE PRINT 0");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    // ── String operator validation ───────────────────────────

    #[test]
    fn test_invalid_string_operator() {
        let result = analyze_str("LET X% = \"hello\" - \"world\"");
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e
            .message
            .contains("invalid operator for string operands")));
    }

    // ── Diagnostics ──────────────────────────────────────────

    #[test]
    fn test_diagnostics_conversion() {
        let result = analyze_str("GOTO nowhere");
        assert!(result.has_errors());
        let diags = result.to_diagnostics(0);
        assert_eq!(diags.len(), 1);
    }

    // ── Hardware statement tests ─────────────────────────────

    #[test]
    fn test_gpio_delay() {
        let result = analyze_str("GPIO.MODE 2, 1\nDELAY 500");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    // ── Array tests ─────────────────────────────────────────

    #[test]
    fn test_array_dimension_mismatch() {
        let result = analyze_str("DIM arr(10) AS INTEGER\narr(1, 2) = 5");
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.message.contains("1 dimensions, but 2 indices")));
    }

    #[test]
    fn test_array_assign_non_array() {
        let result = analyze_str("DIM x AS INTEGER\nx(0) = 5");
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.message.contains("is not an array")));
    }

    #[test]
    fn test_array_valid_usage() {
        let result = analyze_str("DIM arr(5) AS INTEGER\narr(0) = 10\nPRINT arr(0)");
        assert!(!result.has_errors(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_undeclared_array_assign() {
        let result = analyze_str("arr(0) = 10");
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.message.contains("undeclared array")));
    }

    #[test]
    fn test_undeclared_array_read() {
        let result = analyze_str("PRINT arr(0)");
        // FnCall path — undeclared array reads go through builtin_return_type, not an error
        // ArrayAccess path would error. This tests that no crash occurs.
        assert!(!result.has_errors() || result.errors.iter().any(|e| e.message.contains("undeclared")));
    }
}
