use crate::{
    compiler::Compiler,
    interpreter::representation::{
        ArithmeticOperator, BooleanOperator, ComparisonOperator, IRExpr, IRType, IRValue, LValue,
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
}

#[derive(Clone)]
enum CompileResult {
    Source(String),
    LValue(LValue),
}

impl Display for CompileResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(s) => write!(f, "{s}"),
            Self::LValue(l) => write!(f, "{l}"),
        }
    }
}

impl Compiler for CCompiler {
    type Output = String;
    type Error = String;

    fn compile(mut self, expr: &IRValue) -> Result<Self::Output, Self::Error> {
        let mut prelude = String::new();
        let mut cleanup = String::new();
        let expr = self.compile_expr(expr, &mut prelude, &mut cleanup, HashMap::new())?;
        Ok(format!(
            "#include <stdio.h>\n#include <stdlib.h>\n{}\nint main(){{{prelude}printf(\"%i\\n\",{expr});{cleanup}}}",
            self.typedefs
        ))
    }
}

impl CCompiler {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            typedecls: HashSet::new(),
            typedefs: String::new(),
        }
    }

    fn short_type(&mut self, ty: &IRType) -> String {
        match ty {
            IRType::String => "str".to_string(),
            ty => self.write_type(ty).replace("*", ""),
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
                format!("{}*", short_name)
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
        match &expr.0 {
            IRExpr::GetLocal(lvalue) => Ok(self
                .constants
                .get(lvalue)
                .or_else(|| shadows.get(lvalue))
                .cloned()
                .unwrap_or(CompileResult::LValue(*lvalue))),
            IRExpr::SetLocal(lvalue, value, body) => {
                let val = self.compile_expr(value, prelude, cleanup, shadows.clone())?;
                match val {
                    CompileResult::LValue(_) => {
                        shadows.insert(*lvalue, val);
                    }
                    CompileResult::Source(val) => {
                        write!(prelude, "{} {lvalue}={val};", self.write_type(&value.1)).unwrap();
                    }
                }
                self.compile_expr(body, prelude, cleanup, shadows)
            }
            IRExpr::Int(i) => Ok(CompileResult::Source(format!("{i}"))),
            IRExpr::Float(f) => Ok(CompileResult::Source(format!("{f}"))),
            IRExpr::String(s) => Ok(CompileResult::Source(format!("{s:?}"))),
            IRExpr::If {
                condition,
                body,
                else_body,
            } => {
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
                write!(prelude, "{} {lvalue};if({cond}){{{body_prelude}{lvalue}={body};{body_cleanup}}}else{{{else_prelude}{lvalue}={else_body};{else_cleanup}}}", self.write_type(&expr.1)).unwrap();

                Ok(CompileResult::LValue(lvalue))
            }
            IRExpr::Arithmetic(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude, cleanup, shadows.clone())?;
                let rhs = self.compile_expr(rhs, prelude, cleanup, shadows)?;
                match op {
                    ArithmeticOperator::Add => Ok(CompileResult::Source(format!("({lhs}+{rhs})"))),
                    ArithmeticOperator::Sub => Ok(CompileResult::Source(format!("({lhs}-{rhs})"))),
                    ArithmeticOperator::Div => Ok(CompileResult::Source(format!("({lhs}/{rhs})"))),
                    ArithmeticOperator::Mul => Ok(CompileResult::Source(format!("({lhs}*{rhs})"))),
                    ArithmeticOperator::Mod => Ok(CompileResult::Source(format!("({lhs}%{rhs})"))),
                }
            }
            IRExpr::Comparison(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude, cleanup, shadows.clone())?;
                let rhs = self.compile_expr(rhs, prelude, cleanup, shadows)?;
                match op {
                    ComparisonOperator::Eq => Ok(CompileResult::Source(format!("({lhs}=={rhs})"))),
                    ComparisonOperator::Ne => Ok(CompileResult::Source(format!("({lhs}!={rhs})"))),
                    ComparisonOperator::Le => Ok(CompileResult::Source(format!("({lhs}<={rhs})"))),
                    ComparisonOperator::Ge => Ok(CompileResult::Source(format!("({lhs}>={rhs})"))),
                    ComparisonOperator::Lt => Ok(CompileResult::Source(format!("({lhs}<{rhs})"))),
                    ComparisonOperator::Gt => Ok(CompileResult::Source(format!("({lhs}>{rhs})"))),
                }
            }
            IRExpr::Boolean(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude, cleanup, shadows.clone())?;
                let rhs = self.compile_expr(rhs, prelude, cleanup, shadows)?;
                match op {
                    BooleanOperator::And => Ok(CompileResult::Source(format!("({lhs}&&{rhs})"))),
                    BooleanOperator::Or => Ok(CompileResult::Source(format!("({lhs}||{rhs})"))),
                }
            }
            IRExpr::Function {
                params,
                body,
                captures,
            } => {
                let IRType::Function { inputs, output } = &expr.1 else {
                    return Err(format!("Expected function type; got `{:?}`", expr.1));
                };

                let captures: Vec<&(LValue, IRType)> = captures
                    .iter()
                    .filter(|(c, _)| !self.constants.contains_key(c))
                    .collect();

                let output_ty = self.write_type(output);
                let funcname = LValue::new();
                let lambda_ty = self.write_type(&expr.1);

                let captures_ty = if captures.is_empty() {
                    None
                } else {
                    let captures_ty = format!("_captures_{funcname}");

                    // create the typedef for the captures type
                    let mut captures_typedecl = format!(
                        "typedef struct {{{lambda_ty} lambda;",
                        lambda_ty = lambda_ty.replace("*", "")
                    );
                    for (cap, typ) in &captures {
                        let typ = self.write_type(typ);
                        write!(captures_typedecl, "{typ} {cap};").unwrap();
                    }
                    writeln!(self.typedefs, "{captures_typedecl}}} {captures_ty};").unwrap();
                    Some(captures_ty)
                };

                // define the function
                let mut funcdef = format!(
                    "{output_ty} {funcname}({lambda_ty} {captures_name}",
                    captures_name = if captures_ty.is_some() {
                        "captures_tmp"
                    } else {
                        "captures"
                    }
                );
                for (input, param) in inputs.iter().zip(params.iter()) {
                    write!(funcdef, ",{} {param}", self.write_type(input)).unwrap();
                }
                funcdef.push_str("){");
                let mut func_cleanup = String::new();
                if let Some(captures_ty) = &captures_ty {
                    write!(
                        funcdef,
                        "{captures_ty} *captures = ({captures_ty}*)captures_tmp;"
                    )
                    .unwrap();
                }
                let mut func_shadows = HashMap::new();
                for (cap, _) in &captures {
                    func_shadows.insert(*cap, CompileResult::Source(format!("(captures->{cap})")));
                }
                // compile the function body
                let body =
                    self.compile_expr(body, &mut funcdef, &mut func_cleanup, func_shadows)?;
                if output.is_function() {
                    writeln!(funcdef, "if ({body}->refcount != -1) {body}->refcount++;").unwrap();
                }
                writeln!(self.typedefs, "{funcdef}{func_cleanup}return {body};}}").unwrap();

                let lambda_lv = LValue::new();

                if captures.is_empty() {
                    // create a static struct
                    writeln!(
                        self.typedefs,
                        "{lambda_ty_trunc} {lambda_lv} = {{.f={funcname}, .refcount=-1}};",
                        lambda_ty_trunc = lambda_ty.replace("*", "")
                    )
                    .unwrap();
                    self.constants
                        .insert(lambda_lv, CompileResult::Source(format!("(&{lambda_lv})")));
                    return Ok(CompileResult::Source(format!("(&{lambda_lv})")));
                }
                // allocate the lambda and captures
                match captures_ty {
                    Some(captures_ty) => {
                        writeln!(
                            prelude,
                            "{captures_ty} *{lambda_lv} = malloc(sizeof (*{lambda_lv}));\n{lambda_lv}->lambda.f = {funcname};\n{lambda_lv}->lambda.refcount = 1;\n{lambda_lv}->lambda.d=(void (*)({lambda_ty}))free;"
                        )
                        .unwrap();
                        for (cap, _) in &captures {
                            writeln!(prelude, "{lambda_lv}->{cap}={cap};").unwrap();
                        }
                        writeln!(cleanup, "if ({lambda_lv}->lambda.refcount != -1 && --{lambda_lv}->lambda.refcount == 0) {lambda_lv}->lambda.d(({lambda_ty}){lambda_lv});").unwrap();
                        Ok(CompileResult::Source(format!("(({lambda_ty}){lambda_lv})")))
                    }
                    None => {
                        writeln!(
                            prelude,
                            "{lambda_ty} {lambda_lv} = malloc(sizeof (*{lambda_lv}));\n{lambda_lv}->f = {funcname};\n{lambda_lv}->refcount = 1;\n{lambda_lv}->d=(void(*)({lambda_ty}))free;"
                        )
                        .unwrap();
                        writeln!(
                            cleanup,
                            "if ({lambda_lv}->refcount != -1 && --{lambda_lv}->refcount == 0) {lambda_lv}->d({lambda_lv});"
                        )
                        .unwrap();
                        Ok(CompileResult::LValue(lambda_lv))
                    }
                }
            }
            IRExpr::FunctionCall(func, args) => {
                let IRType::Function {
                    inputs: _,
                    output: func_out,
                } = &func.1
                else {
                    return Err(format!("Expected a fucntion type; got `{func:?}`"));
                };
                let args_compiled = args
                    .iter()
                    .map(|v| self.compile_expr(v, prelude, cleanup, shadows.clone()))
                    .collect::<Result<Vec<_>, _>>()?;
                let func_lv = match self.compile_expr(func, prelude, cleanup, shadows)? {
                    CompileResult::LValue(l) => l,
                    CompileResult::Source(s) => {
                        let lv = LValue::new();
                        write!(prelude, "{} {lv} = {s};", self.write_type(&func.1)).unwrap();
                        lv
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
                    funcall.push_str(&format!("{}", arg));
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
                    Ok(CompileResult::LValue(temp_var))
                } else {
                    Ok(CompileResult::Source(funcall))
                }
            }
        }
    }
}
