Objective: Create a simple, strongly and statically typed functional programming language that can compile to multiple backends.

### Lambdas

This example approximates how an `(int, int) -> int` lambda would compile. The first, `lambda_pointers` struct would be generated for the type; so any lambda of the same type would use it. The `f` pointer contains the body of the function, described later. The `d` pointer contains a function that must be called before a pointer goes out of scope. The refcount is the number of references that are maintained. The second structure, `specific_lambda`, is only used by a single lambda invocation to store its captures, in this case ints `x` and `y`. The body of the `f` function will cast the `lambda_pointers` parameter to `specific_lambda` so that it can use the captures.

```c
struct lambda_pointers {
    int (*f)(struct lambda_pointers *, int, int);
    void (*d)(struct lambda_pointers *);
    int refcount;
};

struct specific_lambda {
    struct lambda_pointers p;
    int x;
    int y;
}
```

# TODO

- Generics
- Recursion
- IO Functions
- Structs
- Type aliases
