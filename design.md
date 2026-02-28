Objective: Create a simple, strongly and statically typed functional programming language that can compile to multiple backends.

A function is a type; a lambda is an object

A lambda is a data (captures) pointer and a  function pointer. When invoking a lambda, the captures are passed as the first argument and the rest of the arguments are curried.

Functions may need to compile with a version that takes an unused void * parameter if they are invoked as "function pointers"

