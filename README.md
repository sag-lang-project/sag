# SAG Programming Language

SAG is a functional programming language implemented in Rust. It is a dynamically typed language.

## Features

- Dynamic typing
- Structs and methods
- Pattern matching
- Option and Result types
- Lambda expressions
- Module system
- **Rational number type** for precise arithmetic operations

## Basic Syntax

### Variable Declaration

```sag
// Immutable variable
val x = 42

// Mutable variable
val mut y = "hello"
```

### Function Definition

```sag
fun add(x: number, y: number): number {
    return x + y
}

// Lambda expression
val add = \|x: number, y: number| => x + y
```

### Control Structures

```sag
// if expression 
val message = if (x > 0) {
    "positive"
} else {
    "negative"
}

// for loop
for i in [1, 2, 3] {
    print(i)
}

// Pattern matching
val result = match (x) {
    1 => { "one" }
    2 => { "two" }
    _ => { "other" }
}
```

### Structs

```sag
struct Point {
    x: number,
    y: number
}

// Struct instantiation
val mut point = Point { x: 1, y: 2 }

// Method implementation
impl Point {
    // Method that modifies struct fields requires mut self
    fun move(mut self, dx: number, dy: number) {
        self.x = self.x + dx
        self.y = self.y + dy
    }

    // Method that only reads fields can use self
    fun show(self) {
        print("Point: x=", self.x, " y=", self.y)
    }
}
point.move(1, 2)
point.show()
```

### Type System

SAG is a dynamically typed language that supports the following value types:

- `number`: Rational number type (fraction) for precise arithmetic operations
- `string`: String type
- `bool`: Boolean type
- `void`: Empty type
- `option<T>`: Option type
- `result<T, E>`: Result type
- `struct`: Struct type
- `function`: Function type
- `lambda`: Lambda type
- `List<T>`: List type with element type T

### Module System

```sag
# Module import
import math from "math.sag"

# Specific symbol import
import { add, sub } from "math.sag"
```

## Built-in Functions

SAG provides the following built-in functions:

- `print(...)`: Prints values to the console
- `len(value)`: Returns the length of a list or string
- `range(start, end, step?)`: Generates a list of numbers from start to end (exclusive) with optional step

## Running

Build and run a `.sag` program with the normal interpreter:

```bash
cargo run -- run your_program.sag
```

Run the same program with the Rc-based interpreter:

```bash
cargo run -- run your_program.sag --use-rc
```

Start the REPL:

```bash
cargo run -- repl
```

Start the Rc REPL:

```bash
cargo run -- repl --use-rc
```

## Compilation

SAG currently compiles to VM-consumable compiled files, not native executables.

- `.sagc`: text compiled format
- `.sagb`: binary compiled format

Compile a program:

```bash
cargo run -- compile your_program.sag
```

This creates `your_program.sag.sagc` next to the source file by default.

Write the compiled output to a specific path:

```bash
cargo run -- compile your_program.sag -o out.sagc
```

Write the binary compiled format instead:

```bash
cargo run -- compile your_program.sag -o out.sagb
```

Run a compiled file:

```bash
cargo run -- run your_program.sag.sagc
cargo run -- run out.sagb
```

## Performance Check

For performance checks, avoid benchmarks that print on every iteration because console I/O dominates runtime.

Use [fib_benchmark.sag](./fib_benchmark.sag) instead. It computes Fibonacci repeatedly and prints only once at the end.

Compare the three execution modes:

```bash
time cargo run --release -- run fib_benchmark.sag
time cargo run --release -- run fib_benchmark.sag --use-rc
time cargo run --release -- compile fib_benchmark.sag
time cargo run --release -- run fib_benchmark.sag.sagc
```

Meaning of each mode:

- `run`: normal interpreter
- `run --use-rc`: Rc-based interpreter
- `run fib_benchmark.sag.sagc`: compiled VM execution

If you only want a functional smoke test, `very_simple.sag` or `loop_test.sag` is enough. If you want to compare speed, use `fib_benchmark.sag`.

## List Operations

Lists can be created using square brackets and support the following operations:

```sag
// List creation
val mut numbers = [1, 2, 3]
val strings = ["hello", "world"]

// List methods
numbers.push(4)  // Adds an element to the end of the list
numbers.len()    // Returns the length of the list

// List operations with built-in functions
len(numbers)     // Returns the length of the list
range(5)         // Returns [0, 1, 2, 3, 4]
range(1, 5)      // Returns [1, 2, 3, 4]
range(1, 5, 2)   // Returns [1, 3]
```

## Error Handling

```sag
// Error handling with Result type
fun divide(a: number, b: number): Result<number, string> {
    return if (b == 0) {
        Fail("division by zero")
    } else {
        Suc(a / b)
    }
}

// Null safety with Option type
fun find(list: List<number>, search_value: number): Option<number> {
    for item in list {
        if (search_value == item) {
            return Some(item)
        }
    }
    return None
}

// Pattern matching with Result and Option
val mut result: Result<number, string> = Suc(1)
val match_result = match (result) {
    Suc(v) => { v + 1 }
    Fail(_) => { 0 }
}

val mut option: Option<number> = Some(1)
val match_option = match (option) {
    Some(v) => { v + 1 }
    None => { 0 }
}
```

## Comments

````sag
// Single line comment
// This is a single line comment


``` 
This is a
multi-line comment
``` 
````
## License

MIT License

## Pipeline Operator

```sag
// Using pipeline operator
val x = 1
x -> print  // prints 1

// Function composition with pipeline
|1, 2| -> f1 -> print
```
