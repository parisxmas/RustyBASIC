# RustyBASIC New Language Features

This document describes the 10 advanced language features added to RustyBASIC, extending the classic QBASIC dialect with modern programming constructs while maintaining the simplicity that makes BASIC great for embedded development.

## Table of Contents

1. [ASSERT](#1-assert)
2. [ENUM](#2-enum)
3. [FOR EACH](#3-for-each)
4. [String Interpolation](#4-string-interpolation)
5. [TRY/CATCH](#5-trycatch)
6. [LAMBDA](#6-lambda)
7. [TASK](#7-task)
8. [EVENT System](#8-event-system)
9. [State Machine DSL](#9-state-machine-dsl)
10. [MODULE Namespaces](#10-module-namespaces)

---

## 1. ASSERT

Runtime assertions for defensive programming. If the condition is false, the program aborts with an error message and source offset.

### Syntax

```
ASSERT condition
ASSERT condition, "error message"
```

### Example

```basic
DIM x AS INTEGER
x = 42

ASSERT x > 0, "x must be positive"
ASSERT x = 42
PRINT "All assertions passed!"
END
```

### Details

- The condition is evaluated as an integer (0 = false, non-zero = true)
- If the assertion fails, the program prints the message to stderr and aborts
- The optional message is a string expression
- Useful for catching invalid states early during development
- Runtime: calls `rb_assert_fail(message, offset)`

---

## 2. ENUM

Named integer constants grouped under a type name. Members are accessed with dot notation (`EnumName.Member`).

### Syntax

```
ENUM Name
    Member1 [= value]
    Member2 [= value]
    ...
END ENUM
```

### Example

```basic
ENUM Color
    Red = 1
    Green = 2
    Blue = 3
END ENUM

ENUM Direction
    North
    East
    South
    West
END ENUM

DIM c AS INTEGER
c = Color.Green
PRINT "Color:"; c           ' prints 2

DIM d AS INTEGER
d = Direction.South
PRINT "Direction:"; d        ' prints 2 (auto-incremented from 0)
END
```

### Details

- Members auto-increment starting from 0 unless an explicit value is given
- Explicit values restart the auto-increment counter from that value
- Enum members are resolved at compile time as integer constants (no runtime overhead)
- Dot notation is required: `Color.Red`, not just `Red`

---

## 3. FOR EACH

Iterate over all elements of an array without manual index management.

### Syntax

```
FOR EACH var [AS type] IN array_name
    ...body...
NEXT
```

### Example

```basic
DIM scores(4) AS INTEGER
scores(0) = 90
scores(1) = 85
scores(2) = 92
scores(3) = 78
scores(4) = 95

DIM total AS INTEGER
total = 0

FOR EACH s AS INTEGER IN scores
    PRINT "Score:"; s
    total = total + s
NEXT

PRINT "Total:"; total
PRINT "Average:"; total / 5
END
```

### Details

- The loop variable is automatically assigned each element in order
- The `AS type` clause is optional if the type can be inferred
- Works with arrays of any type (INTEGER, SINGLE, STRING)
- The array must be declared with `DIM` before use
- Internally compiles to a counted loop with bounds checking

---

## 4. String Interpolation

Embed expressions directly in strings using the `$"..."` syntax with `{expr}` placeholders.

### Syntax

```
$"text {expression} more text {expression}"
```

### Example

```basic
DIM name AS STRING
DIM age AS INTEGER
name = "Alice"
age = 30

PRINT $"Hello, {name}! You are {age} years old."
PRINT $"Next year you'll be {age + 1}."
PRINT $"Name length: {LEN(name)} characters"
END
```

### Details

- Expressions inside `{...}` are evaluated and converted to strings
- Numeric expressions are automatically wrapped in `STR$()`
- String expressions are used directly
- The interpolated string is desugared in the parser to a chain of string concatenations
- No runtime overhead beyond normal string operations
- Nested braces are not supported

---

## 5. TRY/CATCH

Structured error handling with try/catch blocks, replacing the older `ON ERROR GOTO` pattern.

### Syntax

```
TRY
    ...try body...
CATCH errorVar
    ...catch body...
END TRY
```

### Example

```basic
PRINT "Before try"

TRY
    PRINT "Inside try block"
    PRINT "Doing some work..."
CATCH err
    PRINT "Caught error: "; err
END TRY

PRINT "After try/catch"
END
```

### Details

- The catch variable receives the error message as a string
- Implemented using setjmp/longjmp at the runtime level
- Supports nesting up to 16 levels deep
- When `rb_panic` is called inside a TRY block, control transfers to the CATCH block instead of aborting
- The `rb_throw(message)` runtime function can be called to trigger a catch
- Runtime functions: `rb_try_begin()`, `rb_try_end()`, `rb_throw()`, `rb_get_error_message()`

---

## 6. LAMBDA

Anonymous function expressions that can be assigned to variables and called indirectly.

### Syntax

```
LAMBDA(param AS type [, param AS type, ...]) => expression
```

### Example

```basic
DIM square AS FUNCTION
square = LAMBDA(x AS INTEGER) => x * x

DIM result AS INTEGER
result = square(5)
PRINT "5 squared = "; result

DIM add AS FUNCTION
add = LAMBDA(a AS INTEGER, b AS INTEGER) => a + b
PRINT "3 + 4 = "; add(3, 4)
END
```

### Details

- Lambda expressions create anonymous LLVM functions at compile time
- Variables of type `FUNCTION` hold function pointers
- The return type is inferred from the body expression
- Parameters require explicit type annotations
- No closure capture — lambdas can only use their own parameters
- Indirect function calls are supported when the variable holds a function pointer

---

## 7. TASK

Spawn concurrent tasks for parallel execution. Uses FreeRTOS `xTaskCreate` on ESP32 and `pthread_create` on host.

### Syntax

```
TASK name_expr, stack_size, priority
    ...body...
END TASK
```

### Example

```basic
PRINT "Main task starting"

TASK "blinker", 2048, 1
    PRINT "Blinker task running"
    DELAY 1000
    PRINT "Blinker task done"
END TASK

TASK "sensor", 4096, 2
    PRINT "Sensor task running"
    DELAY 500
    PRINT "Sensor reading complete"
END TASK

PRINT "Main task continues"
DELAY 2000
PRINT "Done"
END
```

### Details

- `name_expr` is a string expression used as the task name
- `stack_size` is the stack allocation in bytes (ESP32 recommendation: 2048-8192)
- `priority` is the FreeRTOS task priority (higher = more priority)
- The task body is compiled into a separate LLVM function
- On ESP32: uses `xTaskCreate()` from FreeRTOS
- On host: uses `pthread_create()` with a detached thread
- Tasks run independently and do not share variables with the main program
- Runtime: calls `rb_task_create(fn_ptr, name, stack_size, priority)`

---

## 8. EVENT System

Register callback handlers for hardware events, enabling event-driven programming.

### Syntax

```
ON GPIO.CHANGE pin GOSUB label
ON TIMER interval_ms GOSUB label
ON MQTT.MESSAGE GOSUB label
```

### Example

```basic
DIM count AS INTEGER
count = 0

ON TIMER 1000 GOSUB tick
PRINT "Timer event registered"
DELAY 5000
PRINT "Final count: "; count
END

tick:
    count = count + 1
    PRINT "Tick!"
RETURN
```

### Details

- **GPIO.CHANGE**: Triggers when a GPIO pin changes state (rising/falling edge)
- **TIMER**: Triggers periodically at the specified interval in milliseconds
- **MQTT.MESSAGE**: Triggers when an MQTT message is received
- The target must be a valid label in the program
- On ESP32: uses GPIO ISR handlers and `esp_timer` for periodic timers
- On host: provides stub implementations that log the registration
- Runtime functions: `rb_on_gpio_change()`, `rb_on_timer()`, `rb_on_mqtt_message()`

---

## 9. State Machine DSL

Define finite state machines with named states and event-driven transitions.

### Syntax

```
MACHINE MachineName
    STATE StateName
        ON EventName GOTO TargetState
        ...
    END STATE
    ...
END MACHINE

MachineName.EVENT "event_string"
```

### Example

```basic
MACHINE TrafficLight
    STATE RED
        ON TIMER GOTO GREEN
    END STATE
    STATE GREEN
        ON TIMER GOTO YELLOW
    END STATE
    STATE YELLOW
        ON TIMER GOTO RED
    END STATE
END MACHINE

PRINT "Traffic light created"
TrafficLight.EVENT "TIMER"
PRINT "After first timer event"
TrafficLight.EVENT "TIMER"
PRINT "After second timer event"
END
```

### Details

- Machines are initialized at program startup with the first defined state as the initial state
- Transitions are event-driven: `MachineName.EVENT expr$` sends a named event
- If the current state has a matching transition, the machine moves to the target state
- State and event names are strings at runtime
- The semantic analyzer validates that all transition targets reference existing states
- Supports up to 8 machines, 16 states per machine, and 64 transitions per machine
- Runtime functions: `rb_machine_create()`, `rb_machine_add_state()`, `rb_machine_add_transition()`, `rb_machine_event()`, `rb_machine_get_state()`

---

## 10. MODULE Namespaces

Group related SUBs and FUNCTIONs under a named namespace, accessed with dot notation.

### Syntax

```
MODULE Name
    SUB SubName(params)
        ...
    END SUB

    FUNCTION FuncName(params) AS type
        ...
    END FUNCTION
END MODULE
```

### Example

```basic
MODULE Math
    FUNCTION Square(x AS INTEGER) AS INTEGER
        Square = x * x
    END FUNCTION

    FUNCTION Cube(x AS INTEGER) AS INTEGER
        Cube = x * x * x
    END FUNCTION
END MODULE

MODULE StringUtils
    SUB PrintBanner(title AS STRING)
        PRINT "=== "; title; " ==="
    END SUB
END MODULE

DIM a AS INTEGER
a = Math.Square(4)
PRINT "4 squared = "; a
a = Math.Cube(3)
PRINT "3 cubed = "; a

CALL StringUtils.PrintBanner("Hello!")
END
```

### Details

- Module members are accessed with `ModuleName.MemberName` dot notation
- The parser prefixes member names with the module name at parse time
- Modules can contain SUBs and FUNCTIONs (not variables or types)
- No runtime overhead — module namespacing is purely a compile-time mechanism
- Prevents name collisions when organizing larger programs
- Module members are compiled alongside regular SUBs/FUNCTIONs

---

## Platform Support

| Feature | ESP32-C3 | Host (macOS/Linux) |
|---------|----------|-------------------|
| ASSERT | Aborts via `abort()` | Aborts via `abort()` |
| ENUM | Compile-time constants | Compile-time constants |
| FOR EACH | Full support | Full support |
| String Interpolation | Full support | Full support |
| TRY/CATCH | setjmp/longjmp | setjmp/longjmp |
| LAMBDA | LLVM function pointers | LLVM function pointers |
| TASK | FreeRTOS `xTaskCreate` | `pthread_create` (detached) |
| EVENT (GPIO) | GPIO ISR handler | Stub (logs registration) |
| EVENT (Timer) | `esp_timer` periodic | Stub (logs registration) |
| EVENT (MQTT) | MQTT callback | Stub (logs registration) |
| State Machine | Full support | Full support |
| MODULE | Compile-time namespacing | Compile-time namespacing |
