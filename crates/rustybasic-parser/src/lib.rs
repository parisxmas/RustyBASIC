pub mod ast;

use ast::*;
use rustybasic_common::Span;
use rustybasic_lexer::{Token, TokenKind};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("{message}")]
pub struct ParseError {
    pub span: Span,
    pub message: String,
}

pub type ParseResult<T> = Result<T, ParseError>;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse a complete QBASIC program.
    /// Structure: TYPE defs, DECLARE stubs, module-level code, SUB/FUNCTION defs.
    pub fn parse_program(&mut self) -> ParseResult<Program> {
        let mut body = Vec::new();
        let mut subs = Vec::new();
        let mut functions = Vec::new();
        let mut types = Vec::new();
        let mut enums = Vec::new();
        let mut modules = Vec::new();
        let mut machines = Vec::new();

        self.skip_blank_lines();

        while !self.at_end() {
            match self.peek_kind().cloned() {
                Some(TokenKind::Sub) => subs.push(self.parse_sub_def()?),
                Some(TokenKind::Function) => functions.push(self.parse_function_def()?),
                Some(TokenKind::Declare) => {
                    self.advance(); // skip DECLARE — forward declarations are optional
                    // consume rest of line (SUB name(...) or FUNCTION name(...) AS type)
                    while !self.at_newline() && !self.at_end() {
                        self.advance();
                    }
                    self.eat_newline();
                }
                Some(TokenKind::Ident(ref name)) if name == "TYPE" => {
                    types.push(self.parse_type_def()?);
                }
                Some(TokenKind::Enum) => {
                    enums.push(self.parse_enum_def()?);
                }
                Some(TokenKind::Module) => {
                    modules.push(self.parse_module_def()?);
                }
                Some(TokenKind::Machine) => {
                    machines.push(self.parse_machine_def()?);
                }
                _ => {
                    if let Some(stmt) = self.parse_top_level_statement()? {
                        body.push(stmt);
                    }
                }
            }
            self.skip_blank_lines();
        }

        Ok(Program {
            body,
            subs,
            functions,
            types,
            enums,
            modules,
            machines,
        })
    }

    // ── TYPE...END TYPE ─────────────────────────────────────

    fn parse_type_def(&mut self) -> ParseResult<TypeDef> {
        let start = self.current_span();
        self.advance(); // consume TYPE identifier (we already checked it's "TYPE")
        let name = self.expect_ident_name()?;
        self.eat_newline();

        let mut fields = Vec::new();
        loop {
            self.skip_blank_lines();
            // Check for END TYPE
            if self.check_end_ident("TYPE") {
                self.advance(); // END
                self.advance(); // TYPE (Ident)
                break;
            }
            if self.at_end() {
                return Err(self.error("expected END TYPE"));
            }
            // Parse field: fieldName AS type
            let field_start = self.current_span();
            let field_name = self.expect_ident_name()?;
            self.expect(TokenKind::As)?;
            let field_type = self.parse_type_name()?;
            fields.push(TypeField {
                name: field_name,
                field_type,
                span: field_start.merge(self.prev_span()),
            });
            self.eat_newline();
        }

        Ok(TypeDef {
            name,
            fields,
            span: start.merge(self.prev_span()),
        })
    }

    // ── SUB definition ──────────────────────────────────────

    fn parse_sub_def(&mut self) -> ParseResult<SubDef> {
        let start = self.current_span();
        self.advance(); // SUB
        let is_static = false; // could check STATIC keyword
        let name = self.expect_ident_name()?;

        let params = if self.eat(TokenKind::LParen) {
            let p = self.parse_param_list()?;
            self.expect(TokenKind::RParen)?;
            p
        } else {
            Vec::new()
        };

        self.eat_newline();
        let body = self.parse_body_until_end_sub()?;
        self.expect_end_keyword(TokenKind::Sub, "END SUB")?;
        self.eat_newline();

        Ok(SubDef {
            name,
            params,
            body,
            is_static,
            span: start.merge(self.prev_span()),
        })
    }

    // ── FUNCTION definition ─────────────────────────────────

    fn parse_function_def(&mut self) -> ParseResult<FunctionDef> {
        let start = self.current_span();
        self.advance(); // FUNCTION
        let is_static = false;

        // Function name — may have suffix like Add%, Name$, etc.
        let (name, suffix_type) = self.expect_variable()?;

        let params = if self.eat(TokenKind::LParen) {
            let p = self.parse_param_list()?;
            self.expect(TokenKind::RParen)?;
            p
        } else {
            Vec::new()
        };

        // Optional: AS returnType
        let return_type = if self.eat(TokenKind::As) {
            self.parse_type_name()?
        } else {
            suffix_type
        };

        self.eat_newline();
        let body = self.parse_body_until_end_function()?;
        self.expect_end_keyword(TokenKind::Function, "END FUNCTION")?;
        self.eat_newline();

        Ok(FunctionDef {
            name,
            params,
            return_type,
            body,
            is_static,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_param_list(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();
        if self.check(TokenKind::RParen) {
            return Ok(params);
        }
        params.push(self.parse_param()?);
        while self.eat(TokenKind::Comma) {
            params.push(self.parse_param()?);
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> ParseResult<Param> {
        let span = self.current_span();
        let by_ref = if self.eat(TokenKind::ByVal) {
            false
        } else {
            self.eat(TokenKind::ByRef);
            true // default is BYREF in QBASIC
        };
        let (name, suffix_type) = self.expect_variable()?;
        let param_type = if self.eat(TokenKind::As) {
            self.parse_type_name()?
        } else {
            suffix_type
        };
        Ok(Param {
            name,
            param_type,
            by_ref,
            span: span.merge(self.prev_span()),
        })
    }

    fn parse_type_name(&mut self) -> ParseResult<QBType> {
        match self.peek_kind() {
            Some(TokenKind::IntegerType) => {
                self.advance();
                Ok(QBType::Integer)
            }
            Some(TokenKind::LongType) => {
                self.advance();
                Ok(QBType::Long)
            }
            Some(TokenKind::SingleType) => {
                self.advance();
                Ok(QBType::Single)
            }
            Some(TokenKind::DoubleType) => {
                self.advance();
                Ok(QBType::Double)
            }
            Some(TokenKind::StringType) => {
                self.advance();
                Ok(QBType::String)
            }
            Some(TokenKind::Function) => {
                self.advance();
                Ok(QBType::FunctionPtr)
            }
            Some(TokenKind::Ident(_)) => {
                let name = self.expect_ident_name()?;
                Ok(QBType::UserType(name))
            }
            _ => Err(self.error("expected type name (INTEGER, LONG, SINGLE, DOUBLE, STRING, or type name)")),
        }
    }

    // ── Body parsing helpers ────────────────────────────────

    fn parse_body_until_end_sub(&mut self) -> ParseResult<Vec<Statement>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::Sub) {
                break;
            }
            if let Some(stmt) = self.parse_top_level_statement()? {
                stmts.push(stmt);
            }
        }
        Ok(stmts)
    }

    fn parse_body_until_end_function(&mut self) -> ParseResult<Vec<Statement>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::Function) {
                break;
            }
            if let Some(stmt) = self.parse_top_level_statement()? {
                stmts.push(stmt);
            }
        }
        Ok(stmts)
    }

    /// Parse a top-level statement line. Returns None for blank/comment lines.
    fn parse_top_level_statement(&mut self) -> ParseResult<Option<Statement>> {
        if self.at_newline() {
            self.eat_newline();
            return Ok(None);
        }

        // Check for label: identifier followed by colon at start of line
        if let Some(TokenKind::Ident(_)) = self.peek_kind() {
            if self.peek_ahead_kind(1) == Some(&TokenKind::Colon) {
                let span = self.current_span();
                let name = self.expect_ident_name()?;
                self.advance(); // consume ':'
                self.eat_newline();
                return Ok(Some(Statement::Label {
                    name,
                    span: span.merge(self.prev_span()),
                }));
            }
        }

        // Check for numeric line label (legacy support)
        if let Some(TokenKind::IntLiteral(n)) = self.peek_kind() {
            let n = *n;
            let span = self.current_span();
            self.advance();
            let label_stmt = Statement::Label {
                name: n.to_string(),
                span,
            };
            if self.at_newline() || self.at_end() {
                self.eat_newline();
                return Ok(Some(label_stmt));
            }
            let stmt = self.parse_statement()?;
            self.eat_newline();
            return Ok(Some(stmt));
        }

        let stmt = self.parse_statement()?;
        self.eat_newline();
        Ok(Some(stmt))
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        let kind = self.peek_kind().cloned();
        match kind {
            Some(TokenKind::Rem) | Some(TokenKind::Comment) => self.parse_rem(),
            Some(TokenKind::Let) => self.parse_let(),
            Some(TokenKind::Print) => self.parse_print(),
            Some(TokenKind::Input) => self.parse_input(),
            Some(TokenKind::If) => self.parse_if(),
            Some(TokenKind::For) => self.parse_for(),
            Some(TokenKind::Do) => self.parse_do_loop(),
            Some(TokenKind::While) => self.parse_while(),
            // SELECT CASE → Select token followed by Case token
            Some(TokenKind::Select) if self.peek_ahead_kind(1) == Some(&TokenKind::Case) => {
                self.parse_select_case()
            }
            Some(TokenKind::Goto) => self.parse_goto(),
            Some(TokenKind::Gosub) => self.parse_gosub(),
            Some(TokenKind::Return) => self.parse_return(),
            Some(TokenKind::End) => self.parse_end(),
            Some(TokenKind::Dim) | Some(TokenKind::ReDim) => self.parse_dim(),
            Some(TokenKind::Const) => self.parse_const(),
            Some(TokenKind::Call) => self.parse_call(),
            // EXIT FOR/DO/SUB/FUNCTION → Exit token followed by keyword
            Some(TokenKind::Exit) => self.parse_exit(),
            // LINE INPUT → Ident("LINE") followed by Input
            Some(TokenKind::Ident(ref name))
                if name == "LINE"
                    && self.peek_ahead_kind(1) == Some(&TokenKind::Input) =>
            {
                self.parse_line_input()
            }
            // Hardware statements
            Some(TokenKind::GpioMode) => self.parse_gpio_mode(),
            Some(TokenKind::GpioSet) => self.parse_gpio_set(),
            Some(TokenKind::GpioRead) => self.parse_gpio_read(),
            Some(TokenKind::I2cSetup) => self.parse_i2c_setup(),
            Some(TokenKind::I2cWrite) => self.parse_i2c_write(),
            Some(TokenKind::I2cRead) => self.parse_i2c_read(),
            Some(TokenKind::SpiSetup) => self.parse_spi_setup(),
            Some(TokenKind::SpiTransfer) => self.parse_spi_transfer(),
            Some(TokenKind::WifiConnect) => self.parse_wifi_connect(),
            Some(TokenKind::WifiStatus) => self.parse_wifi_status(),
            Some(TokenKind::WifiDisconnect) => self.parse_wifi_disconnect(),
            Some(TokenKind::Delay) => self.parse_delay(),
            Some(TokenKind::AdcRead) => self.parse_adc_read(),
            Some(TokenKind::PwmSetup) => self.parse_pwm_setup(),
            Some(TokenKind::PwmDuty) => self.parse_pwm_duty(),
            Some(TokenKind::UartSetup) => self.parse_uart_setup(),
            Some(TokenKind::UartWrite) => self.parse_uart_write(),
            Some(TokenKind::UartRead) => self.parse_uart_read(),
            Some(TokenKind::TimerStart) => self.parse_timer_start(),
            Some(TokenKind::TimerElapsed) => self.parse_timer_elapsed(),
            Some(TokenKind::HttpGet) => self.parse_http_get(),
            Some(TokenKind::HttpPost) => self.parse_http_post(),
            Some(TokenKind::NvsWrite) => self.parse_nvs_write(),
            Some(TokenKind::NvsRead) => self.parse_nvs_read(),
            Some(TokenKind::MqttConnect) => self.parse_mqtt_connect(),
            Some(TokenKind::MqttDisconnect) => self.parse_mqtt_disconnect(),
            Some(TokenKind::MqttPublish) => self.parse_mqtt_publish(),
            Some(TokenKind::MqttSubscribe) => self.parse_mqtt_subscribe(),
            Some(TokenKind::MqttReceive) => self.parse_mqtt_receive(),
            Some(TokenKind::BleInit) => self.parse_ble_init(),
            Some(TokenKind::BleAdvertise) => self.parse_ble_advertise(),
            Some(TokenKind::BleScan) => self.parse_ble_scan(),
            Some(TokenKind::BleSend) => self.parse_ble_send(),
            Some(TokenKind::BleReceive) => self.parse_ble_receive(),
            Some(TokenKind::JsonGet) => self.parse_json_get(),
            Some(TokenKind::JsonSet) => self.parse_json_set(),
            Some(TokenKind::JsonCount) => self.parse_json_count(),
            Some(TokenKind::LedSetup) => self.parse_led_setup(),
            Some(TokenKind::LedSet) => self.parse_led_set(),
            Some(TokenKind::LedShow) => self.parse_led_show(),
            Some(TokenKind::LedClear) => self.parse_led_clear(),
            Some(TokenKind::DeepSleep) => self.parse_deepsleep(),
            Some(TokenKind::EspnowInit) => self.parse_espnow_init(),
            Some(TokenKind::EspnowSend) => self.parse_espnow_send(),
            Some(TokenKind::EspnowReceive) => self.parse_espnow_receive(),
            Some(TokenKind::Data) => self.parse_data(),
            Some(TokenKind::Read) => self.parse_read(),
            Some(TokenKind::Restore) => self.parse_restore(),
            Some(TokenKind::On) => self.parse_on(),
            Some(TokenKind::Swap) => self.parse_swap(),
            Some(TokenKind::Def) => self.parse_def_fn(),
            Some(TokenKind::Randomize) => self.parse_randomize(),
            // PRINT USING is handled inside parse_print via lookahead
            Some(TokenKind::TouchRead) => self.parse_touch_read(),
            Some(TokenKind::ServoAttach) => self.parse_servo_attach(),
            Some(TokenKind::ServoWrite) => self.parse_servo_write(),
            Some(TokenKind::Tone) => self.parse_tone(),
            Some(TokenKind::IrqAttach) => self.parse_irq_attach(),
            Some(TokenKind::IrqDetach) => self.parse_irq_detach(),
            Some(TokenKind::TempRead) => self.parse_temp_read(),
            Some(TokenKind::OtaUpdate) => self.parse_ota_update(),
            Some(TokenKind::OledInit) => self.parse_oled_init(),
            Some(TokenKind::OledPrint) => self.parse_oled_print(),
            Some(TokenKind::OledPixel) => self.parse_oled_pixel(),
            Some(TokenKind::OledLine) => self.parse_oled_line(),
            Some(TokenKind::OledClear) => self.parse_oled_clear(),
            Some(TokenKind::OledShow) => self.parse_oled_show(),
            Some(TokenKind::LcdInit) => self.parse_lcd_init(),
            Some(TokenKind::LcdPrint) => self.parse_lcd_print(),
            Some(TokenKind::LcdClear) => self.parse_lcd_clear(),
            Some(TokenKind::LcdPos) => self.parse_lcd_pos(),
            Some(TokenKind::UdpInit) => self.parse_udp_init(),
            Some(TokenKind::UdpSend) => self.parse_udp_send(),
            Some(TokenKind::UdpReceive) => self.parse_udp_receive(),
            Some(TokenKind::Assert) => self.parse_assert(),
            Some(TokenKind::Try) => self.parse_try_catch(),
            Some(TokenKind::Task) => self.parse_task_stmt(),
            // Implicit LET or SUB call: identifier ...
            Some(
                TokenKind::Ident(_)
                | TokenKind::IntIdent(_)
                | TokenKind::StringIdent(_)
                | TokenKind::LongIdent(_)
                | TokenKind::SingleIdent(_)
                | TokenKind::DoubleIdent(_),
            ) => self.parse_ident_statement(),
            _ => Err(self.error("expected statement")),
        }
    }

    /// Parse EXIT FOR / EXIT DO / EXIT SUB / EXIT FUNCTION
    fn parse_exit(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance(); // EXIT
        let end_span = self.current_span();
        match self.peek_kind() {
            Some(TokenKind::For) => {
                self.advance();
                Ok(Statement::ExitFor {
                    span: span.merge(end_span),
                })
            }
            Some(TokenKind::Do) => {
                self.advance();
                Ok(Statement::ExitDo {
                    span: span.merge(end_span),
                })
            }
            Some(TokenKind::Sub) => {
                self.advance();
                Ok(Statement::ExitSub {
                    span: span.merge(end_span),
                })
            }
            Some(TokenKind::Function) => {
                self.advance();
                Ok(Statement::ExitFunction {
                    span: span.merge(end_span),
                })
            }
            _ => Err(self.error("expected FOR, DO, SUB, or FUNCTION after EXIT")),
        }
    }

    /// Parse an identifier-leading statement: assignment, array assign, field assign, or sub call.
    fn parse_ident_statement(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        let (name, var_type) = self.expect_variable()?;

        // Array element assignment or sub call with parens: name(args...) = expr  OR  name(args...)
        if self.eat(TokenKind::LParen) {
            let mut indices = Vec::new();
            if !self.check(TokenKind::RParen) {
                indices.push(self.parse_expr()?);
                while self.eat(TokenKind::Comma) {
                    indices.push(self.parse_expr()?);
                }
            }
            self.expect(TokenKind::RParen)?;

            if self.eat(TokenKind::Eq) {
                // Array element assignment: arr(i, j) = expr
                let expr = self.parse_expr()?;
                let span = start.merge(expr.span());
                return Ok(Statement::ArrayAssign {
                    name,
                    var_type,
                    indices,
                    expr,
                    span,
                });
            }

            // Otherwise it's a SUB call with parens: SubName(args...)
            let span = start.merge(self.prev_span());
            return Ok(Statement::CallSub {
                name,
                args: indices,
                span,
            });
        }

        // Check for MachineName.EVENT expr pattern
        if name.ends_with(".EVENT") {
            let machine_name = name[..name.len() - 6].to_string(); // strip ".EVENT"
            let event = self.parse_expr()?;
            let span = start.merge(self.prev_span());
            return Ok(Statement::MachineEvent {
                machine_name,
                event,
                span,
            });
        }

        // Scalar assignment: name = expr
        if self.eat(TokenKind::Eq) {
            let expr = self.parse_expr()?;
            let span = start.merge(expr.span());
            return Ok(Statement::Let {
                name,
                var_type,
                expr,
                span,
            });
        }

        // SUB call without CALL: SubName arg1, arg2
        let mut args = Vec::new();
        if !self.at_newline() && !self.at_end() && !self.check(TokenKind::Colon) {
            args.push(self.parse_expr()?);
            while self.eat(TokenKind::Comma) {
                args.push(self.parse_expr()?);
            }
        }
        let span = start.merge(self.prev_span());
        Ok(Statement::CallSub { name, args, span })
    }

    fn parse_rem(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::Rem { span })
    }

    fn parse_let(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // LET
        let (name, var_type) = self.expect_variable()?;
        self.expect(TokenKind::Eq)?;
        let expr = self.parse_expr()?;
        let span = start.merge(expr.span());
        Ok(Statement::Let {
            name,
            var_type,
            expr,
            span,
        })
    }

    fn parse_print(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // PRINT

        // Check for PRINT USING
        if self.eat(TokenKind::Using) {
            let format = self.parse_expr()?;
            self.expect(TokenKind::Semicolon)?;
            let mut items = Vec::new();
            items.push(self.parse_expr()?);
            while self.eat(TokenKind::Semicolon) || self.eat(TokenKind::Comma) {
                if self.at_newline() || self.at_end() {
                    break;
                }
                items.push(self.parse_expr()?);
            }
            return Ok(Statement::PrintUsing {
                format,
                items,
                span: start.merge(self.prev_span()),
            });
        }

        let mut items = Vec::new();
        while !self.at_newline() && !self.at_end() && !self.check(TokenKind::Colon)
            && !self.check(TokenKind::Else)
        {
            if self.eat(TokenKind::Semicolon) {
                items.push(PrintItem::Semicolon);
            } else if self.eat(TokenKind::Comma) {
                items.push(PrintItem::Comma);
            } else {
                items.push(PrintItem::Expr(self.parse_expr()?));
            }
        }
        let end = if let Some(PrintItem::Expr(e)) = items.last() {
            e.span()
        } else {
            start
        };
        Ok(Statement::Print {
            items,
            span: start.merge(end),
        })
    }

    fn parse_input(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // INPUT
        let prompt = if let Some(TokenKind::StringLiteral(_)) = self.peek_kind() {
            if let TokenKind::StringLiteral(s) = self.advance_and_get() {
                let _ = self.eat(TokenKind::Semicolon) || self.eat(TokenKind::Comma);
                Some(s)
            } else {
                None
            }
        } else {
            None
        };
        let (name, var_type) = self.expect_variable()?;
        let span = start.merge(self.prev_span());
        Ok(Statement::Input {
            prompt,
            name,
            var_type,
            span,
        })
    }

    fn parse_line_input(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // LINE (Ident)
        self.advance(); // INPUT
        let prompt = if let Some(TokenKind::StringLiteral(_)) = self.peek_kind() {
            if let TokenKind::StringLiteral(s) = self.advance_and_get() {
                let _ = self.eat(TokenKind::Semicolon) || self.eat(TokenKind::Comma);
                Some(s)
            } else {
                None
            }
        } else {
            None
        };
        let name = self.expect_ident_name()?;
        let span = start.merge(self.prev_span());
        Ok(Statement::LineInput {
            prompt,
            name,
            span,
        })
    }

    fn parse_if(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // IF
        let condition = self.parse_expr()?;
        self.expect(TokenKind::Then)?;

        // Single-line IF: IF cond THEN stmt [ELSE stmt]
        if !self.at_newline() && !self.at_end() {
            let then_stmt = self.parse_statement()?;
            let else_body = if self.eat(TokenKind::Else) {
                vec![self.parse_statement()?]
            } else {
                Vec::new()
            };
            let span = start.merge(self.prev_span());
            return Ok(Statement::If {
                condition,
                then_body: vec![then_stmt],
                else_if_clauses: Vec::new(),
                else_body,
                span,
            });
        }

        // Multi-line IF...END IF
        self.eat_newline();
        let then_body = self.parse_if_block()?;

        let mut else_if_clauses = Vec::new();
        while self.eat(TokenKind::ElseIf) {
            let ei_start = self.prev_span();
            let ei_cond = self.parse_expr()?;
            self.expect(TokenKind::Then)?;
            self.eat_newline();
            let ei_body = self.parse_if_block()?;
            else_if_clauses.push(ElseIfClause {
                condition: ei_cond,
                body: ei_body,
                span: ei_start.merge(self.prev_span()),
            });
        }

        let else_body = if self.eat(TokenKind::Else) {
            self.eat_newline();
            self.parse_block_until_end_if()?
        } else {
            Vec::new()
        };

        self.expect_end_keyword(TokenKind::If, "END IF")?;
        let span = start.merge(self.prev_span());
        Ok(Statement::If {
            condition,
            then_body,
            else_if_clauses,
            else_body,
            span,
        })
    }

    /// Parse a block inside an IF — terminates at ELSE, ELSEIF, or END IF.
    fn parse_if_block(&mut self) -> ParseResult<Vec<Statement>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() {
                break;
            }
            // Terminates at ELSE, ELSEIF, or END IF
            match self.peek_kind() {
                Some(TokenKind::Else) | Some(TokenKind::ElseIf) => break,
                Some(TokenKind::End) if self.check_end_keyword_ahead(TokenKind::If) => break,
                _ => {}
            }
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.eat_newline();
        }
        Ok(stmts)
    }

    /// Parse a block that terminates at END IF only (used after ELSE).
    fn parse_block_until_end_if(&mut self) -> ParseResult<Vec<Statement>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::If) {
                break;
            }
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.eat_newline();
        }
        Ok(stmts)
    }

    fn parse_for(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // FOR

        // Check for FOR EACH
        if self.eat(TokenKind::Each) {
            return self.parse_for_each(start);
        }

        let var = self.expect_ident_name()?;
        self.expect(TokenKind::Eq)?;
        let from = self.parse_expr()?;
        self.expect(TokenKind::To)?;
        let to = self.parse_expr()?;
        let step = if self.eat(TokenKind::Step) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.eat_newline();
        let body = self.parse_block(&[TokenKind::Next])?;
        self.expect(TokenKind::Next)?;
        // Optional variable name after NEXT
        if matches!(self.peek_kind(), Some(TokenKind::Ident(_))) {
            self.advance();
        }
        let span = start.merge(self.prev_span());
        Ok(Statement::For {
            var,
            from,
            to,
            step,
            body,
            span,
        })
    }

    fn parse_do_loop(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // DO

        // Pre-condition: DO WHILE cond / DO UNTIL cond
        let pre_condition = if self.eat(TokenKind::While) {
            Some(DoCondition {
                is_while: true,
                expr: self.parse_expr()?,
            })
        } else if self.eat(TokenKind::Until) {
            Some(DoCondition {
                is_while: false,
                expr: self.parse_expr()?,
            })
        } else {
            None
        };

        self.eat_newline();
        let body = self.parse_block(&[TokenKind::Loop])?;
        self.expect(TokenKind::Loop)?;

        // Post-condition: LOOP WHILE cond / LOOP UNTIL cond
        let post_condition = if self.eat(TokenKind::While) {
            Some(DoCondition {
                is_while: true,
                expr: self.parse_expr()?,
            })
        } else if self.eat(TokenKind::Until) {
            Some(DoCondition {
                is_while: false,
                expr: self.parse_expr()?,
            })
        } else {
            None
        };

        let span = start.merge(self.prev_span());
        Ok(Statement::DoLoop {
            pre_condition,
            post_condition,
            body,
            span,
        })
    }

    fn parse_while(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // WHILE
        let condition = self.parse_expr()?;
        self.eat_newline();
        let body = self.parse_block(&[TokenKind::Wend])?;
        self.expect(TokenKind::Wend)?;
        let span = start.merge(self.prev_span());
        Ok(Statement::While {
            condition,
            body,
            span,
        })
    }

    fn parse_select_case(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // SELECT
        self.advance(); // CASE
        let expr = self.parse_expr()?;
        self.eat_newline();

        let mut cases = Vec::new();
        let mut else_body = Vec::new();

        loop {
            self.skip_blank_lines();
            // Check for END SELECT
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::Select) {
                break;
            }
            // Check for CASE ELSE (Case followed by Else)
            if self.check_case_else() {
                self.advance(); // CASE
                self.advance(); // ELSE
                self.eat_newline();
                else_body = self.parse_block_until_end_select()?;
                break;
            }
            if self.eat(TokenKind::Case) {
                let case_start = self.prev_span();
                let tests = self.parse_case_tests()?;
                self.eat_newline();
                let body = self.parse_block_until_case_or_end()?;
                cases.push(CaseClause {
                    tests,
                    body,
                    span: case_start.merge(self.prev_span()),
                });
            } else {
                return Err(self.error("expected CASE or END SELECT"));
            }
        }

        self.expect_end_keyword(TokenKind::Select, "END SELECT")?;
        let span = start.merge(self.prev_span());
        Ok(Statement::SelectCase {
            expr,
            cases,
            else_body,
            span,
        })
    }

    fn parse_case_tests(&mut self) -> ParseResult<Vec<CaseTest>> {
        let mut tests = Vec::new();
        tests.push(self.parse_one_case_test()?);
        while self.eat(TokenKind::Comma) {
            tests.push(self.parse_one_case_test()?);
        }
        Ok(tests)
    }

    fn parse_one_case_test(&mut self) -> ParseResult<CaseTest> {
        // CASE IS <op> expr — check for Ident("IS") followed by comparison op
        if self.check_ident("IS") {
            if let Some(op) = self.peek_comparison_op_at(1) {
                self.advance(); // IS
                self.advance(); // comparison op
                let expr = self.parse_expr()?;
                return Ok(CaseTest::Is(op, expr));
            }
        }
        // Check for comparison operator directly (like CASE > 10)
        if let Some(op) = self.peek_comparison_op() {
            self.advance();
            let expr = self.parse_expr()?;
            return Ok(CaseTest::Is(op, expr));
        }
        // CASE expr [TO expr]
        let expr = self.parse_expr()?;
        if self.eat(TokenKind::To) {
            let end = self.parse_expr()?;
            Ok(CaseTest::Range(expr, end))
        } else {
            Ok(CaseTest::Value(expr))
        }
    }

    fn peek_comparison_op(&self) -> Option<BinOp> {
        match self.peek_kind() {
            Some(TokenKind::Lt) => Some(BinOp::Lt),
            Some(TokenKind::Gt) => Some(BinOp::Gt),
            Some(TokenKind::Le) => Some(BinOp::Le),
            Some(TokenKind::Ge) => Some(BinOp::Ge),
            Some(TokenKind::Neq) => Some(BinOp::Neq),
            _ => None,
        }
    }

    fn peek_comparison_op_at(&self, offset: usize) -> Option<BinOp> {
        match self.peek_ahead_kind(offset) {
            Some(TokenKind::Lt) => Some(BinOp::Lt),
            Some(TokenKind::Gt) => Some(BinOp::Gt),
            Some(TokenKind::Le) => Some(BinOp::Le),
            Some(TokenKind::Ge) => Some(BinOp::Ge),
            Some(TokenKind::Neq) => Some(BinOp::Neq),
            Some(TokenKind::Eq) => Some(BinOp::Eq),
            _ => None,
        }
    }

    fn parse_block_until_case_or_end(&mut self) -> ParseResult<Vec<Statement>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() {
                break;
            }
            // Terminate at CASE, CASE ELSE, or END SELECT
            match self.peek_kind() {
                Some(TokenKind::Case) => break,
                Some(TokenKind::End) if self.check_end_keyword_ahead(TokenKind::Select) => break,
                _ => {}
            }
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.eat_newline();
        }
        Ok(stmts)
    }

    fn parse_block_until_end_select(&mut self) -> ParseResult<Vec<Statement>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::Select) {
                break;
            }
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.eat_newline();
        }
        Ok(stmts)
    }

    fn parse_goto(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // GOTO
        let target = self.expect_label_target()?;
        Ok(Statement::Goto {
            target,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_gosub(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // GOSUB
        let target = self.expect_label_target()?;
        Ok(Statement::Gosub {
            target,
            span: start.merge(self.prev_span()),
        })
    }

    fn expect_label_target(&mut self) -> ParseResult<String> {
        match self.peek_kind().cloned() {
            Some(TokenKind::IntLiteral(n)) => {
                self.advance();
                Ok(n.to_string())
            }
            Some(TokenKind::Ident(name)) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("expected label name or line number")),
        }
    }

    fn parse_return(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::Return { span })
    }

    fn parse_end(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::End { span })
    }

    fn parse_dim(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        let is_redim = matches!(self.peek_kind(), Some(TokenKind::ReDim));
        self.advance(); // DIM or REDIM

        let is_shared = self.eat(TokenKind::Shared);

        let (name, suffix_type) = self.expect_variable()?;

        // Optional array dimensions
        let dimensions = if self.eat(TokenKind::LParen) {
            let mut dims = vec![self.parse_expr()?];
            while self.eat(TokenKind::Comma) {
                dims.push(self.parse_expr()?);
            }
            self.expect(TokenKind::RParen)?;
            dims
        } else {
            Vec::new()
        };

        // Optional AS type
        let var_type = if self.eat(TokenKind::As) {
            self.parse_type_name()?
        } else {
            suffix_type
        };

        let span = start.merge(self.prev_span());
        let _ = is_redim;
        Ok(Statement::Dim {
            name,
            var_type,
            dimensions,
            is_shared,
            span,
        })
    }

    fn parse_const(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // CONST
        let name = self.expect_ident_name()?;
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        let span = start.merge(self.prev_span());
        Ok(Statement::Const { name, value, span })
    }

    fn parse_call(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // CALL
        let name = self.expect_ident_name()?;
        let args = if self.eat(TokenKind::LParen) {
            let mut a = Vec::new();
            if !self.check(TokenKind::RParen) {
                a.push(self.parse_expr()?);
                while self.eat(TokenKind::Comma) {
                    a.push(self.parse_expr()?);
                }
            }
            self.expect(TokenKind::RParen)?;
            a
        } else {
            Vec::new()
        };
        let span = start.merge(self.prev_span());
        Ok(Statement::CallSub { name, args, span })
    }

    // ── Hardware statements ──────────────────────────────────

    fn parse_gpio_mode(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let pin = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let mode = self.parse_expr()?;
        Ok(Statement::GpioMode {
            pin,
            mode,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_gpio_set(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let pin = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let value = self.parse_expr()?;
        Ok(Statement::GpioSet {
            pin,
            value,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_gpio_read(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let pin = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::GpioRead {
            pin,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_i2c_setup(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let bus = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let sda = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let scl = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let freq = self.parse_expr()?;
        Ok(Statement::I2cSetup {
            bus,
            sda,
            scl,
            freq,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_i2c_write(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let addr = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let data = self.parse_expr()?;
        Ok(Statement::I2cWrite {
            addr,
            data,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_i2c_read(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let addr = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let length = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::I2cRead {
            addr,
            length,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_spi_setup(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let bus = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let clk = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let mosi = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let miso = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let freq = self.parse_expr()?;
        Ok(Statement::SpiSetup {
            bus,
            clk,
            mosi,
            miso,
            freq,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_spi_transfer(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let data = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::SpiTransfer {
            data,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_wifi_connect(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let ssid = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let password = self.parse_expr()?;
        Ok(Statement::WifiConnect {
            ssid,
            password,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_wifi_status(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::WifiStatus {
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_wifi_disconnect(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::WifiDisconnect { span })
    }

    fn parse_delay(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let ms = self.parse_expr()?;
        Ok(Statement::Delay {
            ms,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_adc_read(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let pin = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::AdcRead {
            pin,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_pwm_setup(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let channel = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let pin = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let freq = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let resolution = self.parse_expr()?;
        Ok(Statement::PwmSetup {
            channel,
            pin,
            freq,
            resolution,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_pwm_duty(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let channel = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let duty = self.parse_expr()?;
        Ok(Statement::PwmDuty {
            channel,
            duty,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_uart_setup(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let port = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let baud = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let tx = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let rx = self.parse_expr()?;
        Ok(Statement::UartSetup {
            port,
            baud,
            tx,
            rx,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_uart_write(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let port = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let data = self.parse_expr()?;
        Ok(Statement::UartWrite {
            port,
            data,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_uart_read(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let port = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::UartRead {
            port,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_timer_start(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::TimerStart { span })
    }

    fn parse_timer_elapsed(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::TimerElapsed {
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_http_get(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let url = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::HttpGet {
            url,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_http_post(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let url = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let body = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::HttpPost {
            url,
            body,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_nvs_write(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let key = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let value = self.parse_expr()?;
        Ok(Statement::NvsWrite {
            key,
            value,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_nvs_read(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let key = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::NvsRead {
            key,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_mqtt_connect(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let broker = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let port = self.parse_expr()?;
        Ok(Statement::MqttConnect {
            broker,
            port,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_mqtt_disconnect(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::MqttDisconnect { span })
    }

    fn parse_mqtt_publish(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let topic = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let message = self.parse_expr()?;
        Ok(Statement::MqttPublish {
            topic,
            message,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_mqtt_subscribe(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let topic = self.parse_expr()?;
        Ok(Statement::MqttSubscribe {
            topic,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_mqtt_receive(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::MqttReceive {
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_ble_init(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let name = self.parse_expr()?;
        Ok(Statement::BleInit {
            name,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_ble_advertise(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let mode = self.parse_expr()?;
        Ok(Statement::BleAdvertise {
            mode,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_ble_scan(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::BleScan {
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_ble_send(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let data = self.parse_expr()?;
        Ok(Statement::BleSend {
            data,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_ble_receive(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::BleReceive {
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_json_get(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let json = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let key = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::JsonGet {
            json,
            key,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_json_set(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let json = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let key = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let value = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::JsonSet {
            json,
            key,
            value,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_json_count(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let json = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::JsonCount {
            json,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_led_setup(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let pin = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let count = self.parse_expr()?;
        Ok(Statement::LedSetup {
            pin,
            count,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_led_set(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let index = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let r = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let g = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let b = self.parse_expr()?;
        Ok(Statement::LedSet {
            index,
            r,
            g,
            b,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_led_show(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::LedShow { span })
    }

    fn parse_led_clear(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::LedClear { span })
    }

    fn parse_deepsleep(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let ms = self.parse_expr()?;
        Ok(Statement::DeepSleep {
            ms,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_espnow_init(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::EspnowInit { span })
    }

    fn parse_espnow_send(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let peer = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let data = self.parse_expr()?;
        Ok(Statement::EspnowSend {
            peer,
            data,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_espnow_receive(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::EspnowReceive {
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    // ── DATA/READ/RESTORE ────────────────────────────────────

    fn parse_data(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // DATA
        let mut items = Vec::new();
        items.push(self.parse_data_item()?);
        while self.eat(TokenKind::Comma) {
            items.push(self.parse_data_item()?);
        }
        Ok(Statement::Data {
            items,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_data_item(&mut self) -> ParseResult<DataItem> {
        // Optional minus prefix for negative numeric literals
        let negative = self.eat(TokenKind::Minus);
        match self.peek_kind().cloned() {
            Some(TokenKind::IntLiteral(v)) => {
                self.advance();
                Ok(DataItem::Int(if negative { -v } else { v }))
            }
            Some(TokenKind::FloatLiteral(v)) => {
                self.advance();
                Ok(DataItem::Float(if negative { -v } else { v }))
            }
            Some(TokenKind::StringLiteral(s)) if !negative => {
                self.advance();
                Ok(DataItem::Str(s))
            }
            _ => Err(self.error("expected integer, float, or string literal in DATA")),
        }
    }

    fn parse_read(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // READ
        let mut variables = Vec::new();
        let (name, var_type) = self.expect_variable()?;
        variables.push((name, var_type));
        while self.eat(TokenKind::Comma) {
            let (name, var_type) = self.expect_variable()?;
            variables.push((name, var_type));
        }
        Ok(Statement::Read {
            variables,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_restore(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance(); // RESTORE
        Ok(Statement::Restore { span })
    }

    // ── Classic BASIC extensions ──────────────────────────────

    /// Parse ON ... GOTO / ON ... GOSUB / ON ERROR GOTO / ON GPIO.CHANGE / ON TIMER / ON MQTT.MESSAGE
    fn parse_on(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // ON

        // ON GPIO.CHANGE pin GOSUB label
        if self.check_ident("GPIO.CHANGE") {
            self.advance(); // GPIO.CHANGE
            let pin = self.parse_expr()?;
            self.expect(TokenKind::Gosub)?;
            let target = self.expect_label_target()?;
            return Ok(Statement::OnGpioChange {
                pin,
                target,
                span: start.merge(self.prev_span()),
            });
        }

        // ON TIMER interval_ms GOSUB label
        if self.check_ident("TIMER") {
            self.advance(); // TIMER
            let interval_ms = self.parse_expr()?;
            self.expect(TokenKind::Gosub)?;
            let target = self.expect_label_target()?;
            return Ok(Statement::OnTimerEvent {
                interval_ms,
                target,
                span: start.merge(self.prev_span()),
            });
        }

        // ON MQTT.MESSAGE GOSUB label
        if self.check_ident("MQTT.MESSAGE") {
            self.advance(); // MQTT.MESSAGE
            self.expect(TokenKind::Gosub)?;
            let target = self.expect_label_target()?;
            return Ok(Statement::OnMqttMessage {
                target,
                span: start.merge(self.prev_span()),
            });
        }

        // ON ERROR GOTO label
        if self.eat(TokenKind::Error) {
            self.expect(TokenKind::Goto)?;
            let target = if let Some(TokenKind::IntLiteral(0)) = self.peek_kind() {
                self.advance();
                None // ON ERROR GOTO 0 disables handler
            } else {
                Some(self.expect_label_target()?)
            };
            return Ok(Statement::OnErrorGoto {
                target,
                span: start.merge(self.prev_span()),
            });
        }

        // ON expr GOTO/GOSUB label1, label2, ...
        let expr = self.parse_expr()?;
        let is_goto = if self.eat(TokenKind::Goto) {
            true
        } else if self.eat(TokenKind::Gosub) {
            false
        } else {
            return Err(self.error("expected GOTO or GOSUB after ON expression"));
        };

        let mut targets = Vec::new();
        targets.push(self.expect_label_target()?);
        while self.eat(TokenKind::Comma) {
            targets.push(self.expect_label_target()?);
        }

        let span = start.merge(self.prev_span());
        if is_goto {
            Ok(Statement::OnGoto { expr, targets, span })
        } else {
            Ok(Statement::OnGosub { expr, targets, span })
        }
    }

    fn parse_swap(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // SWAP
        let (var1, var1_type) = self.expect_variable()?;
        self.expect(TokenKind::Comma)?;
        let (var2, var2_type) = self.expect_variable()?;
        Ok(Statement::Swap {
            var1,
            var1_type,
            var2,
            var2_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_def_fn(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // DEF
        // Expect function name starting with FN
        let (name, _) = self.expect_variable()?;
        if !name.starts_with("FN") {
            return Err(self.error("DEF function name must start with FN"));
        }
        // Optional parameter list
        let params = if self.eat(TokenKind::LParen) {
            let mut p = Vec::new();
            if !self.check(TokenKind::RParen) {
                let (pname, ptype) = self.expect_variable()?;
                p.push((pname, ptype));
                while self.eat(TokenKind::Comma) {
                    let (pname, ptype) = self.expect_variable()?;
                    p.push((pname, ptype));
                }
            }
            self.expect(TokenKind::RParen)?;
            p
        } else {
            Vec::new()
        };
        self.expect(TokenKind::Eq)?;
        let body = self.parse_expr()?;
        Ok(Statement::DefFn {
            name,
            params,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_randomize(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // RANDOMIZE
        let seed = self.parse_expr()?;
        Ok(Statement::Randomize {
            seed,
            span: start.merge(self.prev_span()),
        })
    }

    // ── New hardware parse functions ────────────────────────

    fn parse_touch_read(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let pin = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::TouchRead {
            pin,
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_servo_attach(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let channel = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let pin = self.parse_expr()?;
        Ok(Statement::ServoAttach {
            channel,
            pin,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_servo_write(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let channel = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let angle = self.parse_expr()?;
        Ok(Statement::ServoWrite {
            channel,
            angle,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_tone(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let pin = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let freq = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let duration = self.parse_expr()?;
        Ok(Statement::Tone {
            pin,
            freq,
            duration,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_irq_attach(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let pin = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let mode = self.parse_expr()?;
        Ok(Statement::IrqAttach {
            pin,
            mode,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_irq_detach(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let pin = self.parse_expr()?;
        Ok(Statement::IrqDetach {
            pin,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_temp_read(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::TempRead {
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_ota_update(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let url = self.parse_expr()?;
        Ok(Statement::OtaUpdate {
            url,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_oled_init(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let width = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let height = self.parse_expr()?;
        Ok(Statement::OledInit {
            width,
            height,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_oled_print(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let x = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let y = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let text = self.parse_expr()?;
        Ok(Statement::OledPrint {
            x,
            y,
            text,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_oled_pixel(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let x = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let y = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let color = self.parse_expr()?;
        Ok(Statement::OledPixel {
            x,
            y,
            color,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_oled_line(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let x1 = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let y1 = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let x2 = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let y2 = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let color = self.parse_expr()?;
        Ok(Statement::OledLine {
            x1,
            y1,
            x2,
            y2,
            color,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_oled_clear(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::OledClear { span })
    }

    fn parse_oled_show(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::OledShow { span })
    }

    fn parse_lcd_init(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let cols = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let rows = self.parse_expr()?;
        Ok(Statement::LcdInit {
            cols,
            rows,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_lcd_print(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let text = self.parse_expr()?;
        Ok(Statement::LcdPrint {
            text,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_lcd_clear(&mut self) -> ParseResult<Statement> {
        let span = self.current_span();
        self.advance();
        Ok(Statement::LcdClear { span })
    }

    fn parse_lcd_pos(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let col = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let row = self.parse_expr()?;
        Ok(Statement::LcdPos {
            col,
            row,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_udp_init(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let port = self.parse_expr()?;
        Ok(Statement::UdpInit {
            port,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_udp_send(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let host = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let port = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let data = self.parse_expr()?;
        Ok(Statement::UdpSend {
            host,
            port,
            data,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_udp_receive(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance();
        let (target, var_type) = self.expect_variable()?;
        Ok(Statement::UdpReceive {
            target,
            var_type,
            span: start.merge(self.prev_span()),
        })
    }

    // ── New language features ──────────────────────────────

    fn parse_assert(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // ASSERT
        let condition = self.parse_expr()?;
        let message = if self.eat(TokenKind::Comma) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(Statement::Assert {
            condition,
            message,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_for_each(&mut self, start: Span) -> ParseResult<Statement> {
        let (var, var_type) = self.expect_variable()?;
        self.expect(TokenKind::In)?;
        let array_name = self.expect_ident_name()?;
        self.eat_newline();
        let body = self.parse_block(&[TokenKind::Next])?;
        self.expect(TokenKind::Next)?;
        // Optional variable name after NEXT
        if matches!(self.peek_kind(), Some(TokenKind::Ident(_))) {
            self.advance();
        }
        Ok(Statement::ForEach {
            var,
            var_type,
            array_name,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_try_catch(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // TRY
        self.eat_newline();

        // Parse try body until CATCH
        let mut try_body = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() || matches!(self.peek_kind(), Some(TokenKind::Catch)) {
                break;
            }
            let stmt = self.parse_statement()?;
            try_body.push(stmt);
            self.eat_newline();
        }

        self.expect(TokenKind::Catch)?;
        let catch_var = self.expect_ident_name()?;
        self.eat_newline();

        // Parse catch body until END TRY
        let mut catch_body = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::Try) {
                break;
            }
            let stmt = self.parse_statement()?;
            catch_body.push(stmt);
            self.eat_newline();
        }

        // Expect END TRY
        self.expect_end_keyword(TokenKind::Try, "END TRY")?;

        Ok(Statement::TryCatch {
            try_body,
            catch_var,
            catch_body,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_task_stmt(&mut self) -> ParseResult<Statement> {
        let start = self.current_span();
        self.advance(); // TASK
        let name = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let stack_size = self.parse_expr()?;
        self.expect(TokenKind::Comma)?;
        let priority = self.parse_expr()?;
        self.eat_newline();

        // Parse body until END TASK
        let mut body = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::Task) {
                break;
            }
            let stmt = self.parse_statement()?;
            body.push(stmt);
            self.eat_newline();
        }

        self.expect_end_keyword(TokenKind::Task, "END TASK")?;

        Ok(Statement::Task {
            name,
            stack_size,
            priority,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_enum_def(&mut self) -> ParseResult<EnumDef> {
        let start = self.current_span();
        self.advance(); // ENUM
        let name = self.expect_ident_name()?;
        self.eat_newline();

        let mut members = Vec::new();
        let mut next_value: i32 = 0;
        loop {
            self.skip_blank_lines();
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::Enum) {
                break;
            }
            let member_start = self.current_span();
            let member_name = self.expect_ident_name()?;
            let value = if self.eat(TokenKind::Eq) {
                match self.peek_kind() {
                    Some(TokenKind::IntLiteral(v)) => {
                        let v = *v;
                        self.advance();
                        next_value = v + 1;
                        v
                    }
                    _ => return Err(self.error("expected integer value for enum member")),
                }
            } else {
                let v = next_value;
                next_value += 1;
                v
            };
            members.push(EnumMember {
                name: member_name,
                value,
                span: member_start.merge(self.prev_span()),
            });
            self.eat_newline();
        }

        self.expect_end_keyword(TokenKind::Enum, "END ENUM")?;

        Ok(EnumDef {
            name,
            members,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_machine_def(&mut self) -> ParseResult<MachineDef> {
        let start = self.current_span();
        self.advance(); // MACHINE
        let name = self.expect_ident_name()?;
        self.eat_newline();

        let mut states = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::Machine) {
                break;
            }
            if matches!(self.peek_kind(), Some(TokenKind::Ident(s)) if s == "STATE") {
                self.advance(); // consume STATE ident
                let state_start = self.prev_span();
                let state_name = self.expect_ident_name()?;
                self.eat_newline();

                let mut transitions = Vec::new();
                loop {
                    self.skip_blank_lines();
                    if self.at_end()
                        || matches!(self.peek_kind(), Some(TokenKind::Ident(s)) if s == "STATE")
                        || self.check_end_keyword_ahead(TokenKind::Machine)
                        || self.check_end_ident("STATE")
                    {
                        break;
                    }
                    // ON event_name GOTO target_state
                    if self.eat(TokenKind::On) {
                        let event_name = self.expect_ident_name()?;
                        self.expect(TokenKind::Goto)?;
                        let target_state = self.expect_ident_name()?;
                        let t_span = self.prev_span();
                        transitions.push(Transition::OnEvent {
                            event_name,
                            target_state,
                            span: t_span,
                        });
                        self.eat_newline();
                    } else {
                        return Err(self.error("expected ON transition in STATE block"));
                    }
                }

                // Optional END STATE
                if self.check_end_ident("STATE") {
                    self.advance(); // END
                    self.advance(); // STATE (Ident)
                    self.eat_newline();
                }

                states.push(StateDef {
                    name: state_name,
                    transitions,
                    span: state_start.merge(self.prev_span()),
                });
            } else {
                return Err(self.error("expected STATE inside MACHINE"));
            }
        }

        self.expect_end_keyword(TokenKind::Machine, "END MACHINE")?;

        Ok(MachineDef {
            name,
            states,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_module_def(&mut self) -> ParseResult<ModuleDef> {
        let start = self.current_span();
        self.advance(); // MODULE
        let mod_name = self.expect_ident_name()?;
        self.eat_newline();

        let mut subs = Vec::new();
        let mut functions = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() || self.check_end_keyword_ahead(TokenKind::Module) {
                break;
            }
            match self.peek_kind().cloned() {
                Some(TokenKind::Sub) => {
                    let mut sub = self.parse_sub_def()?;
                    sub.name = format!("{}.{}", mod_name, sub.name);
                    subs.push(sub);
                }
                Some(TokenKind::Function) => {
                    let mut func = self.parse_function_def()?;
                    func.name = format!("{}.{}", mod_name, func.name);
                    functions.push(func);
                }
                _ => return Err(self.error("expected SUB or FUNCTION inside MODULE")),
            }
        }

        self.expect_end_keyword(TokenKind::Module, "END MODULE")?;

        Ok(ModuleDef {
            name: mod_name,
            subs,
            functions,
            span: start.merge(self.prev_span()),
        })
    }

    // ── Block parsing ───────────────────────────────────────

    /// Parse statements until one of the direct terminator tokens is found.
    fn parse_block(&mut self, terminators: &[TokenKind]) -> ParseResult<Vec<Statement>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_blank_lines();
            if self.at_end() {
                break;
            }
            if let Some(kind) = self.peek_kind() {
                if terminators
                    .iter()
                    .any(|t| std::mem::discriminant(kind) == std::mem::discriminant(t))
                {
                    break;
                }
            }
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.eat_newline();
        }
        Ok(stmts)
    }

    // ── Expression parsing (Pratt / precedence climbing) ────

    pub fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> ParseResult<Expr> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let Some(op) = self.peek_binop() else { break };
            let (l_bp, r_bp) = infix_binding_power(op);
            if l_bp < min_bp {
                break;
            }
            self.advance(); // consume operator
            let rhs = self.parse_expr_bp(r_bp)?;
            let span = lhs.span().merge(rhs.span());
            lhs = Expr::BinaryOp {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> ParseResult<Expr> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Minus) => {
                let start = self.current_span();
                self.advance();
                let operand = self.parse_expr_bp(prefix_bp())?;
                let span = start.merge(operand.span());
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            Some(TokenKind::Not) => {
                let start = self.current_span();
                self.advance();
                let operand = self.parse_expr_bp(prefix_bp())?;
                let span = start.merge(operand.span());
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            _ => self.parse_atom(),
        }
    }

    fn parse_atom(&mut self) -> ParseResult<Expr> {
        let span = self.current_span();
        match self.peek_kind().cloned() {
            Some(TokenKind::IntLiteral(v)) => {
                self.advance();
                Ok(Expr::IntLiteral { value: v, span })
            }
            Some(TokenKind::FloatLiteral(v)) => {
                self.advance();
                Ok(Expr::FloatLiteral { value: v, span })
            }
            Some(TokenKind::StringLiteral(v)) => {
                self.advance();
                Ok(Expr::StringLiteral {
                    value: v.clone(),
                    span,
                })
            }
            Some(TokenKind::Ident(name)) => {
                let name = name.clone();
                self.advance();
                self.parse_ident_expr(name, QBType::Inferred, span)
            }
            Some(TokenKind::IntIdent(name)) => {
                let name = name.clone();
                self.advance();
                self.parse_ident_expr(name, QBType::Integer, span)
            }
            Some(TokenKind::StringIdent(name)) => {
                let name = name.clone();
                self.advance();
                self.parse_ident_expr(name, QBType::String, span)
            }
            Some(TokenKind::LongIdent(name)) => {
                let name = name.clone();
                self.advance();
                self.parse_ident_expr(name, QBType::Long, span)
            }
            Some(TokenKind::SingleIdent(name)) => {
                let name = name.clone();
                self.advance();
                self.parse_ident_expr(name, QBType::Single, span)
            }
            Some(TokenKind::DoubleIdent(name)) => {
                let name = name.clone();
                self.advance();
                self.parse_ident_expr(name, QBType::Double, span)
            }
            Some(TokenKind::InterpolatedString(template)) => {
                let template = template.clone();
                self.advance();
                // Desugar $"Hello {name}, you are {age} years old" into
                // "Hello " + STR$(name) + ", you are " + STR$(age) + " years old"
                let mut parts: Vec<Expr> = Vec::new();
                let mut current = String::new();
                let mut chars = template.chars().peekable();
                while let Some(ch) = chars.next() {
                    if ch == '{' {
                        // Flush literal part
                        if !current.is_empty() {
                            parts.push(Expr::StringLiteral {
                                value: current.clone(),
                                span,
                            });
                            current.clear();
                        }
                        // Collect expression text until '}'
                        let mut expr_text = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '}' {
                                chars.next();
                                break;
                            }
                            expr_text.push(c);
                            chars.next();
                        }
                        // Parse the expression
                        let expr_tokens = rustybasic_lexer::tokenize(&expr_text)
                            .map_err(|e| ParseError {
                                span,
                                message: format!("error in interpolation: {}", e),
                            })?;
                        let mut sub_parser = Parser::new(expr_tokens);
                        let expr = sub_parser.parse_expr()?;
                        // Wrap in STR$ if needed
                        parts.push(Expr::FnCall {
                            name: "STR$".to_string(),
                            args: vec![expr],
                            span,
                        });
                    } else {
                        current.push(ch);
                    }
                }
                if !current.is_empty() {
                    parts.push(Expr::StringLiteral {
                        value: current,
                        span,
                    });
                }
                // Chain parts with BinaryOp::Add
                if parts.is_empty() {
                    Ok(Expr::StringLiteral {
                        value: String::new(),
                        span,
                    })
                } else {
                    let mut result = parts.remove(0);
                    for part in parts {
                        let merged = result.span().merge(part.span());
                        result = Expr::BinaryOp {
                            op: BinOp::Add,
                            left: Box::new(result),
                            right: Box::new(part),
                            span: merged,
                        };
                    }
                    Ok(result)
                }
            }
            Some(TokenKind::Lambda) => {
                self.advance(); // LAMBDA
                self.expect(TokenKind::LParen)?;
                let mut params = Vec::new();
                if !self.check(TokenKind::RParen) {
                    let (pname, ptype) = self.expect_variable()?;
                    let ptype = if self.eat(TokenKind::As) {
                        self.parse_type_name()?
                    } else {
                        ptype
                    };
                    params.push((pname, ptype));
                    while self.eat(TokenKind::Comma) {
                        let (pname, ptype) = self.expect_variable()?;
                        let ptype = if self.eat(TokenKind::As) {
                            self.parse_type_name()?
                        } else {
                            ptype
                        };
                        params.push((pname, ptype));
                    }
                }
                self.expect(TokenKind::RParen)?;
                self.expect(TokenKind::FatArrow)?;
                let body = self.parse_expr()?;
                Ok(Expr::Lambda {
                    params,
                    body: Box::new(body),
                    span: span.merge(self.prev_span()),
                })
            }
            _ => Err(self.error("expected expression")),
        }
    }

    /// After parsing an identifier, check for function call or array access.
    fn parse_ident_expr(
        &mut self,
        name: String,
        var_type: QBType,
        start_span: Span,
    ) -> ParseResult<Expr> {
        if self.check(TokenKind::LParen) {
            self.advance();
            let mut args = Vec::new();
            if !self.check(TokenKind::RParen) {
                args.push(self.parse_expr()?);
                while self.eat(TokenKind::Comma) {
                    args.push(self.parse_expr()?);
                }
            }
            let end = self.current_span();
            self.expect(TokenKind::RParen)?;
            // Could be function call or array access — sema will resolve
            Ok(Expr::FnCall {
                name,
                args,
                span: start_span.merge(end),
            })
        } else {
            Ok(Expr::Variable {
                name,
                var_type,
                span: start_span,
            })
        }
    }

    fn peek_binop(&self) -> Option<BinOp> {
        match self.peek_kind()? {
            TokenKind::Plus => Some(BinOp::Add),
            TokenKind::Minus => Some(BinOp::Sub),
            TokenKind::Star => Some(BinOp::Mul),
            TokenKind::Slash => Some(BinOp::Div),
            TokenKind::Backslash => Some(BinOp::IntDiv),
            TokenKind::Mod => Some(BinOp::Mod),
            TokenKind::Caret => Some(BinOp::Pow),
            TokenKind::Eq => Some(BinOp::Eq),
            TokenKind::Neq => Some(BinOp::Neq),
            TokenKind::Lt => Some(BinOp::Lt),
            TokenKind::Gt => Some(BinOp::Gt),
            TokenKind::Le => Some(BinOp::Le),
            TokenKind::Ge => Some(BinOp::Ge),
            TokenKind::And => Some(BinOp::And),
            TokenKind::Or => Some(BinOp::Or),
            TokenKind::Xor => Some(BinOp::Xor),
            _ => None,
        }
    }

    // ── Compound keyword helpers ────────────────────────────

    /// Check if the current position is "END keyword" (e.g. END IF, END SUB).
    fn check_end_keyword_ahead(&self, keyword: TokenKind) -> bool {
        if !matches!(self.peek_kind(), Some(TokenKind::End)) {
            return false;
        }
        if let Some(k2) = self.peek_ahead_kind(1) {
            return std::mem::discriminant(k2) == std::mem::discriminant(&keyword);
        }
        false
    }

    /// Check if the current position is "END ident" where ident matches a name
    /// (used for END TYPE where TYPE is an Ident, not a keyword).
    fn check_end_ident(&self, name: &str) -> bool {
        if !matches!(self.peek_kind(), Some(TokenKind::End)) {
            return false;
        }
        if let Some(TokenKind::Ident(ref n)) = self.peek_ahead_kind(1) {
            return n == name;
        }
        false
    }

    /// Expect and consume "END keyword" (e.g. END SUB). Returns error if not found.
    fn expect_end_keyword(&mut self, keyword: TokenKind, display: &str) -> ParseResult<()> {
        if self.check_end_keyword_ahead(keyword) {
            self.advance(); // END
            self.advance(); // keyword
            Ok(())
        } else {
            Err(self.error(&format!("expected {display}")))
        }
    }

    /// Check if the current position is CASE ELSE (Case followed by Else).
    fn check_case_else(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Case))
            && matches!(self.peek_ahead_kind(1), Some(TokenKind::Else))
    }

    /// Check if the current token is an Ident with the given name.
    fn check_ident(&self, name: &str) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Ident(ref n)) if n == name)
    }

    // ── Token helpers ─────────────────────────────────────

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }

    fn peek_ahead_kind(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens.get(self.pos + offset).map(|t| &t.kind)
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span::new(0, 0))
    }

    fn prev_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            Span::new(0, 0)
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn at_newline(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Newline) | None)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn advance_and_get(&mut self) -> TokenKind {
        let kind = self.tokens[self.pos].kind.clone();
        self.pos += 1;
        kind
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek_kind().map_or(false, |k| {
            std::mem::discriminant(k) == std::mem::discriminant(&kind)
        })
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> ParseResult<()> {
        if self.check(kind.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!(
                "expected {kind}, found {}",
                self.peek_kind()
                    .map(|k| k.to_string())
                    .unwrap_or_else(|| "end of file".to_string())
            )))
        }
    }

    fn expect_ident_name(&mut self) -> ParseResult<String> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Ident(name)) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("expected identifier")),
        }
    }

    fn expect_variable(&mut self) -> ParseResult<(String, QBType)> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Ident(name)) => {
                self.advance();
                Ok((name, QBType::Inferred))
            }
            Some(TokenKind::IntIdent(name)) => {
                self.advance();
                Ok((name, QBType::Integer))
            }
            Some(TokenKind::StringIdent(name)) => {
                self.advance();
                Ok((name, QBType::String))
            }
            Some(TokenKind::LongIdent(name)) => {
                self.advance();
                Ok((name, QBType::Long))
            }
            Some(TokenKind::SingleIdent(name)) => {
                self.advance();
                Ok((name, QBType::Single))
            }
            Some(TokenKind::DoubleIdent(name)) => {
                self.advance();
                Ok((name, QBType::Double))
            }
            _ => Err(self.error("expected variable name")),
        }
    }

    fn eat_newline(&mut self) {
        while matches!(self.peek_kind(), Some(TokenKind::Newline)) {
            self.advance();
        }
    }

    fn skip_blank_lines(&mut self) {
        loop {
            match self.peek_kind() {
                Some(TokenKind::Newline) | Some(TokenKind::Rem) | Some(TokenKind::Comment) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError {
            span: self.current_span(),
            message: message.to_string(),
        }
    }
}

/// Binding power for infix operators.
fn infix_binding_power(op: BinOp) -> (u8, u8) {
    match op {
        BinOp::Or | BinOp::Xor => (1, 2),
        BinOp::And => (3, 4),
        BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => (5, 6),
        BinOp::Add | BinOp::Sub => (7, 8),
        BinOp::Mul | BinOp::Div | BinOp::IntDiv | BinOp::Mod => (9, 10),
        BinOp::Pow => (12, 11), // right-associative
    }
}

fn prefix_bp() -> u8 {
    13
}

/// Convenience function: parse source tokens into an AST.
pub fn parse(tokens: Vec<Token>) -> ParseResult<Program> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustybasic_lexer::tokenize;

    fn parse_str(input: &str) -> ParseResult<Program> {
        let tokens = tokenize(input).expect("lex error");
        parse(tokens)
    }

    #[test]
    fn test_dim_as_integer() {
        let prog = parse_str("DIM x AS INTEGER").unwrap();
        assert!(matches!(
            &prog.body[0],
            Statement::Dim {
                var_type: QBType::Integer,
                ..
            }
        ));
    }

    #[test]
    fn test_dim_as_string() {
        let prog = parse_str("DIM name AS STRING").unwrap();
        assert!(matches!(
            &prog.body[0],
            Statement::Dim {
                var_type: QBType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_const() {
        let prog = parse_str("CONST PI = 3.14").unwrap();
        assert!(matches!(&prog.body[0], Statement::Const { .. }));
    }

    #[test]
    fn test_sub_def() {
        let prog = parse_str("SUB Hello\nPRINT \"hi\"\nEND SUB").unwrap();
        assert_eq!(prog.subs.len(), 1);
        assert_eq!(prog.subs[0].name, "HELLO");
    }

    #[test]
    fn test_function_def() {
        let prog = parse_str(
            "FUNCTION Add%(a AS INTEGER, b AS INTEGER)\nAdd% = a + b\nEND FUNCTION",
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
        assert_eq!(prog.functions[0].params.len(), 2);
    }

    #[test]
    fn test_type_def() {
        let prog = parse_str("TYPE Point\nx AS SINGLE\ny AS SINGLE\nEND TYPE").unwrap();
        assert_eq!(prog.types.len(), 1);
        assert_eq!(prog.types[0].name, "POINT");
        assert_eq!(prog.types[0].fields.len(), 2);
    }

    #[test]
    fn test_select_case() {
        let prog = parse_str("SELECT CASE x\nCASE 1\nPRINT \"one\"\nCASE 2, 3\nPRINT \"two or three\"\nCASE ELSE\nPRINT \"other\"\nEND SELECT").unwrap();
        assert!(matches!(&prog.body[0], Statement::SelectCase { .. }));
    }

    #[test]
    fn test_do_while_loop() {
        let prog = parse_str("DO WHILE x > 0\nx = x - 1\nLOOP").unwrap();
        if let Statement::DoLoop { pre_condition, .. } = &prog.body[0] {
            assert!(pre_condition.is_some());
            assert!(pre_condition.as_ref().unwrap().is_while);
        } else {
            panic!("expected DoLoop");
        }
    }

    #[test]
    fn test_do_loop_until() {
        let prog = parse_str("DO\nx = x + 1\nLOOP UNTIL x > 10").unwrap();
        if let Statement::DoLoop { post_condition, .. } = &prog.body[0] {
            assert!(post_condition.is_some());
            assert!(!post_condition.as_ref().unwrap().is_while);
        } else {
            panic!("expected DoLoop");
        }
    }

    #[test]
    fn test_for_next() {
        let prog = parse_str("FOR i = 1 TO 10\nPRINT i\nNEXT i").unwrap();
        assert!(matches!(&prog.body[0], Statement::For { .. }));
    }

    #[test]
    fn test_if_end_if() {
        let prog = parse_str("IF x > 0 THEN\nPRINT x\nEND IF").unwrap();
        assert!(matches!(&prog.body[0], Statement::If { .. }));
    }

    #[test]
    fn test_single_line_if() {
        let prog = parse_str("IF x > 0 THEN PRINT x ELSE PRINT 0").unwrap();
        assert!(matches!(&prog.body[0], Statement::If { .. }));
    }

    #[test]
    fn test_qbasic_comment() {
        let prog = parse_str("' This is a comment\nPRINT 42").unwrap();
        assert!(matches!(&prog.body[0], Statement::Print { .. }));
    }

    #[test]
    fn test_call_sub() {
        let prog = parse_str("CALL MySub(1, 2)").unwrap();
        assert!(matches!(&prog.body[0], Statement::CallSub { .. }));
    }

    #[test]
    fn test_exit_for() {
        let prog = parse_str("FOR i = 1 TO 10\nIF i = 5 THEN EXIT FOR\nNEXT i").unwrap();
        if let Statement::For { body, .. } = &prog.body[0] {
            if let Statement::If { then_body, .. } = &body[0] {
                assert!(matches!(&then_body[0], Statement::ExitFor { .. }));
            }
        }
    }

    #[test]
    fn test_integer_division() {
        let prog = parse_str("x = 10 \\ 3").unwrap();
        if let Statement::Let { expr, .. } = &prog.body[0] {
            assert!(matches!(
                expr,
                Expr::BinaryOp {
                    op: BinOp::IntDiv,
                    ..
                }
            ));
        }
    }

    #[test]
    fn test_gpio_delay() {
        let prog = parse_str("GPIO.MODE 2, 1\nDELAY 500").unwrap();
        assert!(matches!(&prog.body[0], Statement::GpioMode { .. }));
        assert!(matches!(&prog.body[1], Statement::Delay { .. }));
    }

    #[test]
    fn test_dim_array_1d() {
        let prog = parse_str("DIM arr(10) AS INTEGER").unwrap();
        if let Statement::Dim { dimensions, var_type, .. } = &prog.body[0] {
            assert_eq!(dimensions.len(), 1);
            assert_eq!(*var_type, QBType::Integer);
        } else {
            panic!("expected Dim");
        }
    }

    #[test]
    fn test_dim_array_2d() {
        let prog = parse_str("DIM matrix(3, 4) AS SINGLE").unwrap();
        if let Statement::Dim { dimensions, var_type, .. } = &prog.body[0] {
            assert_eq!(dimensions.len(), 2);
            assert_eq!(*var_type, QBType::Single);
        } else {
            panic!("expected Dim");
        }
    }

    #[test]
    fn test_array_assign() {
        let prog = parse_str("arr(0) = 10").unwrap();
        assert!(matches!(&prog.body[0], Statement::ArrayAssign { .. }));
        if let Statement::ArrayAssign { name, indices, .. } = &prog.body[0] {
            assert_eq!(name, "ARR");
            assert_eq!(indices.len(), 1);
        }
    }

    #[test]
    fn test_try_catch() {
        let prog = parse_str("TRY\nPRINT 1\nCATCH err\nPRINT 2\nEND TRY").unwrap();
        assert!(matches!(&prog.body[0], Statement::TryCatch { .. }));
    }

    #[test]
    fn test_array_assign_2d() {
        let prog = parse_str("matrix(1, 2) = 99.5").unwrap();
        assert!(matches!(&prog.body[0], Statement::ArrayAssign { .. }));
        if let Statement::ArrayAssign { name, indices, .. } = &prog.body[0] {
            assert_eq!(name, "MATRIX");
            assert_eq!(indices.len(), 2);
        }
    }
}
