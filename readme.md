# Functional C

What if C had lambda functions? What if all it had was lambda functions? This project uses a C-like syntax for a functional language agewith no mutable state, no memory allocations, and no structs (yet). For now, types are limited to ints, floats, strings, and functions thereupon.

## Getting Started

Your program is an expression. When your program runs, your expression will be evaluated and printed. "Hello World" is the simplest program:

```
"Hello World"
```

You can also do all the normal math:

```
2 + 2 * 4
```

There's a ternary operator:

```
2 == 0 ? 1 : -1
```

You can use a let statement to set a local variable, but you can't change it later.

```
let x = 2;
x + x * 4
```

You can create functions with an arrow syntax.

```
let add = (x: int, y: int) => x + y;
add(2, 2 * 4)
```

Functions can also return functions, which can capture variables from their environment.

```
let curry = (x: int) => (y: int) => x + y;
curry(2)(2 * 4)
```
