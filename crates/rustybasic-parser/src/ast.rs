use rustybasic_common::Span;

/// A complete QBASIC program: top-level statements + procedure definitions + type definitions.
#[derive(Debug, Clone)]
pub struct Program {
    /// Top-level module code (runs when program starts).
    pub body: Vec<Statement>,
    /// SUB procedure definitions.
    pub subs: Vec<SubDef>,
    /// FUNCTION definitions.
    pub functions: Vec<FunctionDef>,
    /// TYPE...END TYPE struct definitions.
    pub types: Vec<TypeDef>,
    /// ENUM definitions.
    pub enums: Vec<EnumDef>,
    /// MODULE definitions.
    pub modules: Vec<ModuleDef>,
    /// STATE MACHINE definitions.
    pub machines: Vec<MachineDef>,
}

// ── TYPE (struct) definitions ───────────────────────────

/// TYPE myType ... END TYPE — user-defined record type.
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<TypeField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeField {
    pub name: String,
    pub field_type: QBType,
    pub span: Span,
}

// ── ENUM definitions ────────────────────────────────────

/// ENUM Color ... END ENUM — enumeration type.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub members: Vec<EnumMember>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumMember {
    pub name: String,
    pub value: i32,
    pub span: Span,
}

// ── MODULE definitions ──────────────────────────────────

/// MODULE Name ... END MODULE — namespace for grouping SUBs/FUNCTIONs.
#[derive(Debug, Clone)]
pub struct ModuleDef {
    pub name: String,
    pub subs: Vec<SubDef>,
    pub functions: Vec<FunctionDef>,
    pub span: Span,
}

// ── State Machine definitions ───────────────────────────

/// MACHINE Name ... END MACHINE — state machine DSL.
#[derive(Debug, Clone)]
pub struct MachineDef {
    pub name: String,
    pub states: Vec<StateDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StateDef {
    pub name: String,
    pub transitions: Vec<Transition>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Transition {
    OnEvent {
        event_name: String,
        target_state: String,
        span: Span,
    },
}

// ── Procedure definitions ───────────────────────────────

/// SUB Name (params...) ... END SUB
#[derive(Debug, Clone)]
pub struct SubDef {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Statement>,
    pub is_static: bool,
    pub span: Span,
}

/// FUNCTION Name (params...) AS type ... END FUNCTION
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: QBType,
    pub body: Vec<Statement>,
    pub is_static: bool,
    pub span: Span,
}

/// Procedure parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub param_type: QBType,
    pub by_ref: bool, // BYREF (default in QBASIC) vs BYVAL
    pub span: Span,
}

// ── Type system ─────────────────────────────────────────

/// QBASIC type — from `AS type` or suffix inference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QBType {
    Integer,         // INTEGER or % suffix — i16 (we use i32 for ESP32)
    Long,            // LONG or & suffix — i32
    Single,          // SINGLE or ! suffix — f32
    Double,          // DOUBLE or # suffix — f64 (we use f32 on ESP32)
    String,          // STRING or $ suffix — refcounted
    UserType(String), // TYPE name
    FunctionPtr,     // FUNCTION pointer (for LAMBDA)
    Inferred,        // Not yet known (will be resolved by sema)
}

/// Convert from the old VarType for compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarType {
    Float,   // SINGLE/DOUBLE — f32
    Integer, // INTEGER/LONG — i32
    String,  // STRING — rb_string_t*
}

impl From<&QBType> for VarType {
    fn from(qb: &QBType) -> Self {
        match qb {
            QBType::Integer | QBType::Long => VarType::Integer,
            QBType::Single | QBType::Double | QBType::Inferred => VarType::Float,
            QBType::String => VarType::String,
            QBType::UserType(_) => VarType::Integer, // pointer, treated as i32 for now
            QBType::FunctionPtr => VarType::Integer, // pointer treated as i32 for now
        }
    }
}

// ── DATA item (for DATA/READ/RESTORE) ───────────────────

#[derive(Debug, Clone)]
pub enum DataItem {
    Int(i32),
    Float(f32),
    Str(String),
}

// ── Statements ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Statement {
    /// DIM var AS type  or  DIM var(dims) AS type
    Dim {
        name: String,
        var_type: QBType,
        dimensions: Vec<Expr>,
        is_shared: bool,
        span: Span,
    },
    /// CONST name = value
    Const {
        name: String,
        value: Expr,
        span: Span,
    },
    /// LET var = expr  or  var = expr
    Let {
        name: String,
        var_type: QBType,
        expr: Expr,
        span: Span,
    },
    /// member assignment:  myVar.field = expr
    FieldAssign {
        object: String,
        field: String,
        expr: Expr,
        span: Span,
    },
    /// PRINT items...
    Print {
        items: Vec<PrintItem>,
        span: Span,
    },
    /// INPUT "prompt"; var  or  INPUT var
    Input {
        prompt: Option<String>,
        name: String,
        var_type: QBType,
        span: Span,
    },
    /// LINE INPUT "prompt"; var$
    LineInput {
        prompt: Option<String>,
        name: String,
        span: Span,
    },
    /// IF...THEN...ELSEIF...ELSE...END IF
    If {
        condition: Expr,
        then_body: Vec<Statement>,
        else_if_clauses: Vec<ElseIfClause>,
        else_body: Vec<Statement>,
        span: Span,
    },
    /// FOR var = from TO to [STEP step] ... NEXT [var]
    For {
        var: String,
        from: Expr,
        to: Expr,
        step: Option<Expr>,
        body: Vec<Statement>,
        span: Span,
    },
    /// DO [WHILE|UNTIL cond] ... LOOP [WHILE|UNTIL cond]
    DoLoop {
        pre_condition: Option<DoCondition>,
        post_condition: Option<DoCondition>,
        body: Vec<Statement>,
        span: Span,
    },
    /// WHILE cond ... WEND (legacy, but QBASIC supports it)
    While {
        condition: Expr,
        body: Vec<Statement>,
        span: Span,
    },
    /// SELECT CASE expr ... CASE ... END SELECT
    SelectCase {
        expr: Expr,
        cases: Vec<CaseClause>,
        else_body: Vec<Statement>,
        span: Span,
    },
    /// GOTO label
    Goto {
        target: String,
        span: Span,
    },
    /// GOSUB label
    Gosub {
        target: String,
        span: Span,
    },
    /// label: (line label definition — just the label, parsed inline)
    Label {
        name: String,
        span: Span,
    },
    /// CALL subName(args...)  or  subName args...
    CallSub {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    Return {
        span: Span,
    },
    End {
        span: Span,
    },
    ExitFor {
        span: Span,
    },
    ExitDo {
        span: Span,
    },
    ExitSub {
        span: Span,
    },
    ExitFunction {
        span: Span,
    },
    Rem {
        span: Span,
    },

    // ── Hardware statements (ESP32 extensions) ──
    GpioMode {
        pin: Expr,
        mode: Expr,
        span: Span,
    },
    GpioSet {
        pin: Expr,
        value: Expr,
        span: Span,
    },
    GpioRead {
        pin: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    I2cSetup {
        bus: Expr,
        sda: Expr,
        scl: Expr,
        freq: Expr,
        span: Span,
    },
    I2cWrite {
        addr: Expr,
        data: Expr,
        span: Span,
    },
    I2cRead {
        addr: Expr,
        length: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    SpiSetup {
        bus: Expr,
        clk: Expr,
        mosi: Expr,
        miso: Expr,
        freq: Expr,
        span: Span,
    },
    SpiTransfer {
        data: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    WifiConnect {
        ssid: Expr,
        password: Expr,
        span: Span,
    },
    WifiStatus {
        target: String,
        var_type: QBType,
        span: Span,
    },
    WifiDisconnect {
        span: Span,
    },
    Delay {
        ms: Expr,
        span: Span,
    },
    AdcRead {
        pin: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    PwmSetup {
        channel: Expr,
        pin: Expr,
        freq: Expr,
        resolution: Expr,
        span: Span,
    },
    PwmDuty {
        channel: Expr,
        duty: Expr,
        span: Span,
    },
    UartSetup {
        port: Expr,
        baud: Expr,
        tx: Expr,
        rx: Expr,
        span: Span,
    },
    UartWrite {
        port: Expr,
        data: Expr,
        span: Span,
    },
    UartRead {
        port: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    TimerStart {
        span: Span,
    },
    TimerElapsed {
        target: String,
        var_type: QBType,
        span: Span,
    },
    HttpGet {
        url: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    HttpPost {
        url: Expr,
        body: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    NvsWrite {
        key: Expr,
        value: Expr,
        span: Span,
    },
    NvsRead {
        key: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    MqttConnect {
        broker: Expr,
        port: Expr,
        span: Span,
    },
    MqttDisconnect {
        span: Span,
    },
    MqttPublish {
        topic: Expr,
        message: Expr,
        span: Span,
    },
    MqttSubscribe {
        topic: Expr,
        span: Span,
    },
    MqttReceive {
        target: String,
        var_type: QBType,
        span: Span,
    },
    BleInit {
        name: Expr,
        span: Span,
    },
    BleAdvertise {
        mode: Expr,
        span: Span,
    },
    BleScan {
        target: String,
        var_type: QBType,
        span: Span,
    },
    BleSend {
        data: Expr,
        span: Span,
    },
    BleReceive {
        target: String,
        var_type: QBType,
        span: Span,
    },
    JsonGet {
        json: Expr,
        key: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    JsonSet {
        json: Expr,
        key: Expr,
        value: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    JsonCount {
        json: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    LedSetup {
        pin: Expr,
        count: Expr,
        span: Span,
    },
    LedSet {
        index: Expr,
        r: Expr,
        g: Expr,
        b: Expr,
        span: Span,
    },
    LedShow {
        span: Span,
    },
    LedClear {
        span: Span,
    },
    DeepSleep {
        ms: Expr,
        span: Span,
    },
    EspnowInit {
        span: Span,
    },
    EspnowSend {
        peer: Expr,
        data: Expr,
        span: Span,
    },
    EspnowReceive {
        target: String,
        var_type: QBType,
        span: Span,
    },

    /// DATA v1, v2, ... — inline data declaration
    Data {
        items: Vec<DataItem>,
        span: Span,
    },
    /// READ var1, var2, ... — read next items from data pool
    Read {
        variables: Vec<(String, QBType)>,
        span: Span,
    },
    /// RESTORE — reset data read pointer
    Restore {
        span: Span,
    },

    // ── Classic BASIC extensions ──
    /// ON expr GOTO label1, label2, ...
    OnGoto {
        expr: Expr,
        targets: Vec<String>,
        span: Span,
    },
    /// ON expr GOSUB label1, label2, ...
    OnGosub {
        expr: Expr,
        targets: Vec<String>,
        span: Span,
    },
    /// SWAP var1, var2
    Swap {
        var1: String,
        var1_type: QBType,
        var2: String,
        var2_type: QBType,
        span: Span,
    },
    /// DEF FNname(params) = expr
    DefFn {
        name: String,
        params: Vec<(String, QBType)>,
        body: Expr,
        span: Span,
    },
    /// PRINT USING fmt$; items...
    PrintUsing {
        format: Expr,
        items: Vec<Expr>,
        span: Span,
    },
    /// ON ERROR GOTO label (None = GOTO 0, disable handler)
    OnErrorGoto {
        target: Option<String>,
        span: Span,
    },
    /// RANDOMIZE seed
    Randomize {
        seed: Expr,
        span: Span,
    },

    // ── New hardware statements (ESP32 extensions) ──
    TouchRead {
        pin: Expr,
        target: String,
        var_type: QBType,
        span: Span,
    },
    ServoAttach {
        channel: Expr,
        pin: Expr,
        span: Span,
    },
    ServoWrite {
        channel: Expr,
        angle: Expr,
        span: Span,
    },
    Tone {
        pin: Expr,
        freq: Expr,
        duration: Expr,
        span: Span,
    },
    IrqAttach {
        pin: Expr,
        mode: Expr,
        span: Span,
    },
    IrqDetach {
        pin: Expr,
        span: Span,
    },
    TempRead {
        target: String,
        var_type: QBType,
        span: Span,
    },
    OtaUpdate {
        url: Expr,
        span: Span,
    },
    OledInit {
        width: Expr,
        height: Expr,
        span: Span,
    },
    OledPrint {
        x: Expr,
        y: Expr,
        text: Expr,
        span: Span,
    },
    OledPixel {
        x: Expr,
        y: Expr,
        color: Expr,
        span: Span,
    },
    OledLine {
        x1: Expr,
        y1: Expr,
        x2: Expr,
        y2: Expr,
        color: Expr,
        span: Span,
    },
    OledClear {
        span: Span,
    },
    OledShow {
        span: Span,
    },
    LcdInit {
        cols: Expr,
        rows: Expr,
        span: Span,
    },
    LcdPrint {
        text: Expr,
        span: Span,
    },
    LcdClear {
        span: Span,
    },
    LcdPos {
        col: Expr,
        row: Expr,
        span: Span,
    },
    UdpInit {
        port: Expr,
        span: Span,
    },
    UdpSend {
        host: Expr,
        port: Expr,
        data: Expr,
        span: Span,
    },
    UdpReceive {
        target: String,
        var_type: QBType,
        span: Span,
    },

    // ── New hardware statements (ESP32 extensions, phase 2) ──
    // NTP
    NtpSync { server: Expr, span: Span },
    NtpTime { target: String, var_type: QBType, span: Span },
    NtpEpoch { target: String, var_type: QBType, span: Span },
    // File System
    FileOpen { path: Expr, mode: Expr, span: Span },
    FileWrite { data: Expr, span: Span },
    FileReadStr { target: String, var_type: QBType, span: Span },
    FileClose { span: Span },
    FileDelete { path: Expr, span: Span },
    FileExists { path: Expr, target: String, var_type: QBType, span: Span },
    // WebSocket
    WsConnect { url: Expr, span: Span },
    WsSend { data: Expr, span: Span },
    WsReceiveStr { target: String, var_type: QBType, span: Span },
    WsClose { span: Span },
    // TCP
    TcpListen { port: Expr, span: Span },
    TcpAccept { target: String, var_type: QBType, span: Span },
    TcpSend { data: Expr, span: Span },
    TcpReceiveStr { target: String, var_type: QBType, span: Span },
    TcpClose { span: Span },
    // Watchdog
    WdtEnable { timeout_ms: Expr, span: Span },
    WdtFeed { span: Span },
    WdtDisable { span: Span },
    // HTTPS
    HttpsGet { url: Expr, target: String, var_type: QBType, span: Span },
    HttpsPost { url: Expr, body: Expr, target: String, var_type: QBType, span: Span },
    // I2S
    I2sInit { rate: Expr, bits: Expr, channels: Expr, span: Span },
    I2sWrite { data: Expr, span: Span },
    I2sStop { span: Span },

    /// Array element assignment: arr(i, j) = expr
    ArrayAssign {
        name: String,
        var_type: QBType,
        indices: Vec<Expr>,
        expr: Expr,
        span: Span,
    },

    /// ASSERT condition [, message]
    Assert {
        condition: Expr,
        message: Option<Expr>,
        span: Span,
    },
    /// FOR EACH var IN array ... NEXT
    ForEach {
        var: String,
        var_type: QBType,
        array_name: String,
        body: Vec<Statement>,
        span: Span,
    },
    /// TRY ... CATCH var$ ... END TRY
    TryCatch {
        try_body: Vec<Statement>,
        catch_var: String,
        catch_body: Vec<Statement>,
        span: Span,
    },
    /// TASK name, stack_size, priority ... END TASK
    Task {
        name: Expr,
        stack_size: Expr,
        priority: Expr,
        body: Vec<Statement>,
        span: Span,
    },
    /// ON GPIO.CHANGE pin GOSUB label
    OnGpioChange {
        pin: Expr,
        target: String,
        span: Span,
    },
    /// ON TIMER interval_ms GOSUB label
    OnTimerEvent {
        interval_ms: Expr,
        target: String,
        span: Span,
    },
    /// ON MQTT.MESSAGE GOSUB label
    OnMqttMessage {
        target: String,
        span: Span,
    },
    /// MachineName.EVENT expr
    MachineEvent {
        machine_name: String,
        event: Expr,
        span: Span,
    },
}

// ── DO...LOOP condition ─────────────────────────────────

#[derive(Debug, Clone)]
pub struct DoCondition {
    pub is_while: bool, // true = WHILE, false = UNTIL
    pub expr: Expr,
}

// ── SELECT CASE clause ──────────────────────────────────

#[derive(Debug, Clone)]
pub struct CaseClause {
    pub tests: Vec<CaseTest>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum CaseTest {
    /// CASE expr
    Value(Expr),
    /// CASE expr TO expr
    Range(Expr, Expr),
    /// CASE IS > expr (comparison)
    Is(BinOp, Expr),
}

// ── IF helpers ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ElseIfClause {
    pub condition: Expr,
    pub body: Vec<Statement>,
    pub span: Span,
}

/// PRINT item types.
#[derive(Debug, Clone)]
pub enum PrintItem {
    Expr(Expr),
    Semicolon,
    Comma,
}

// ── Expressions ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    IntLiteral {
        value: i32,
        span: Span,
    },
    FloatLiteral {
        value: f32,
        span: Span,
    },
    StringLiteral {
        value: String,
        span: Span,
    },
    Variable {
        name: String,
        var_type: QBType,
        span: Span,
    },
    /// myVar.field — struct field access
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    BinaryOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// FnCall: function call or built-in (ABS, INT, CHR$, etc.)
    FnCall {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// Array element access: arr(i, j)
    ArrayAccess {
        name: String,
        var_type: QBType,
        indices: Vec<Expr>,
        span: Span,
    },
    /// LAMBDA (params) => expr
    Lambda {
        params: Vec<(String, QBType)>,
        body: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLiteral { span, .. }
            | Expr::FloatLiteral { span, .. }
            | Expr::StringLiteral { span, .. }
            | Expr::Variable { span, .. }
            | Expr::FieldAccess { span, .. }
            | Expr::BinaryOp { span, .. }
            | Expr::UnaryOp { span, .. }
            | Expr::FnCall { span, .. }
            | Expr::ArrayAccess { span, .. }
            | Expr::Lambda { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv, // \ (integer division)
    Mod,
    Pow,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}
