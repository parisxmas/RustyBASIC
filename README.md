# RustyBASIC

A QBASIC compiler written in Rust that targets the **ESP32-C3** (RISC-V) microcontroller. Write embedded programs in familiar BASIC syntax instead of C/C++.

```
.bas source --> [Lexer] --> [Parser] --> [Sema] --> [LLVM Codegen] --> .o (RISC-V)
                                                                        |
                                                         ESP-IDF link: .o + C runtime --> .elf
```

## Quick Start

### Prerequisites

- Rust toolchain (stable)
- LLVM 18 (`brew install llvm@18` on macOS)
- ESP-IDF toolchain (for firmware/flash commands)
- zstd library (`brew install zstd` on macOS)

### Build

```bash
export LLVM_SYS_180_PREFIX=/opt/homebrew/opt/llvm@18
export LIBRARY_PATH=/opt/homebrew/opt/zstd/lib

cargo build
```

### Usage

```bash
# Check syntax and types
rustybasic program.bas check

# Dump LLVM IR
rustybasic program.bas dump-ir [--target host|esp32c3]

# Compile to object file
rustybasic program.bas build [-o output.o] [--target esp32c3|host]

# Build ESP-IDF firmware
rustybasic program.bas firmware [--project-dir esp-project]

# Flash to device
rustybasic program.bas flash --port /dev/ttyUSB0
```

## Examples

### Hello World

```basic
' Hello World for RustyBASIC (QBASIC dialect)
DIM msg AS STRING
msg = "Hello from RustyBASIC!"

PRINT msg
PRINT "Running on ESP32-C3"

DIM answer AS INTEGER
answer = 42
PRINT "The answer is:"; answer

END
```

### FizzBuzz

```basic
DIM i AS INTEGER

FOR i = 1 TO 100
    SELECT CASE 0
        CASE i MOD 15
            PRINT "FizzBuzz"
        CASE i MOD 3
            PRINT "Fizz"
        CASE i MOD 5
            PRINT "Buzz"
        CASE ELSE
            PRINT i
    END SELECT
NEXT i

END
```

### Blink LED (ESP32-C3)

```basic
CONST LED_PIN = 2
CONST OUTPUT_MODE = 1

GPIO.MODE LED_PIN, OUTPUT_MODE

DO
    GPIO.SET LED_PIN, 1
    PRINT "LED ON"
    DELAY 500

    GPIO.SET LED_PIN, 0
    PRINT "LED OFF"
    DELAY 500
LOOP
```

### Button Input (GPIO.READ)

```basic
' Reads a push button and toggles an LED

CONST BUTTON_PIN = 9
CONST LED_PIN = 2
CONST INPUT_MODE = 0
CONST OUTPUT_MODE = 1

DIM state AS INTEGER
DIM ledOn AS INTEGER

GPIO.MODE BUTTON_PIN, INPUT_MODE
GPIO.MODE LED_PIN, OUTPUT_MODE

ledOn = 0
GPIO.SET LED_PIN, 0

PRINT "Press the button to toggle LED..."

DO
    GPIO.READ BUTTON_PIN, state

    IF state = 0 THEN
        ' Button pressed (active low)
        IF ledOn = 0 THEN
            ledOn = 1
            GPIO.SET LED_PIN, 1
            PRINT "LED ON"
        ELSE
            ledOn = 0
            GPIO.SET LED_PIN, 0
            PRINT "LED OFF"
        END IF

        ' Simple debounce
        DELAY 300
    END IF

    DELAY 50
LOOP
```

### SUB and FUNCTION Procedures

```basic
DECLARE SUB ShowResult (label AS STRING, value AS SINGLE)
DECLARE FUNCTION Add! (a AS SINGLE, b AS SINGLE)

DIM a AS SINGLE
DIM b AS SINGLE

INPUT "First number: "; a
INPUT "Second number: "; b

CALL ShowResult("A + B", Add!(a, b))
CALL ShowResult("A * B", a * b)

END

SUB ShowResult (label AS STRING, value AS SINGLE)
    PRINT label; " = "; value
END SUB

FUNCTION Add! (a AS SINGLE, b AS SINGLE)
    Add! = a + b
END FUNCTION
```

### User-Defined Types (Structs)

```basic
TYPE Point
    x AS SINGLE
    y AS SINGLE
END TYPE

TYPE Rect
    left AS SINGLE
    top AS SINGLE
    width AS SINGLE
    height AS SINGLE
END TYPE

DECLARE FUNCTION RectArea! (r AS Rect)
DECLARE SUB PrintPoint (p AS Point)

DIM origin AS Point
origin.x = 0.0
origin.y = 0.0

DIM cursor AS Point
cursor.x = 10.5
cursor.y = 20.3

CALL PrintPoint(origin)
CALL PrintPoint(cursor)

DIM r AS Rect
r.left = 0
r.top = 0
r.width = 100
r.height = 50

PRINT "Rectangle area:"; RectArea!(r)

END

SUB PrintPoint (p AS Point)
    PRINT "Point("; p.x; ","; p.y; ")"
END SUB

FUNCTION RectArea! (r AS Rect)
    RectArea! = r.width * r.height
END FUNCTION
```

### Arrays

```basic
' 1D array (indices 0..5)
DIM arr(5) AS INTEGER
arr(0) = 10
arr(1) = 20
arr(2) = 30
PRINT arr(0); arr(1); arr(2)

' 2D array (indices 0..2 x 0..3)
DIM matrix(2, 3) AS SINGLE
matrix(1, 2) = 99.5
PRINT "Matrix(1,2) ="; matrix(1, 2)
```

### I2C Communication

```basic
' Reads temperature from a BMP280 sensor over I2C

CONST I2C_BUS = 0
CONST SDA_PIN = 4
CONST SCL_PIN = 5
CONST I2C_FREQ = 100000

CONST BMP280_ADDR = 118
CONST REG_CHIP_ID = 208
CONST REG_TEMP_MSB = 250

DIM chipId AS INTEGER
DIM rawTemp AS INTEGER

PRINT "Initializing I2C bus..."
I2C.SETUP I2C_BUS, SDA_PIN, SCL_PIN, I2C_FREQ

' Write register address, then read chip ID
I2C.WRITE BMP280_ADDR, REG_CHIP_ID
I2C.READ BMP280_ADDR, 1, chipId
PRINT "Chip ID:"; chipId

IF chipId = 88 THEN
    PRINT "BMP280 detected!"

    ' Read raw temperature MSB
    I2C.WRITE BMP280_ADDR, REG_TEMP_MSB
    I2C.READ BMP280_ADDR, 1, rawTemp
    PRINT "Raw temp MSB:"; rawTemp
ELSE
    PRINT "Unknown device"
END IF

END
```

### SPI Communication

```basic
' Read the WHO_AM_I register from an SPI sensor (e.g. BME280)

CONST SPI_BUS = 2
CONST CLK_PIN = 6
CONST MOSI_PIN = 7
CONST MISO_PIN = 2
CONST SPI_FREQ = 1000000

DIM response AS INTEGER

PRINT "Initializing SPI bus..."
SPI.SETUP SPI_BUS, CLK_PIN, MOSI_PIN, MISO_PIN, SPI_FREQ

PRINT "Reading WHO_AM_I register..."
SPI.TRANSFER 208, response
PRINT "Device ID:"; response

IF response = 96 THEN
    PRINT "BME280 detected!"
ELSEIF response = 88 THEN
    PRINT "BMP280 detected!"
ELSE
    PRINT "Unknown device"
END IF

END
```

### WiFi Disconnect

```basic
' Demonstrates the full WiFi lifecycle

DIM ssid AS STRING
DIM pass AS STRING
DIM status AS INTEGER

ssid = "MyNetwork"
pass = "MyPassword"

PRINT "Connecting to WiFi..."
WIFI.CONNECT ssid, pass
DELAY 3000

WIFI.STATUS status
IF status = 1 THEN
    PRINT "Connected!"

    PRINT "Doing some work..."
    DELAY 2000

    PRINT "Disconnecting..."
    WIFI.DISCONNECT
    DELAY 1000

    WIFI.STATUS status
    IF status = 0 THEN
        PRINT "Disconnected successfully."
    ELSE
        PRINT "Still connected."
    END IF
ELSE
    PRINT "Failed to connect."
END IF

END
```

### INCLUDE Directive

Split code across multiple files with `INCLUDE`:

**include_lib.bas**
```basic
CONST PI = 3.14159
CONST GREETING = "Hello from the library!"

DECLARE SUB PrintBanner (title AS STRING)
DECLARE FUNCTION CircleArea! (radius AS SINGLE)

SUB PrintBanner (title AS STRING)
    PRINT "==========================="
    PRINT " "; title
    PRINT "==========================="
END SUB

FUNCTION CircleArea! (radius AS SINGLE)
    CircleArea! = PI * radius * radius
END FUNCTION
```

**include_main.bas**
```basic
INCLUDE "include_lib.bas"

DIM r AS SINGLE
r = 5.0

CALL PrintBanner(GREETING)
PRINT "Radius:"; r
PRINT "Area:"; CircleArea!(r)

END
```

Paths are resolved relative to the including file's directory. Circular includes are detected and reported as errors.

### DO...LOOP Variations

```basic
DIM count AS INTEGER
DIM sum AS INTEGER

' Pre-condition: DO WHILE...LOOP
count = 1
sum = 0
DO WHILE count <= 10
    sum = sum + count
    count = count + 1
LOOP
PRINT "Sum 1..10 ="; sum

' Post-condition: DO...LOOP UNTIL
count = 10
DO
    PRINT count;
    count = count - 1
LOOP UNTIL count < 1
PRINT

' EXIT DO
count = 0
DO
    count = count + 1
    IF count = 5 THEN EXIT DO
LOOP
PRINT "Exited at count ="; count

END
```

## Language Reference

### Types

| Type | Suffix | Description |
|------|--------|-------------|
| `INTEGER` | `%` | 32-bit signed integer |
| `LONG` | `&` | 32-bit signed long |
| `SINGLE` | `!` | 32-bit float |
| `DOUBLE` | `#` | 64-bit float (soft-float on ESP32) |
| `STRING` | `$` | Reference-counted heap string |
| User type | — | Defined with `TYPE...END TYPE` |

### Operators

| Category | Operators |
|----------|-----------|
| Arithmetic | `+` `-` `*` `/` `\` (int div) `^` (power) `MOD` |
| Comparison | `=` `<>` `<` `>` `<=` `>=` |
| Logical | `AND` `OR` `NOT` `XOR` |

### Arrays

| Statement | Description |
|-----------|-------------|
| `DIM arr(N) AS type` | Declare 1D array with indices 0..N |
| `DIM mat(R, C) AS type` | Declare multi-dimensional array |
| `arr(i) = expr` | Assign to array element |
| `arr(i)` | Read array element (in expressions) |

Arrays are fixed-size, heap-allocated, zero-initialized, and bounds-checked at runtime.

### Control Flow

| Statement | Description |
|-----------|-------------|
| `IF...THEN...ELSEIF...ELSE...END IF` | Conditional branching |
| `SELECT CASE...CASE...CASE ELSE...END SELECT` | Multi-way branch |
| `FOR...TO...STEP...NEXT` | Counted loop |
| `DO WHILE/UNTIL...LOOP` | Pre-condition loop |
| `DO...LOOP WHILE/UNTIL` | Post-condition loop |
| `WHILE...WEND` | Legacy while loop |
| `GOTO label` | Unconditional jump |
| `GOSUB label` / `RETURN` | Subroutine call/return |
| `EXIT FOR/DO/SUB/FUNCTION` | Early exit |

### Preprocessor

| Directive | Description |
|-----------|-------------|
| `INCLUDE "file.bas"` | Inline another source file (resolved relative to current file) |

### I/O

| Statement | Description |
|-----------|-------------|
| `PRINT expr; expr` | Output (`;` = no space, `,` = tab) |
| `INPUT "prompt"; var` | Read user input |
| `LINE INPUT "prompt"; var$` | Read entire line |

### Hardware (ESP32-C3)

| Statement | Description |
|-----------|-------------|
| `GPIO.MODE pin, mode` | Configure GPIO pin |
| `GPIO.SET pin, value` | Write digital output |
| `GPIO.READ pin, var` | Read digital input |
| `I2C.SETUP bus, sda, scl, freq` | Initialize I2C |
| `I2C.WRITE addr, data` | Write to I2C device |
| `I2C.READ addr, len, var` | Read from I2C device |
| `SPI.SETUP bus, clk, mosi, miso, freq` | Initialize SPI |
| `SPI.TRANSFER data, var` | SPI send/receive |
| `WIFI.CONNECT ssid, password` | Connect to WiFi |
| `WIFI.STATUS var` | Check WiFi status |
| `WIFI.DISCONNECT` | Disconnect WiFi |
| `DELAY ms` | Pause execution (milliseconds) |

## Project Structure

```
RustyBASIC/
├── Cargo.toml                     # Workspace root
├── crates/
│   ├── rustybasic-common/         # Span, source types
│   ├── rustybasic-lexer/          # logos-based tokenizer
│   ├── rustybasic-parser/         # Recursive descent parser + AST
│   ├── rustybasic-sema/           # Type checking, scope resolution
│   ├── rustybasic-codegen/        # LLVM IR generation (inkwell)
│   └── rustybasic-driver/         # CLI entry point
├── runtime/                       # C runtime library (ESP-IDF component)
│   ├── include/rb_runtime.h
│   └── src/                       # rb_print.c, rb_string.c, rb_array.c, rb_gpio.c, ...
├── esp-project/                   # ESP-IDF project template for linking
├── examples/                      # Example .bas programs
│   ├── hello.bas
│   ├── fizzbuzz.bas
│   ├── blink.bas
│   ├── calculator.bas
│   ├── arrays.bas
│   ├── structs.bas
│   ├── button.bas
│   ├── doloop.bas
│   ├── i2c.bas
│   ├── spi.bas
│   ├── wifi_disconnect.bas
│   ├── wifi_scan.bas
│   ├── include_main.bas
│   └── include_lib.bas
└── tests/
```

## Design Decisions

| Area | Choice | Rationale |
|------|--------|-----------|
| Lexer | logos crate | Zero-copy, fast, case-insensitive keywords |
| Parser | Hand-written recursive descent | BASIC's line-oriented grammar needs custom handling |
| Expressions | Pratt parsing (precedence climbing) | Clean operator precedence |
| Variables | alloca + LLVM mem2reg | Standard pattern, avoids manual phi nodes |
| Strings | Refcounted heap (`rb_string_t*`) | Memory-efficient for ESP32-C3's 320KB RAM |
| Floats | f32 (not f64) | No hardware FPU; f32 is 2x cheaper in soft-float |
| Runtime | C library linked via ESP-IDF | Direct access to ESP-IDF APIs |
| Target | `riscv32-unknown-none-elf` | ESP32-C3 = RV32IMC |

## License

MIT
