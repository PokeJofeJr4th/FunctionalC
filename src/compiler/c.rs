use crate::{
    compiler::Compiler,
    interpreter::representation::{
        ArithmeticOperator, BooleanOperator, Builtin, ComparisonOperator, IRExpr, IRType, IRValue,
        LValue,
    },
};
use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Write},
};

pub struct CCompiler {
    constants: HashMap<LValue, CompileResult>,
    typedecls: HashSet<IRType>,
    typedefs: String,
    funcdefs: String,
    builtins: HashSet<Builtin>,
    return_struct: LValue,
    return_func: LValue,
    write_line_struct: LValue,
    write_line_func: LValue,
}

#[derive(Clone)]
enum CompileResult {
    /// Something that needs to be computed once; can't be duplicated
    Computation(String),
    /// Something that doesn't need to be computed (like a variable name); can be duplicated in source code
    BaseValue(String),
    /// A function
    ConstFunction {
        /// Name of the static struct referenced
        struct_name: LValue,
        /// Name of the non-lambda function implementation (for optimization)
        func_name: LValue,
    },
}

impl Display for CompileResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Computation(s) => write!(f, "{s}"),
            Self::BaseValue(l) => write!(f, "{l}"),
            Self::ConstFunction {
                struct_name,
                func_name: _,
            } => write!(f, "(&{struct_name})"),
        }
    }
}

impl Compiler for CCompiler {
    type Output = String;
    type Error = String;

    fn compile(mut self, expr: &IRValue) -> Result<Self::Output, Self::Error> {
        let (print_spec, is_monad) = match &expr.type_hint() {
            IRType::Float => ("\"%f\\n\"", false),
            IRType::String => ("\"%s\\n\"", false),
            IRType::Int | IRType::Boolean => ("\"%i\"", false),
            IRType::IOMonad(Some(ty)) => (
                match **ty {
                    IRType::Float => "\"%f\\n\"",
                    IRType::String => "\"%s\\n\"",
                    IRType::Int | IRType::Boolean => "\"%i\\n\"",
                    _ => return Err(format!("Expected a primitive type; got `{ty:?}`")),
                },
                true,
            ),
            IRType::IOMonad(None) => ("", true),
            IRType::Function { .. } => {
                return Err(format!(
                    "Expected a primitive type or `IO<T>`; got `{:?}`",
                    expr.type_hint()
                ));
            }
        };
        let mut prelude = String::new();
        let mut cleanup = String::new();
        let expr_type = self.write_type(expr.type_hint());
        let expr = self.compile_expr(expr, &mut prelude, &mut cleanup, HashMap::new())?;
        if self.builtins.contains(&Builtin::ReadLine) {
            let lv = LValue::new();
            let monad_ty = self.write_type(&IRType::IOMonad(Some(Box::new(IRType::String))));
            writeln!(
                self.typedefs,
                "{getline}char *{lv}({monad_ty} _) {{size_t size; char *line=NULL; getline(&line, &size, stdin);return line;}}{monad_ty_short} readLine = {{.f={lv}, .refcount=-1}};",
                monad_ty_short=monad_ty.replace('*', ""),
                getline=include_str!("../getline")
            )
            .unwrap();
        }
        if self.builtins.contains(&Builtin::WriteLine) {
            let monad_ty = IRType::IOMonad(None);
            let monad_ty_name = self.write_type(&monad_ty);
            let func_ty = IRType::Function {
                inputs: vec![IRType::String],
                output: Box::new(monad_ty),
            };
            let func_ty_name = self.write_type(&func_ty);
            writeln!(self.typedefs, "typedef struct _writeLine_captures {{{monad_ty_name_short} lambda;char *str;}} _writeLine_captures;\nvoid writeLine_inner({monad_ty_name} captures){{printf(\"%s\\n\", ((_writeLine_captures *)captures)->str);}}\n{monad_ty_name} {wl}(char *str){{\n_writeLine_captures *wl = malloc(sizeof(*wl));\nwl->lambda.f=writeLine_inner;\nwl->lambda.d=NULL;\nwl->lambda.refcount=1;\nwl->str=str;\nreturn (_io_void*)wl;\n}}\n{monad_ty_name} _writeLine_lambda({func_ty_name} _, char *str){{\n_writeLine_captures *wl = malloc(sizeof(*wl));\nwl->lambda.f=writeLine_inner;\nwl->lambda.d=NULL;\nwl->lambda.refcount=1;\nwl->str=str;\nreturn (_io_void*)wl;\n}}\n{func_ty_name_short} {wls} = {{.f=_writeLine_lambda, .refcount=-1}};", wl=self.write_line_func, wls=self.write_line_struct,monad_ty_name_short = monad_ty_name.replace('*', ""), func_ty_name_short = func_ty_name.replace('*', "")).unwrap();
        }
        if is_monad {
            if print_spec.is_empty() {
                Ok(format!(
                    "#include <stdio.h>\n#include <stdlib.h>\n{}{}\nint main(){{{prelude}{expr_type} _monad = {expr};_monad->f(_monad);{cleanup}}}",
                    self.typedefs, self.funcdefs
                ))
            } else {
                Ok(format!(
                    "#include <stdio.h>\n#include <stdlib.h>\n{}{}\nint main(){{{prelude}{expr_type} _monad = {expr};printf({print_spec},_monad->f(_monad));{cleanup}}}",
                    self.typedefs, self.funcdefs
                ))
            }
        } else {
            Ok(format!(
                "#include <stdio.h>\n#include <stdlib.h>\n{}{}\nint main(){{{prelude}printf({print_spec},{expr});{cleanup}}}",
                self.typedefs, self.funcdefs
            ))
        }
    }
}

impl CCompiler {
    pub fn new() -> Self {
        let mut s = Self {
            constants: HashMap::new(),
            typedecls: HashSet::new(),
            typedefs: String::new(),
            funcdefs: String::new(),
            builtins: HashSet::new(),
            return_struct: LValue::new(),
            return_func: LValue::new(),
            write_line_struct: LValue::new(),
            write_line_func: LValue::new(),
        };

        let io_monad = s.write_type(&IRType::IOMonad(None));
        let first_lv = LValue::new();
        let second_lv = LValue::new();
        let inner_lv = LValue::new();
        let (caps, free) = s.compile_lambda_captures_structure(
            &[
                (first_lv, IRType::IOMonad(None)),
                (second_lv, IRType::IOMonad(None)),
            ],
            inner_lv,
            &io_monad,
        );

        writeln!(
            s.funcdefs,
            "
void {inner_lv}({io_monad} c) {{
    (({caps}*)c)->{first_lv}->f((({caps}*)c)->{first_lv});
    (({caps}*)c)->{second_lv}->f((({caps}*)c)->{second_lv});
}}
{io_monad} _compose_monads({io_monad} {first_lv}, {io_monad} {second_lv}) {{
    {caps} *monad = ({caps}*)malloc(sizeof(*monad));
    monad->lambda.f = {inner_lv};
    monad->lambda.d = {free};
    monad->lambda.refcount = 1;
    monad->{first_lv}={first_lv};
    monad->{second_lv}={second_lv};
    return ({io_monad})monad;
}}"
        )
        .unwrap();

        s
    }

    fn short_type(&mut self, ty: &IRType) -> String {
        match ty {
            IRType::String => "str".to_string(),
            ty => self.write_type(ty).replace('*', ""),
        }
    }

    fn write_type(&mut self, ty: &IRType) -> String {
        match ty {
            IRType::Int | IRType::Boolean => "int".to_string(),
            IRType::Float => "float".to_string(),
            IRType::String => "char *".to_string(),
            IRType::Function { inputs, output } => {
                let mut short_name = format!("_func_{}", self.short_type(output));
                for i in inputs {
                    short_name.push('_');
                    short_name.push_str(&self.short_type(i));
                }
                short_name.push('_');
                if !self.typedecls.contains(ty) {
                    self.typedecls.insert(ty.clone());
                    let mut lambda_typedecl = format!(
                        "typedef struct {short_name} {{{} (*f)(struct {short_name}*",
                        self.write_type(output)
                    );
                    for input in inputs {
                        lambda_typedecl.push(',');
                        lambda_typedecl.push_str(&self.write_type(input));
                    }
                    writeln!(
                        self.typedefs,
                        "{lambda_typedecl}); void (*d)(struct {short_name}*); int refcount;}} {short_name};"
                    )
                    .unwrap();
                }
                format!("{short_name}*")
            }
            IRType::IOMonad(t) => {
                let output_ty = t
                    .as_ref()
                    .map_or_else(|| "void".to_string(), |t| self.write_type(t));
                let short_name = format!(
                    "_io_{}",
                    t.as_ref()
                        .map_or_else(|| "void".to_string(), |t| self.short_type(t))
                );
                if !self.typedecls.contains(ty) {
                    self.typedecls.insert(ty.clone());
                    writeln!(self.typedefs, "typedef struct {short_name} {{{output_ty} (*f)(struct {short_name}*); void(*d)(struct {short_name}*); int refcount;}} {short_name};").unwrap();
                }
                format!("{short_name}*")
            }
        }
    }

    fn compile_expr(
        &mut self,
        expr: &IRValue,
        prelude: &mut String,
        cleanup: &mut String,
        mut shadows: HashMap<LValue, CompileResult>,
    ) -> Result<CompileResult, String> {
        match &expr.expr() {
            IRExpr::GetLocal(lvalue) => Ok(self
                .constants
                .get(lvalue)
                .or_else(|| shadows.get(lvalue))
                .cloned()
                .unwrap_or_else(|| CompileResult::BaseValue(format!("{lvalue}")))),
            IRExpr::SetLocal(lvalue, value, body) => {
                let val = self.compile_expr(value, prelude, cleanup, shadows.clone())?;
                match val {
                    CompileResult::ConstFunction { .. } | CompileResult::BaseValue(_) => {
                        shadows.insert(*lvalue, val);
                    }
                    CompileResult::Computation(val) => {
                        write!(
                            prelude,
                            "{} {lvalue}={val};",
                            self.write_type(value.type_hint())
                        )
                        .unwrap();
                    }
                }
                self.compile_expr(body, prelude, cleanup, shadows)
            }
            IRExpr::Int(i) => Ok(CompileResult::BaseValue(format!("{i}"))),
            IRExpr::Float(f) => Ok(CompileResult::BaseValue(format!("{f}"))),
            IRExpr::String(s) => Ok(CompileResult::BaseValue(format!("{s:?}"))),
            IRExpr::If {
                condition,
                body,
                else_body,
            } => self.compile_if_else(expr, prelude, cleanup, shadows, condition, body, else_body),
            IRExpr::Arithmetic(lhs, op, rhs) => {
                self.compile_arithmetic(prelude, cleanup, shadows, lhs, *op, rhs)
            }
            IRExpr::Comparison(lhs, op, rhs) => {
                self.compile_comparison(prelude, cleanup, shadows, lhs, *op, rhs)
            }
            IRExpr::Boolean(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude, cleanup, shadows.clone())?;
                let rhs = self.compile_expr(rhs, prelude, cleanup, shadows)?;
                match op {
                    BooleanOperator::And => {
                        Ok(CompileResult::Computation(format!("({lhs}&&{rhs})")))
                    }
                    BooleanOperator::Or => {
                        Ok(CompileResult::Computation(format!("({lhs}||{rhs})")))
                    }
                }
            }
            IRExpr::Function {
                params,
                body,
                captures,
            } => self.compile_function(expr, prelude, cleanup, params, body, captures),
            IRExpr::FunctionCall(func, args) => {
                self.compile_function_call(prelude, cleanup, shadows, func, args)
            }
            IRExpr::BindIoMonad {
                var_name: var,
                var_value: val,
                body,
                captures,
            } => self.compile_bind_io_monad(prelude, cleanup, shadows, *var, val, body, captures),
            IRExpr::Builtin(b) => {
                self.builtins.insert(*b);
                match b {
                    Builtin::Return => Ok(CompileResult::ConstFunction {
                        struct_name: self.return_struct,
                        func_name: self.return_func,
                    }),
                    Builtin::ReadLine => Ok(CompileResult::BaseValue("(&readLine)".to_string())),
                    Builtin::WriteLine => Ok(CompileResult::ConstFunction {
                        struct_name: self.write_line_struct,
                        func_name: self.write_line_func,
                    }),
                }
            }
            IRExpr::ComposeMonads(first, second) => {
                let first_res = self.compile_expr(first, prelude, cleanup, shadows.clone())?;
                let second_res = self.compile_expr(second, prelude, cleanup, shadows)?;
                Ok(CompileResult::Computation(format!(
                    "_compose_monads({first_res},{second_res})"
                )))
            }
        }
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn compile_bind_io_monad(
        &mut self,
        prelude: &mut String,
        cleanup: &mut String,
        mut shadows: HashMap<LValue, CompileResult>,
        var: LValue,
        val: &IRValue,
        body: &IRValue,
        captures: &[(LValue, IRType)],
    ) -> Result<CompileResult, String> {
        let IRType::IOMonad(Some(val_ret_ty)) = val.type_hint() else {
            return Err(format!(
                "Let binding using `:=` should have an `IO<T>` as the right-hand side; got `{:?}`",
                val.type_hint()
            ));
        };
        let IRType::IOMonad(body_ret_ty) = body.type_hint() else {
            return Err(format!(
                "Let binding using `:=` should have an `IO<T>` or `IO<void>` as the body; got `{:?}`",
                val.type_hint()
            ));
        };
        let val_ret_ty_name = self.write_type(val_ret_ty);
        let body_ty_name = self.write_type(body.type_hint());
        let body_ret_ty_name = body_ret_ty
            .as_ref()
            .map_or_else(|| "void".to_string(), |ty| self.write_type(ty));
        let binding_monad_ty_name = self.write_type(val.type_hint());
        let binding_monad = self.compile_expr(val, prelude, cleanup, shadows.clone())?;
        let var_actual = match binding_monad {
            CompileResult::Computation(var_actual) => {
                writeln!(prelude, "{binding_monad_ty_name} {var} = {var_actual};").unwrap();
                CompileResult::BaseValue(format!("{var}"))
            }
            var @ (CompileResult::BaseValue(_) | CompileResult::ConstFunction { .. }) => var,
        };
        shadows.insert(var, var_actual);
        let body_env: Vec<(LValue, IRType)> = captures
            .iter()
            .filter(|(lv, _)| !self.constants.contains_key(lv))
            .cloned()
            .collect();
        let full_captures: Vec<_> = body_env
            .iter()
            .cloned()
            .map(|(lv, ty)| {
                if lv == var {
                    (lv, val.type_hint().clone())
                } else {
                    (lv, ty)
                }
            })
            .collect();
        // val is an IO<val_ret_ty>
        // this function will be used as the IO monad returned from the binding
        let outer_io_monad_name = LValue::new();
        let mut body_prelude = String::new();
        let mut body_cleanup = String::new();
        let mut body_shadows = HashMap::new();

        let (captures_ty, free_captures) = self.compile_lambda_captures_structure(
            &full_captures,
            outer_io_monad_name,
            &body_ty_name,
        );
        for (cap, _) in &full_captures {
            body_shadows.insert(
                *cap,
                CompileResult::BaseValue(format!("((({captures_ty} *)captures)->{cap})")),
            );
        }
        body_shadows.insert(var, CompileResult::BaseValue(format!("{var}")));

        writeln!(
            body_prelude,
            "{val_ret_ty_name} {var} = (({captures_ty} *)captures)->{var}->f((({captures_ty} *)captures)->{var});"
        )
        .unwrap();

        // outer_io_monad_ret is the IO<T> returned by the body of the let binding
        let body_ret =
            self.compile_expr(body, &mut body_prelude, &mut body_cleanup, body_shadows)?;

        let fn_def = format!("{body_ret_ty_name} {outer_io_monad_name}({body_ty_name} captures)");

        let inner_monad_result = match body_ret {
            CompileResult::Computation(cmp) => {
                let lv = LValue::new();
                write!(
                    body_prelude,
                    "{monad_ty_name} {lv} = {cmp};",
                    monad_ty_name = self.write_type(body.type_hint())
                )
                .unwrap();
                CompileResult::BaseValue(format!("{lv}"))
            }
            result @ (CompileResult::BaseValue(..) | CompileResult::ConstFunction { .. }) => result,
        };
        match inner_monad_result {
            CompileResult::Computation(_) => panic!(),
            CompileResult::BaseValue(func) => {
                if let Some(body_ret_ty) = body_ret_ty {
                    let vrt = self.write_type(body_ret_ty);
                    writeln!(self.funcdefs, "{fn_def}{{{body_prelude}{vrt} _rv = {func}->f({func});{body_cleanup}return _rv;}}").unwrap();
                } else {
                    writeln!(
                        self.funcdefs,
                        "{fn_def}{{{body_prelude}{func}->f({func});{body_cleanup}}}"
                    )
                    .unwrap();
                }
            }
            CompileResult::ConstFunction {
                struct_name: _,
                func_name,
            } => {
                if let Some(body_ret_ty) = body_ret_ty {
                    let vrt = self.write_type(body_ret_ty);
                    writeln!(self.funcdefs, "{fn_def}{{{body_prelude}{vrt} _rv = {func_name}();{body_cleanup}return _rv;}}").unwrap();
                } else {
                    writeln!(
                        self.funcdefs,
                        "{fn_def}{{{body_prelude}{func_name}();{body_cleanup}}}"
                    )
                    .unwrap();
                }
            }
        }
        let ret_monad = LValue::new();
        writeln!(prelude, "{captures_ty} *{ret_monad} = malloc(sizeof (*{ret_monad}));\n{ret_monad}->lambda.f={outer_io_monad_name};\n{ret_monad}->lambda.d={free_captures};\n{ret_monad}->lambda.refcount=1;").unwrap();
        for (cap, _) in body_env {
            writeln!(
                prelude,
                "{ret_monad}->{cap} = {};",
                shadows
                    .get(&cap)
                    .map_or_else(|| format!("{cap}"), |t| format!("{t}"))
            )
            .unwrap();
        }

        Ok(CompileResult::BaseValue(format!(
            "(({body_ty_name}){ret_monad})"
        )))
    }

    fn compile_comparison(
        &mut self,
        prelude: &mut String,
        cleanup: &mut String,
        shadows: HashMap<LValue, CompileResult>,
        lhs: &IRValue,
        op: ComparisonOperator,
        rhs: &IRValue,
    ) -> Result<CompileResult, String> {
        let lhs = self.compile_expr(lhs, prelude, cleanup, shadows.clone())?;
        let rhs = self.compile_expr(rhs, prelude, cleanup, shadows)?;
        match op {
            ComparisonOperator::Eq => Ok(CompileResult::Computation(format!("({lhs}=={rhs})"))),
            ComparisonOperator::Ne => Ok(CompileResult::Computation(format!("({lhs}!={rhs})"))),
            ComparisonOperator::Le => Ok(CompileResult::Computation(format!("({lhs}<={rhs})"))),
            ComparisonOperator::Ge => Ok(CompileResult::Computation(format!("({lhs}>={rhs})"))),
            ComparisonOperator::Lt => Ok(CompileResult::Computation(format!("({lhs}<{rhs})"))),
            ComparisonOperator::Gt => Ok(CompileResult::Computation(format!("({lhs}>{rhs})"))),
        }
    }

    fn compile_arithmetic(
        &mut self,
        prelude: &mut String,
        cleanup: &mut String,
        shadows: HashMap<LValue, CompileResult>,
        lhs: &IRValue,
        op: ArithmeticOperator,
        rhs: &IRValue,
    ) -> Result<CompileResult, String> {
        let lhs = self.compile_expr(lhs, prelude, cleanup, shadows.clone())?;
        let rhs = self.compile_expr(rhs, prelude, cleanup, shadows)?;
        match op {
            ArithmeticOperator::Add => Ok(CompileResult::Computation(format!("({lhs}+{rhs})"))),
            ArithmeticOperator::Sub => Ok(CompileResult::Computation(format!("({lhs}-{rhs})"))),
            ArithmeticOperator::Div => Ok(CompileResult::Computation(format!("({lhs}/{rhs})"))),
            ArithmeticOperator::Mul => Ok(CompileResult::Computation(format!("({lhs}*{rhs})"))),
            ArithmeticOperator::Mod => Ok(CompileResult::Computation(format!("({lhs}%{rhs})"))),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn compile_if_else(
        &mut self,
        expr: &IRValue,
        prelude: &mut String,
        cleanup: &mut String,
        shadows: HashMap<LValue, CompileResult>,
        condition: &IRValue,
        body: &IRValue,
        else_body: &IRValue,
    ) -> Result<CompileResult, String> {
        let cond = self.compile_expr(condition, prelude, cleanup, shadows.clone())?;
        let mut body_prelude = String::new();
        let mut body_cleanup = String::new();
        let body =
            self.compile_expr(body, &mut body_prelude, &mut body_cleanup, shadows.clone())?;
        let mut else_prelude = String::new();
        let mut else_cleanup = String::new();
        let else_body =
            self.compile_expr(else_body, &mut else_prelude, &mut else_cleanup, shadows)?;
        let lvalue = LValue::new();
        write!(prelude, "{} {lvalue};if({cond}){{{body_prelude}{lvalue}={body};{body_cleanup}}}else{{{else_prelude}{lvalue}={else_body};{else_cleanup}}}", self.write_type(expr.type_hint())).unwrap();

        Ok(CompileResult::BaseValue(format!("{lvalue}")))
    }

    fn compile_function_call(
        &mut self,
        prelude: &mut String,
        cleanup: &mut String,
        shadows: HashMap<LValue, CompileResult>,
        func: &IRValue,
        args: &[IRValue],
    ) -> Result<CompileResult, String> {
        let IRType::Function {
            inputs: _,
            output: func_out,
        } = &func.type_hint()
        else {
            return Err(format!("Expected a fucntion type; got `{func:?}`"));
        };
        let args_compiled = args
            .iter()
            .map(|v| self.compile_expr(v, prelude, cleanup, shadows.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let func_lv = match self.compile_expr(func, prelude, cleanup, shadows)? {
            CompileResult::BaseValue(l) => l,
            CompileResult::Computation(s) => {
                let lv = LValue::new();
                write!(prelude, "{} {lv} = {s};", self.write_type(func.type_hint())).unwrap();
                format!("{lv}")
            }
            CompileResult::ConstFunction { func_name, .. } => {
                let mut funcall = format!("{func_name}(");
                let mut first = true;
                for arg in args_compiled {
                    if first {
                        first = false;
                    } else {
                        funcall.push(',');
                    }
                    write!(funcall, "{arg}").unwrap();
                }
                funcall.push(')');
                if func_out.is_function() {
                    let temp_var = LValue::new();
                    writeln!(cleanup, "if ({temp_var}->refcount != -1 && --{temp_var}->refcount == 0) {temp_var}->d({temp_var});").unwrap();
                    writeln!(
                        prelude,
                        "{} {temp_var} = {funcall};",
                        self.write_type(func_out)
                    )
                    .unwrap();
                    return Ok(CompileResult::BaseValue(format!("{temp_var}")));
                }
                return Ok(CompileResult::Computation(funcall));
            }
        };
        let mut funcall = format!("{func_lv}->f({func_lv},");
        let mut first = true;
        for arg in &args_compiled {
            if first {
                first = false;
            } else {
                funcall.push(',');
            }
            write!(funcall, "{arg}").unwrap();
        }
        funcall.push(')');
        // println!("{func:?}{args:?}");
        if func_out.is_function() {
            let temp_var = LValue::new();
            writeln!(cleanup, "if ({temp_var}->refcount != -1 && --{temp_var}->refcount == 0) {temp_var}->d({temp_var});").unwrap();
            writeln!(
                prelude,
                "{} {temp_var} = {funcall};",
                self.write_type(func_out)
            )
            .unwrap();
            Ok(CompileResult::BaseValue(format!("{temp_var}")))
        } else {
            Ok(CompileResult::Computation(funcall))
        }
    }

    fn compile_function(
        &mut self,
        expr: &IRValue,
        prelude: &mut String,
        cleanup: &mut String,
        params: &[LValue],
        body: &IRValue,
        captures: &[(LValue, IRType)],
    ) -> Result<CompileResult, String> {
        let IRType::Function { inputs, output } = expr.type_hint() else {
            return Err(format!(
                "Expected function type; got `{:?}`",
                expr.type_hint()
            ));
        };

        let captures: Vec<(LValue, IRType)> = captures
            .iter()
            .filter(|(c, _)| !self.constants.contains_key(c))
            .cloned()
            .collect();

        let output_ty = self.write_type(output);
        let funcname = LValue::new();
        let lambda_ty = self.write_type(expr.type_hint());

        let (captures_ty, free_function) = if captures.is_empty() {
            (None, "free".to_string())
        } else {
            let (ty, f) = self.compile_lambda_captures_structure(&captures, funcname, &lambda_ty);
            (Some(ty), f)
        };

        // define the function
        let mut funcdef = format!("{output_ty} {funcname}({lambda_ty} captures_tmp");
        for (input, param) in inputs.iter().zip(params.iter()) {
            write!(funcdef, ",{} {param}", self.write_type(input)).unwrap();
        }
        let mut func_body = String::new();
        let mut func_cleanup = String::new();
        if let Some(captures_ty) = &captures_ty {
            write!(
                func_body,
                "{captures_ty} *captures = ({captures_ty}*)captures_tmp;"
            )
            .unwrap();
        }
        let mut func_shadows = HashMap::new();
        for (cap, _) in &captures {
            func_shadows.insert(*cap, CompileResult::BaseValue(format!("(captures->{cap})")));
        }
        // compile the function body
        let body = self.compile_expr(body, &mut func_body, &mut func_cleanup, func_shadows)?;
        if output.is_function() {
            writeln!(func_body, "if ({body}->refcount != -1) {body}->refcount++;").unwrap();
        }
        writeln!(
            self.funcdefs,
            "{funcdef}){{{func_body}{func_cleanup}return {body};}}"
        )
        .unwrap();

        let lambda_lv = LValue::new();

        if captures.is_empty() {
            // create a static struct
            return Ok(self.compile_static_function(
                params,
                inputs,
                &output_ty,
                funcname,
                &lambda_ty,
                &func_body,
                &func_cleanup,
                &body,
                lambda_lv,
            ));
        }
        // allocate the lambda and captures
        if let Some(captures_ty) = captures_ty {
            writeln!(
                prelude,
                "{captures_ty} *{lambda_lv} = malloc(sizeof (*{lambda_lv}));\n{lambda_lv}->lambda.f = {funcname};\n{lambda_lv}->lambda.refcount = 1;\n{lambda_lv}->lambda.d=(void (*)({lambda_ty})){free_function};"
            )
            .unwrap();
            for (cap, _) in &captures {
                writeln!(prelude, "{lambda_lv}->{cap}={cap};").unwrap();
            }
            writeln!(cleanup, "if ({lambda_lv}->lambda.refcount != -1 && --{lambda_lv}->lambda.refcount == 0) {lambda_lv}->lambda.d(({lambda_ty}){lambda_lv});").unwrap();
            Ok(CompileResult::BaseValue(format!(
                "(({lambda_ty}){lambda_lv})"
            )))
        } else {
            writeln!(
                prelude,
                "{lambda_ty} {lambda_lv} = malloc(sizeof (*{lambda_lv}));\n{lambda_lv}->f = {funcname};\n{lambda_lv}->refcount = 1;\n{lambda_lv}->d=(void(*)({lambda_ty})){free_function};"
            )
            .unwrap();
            writeln!(
                cleanup,
                "if ({lambda_lv}->refcount != -1 && --{lambda_lv}->refcount == 0) {lambda_lv}->d({lambda_lv});"
            )
            .unwrap();
            Ok(CompileResult::BaseValue(format!("{lambda_lv}")))
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn compile_static_function(
        &mut self,
        params: &[LValue],
        inputs: &[IRType],
        output_ty: &str,
        funcname: LValue,
        lambda_ty: &str,
        func_body: &str,
        func_cleanup: &str,
        body: &CompileResult,
        lambda_lv: LValue,
    ) -> CompileResult {
        writeln!(
            self.funcdefs,
            "{lambda_ty_trunc} {lambda_lv} = {{.f={funcname}, .refcount=-1}};",
            lambda_ty_trunc = lambda_ty.replace('*', "")
        )
        .unwrap();
        let const_fn_lv = LValue::new();
        let mut const_fn_def = format!("{output_ty} {const_fn_lv}(");
        let mut first = true;
        for (input, param) in inputs.iter().zip(params.iter()) {
            if first {
                first = false;
            } else {
                const_fn_def.push(',');
            }
            write!(const_fn_def, "{} {param}", self.write_type(input)).unwrap();
        }
        writeln!(
            self.funcdefs,
            "{const_fn_def}){{{func_body}{func_cleanup}return {body};}}"
        )
        .unwrap();
        self.constants.insert(
            lambda_lv,
            CompileResult::ConstFunction {
                struct_name: lambda_lv,
                func_name: const_fn_lv,
            },
        );
        CompileResult::ConstFunction {
            struct_name: lambda_lv,
            func_name: const_fn_lv,
        }
    }

    /// Returns the type of the struct used for captures, and the function used to free that struct
    fn compile_lambda_captures_structure(
        &mut self,
        captures: &[(LValue, IRType)],
        funcname: LValue,
        lambda_ty: &str,
    ) -> (String, String) {
        let captures_ty = format!("_captures_{funcname}");

        // create the typedef for the captures type
        let mut captures_typedecl = format!(
            "typedef struct {{{lambda_ty} lambda;",
            lambda_ty = lambda_ty.replace('*', "")
        );
        for (cap, typ) in captures {
            let typ = self.write_type(typ);
            write!(captures_typedecl, "{typ} {cap};").unwrap();
        }
        writeln!(self.typedefs, "{captures_typedecl}}} {captures_ty};").unwrap();
        let free_fn_tasks: Vec<&(LValue, IRType)> =
            captures.iter().filter(|(_, ty)| ty.is_function()).collect();
        if free_fn_tasks.is_empty() {
            (captures_ty, format!("(void (*)({lambda_ty}))free"))
        } else {
            let free_fn_name = format!("_free{captures_ty}");

            let mut free_fn_decl = format!("void {free_fn_name}({captures_ty} *captures){{");
            for (cap, _) in free_fn_tasks {
                writeln!(free_fn_decl, "if (captures->{cap}->refcount != -1 && --captures->{cap}->refcount == 0) captures->{cap}->d(captures->{cap});").unwrap();
            }
            writeln!(self.typedefs, "{free_fn_decl}free(captures);}}").unwrap();

            (captures_ty, free_fn_name)
        }
    }
}
